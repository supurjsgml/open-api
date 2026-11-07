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
use axum::routing::{delete, get, post, put};
use axum::Router;
use tokio::sync::RwLock;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::common::core::{
    handle_no_resource_found, request_logging_middleware, security_origin_middleware,
};
use crate::common::logging::{AxiomBatchSender, LokiSender};
use crate::common::service::CommonService;
use crate::common::util::web_client_util::WebClientUtil;
use crate::config::AppConfig;
use crate::gemini::controller::TokenCache;
use crate::gemini::rate_limit::RateLimitService;
use crate::gemini::service::GeminiService;
use crate::internal::service::InternalService;
use crate::kafka::service::KafkaProducerService;
use crate::kakao::service::KakaoService;
use crate::member::service::MemberService;

/// 애플리케이션 전역 공유 상태
/// `Arc<AppState>` 형태로 각 핸들러에 주입되어 스레드 안전하게 공유됩니다.
pub struct AppState {
    pub config: AppConfig,
    pub http_client: reqwest::Client,
    pub web_client: WebClientUtil,
    pub redis_client: Option<redis::Client>,
    pub token_cache: TokenCache,
    pub rate_limit_service: RateLimitService,
    pub gemini_service: GeminiService,
    pub internal_service: InternalService,
    pub kakao_service: KakaoService,
    pub member_service: MemberService,
    pub common_service: CommonService,
    pub kafka_producer: KafkaProducerService,
    pub axiom_sender: AxiomBatchSender,
    pub loki_sender: LokiSender,
}

impl AppState {
    /// 환경 설정(`AppConfig`)으로부터 모든 하위 서비스와 클라이언트를 초기화합니다.
    pub fn from_config(config: AppConfig) -> Self {
        let web_client = WebClientUtil::new();
        let http_client = web_client.raw_client().clone();

        let redis_url = config.redis.connection_url();
        let redis_client = redis::Client::open(redis_url).ok();

        let token_cache: TokenCache = Arc::new(RwLock::new(HashMap::new()));
        let rate_limit_service = RateLimitService::new(redis_client.clone(), config.clone());
        let gemini_service = GeminiService::new(http_client.clone(), config.clone());
        let internal_service = InternalService::new(redis_client.clone());
        let kakao_service =
            KakaoService::new(http_client.clone(), redis_client.clone(), config.clone());
        let member_service = MemberService::new(web_client.clone(), config.clone());
        let common_service = CommonService::new(config.common_constants.clone());
        let kafka_producer = KafkaProducerService::new();

        let axiom_sender = AxiomBatchSender::start(&config.axiom);
        let loki_sender = LokiSender::start(&config.loki);

        Self {
            config,
            http_client,
            web_client,
            redis_client,
            token_cache,
            rate_limit_service,
            gemini_service,
            internal_service,
            kakao_service,
            member_service,
            common_service,
            kafka_producer,
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
        // 문서 및 뷰 라우트
        .route("/", get(view::controller::root_redirect))
        .route("/redoc", get(view::controller::redoc_page))
        .route("/jsp", get(view::controller::jsp_page))
        .route("/swagger-ui", get(view::controller::swagger_ui_page))
        .route(
            "/swagger-ui/index.html",
            get(view::controller::swagger_ui_page),
        )
        .route("/v3/api-docs", get(view::controller::openapi_json))
        // Gemini AI 라우트
        .route("/api/gemini/summarize", post(gemini::controller::summarize))
        .route("/api/gemini/models", get(gemini::controller::get_models))
        // 대시보드 및 방문자 통계 라우트
        .route(
            "/api/front/dashboardStats",
            get(internal::controller::get_dashboard_stats),
        )
        .route(
            "/api/front/pageStats",
            get(internal::controller::get_page_stats),
        )
        .route("/api/front/hit", post(internal::controller::record_hit))
        // 카카오 연동 라우트
        .route(
            "/api/kakao/refresh-token",
            post(kakao::controller::refresh_kakao_token),
        )
        .route(
            "/api/kakao/send",
            post(kakao::controller::send_kakao_message),
        )
        // 회원(잡코리아) 연동 라우트
        .route(
            "/member/jobKorea/login",
            post(member::controller::job_korea_login),
        )
        .route(
            "/member/jobKorea/logout",
            post(member::controller::job_korea_logout),
        )
        // 클라우드 컨테이너 원격 재시작 라우트
        .route(
            "/api/lightsail/restart",
            get(common::lightsail_controller::restart_scheduler),
        )
        .route(
            "/api/lightsail/restart-api",
            get(common::lightsail_controller::restart_rest_api),
        )
        // 공통 템플릿 생성 및 조회 라우트
        .route("/com/genFile", post(common::controller::gen_file))
        .route("/com/getFileInfo", post(common::controller::get_file_info))
        // 메시지 브로커 라우트
        .route("/kafka/send", get(kafka::controller::send_msg))
        // 대용량 DTO 변환 테스트 라우트
        .route("/test/commonUtil", get(junit::controller::test_common_util))
        .route("/test/mapStruct", get(junit::controller::test_map_struct))
        // Redocly 샘플 라우트
        .route("/api/getUrl", get(redocly::controller::get_url))
        .route("/api/add", post(redocly::controller::add_url))
        .route("/api/mod", put(redocly::controller::mod_url))
        .route("/api/del", delete(redocly::controller::del_url))
        // 모니터링 및 헬스체크 라우트
        .route("/actuator/threads", get(grafana::controller::get_all_threads))
        .route("/actuator/health", get(grafana::controller::health))
        .route("/actuator/info", get(grafana::controller::info))
        .route(
            "/actuator/prometheus",
            get(grafana::controller::prometheus_metrics),
        )
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
