use std::sync::Arc;

use axum::http::StatusCode;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};

use crate::AppState;
use crate::redocly::dto::SampleDTO;

/// Redocly 샘플 도메인 라우터를 생성합니다.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/getUrl", get(get_url))
        .route("/api/add", post(add_url))
        .route("/api/mod", put(mod_url))
        .route("/api/del", delete(del_url))
}

/// GET `/api/getUrl`
/// 서버 깨우기 및 연결 확인용 텍스트를 반환합니다.
pub async fn get_url() -> &'static str {
    "뭐에요 왜 깨워요 !!!!!"
}

/// POST `/api/add`
/// 샘플 등록 엔드포인트
pub async fn add_url(body: Option<Json<SampleDTO>>) -> StatusCode {
    let _ = body;
    StatusCode::OK
}

/// PUT `/api/mod`
/// 샘플 수정 엔드포인트
pub async fn mod_url() -> StatusCode {
    StatusCode::OK
}

/// DELETE `/api/del`
/// 샘플 삭제 엔드포인트
pub async fn del_url() -> StatusCode {
    StatusCode::OK
}
