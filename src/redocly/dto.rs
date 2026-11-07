use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 샘플 API 요청 DTO
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SampleDTO {
    /// 테스트용 샘플 문자열
    pub sample: Option<String>,
}
