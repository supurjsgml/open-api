pub mod common;
pub mod config;
pub mod gemini;
pub mod grafana;
pub mod internal;
pub mod junit;
pub mod kafka;
pub mod kakao;
pub mod member;
pub mod redocly;
pub mod view;

use std::collections::HashMap;
use std::sync::Arc;

use axum::http::{HeaderValue, Method};
use axum::middleware;
use axum::Router;
use tokio::sync::RwLock;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::common::core::{
    handle_no_resource_found, request_logging_middleware, security_origin_middleware,
};
use crate::common::logging::{AxiomBatchSender, LokiSender};
use crate::common::util::web_client_util;
use crate::config::AppConfig;
use crate::gemini::controller::TokenCache;

/// 애플리케이션 전역 공유 상태
/// `Arc<AppState>` 형태로 각 핸들러에 주입되어 스레드 안전하게 공유됩니다.
pub struct AppState {
    pub config: AppConfig,
    pub http_client: reqwest::Client,
    pub redis_client: Option<redis::Client>,
    pub token_cache: TokenCache,
    pub axiom_sender: AxiomBatchSender,
    pub loki_sender: LokiSender,
}

impl AppState {
    /// 환경 설정(`AppConfig`)으로부터 모든 하위 서비스와 클라이언트를 초기화합니다.
    pub fn from_config(config: AppConfig) -> Self {
        let http_client = web_client_util::create_client();

        let redis_url = config.redis.connection_url();
        let redis_client = redis::Client::open(redis_url).ok();

        let token_cache: TokenCache = Arc::new(RwLock::new(HashMap::new()));

        let axiom_sender = AxiomBatchSender::start(&config.axiom);
        let loki_sender = LokiSender::start(&config.loki);
        crate::grafana::metrics::start_otlp_metrics_exporter(http_client.clone(), &config);

        Self {
            config,
            http_client,
            redis_client,
            token_cache,
            axiom_sender,
            loki_sender,
        }
    }
}

/// 모든 API 라우트, 정적 리소스 핸들러, 보안/로깅/압축 미들웨어가 결합된 Axum 라우터를 생성합니다.
pub fn build_router(state: Arc<AppState>) -> Router {
    let allowed_origins = state.config.allowed_origins.clone();
    let cors_layer = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(
            move |origin: &HeaderValue, _request_parts| {
                origin
                    .to_str()
                    .map(|val| allowed_origins.iter().any(|o| o == val))
                    .unwrap_or(false)
            },
        ))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true);

    Router::new()
        .merge(view::controller::router())
        .merge(gemini::controller::router())
        .merge(internal::controller::router())
        .merge(kakao::controller::router())
        .merge(member::controller::router())
        .merge(common::lightsail_controller::router())
        .merge(common::controller::router())
        .merge(kafka::controller::router())
        .merge(junit::controller::router())
        .merge(redocly::controller::router())
        .merge(grafana::controller::router())
        // 정적 리소스 서빙 (`/css`, `/js`, `/images`, `/favicon.ico`)
        .nest_service("/css", ServeDir::new("resources/static/css"))
        .nest_service("/js", ServeDir::new("resources/static/js"))
        .nest_service("/images", ServeDir::new("resources/static/images"))
        .route_service(
            "/favicon.ico",
            ServeFile::new("resources/static/favicon.ico"),
        )
        // 매칭되지 않는 모든 경로에 대한 404 폴백 처리
        .fallback(handle_no_resource_found)
        // 미들웨어 체인 등록
        .layer(middleware::from_fn(request_logging_middleware))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            security_origin_middleware,
        ))
        .layer(CompressionLayer::new())
        .layer(cors_layer)
        .with_state(state)
}
