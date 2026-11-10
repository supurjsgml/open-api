use crate::common::{
    dto::{ApiBodyResponse, HeaderSuccess, PageDTO},
    enums::{ApiCommEnum, MessageEnum},
    util::messages_utils,
};

/// 데이터 없이 기본 성공 응답 생성
pub fn success<T>() -> ApiBodyResponse<T> {
    generate_success(None, None, None, ApiCommEnum::Success.message())
}

/// 커스텀 메시지만 포함한 성공 응답 생성
pub fn success_message<T>(message: impl Into<String>) -> ApiBodyResponse<T> {
    generate_success(None, None, None, message)
}

/// 결과 데이터를 포함한 성공 응답 생성
pub fn success_data<T>(data: T) -> ApiBodyResponse<T> {
    generate_success(Some(data), None, None, ApiCommEnum::Success.message())
}

/// 결과 데이터와 커스텀 메시지를 포함한 성공 응답 생성
pub fn success_data_message<T>(data: T, message: impl Into<String>) -> ApiBodyResponse<T> {
    generate_success(Some(data), None, None, message)
}

/// 결과 데이터와 페이징 정보를 포함한 성공 응답 생성
pub fn success_data_page<T>(data: T, page: PageDTO) -> ApiBodyResponse<T> {
    generate_success(
        Some(data),
        Some(page),
        None,
        ApiCommEnum::Success.message(),
    )
}

/// 결과 데이터, 페이징 정보, 메시지를 포함한 성공 응답 생성
pub fn success_data_page_message<T>(
    data: T,
    page: PageDTO,
    message: impl Into<String>,
) -> ApiBodyResponse<T> {
    generate_success(Some(data), Some(page), None, message)
}

/// 결과 데이터, HeaderSuccess, 메시지를 포함한 성공 응답 생성
pub fn success_with_header<T>(
    data: T,
    header: HeaderSuccess,
    message: impl Into<String>,
) -> ApiBodyResponse<T> {
    generate_success(Some(data), None, Some(header), message)
}

/// 모든 필드를 조합하여 최종 ApiBodyResponse 구조체를 생성
pub fn generate_success<T>(
    data: Option<T>,
    page: Option<PageDTO>,
    header: Option<HeaderSuccess>,
    message: impl Into<String>,
) -> ApiBodyResponse<T> {
    ApiBodyResponse {
        success: true,
        message: message.into(),
        data,
        page,
        header,
    }
}

/// 기본 실패 응답 생성
pub fn failed<T>() -> ApiBodyResponse<T> {
    failed_message(ApiCommEnum::Failed.message())
}

/// 에러 메시지를 포함한 실패 응답 생성
pub fn failed_message<T>(message: impl Into<String>) -> ApiBodyResponse<T> {
    let header = HeaderSuccess::new(
        "INTERNAL_SERVER_ERROR",
        messages_utils::get_message(MessageEnum::Default.code()),
    );

    ApiBodyResponse {
        success: false,
        message: message.into(),
        header: Some(header),
        page: None,
        data: None,
    }
}
