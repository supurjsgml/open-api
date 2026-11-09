use std::sync::Arc;

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::Value;

use crate::AppState;
use crate::common::dto::ApiBodyResponse;
use crate::common::util::api_res_util::ApiResUtil;
use crate::member::dto::{MemberReqDTO, MemberResDTO};

/// 회원(잡코리아) 연동 도메인 라우터를 생성합니다.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/member/jobKorea/login", post(job_korea_login))
        .route("/member/jobKorea/logout", post(job_korea_logout))
}

/// POST `/member/jobKorea/login`
/// 잡코리아 계정 로그인 및 자동 이력서 갱신 스케줄러 등록을 요청합니다.
pub async fn job_korea_login(
    State(state): State<Arc<AppState>>,
    Json(req_dto): Json<MemberReqDTO>,
) -> Json<ApiBodyResponse<MemberResDTO>> {
    let res_opt = state.member_service.job_korea_login(&req_dto).await;

    match res_opt {
        None => Json(ApiResUtil::failed()),
        Some(res) => {
            if let Some(ref err_code) = res.err_code {
                if !err_code.trim().is_empty() {
                    let msg = res.err_msg.clone().unwrap_or_default();
                    return Json(ApiResUtil::failed_message(msg));
                }
            }
            Json(ApiResUtil::success_data_message(res, "SUCCESS"))
        }
    }
}

/// POST `/member/jobKorea/logout`
/// 등록된 잡코리아 스케줄러 트리거를 일시 중지합니다.
pub async fn job_korea_logout(
    State(state): State<Arc<AppState>>,
    Json(req_dto): Json<MemberReqDTO>,
) -> Json<ApiBodyResponse<Value>> {
    state.member_service.job_korea_logout(&req_dto).await;
    Json(ApiResUtil::success())
}
