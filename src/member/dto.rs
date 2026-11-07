use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 회원 로그인 및 로그아웃 요청 DTO
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct MemberReqDTO {
    pub id: Option<String>,
    pub pw: Option<String>,
    pub token: Option<String>,
}

/// 회원 로그인 응답 DTO
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberResDTO {
    pub id: Option<String>,
    pub pw: Option<String>,
    pub err_code: Option<String>,
    pub err_msg: Option<String>,
}
