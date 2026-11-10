use tracing::error;

use crate::AppState;
use crate::common::util::web_client_util;
use crate::member::dto::{MemberReqDTO, MemberResDTO};

/// 배치 서버의 Quartz 로그인 엔드포인트(`/api/quartz/login`)를 호출합니다.
pub async fn job_korea_login(state: &AppState, req_dto: &MemberReqDTO) -> Option<MemberResDTO> {
    let url = format!("{}/api/quartz/login", state.config.rest_api.batch.base_url);
    match web_client_util::post_async::<MemberReqDTO, MemberResDTO>(
        &state.http_client,
        &url,
        req_dto,
    )
    .await
    {
        Ok(res) => Some(res),
        Err(e) => {
            error!("Batch login request failed: {}", e);
            None
        }
    }
}

/// 배치 서버의 Quartz 일시정지 엔드포인트(`/api/quartz/pause`)를 비동기로 호출하여 로그아웃 처리합니다.
pub async fn job_korea_logout(state: &AppState, req_dto: &MemberReqDTO) {
    let id = req_dto.id.clone().unwrap_or_default();
    let url = format!(
        "{}/api/quartz/pause?triggerName={}&groupName=userGroup",
        state.config.rest_api.batch.base_url,
        urlencoding::encode(&id)
    );
    let http_client = state.http_client.clone();

    // 비동기 백그라운드 태스크로 전송하여 즉시 응답 반환
    tokio::spawn(async move {
        if let Err(e) = web_client_util::post_empty_async(&http_client, &url).await {
            error!("Batch logout pause request failed: {}", e);
        }
    });
}
