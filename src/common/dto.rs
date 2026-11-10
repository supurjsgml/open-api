use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::core::get_current_uuid;
use crate::common::enums::MessageEnum;
use crate::common::util::messages_utils;

/// 공통 페이징 계산 및 SQL LIMIT 구문 생성을 담당하는 구조체
/// #[serde(rename_all = "camelCase")]는 snake_case 필드명을 JSON 직렬화 시 camelCase로 자동 변환합니다.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageDTO {
    pub page_no: i32,
    pub page_size: i32,
    pub total_records: Option<i32>,
    /// #[serde(skip)] 어트리뷰트가 붙은 필드는 JSON 직렬화/역직렬화 대상에서 제외됩니다.
    #[serde(skip)]
    pub str_paging: String,
}

impl Default for PageDTO {
    fn default() -> Self {
        Self::new(None, None, None)
    }
}

impl PageDTO {
    pub fn new(page_no: Option<i32>, page_size: Option<i32>, total_records: Option<i32>) -> Self {
        Self {
            page_no: page_no.unwrap_or(1),
            page_size: page_size.unwrap_or(10),
            total_records,
            str_paging: String::new(),
        }
    }

    pub fn get_start_num(&self, p: Option<i32>) -> i32 {
        match p {
            None => {
                if self.page_no == 0 {
                    0
                } else {
                    (self.page_no - 1) * self.page_size
                }
            }
            Some(p_val) => ((p_val - 1) * self.page_size) + 1,
        }
    }

    pub fn gen_grid(&mut self) {
        self.str_paging = self.gen_str_paging();
    }

    pub fn gen_str_paging(&self) -> String {
        let per_page = if self.page_size == -1 || self.page_size >= 500 {
            500
        } else {
            self.page_size
        };
        let start_num = if self.page_no == 0 {
            0
        } else {
            (self.page_no - 1) * self.page_size
        };

        if start_num >= 0 && per_page > 0 {
            format!("LIMIT {}, {}", start_num, per_page)
        } else {
            String::new()
        }
    }
}

/// API 성공 응답 헤더 구조체
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HeaderSuccess {
    pub uuid: Option<String>,
    pub code: String,
    pub message: String,
}

impl Default for HeaderSuccess {
    /// 생성 시 현재 비동기 태스크 컨텍스트에 저장된 요청 UUID를 자동으로 가져옵니다.
    fn default() -> Self {
        Self {
            uuid: get_current_uuid(),
            code: "SUCCESS".to_string(),
            message: messages_utils::get_message(MessageEnum::SuccessPrcs.code()),
        }
    }
}

impl HeaderSuccess {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            uuid: get_current_uuid(),
            code: code.into(),
            message: message.into(),
        }
    }
}

/// API 에러 응답 헤더 구조체
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HeaderError {
    pub uuid: Option<String>,
    pub code: String,
    pub message: Option<String>,
}

impl Default for HeaderError {
    fn default() -> Self {
        Self {
            uuid: get_current_uuid(),
            code: "BAD_REQUEST".to_string(),
            message: None,
        }
    }
}

impl HeaderError {
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            uuid: get_current_uuid(),
            code: "BAD_REQUEST".to_string(),
            message: Some(message.into()),
        }
    }

    pub fn with_code_message(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            uuid: get_current_uuid(),
            code: code.into(),
            message: Some(message.into()),
        }
    }
}

/// API 문서 표준 성공 응답 래퍼
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiDocumentSuccess<T> {
    pub header: Option<HeaderSuccess>,
    pub data: Option<T>,
    pub page: Option<PageDTO>,
}

/// 전역 에러 처리기가 반환하는 표준 에러 응답 래퍼
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiDocumentError<T> {
    pub header: Option<HeaderError>,
    pub data: Option<T>,
    pub page: Option<PageDTO>,
}

impl<T> Default for ApiDocumentError<T> {
    fn default() -> Self {
        Self {
            header: Some(HeaderError::default()),
            data: None,
            page: None,
        }
    }
}

impl<T> ApiDocumentError<T> {
    pub fn from_message(message: impl Into<String>) -> Self {
        Self {
            header: Some(HeaderError::with_message(message)),
            data: None,
            page: None,
        }
    }
}

/// 공통 요청 바디 래퍼
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiBodyRequest<T> {
    pub data: T,
    pub page: Option<PageDTO>,
}

/// 컨트롤러 공통 응답 바디 래퍼
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiBodyResponse<T> {
    pub success: bool,
    pub header: Option<HeaderSuccess>,
    pub message: String,
    /// 값이 None이면 JSON 출력 시 page 필드 자체를 생략합니다.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<PageDTO>,
    pub data: Option<T>,
}

impl<T> Default for ApiBodyResponse<T> {
    fn default() -> Self {
        Self {
            success: false,
            header: None,
            message: "통신에 실패하였습니다.".to_string(),
            page: None,
            data: None,
        }
    }
}

/// 보일러플레이트 파일 생성 요청 DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileGenReqDTO {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub file_nm: Option<String>,
    #[serde(default = "default_method_nm")]
    pub method_nm: String,
    pub camel_str: Option<String>,
}

fn default_method_nm() -> String {
    "methodNm".to_string()
}

impl FileGenReqDTO {
    /// 필수 입력 필드인 camel_str 누락 여부를 검증
    pub fn validate(&self) -> Result<(), String> {
        if self.camel_str.is_none() {
            return Err(format!(
                "camelStr : {}",
                messages_utils::get_message(MessageEnum::BadRequest.code())
            ));
        }
        Ok(())
    }
}

/// 기본 HTTP 결과 코드/메시지 DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseDto {
    pub result_code: String,
    pub result_message: String,
}

impl Default for HttpResponseDto {
    fn default() -> Self {
        Self {
            result_code: "200".to_string(),
            result_message: "Success".to_string(),
        }
    }
}

/// 결과 코드, 메시지, 데이터를 함께 담는 응답 DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiDataResponse<T> {
    pub result_code: String,
    pub result_message: String,
    pub data: Option<T>,
}

impl<T> Default for ApiDataResponse<T> {
    fn default() -> Self {
        Self {
            result_code: "200".to_string(),
            result_message: "Success".to_string(),
            data: None,
        }
    }
}

impl<T> ApiDataResponse<T> {
    pub fn ok(
        data: Option<T>,
        result_code: impl Into<String>,
        result_message: impl Into<String>,
    ) -> Self {
        Self {
            result_code: result_code.into(),
            result_message: result_message.into(),
            data,
        }
    }

    pub fn fail() -> Self {
        Self {
            result_code: "500".to_string(),
            result_message: "Internal Server Error".to_string(),
            data: None,
        }
    }

    pub fn validation_fail(result_message: impl Into<String>, data: Option<T>) -> Self {
        Self {
            result_code: "422".to_string(),
            result_message: result_message.into(),
            data,
        }
    }
}
