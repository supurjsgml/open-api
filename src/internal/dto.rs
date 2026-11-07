use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 대시보드 주간 방문자 통계 응답 DTO
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStatsDTO {
    pub label: String,
    pub value: String,
    pub description: Option<String>,
    pub trend: Option<String>,
    pub trend_direction: Option<String>,
    pub sparkline_values: Vec<i64>,
    pub days: Vec<String>,
}

/// 페이지별 누적 방문 횟수 응답 DTO
/// 각 필드명을 대문자로 시작하는 JSON 키(`Main`, `Camel` 등)로 직렬화합니다.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct PageVisitsDTO {
    #[serde(rename = "Main")]
    pub main: i64,
    #[serde(rename = "Camel")]
    pub camel: i64,
    #[serde(rename = "Translate")]
    pub translate: i64,
    #[serde(rename = "Diff")]
    pub diff: i64,
    #[serde(rename = "Jasypt")]
    pub jasypt: i64,
}

/// 방문자 히트(조회수 증가) 요청 DTO
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VisitorHitReqDTO {
    pub page_name: Option<String>,
    #[serde(default)]
    pub is_new_session: bool,
}
