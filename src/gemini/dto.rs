use std::fmt;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 멀티모달 분석용 첨부 이미지 DTO
#[derive(Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ImageAttachment {
    #[serde(rename = "mimeType", default)]
    pub mime_type: String,
    #[serde(default)]
    pub data: String,
}

impl fmt::Debug for ImageAttachment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ImageAttachment")
            .field("mime_type", &self.mime_type)
            .field("data_len", &self.data.len())
            .finish()
    }
}

/// Gemini 요약 요청 DTO
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct GeminiReqDTO {
    pub text: Option<String>,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub images: Option<Vec<ImageAttachment>>,
}

/// Gemini API 요청 바디
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiApiRequest {
    pub contents: Vec<Content>,
}

impl GeminiApiRequest {
    /// 멀티모달 파트 목록으로 Gemini API 요청 객체를 생성하는 생성자 함수
    pub fn from_parts(parts: Vec<Part>) -> Self {
        Self {
            contents: vec![Content { parts }],
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Content {
    pub parts: Vec<Part>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(rename = "inlineData", skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<InlineData>,
}

impl Part {
    pub fn text(text: String) -> Self {
        Self {
            text: Some(text),
            inline_data: None,
        }
    }

    pub fn inline_data(mime_type: String, data: String) -> Self {
        Self {
            text: None,
            inline_data: Some(InlineData { mime_type, data }),
        }
    }
}

/// Google Gemini API 인라인 이미지 데이터 구조체
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct InlineData {
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    pub data: String,
}

impl fmt::Debug for InlineData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InlineData")
            .field("mime_type", &self.mime_type)
            .field("data_len", &self.data.len())
            .finish()
    }
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
    #[serde(rename = "name")]
    pub display_name: String,
    /// 모델 설명 (한국어 친화적 설명)
    #[serde(rename = "desc")]
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
