use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::AppState;
use crate::grafana::metrics::{
    ThreadStatusDto, collect_process_threads, resolve_pod_name, uptime_seconds,
};

/// 모니터링 및 헬스체크 라우터를 생성합니다.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/actuator/threads", get(get_all_threads))
        .route("/actuator/health", get(health))
        .route("/actuator/info", get(info))
        .route("/actuator/prometheus", get(prometheus_metrics))
}

/// 누적 HTTP 요청 카운터 (Prometheus 메트릭 노출용)
static HTTP_REQUESTS_TOTAL: AtomicU64 = AtomicU64::new(0);

/// 요청 카운터를 1 증가시킵니다.
pub fn increment_http_requests() {
    HTTP_REQUESTS_TOTAL.fetch_add(1, Ordering::Relaxed);
}

/// GET `/actuator/threads`
/// 현재 프로세스 및 워커 스레드의 상태 목록을 반환합니다.
pub async fn get_all_threads() -> Json<Vec<ThreadStatusDto>> {
    let pod_name = resolve_pod_name(Some("unknown-pod"));
    Json(collect_process_threads(&pod_name))
}

/// GET `/actuator/health`
/// 컨테이너 상태 프로브(Startup, Liveness, Readiness)를 위한 헬스체크 응답을 반환합니다.
pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "UP"
    }))
}

/// GET `/actuator/info`
/// 애플리케이션 기본 메타 정보를 반환합니다.
pub async fn info() -> Json<Value> {
    Json(json!({
        "app": {
            "name": "rest-api"
        }
    }))
}

/// GET `/actuator/prometheus`
/// Grafana 및 Prometheus 수집기를 위한 메트릭 데이터를 텍스트 포맷으로 반환합니다.
pub async fn prometheus_metrics() -> Response {
    let pod = resolve_pod_name(Some("unknown-pod"));
    let app_name = "rest-api";
    let uptime_secs = uptime_seconds();
    let total_reqs = HTTP_REQUESTS_TOTAL.load(Ordering::Relaxed);
    let threads = collect_process_threads(&pod);

    let body = format!(
        "# HELP process_uptime_seconds The uptime of the application\n\
         # TYPE process_uptime_seconds gauge\n\
         process_uptime_seconds{{application=\"{app}\",instance=\"{pod}\",pod=\"{pod}\"}} {uptime:.3}\n\
         # HELP http_server_requests_seconds_count Total number of HTTP requests\n\
         # TYPE http_server_requests_seconds_count counter\n\
         http_server_requests_seconds_count{{application=\"{app}\",instance=\"{pod}\",pod=\"{pod}\"}} {reqs}\n\
         # HELP jvm_threads_live_threads The current number of live threads\n\
         # TYPE jvm_threads_live_threads gauge\n\
         jvm_threads_live_threads{{application=\"{app}\",instance=\"{pod}\",pod=\"{pod}\"}} {threads}\n",
        app = app_name,
        pod = pod,
        uptime = uptime_secs,
        reqs = total_reqs,
        threads = threads.len()
    );

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )
        .body(axum::body::Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
