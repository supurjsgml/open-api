use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 카카오톡 메시지 전송 요청 DTO
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct KakaoSendReqDTO {
    pub msg: Option<String>,
}

/// 카카오 토큰 갱신 응답 DTO
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KakaoTokenResDTO {
    pub access_token: Option<String>,
    pub new_refresh_token: Option<String>,
}
