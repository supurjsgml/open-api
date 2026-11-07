use axum::http::HeaderMap;

/// API 공통 응답 상태 코드 및 기본 메시지 열거형
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiCommEnum {
    Success,
    Failed,
}

impl ApiCommEnum {
    pub fn code(&self) -> u16 {
        match self {
            ApiCommEnum::Success => 200,
            ApiCommEnum::Failed => 500,
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            ApiCommEnum::Success => "성공하였습니다.",
            ApiCommEnum::Failed => "Internal Server Error",
        }
    }
}

/// 프록시/로드밸런서 헤더에서 클라이언트 실제 IP를 추출하는 유틸리티
pub struct IpExtractor;

impl IpExtractor {
    pub const HEADERS: [&'static str; 5] = [
        "X-Forwarded-For",
        "Proxy-Client-IP",
        "WL-Proxy-Client-IP",
        "HTTP_CLIENT_IP",
        "HTTP_X_FORWARDED_FOR",
    ];

    /// 지정된 헤더 목록을 순서대로 확인하여 비어있지 않은 첫 번째 IP를 반환하고, 없으면 기본 주소를 반환
    pub fn extract(headers: &HeaderMap, remote_addr: Option<&str>) -> String {
        for key in Self::HEADERS {
            if let Some(val) = headers.get(key).and_then(|v| v.to_str().ok()) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
        remote_addr.unwrap_or("127.0.0.1").to_string()
    }
}

/// 다국어 메시지 사전 조회에 사용하는 메시지 키 열거형
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageEnum {
    Accepted,
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    UnsupportedMediaType,
    InternalServerError,
    Success,
    SuccessPrcs,
    SuccessSrch,
    SuccessAdd,
    SuccessMod,
    SuccessDel,
    SuccessUpl,
    SuccessApl,
    SuccessInit,
    SuccessPym,
    NoData,
    NoDataSlct,
    NoDataSrch,
    Default,
    FieldInvalidFail,
    ErrorSrch,
    ErrorAdd,
    ErrorMod,
    ErrorDel,
    CommonValidEtc1,
    CommonValidEtc2,
    CommonValidEtc4,
    ErrorValidDate,
    ErrorValidCustom,
}

impl MessageEnum {
    pub fn code(&self) -> &'static str {
        match self {
            MessageEnum::Accepted => "ACCEPTED",
            MessageEnum::BadRequest => "BAD_REQUEST",
            MessageEnum::Unauthorized => "UNAUTHORIZED",
            MessageEnum::Forbidden => "FORBIDDEN",
            MessageEnum::NotFound => "NOT_FOUND",
            MessageEnum::MethodNotAllowed => "METHOD_NOT_ALLOWED",
            MessageEnum::UnsupportedMediaType => "UNSUPPORTED_MEDIA_TYPE",
            MessageEnum::InternalServerError => "INTERNAL_SERVER_ERROR",
            MessageEnum::Success => "SUCCESS",
            MessageEnum::SuccessPrcs => "SUCCESS_PRCS",
            MessageEnum::SuccessSrch => "SUCCESS_SRCH",
            MessageEnum::SuccessAdd => "SUCCESS_ADD",
            MessageEnum::SuccessMod => "SUCCESS_MOD",
            MessageEnum::SuccessDel => "SUCCESS_DEL",
            MessageEnum::SuccessUpl => "SUCCESS_UPL",
            MessageEnum::SuccessApl => "SUCCESS_APL",
            MessageEnum::SuccessInit => "SUCCESS_INIT",
            MessageEnum::SuccessPym => "SUCCESS_PYM",
            MessageEnum::NoData => "NO_DATA",
            MessageEnum::NoDataSlct => "NO_DATA_SLCT",
            MessageEnum::NoDataSrch => "NO_DATA_SRCH",
            MessageEnum::Default => "DEFAULT",
            MessageEnum::FieldInvalidFail => "FIELD_INVALID_FAIL",
            MessageEnum::ErrorSrch => "ERROR_SRCH",
            MessageEnum::ErrorAdd => "ERROR_ADD",
            MessageEnum::ErrorMod => "ERROR_MOD",
            MessageEnum::ErrorDel => "ERROR_DEL",
            MessageEnum::CommonValidEtc1 => "COMMON_VALID_ETC1",
            MessageEnum::CommonValidEtc2 => "COMMON_VALID_ETC2",
            MessageEnum::CommonValidEtc4 => "COMMON_VALID_ETC4",
            MessageEnum::ErrorValidDate => "ERROR_VALID_DATE",
            MessageEnum::ErrorValidCustom => "ERROR_VALID_CUSTOM",
        }
    }
}

/// 보일러플레이트 패키지 생성 시 사용하는 템플릿(.vm) 메타 정보 열거형
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplatesEnum {
    Controller,
    Service,
    Implement,
    Repository,
    Dto,
}

impl TemplatesEnum {
    pub fn values() -> &'static [TemplatesEnum] {
        &[
            TemplatesEnum::Controller,
            TemplatesEnum::Service,
            TemplatesEnum::Implement,
            TemplatesEnum::Repository,
            TemplatesEnum::Dto,
        ]
    }

    pub fn folder_nm(&self) -> &'static str {
        match self {
            TemplatesEnum::Controller => "controller",
            TemplatesEnum::Service => "service",
            TemplatesEnum::Implement => "impl",
            TemplatesEnum::Repository => "repository",
            TemplatesEnum::Dto => "dto",
        }
    }

    pub fn file_nm(&self) -> &'static str {
        match self {
            TemplatesEnum::Controller => "Controller",
            TemplatesEnum::Service => "Service",
            TemplatesEnum::Implement => "ServiceImpl",
            TemplatesEnum::Repository => "Mapper",
            TemplatesEnum::Dto => "DTO",
        }
    }

    pub fn vm_path(&self) -> &'static str {
        match self {
            TemplatesEnum::Controller => "templates/vm/controller.vm",
            TemplatesEnum::Service => "templates/vm/service.vm",
            TemplatesEnum::Implement => "templates/vm/implement.vm",
            TemplatesEnum::Repository => "templates/vm/repository.vm",
            TemplatesEnum::Dto => "templates/vm/dto.vm",
        }
    }
}
