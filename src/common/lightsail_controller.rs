use std::{env, path::Path, process::Command, sync::Arc};

use axum::{Json, Router, http::StatusCode, routing::get};
use tracing::{error, info};

use crate::AppState;

/// 클라우드 컨테이너 원격 재시작 라우터를 생성합니다.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/lightsail/restart", get(restart_scheduler))
        .route("/api/lightsail/restart-api", get(restart_rest_api))
}

/// GET /api/lightsail/restart - 리눅스 서버의 PM2 프로세스(scheduler-job) 재시작 핸들러
pub async fn restart_scheduler() -> (StatusCode, Json<&'static str>) {
    info!("라이트세일 PM2 프로세스(scheduler-job) 재시작 요청 수신");
    match Command::new("/bin/sh")
        .arg("-c")
        .arg("pm2 restart scheduler-job")
        .spawn()
    {
        Ok(_) => info!("PM2 재시작 명령 전송 완료"),
        Err(e) => error!("PM2 프로세스 재시작 실패: {}", e),
    }

    (StatusCode::OK, Json("OK"))
}

/// GET /api/lightsail/restart-api - 무중단 배포 스크립트(deploy.sh) 비동기 실행 핸들러
pub async fn restart_rest_api() -> (StatusCode, Json<&'static str>) {
    info!("라이트세일 API 서버 무중단 재기동 요청 수신");

    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c")
        .arg("/home/ubuntu/rest-api/deploy.sh")
        .current_dir(Path::new("/home/ubuntu/rest-api"));

    // 현재 프로세스의 모든 환경변수를 자식 프로세스에 그대로 전달하고 PORT_A, PORT_B 기본값 보장
    for (k, v) in env::vars() {
        cmd.env(k, v);
    }
    if env::var("PORT_A").is_err() {
        cmd.env("PORT_A", "8080");
    }
    if env::var("PORT_B").is_err() {
        cmd.env("PORT_B", "9080");
    }

    match cmd.spawn() {
        Ok(_) => info!("무중단 재기동 스크립트(deploy.sh) 실행 시작 완료"),
        Err(e) => error!("무중단 재기동 스크립트 실행 실패: {}", e),
    }

    (StatusCode::OK, Json("OK"))
}
