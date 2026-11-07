use std::sync::Arc;

use axum::{
    Json,
    body::Body,
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::Value;
use tracing::{error, info, warn};
use url::Url;
use uuid::Uuid;

use crate::{
    AppState,
    common::{
        dto::{ApiDataResponse, ApiDocumentError},
        enums::IpExtractor,
        exception::ApiDataResponseSuccessException,
    },
    grafana::controller::increment_http_requests,
};

// 비동기 태스크별로 독립적인 데이터를 유지하기 위한 태스크 로컬 저장소입니다.
// 비동기 런타임에서는 하나의 스레드가 여러 요청을 번갈아 처리하므로 태스크 단위로 UUID를 격리합니다.
tokio::task_local! {
    pub static CURRENT_REQUEST_UUID: String;
}

/// 현재 처리 중인 HTTP 요청의 고유 UUID를 반환
pub fn get_current_uuid() -> Option<String> {
    CURRENT_REQUEST_UUID.try_with(|u| u.clone()).ok()
}

/// 헬스체크 프로브, 메트릭 수집, 정적 리소스 등 컨트롤러 외 프레임워크 경로인지 판별
fn is_excluded_from_request_log(path: &str) -> bool {
    path == "/actuator/health"
        || path == "/actuator/info"
        || path == "/actuator/prometheus"
        || path == "/favicon.ico"
        || path.starts_with("/css/")
        || path.starts_with("/js/")
        || path.starts_with("/images/")
        || path.starts_with("/swagger-ui")
        || path.starts_with("/v3/api-docs")
}

/// 모든 요청 진입 시 고유 UUID를 발급하고 컨트롤러 요청 메타데이터(Origin, Referer, IP 등)를 기록하는 미들웨어
pub async fn request_logging_middleware(req: Request<Body>, next: Next) -> Response {
    increment_http_requests();

    let uuid = Uuid::new_v4().to_string();
    let path = req.uri().path();

    if !is_excluded_from_request_log(path) {
        let headers = req.headers();

        let origin = headers
            .get(header::ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let referer = headers
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Referer 헤더가 있으면 URL을 파싱하여 호스트명(ClientDomain) 추출
        let client_domain = referer.as_deref().and_then(|r| match Url::parse(r) {
            Ok(parsed) => parsed.host_str().map(|h| h.to_string()),
            Err(e) => {
                error!("RequestLoggingAspect logRequest ERROR : {}", e);
                None
            }
        });

        let ip = IpExtractor::extract(headers, None);
        let request_url = req.uri().to_string();
        let method = req.method().to_string();

        info!(
            "Request UUID : {} - Origin : {:?} - RequestURL : {} - ClientDomain : {:?} - IP : {} - Package Info : {} {}",
            uuid, origin, request_url, client_domain, ip, method, request_url
        );
    }

    // 생성된 UUID를 현재 요청의 비동기 실행 범위(Scope)에 바인딩하여 하위 핸들러에서 꺼내 쓸 수 있게 함
    CURRENT_REQUEST_UUID.scope(uuid, next.run(req)).await
}

/// 허용되지 않은 Origin 헤더 차단 및 비공개 경로(/api/private/**) 접근을 제어하는 보안 미들웨어
pub async fn security_origin_middleware(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let request_uri = req.uri().path();
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok());

    let is_swagger = request_uri.starts_with("/swagger-ui")
        || request_uri.starts_with("/v3/api-docs")
        || request_uri.starts_with("/swagger-resources");

    // API 문서 경로가 아닌데 허용 목록에 없는 Origin 헤더가 들어오면 403 Forbidden 반환
    if !is_swagger {
        if let Some(origin_val) = origin {
            if !state.config.allowed_origins.iter().any(|o| o == origin_val) {
                return StatusCode::FORBIDDEN.into_response();
            }
        }
    }

    // 비공개 경로(/api/private/**)는 외부 접근을 차단
    if request_uri.starts_with("/api/private/") {
        return StatusCode::FORBIDDEN.into_response();
    }

    next.run(req).await
}

/// 등록되지 않은 라우트(404) 요청 시 호출되는 폴백 핸들러
pub async fn handle_no_resource_found() -> impl IntoResponse {
    warn!("Resource Not Found (404)");
    let er: ApiDocumentError<Value> = ApiDocumentError::from_message("존재하지 않는 리소스입니다.");
    (StatusCode::OK, Json(er))
}

/// 애플리케이션 전역 에러 열거형
/// 핸들러 함수에서 Result<T, AppError>를 반환하면 아래 IntoResponse 구현을 통해 자동으로 표준 에러 JSON 응답으로 변환됩니다.
pub enum AppError {
    MethodArgumentNotValid(String),
    RollbackSuccess(ApiDataResponseSuccessException),
    NoResourceFound(String),
    ServerException {
        class_name: String,
        message: String,
        state: Option<Arc<AppState>>,
    },
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::MethodArgumentNotValid(msg) => {
                let er: ApiDocumentError<Value> = ApiDocumentError::from_message(msg);
                (StatusCode::OK, Json(er)).into_response()
            }
            AppError::RollbackSuccess(ex) => {
                warn!(
                    "RollbackSuccessException - Code : {}, Msg : {} Data : {:?}",
                    ex.code, ex.message, ex.data
                );
                let body = ApiDataResponse::ok(ex.data, ex.code, ex.message);
                (StatusCode::OK, Json(body)).into_response()
            }
            AppError::NoResourceFound(msg) => {
                warn!("Resource Not Found (404) : {}", msg);
                let er: ApiDocumentError<Value> =
                    ApiDocumentError::from_message("존재하지 않는 리소스입니다.");
                (StatusCode::OK, Json(er)).into_response()
            }
            AppError::ServerException {
                class_name,
                message,
                state,
            } => {
                error!("Server Exception ERROR : {}", message);
                // 서버 내부 에러 발생 시 카카오톡으로 에러 알림을 비동기 전송 (최대 180자)
                if let Some(app_state) = state {
                    let error_msg = format!("[restApi 서버 에러] {} : {}", class_name, message);
                    let truncated: String = error_msg.chars().take(180).collect();
                    let kakao_service = app_state.kakao_service.clone();
                    tokio::spawn(async move {
                        kakao_service.send_kakao(&truncated).await;
                    });
                }

                let er: ApiDocumentError<Value> = ApiDocumentError::from_message(
                    "서버 에러가 발생하였습니다. 관리자에게 문의해 주세요.",
                );
                (StatusCode::OK, Json(er)).into_response()
            }
        }
    }
}
