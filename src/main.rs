use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use open_api::common::logging::RemoteLogLayer;
use open_api::config::AppConfig;
use open_api::{build_router, AppState};

#[tokio::main]
async fn main() {
    let config = AppConfig::from_env();

    // 로그 레벨 설정 (local/dev는 debug, prod는 info 기본)
    let default_filter = if config.profile == "prod" {
        "info,open_api=info"
    } else {
        "info,open_api=debug"
    };

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    let port = config.port;
    let profile = config.profile.clone();

    let state = Arc::new(AppState::from_config(config));

    // 콘솔 출력 레이어와 외부 로그 수집기(Axiom, Loki) 전송 레이어를 함께 등록
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(true);
    let remote_layer = RemoteLogLayer::new(state.axiom_sender.clone(), state.loki_sender.clone());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(remote_layer)
        .init();

    let app = build_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!(
        "Starting open-api server on http://{} (profile: {})",
        addr, profile
    );

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind TCP listener");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");
}

/// 프로세스 종료 시그널(Ctrl+C)을 감지하여 진행 중인 요청을 안전하게 마무리(Graceful Shutdown)합니다.
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("Shutdown signal received, stopping server gracefully...");
}
