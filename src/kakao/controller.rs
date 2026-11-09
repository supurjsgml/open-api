use std::sync::Arc;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;
use tracing::info;

use crate::AppState;
use crate::common::core::AppError;
use crate::common::dto::ApiBodyResponse;
use crate::common::util::api_res_util::ApiResUtil;
use crate::kakao::dto::{KakaoSendReqDTO, KakaoTokenResDTO};

/// 카카오 연동 도메인 라우터를 생성합니다.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/kakao/refresh-token", post(refresh_kakao_token))
        .route("/api/kakao/send", post(send_kakao_message))
}

/// POST `/api/kakao/refresh-token`
/// 카카오 Access Token 및 Refresh Token을 수동으로 갱신하고 Redis에 저장합니다.
pub async fn refresh_kakao_token(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiBodyResponse<KakaoTokenResDTO>>, AppError> {
    info!("외부 카카오 토큰 갱신 요청 수신 (Scheduler-Job)");
    match state.kakao_service.refresh_kakao_token().await {
        Ok(res) => Ok(Json(ApiResUtil::success_data(res))),
        Err(msg) => Err(AppError::ServerException {
            class_name: "RuntimeException".to_string(),
            message: msg,
            state: Some(state),
        }),
    }
}

/// POST `/api/kakao/send`
/// 외부 시스템(배치 서버 등)에서 전달받은 에러 메시지를 카카오톡으로 발송합니다.
pub async fn send_kakao_message(
    State(state): State<Arc<AppState>>,
    Json(req): Json<KakaoSendReqDTO>,
) -> Json<ApiBodyResponse<Value>> {
    let msg = req.msg.unwrap_or_default();
    info!("외부 카카오 메시지 전송 요청 수신 (Scheduler-Job): {}", msg);
    state.kakao_service.send_kakao_external(&msg).await;
    Json(ApiResUtil::success())
}
