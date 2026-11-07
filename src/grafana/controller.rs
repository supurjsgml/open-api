use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;
use std::time::Instant;

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};

use crate::grafana::metrics::resolve_pod_name;

/// 애플리케이션 시작 시각 (업타임 측정용)
static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);
/// 누적 HTTP 요청 카운터 (Prometheus 메트릭 노출용)
static HTTP_REQUESTS_TOTAL: AtomicU64 = AtomicU64::new(0);

/// 요청 카운터를 1 증가시킵니다.
pub fn increment_http_requests() {
    HTTP_REQUESTS_TOTAL.fetch_add(1, Ordering::Relaxed);
}

/// 스레드 상태 응답 구조체
#[derive(Debug, Clone, Serialize)]
pub struct ThreadStatusDto {
    pub pod: String,
    pub name: String,
    pub state: String,
    pub daemon: bool,
}

/// GET `/actuator/threads`
/// 현재 프로세스 및 Tokio 워커 스레드의 상태 목록을 반환합니다.
pub async fn get_all_threads() -> Json<Vec<ThreadStatusDto>> {
    let pod_name = resolve_pod_name(Some("unknown-pod"));
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let mut threads = Vec::with_capacity(worker_count + 1);
    threads.push(ThreadStatusDto {
        pod: pod_name.clone(),
        name: "main".to_string(),
        state: "RUNNABLE".to_string(),
        daemon: false,
    });

    for idx in 0..worker_count {
        threads.push(ThreadStatusDto {
            pod: pod_name.clone(),
            name: format!("tokio-runtime-worker-{}", idx),
            state: "RUNNABLE".to_string(),
            daemon: true,
        });
    }

    Json(threads)
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
    let uptime_secs = START_TIME.elapsed().as_secs_f64();
    let total_reqs = HTTP_REQUESTS_TOTAL.load(Ordering::Relaxed);
    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

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
        threads = worker_count + 1
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
