use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Gemini 요약 요청 DTO
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct GeminiReqDTO {
    pub text: Option<String>,
    pub prompt: Option<String>,
    pub model: Option<String>,
}

/// Gemini API 요청 바디
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiApiRequest {
    pub contents: Vec<Content>,
}

impl GeminiApiRequest {
    /// 단일 텍스트 프롬프트로 Gemini API 요청 객체를 생성하는 생성자 함수
    pub fn from_text(text: &str) -> Self {
        Self {
            contents: vec![Content {
                parts: vec![Part {
                    text: text.to_string(),
                }],
            }],
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Content {
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Part {
    pub text: String,
}

/// Gemini API 응답 바디
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiApiResponse {
    pub candidates: Option<Vec<Candidate>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Candidate {
    pub content: Option<ResContent>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResContent {
    pub parts: Option<Vec<ResPart>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResPart {
    pub text: Option<String>,
}

/// Gemini 사용 가능 모델 조회 응답 DTO
/// `#[serde(rename_all = "camelCase")]`를 통해 Rust의 snake_case 필드명을 JSON의 camelCase로 자동 매핑합니다.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GeminiModelResDTO {
    /// 모델 식별자 (예: gemini-3.7-flash)
    pub id: String,
    /// 모델 표시 이름 (예: Gemini 3.7 Flash)
    pub display_name: String,
    /// 모델 설명 (한국어 친화적 설명)
    pub description: String,
    /// 기본 선택 여부
    pub is_default: bool,
}

/// Google Gemini Models API 원본 응답 래퍼
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiModelListResponse {
    pub models: Option<Vec<GeminiModelInfo>>,
}

/// Google Gemini Models API 개별 모델 상세 정보
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiModelInfo {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub supported_generation_methods: Option<Vec<String>>,
}
