use std::fmt;

use serde_json::Value;

use crate::common::enums::{ApiCommEnum, MessageEnum};
use crate::common::util::messages_utils::MessagesUtils;

/// 공통 에러 코드, 메시지, 성공 여부를 담는 기본 에러 구조체
#[derive(Debug, Clone)]
pub struct BaseException {
    pub code: String,
    pub base_message: Option<String>,
    pub result: bool,
}

impl BaseException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: "code".to_string(),
            base_message: Some(message.into()),
            result: false,
        }
    }

    pub fn with_code(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            base_message: Some(message.into()),
            result: false,
        }
    }
}

impl fmt::Display for BaseException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.base_message.as_deref().unwrap_or(self.code.as_str())
        )
    }
}

impl std::error::Error for BaseException {}

/// 데이터 처리 및 파일 입출력 실패 시 사용하는 커스텀 에러 구조체
#[derive(Debug, Clone)]
pub struct DataCustomException {
    pub base: BaseException,
}

impl DataCustomException {
    pub fn default_err() -> Self {
        Self {
            base: BaseException::with_code(
                ApiCommEnum::Failed.code().to_string(),
                ApiCommEnum::Failed.message(),
            ),
        }
    }

    pub fn new(message: impl Into<String>) -> Self {
        Self {
            base: BaseException::with_code(ApiCommEnum::Failed.code().to_string(), message),
        }
    }

    pub fn with_code(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            base: BaseException::with_code(code, message),
        }
    }

    pub fn message(&self) -> &str {
        self.base.base_message.as_deref().unwrap_or("")
    }
}

impl fmt::Display for DataCustomException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for DataCustomException {}

/// 비즈니스 로직 및 입력값 유효성 검증 실패 시 사용하는 에러 구조체
#[derive(Debug, Clone)]
pub struct ValidException {
    pub message: String,
}

impl ValidException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn from_enum(e: MessageEnum) -> Self {
        Self {
            message: MessagesUtils::get_message(e.code()),
        }
    }

    pub fn from_enum_with_arg(e: MessageEnum, arg: &str) -> Self {
        Self {
            message: MessagesUtils::get_message_with_args(e.code(), &[arg]),
        }
    }

    pub fn from_enum_with_args(e: MessageEnum, args: &[&str]) -> Self {
        Self {
            message: MessagesUtils::get_message_with_args(e.code(), args),
        }
    }
}

impl fmt::Display for ValidException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ValidException {}

/// 작업 중단(롤백)이 필요하지만 클라이언트에는 정상(200 OK) 포맷으로 데이터를 내려줄 때 사용하는 에러 구조체
#[derive(Debug, Clone)]
pub struct ApiDataResponseSuccessException {
    pub code: String,
    pub message: String,
    pub data: Option<Value>,
}

impl ApiDataResponseSuccessException {
    pub fn from_message(message: impl Into<String>) -> Self {
        Self {
            code: "200".to_string(),
            message: message.into(),
            data: None,
        }
    }

    pub fn from_code_message(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data: None,
        }
    }

    pub fn from_data(data: Value) -> Self {
        Self {
            code: "200".to_string(),
            message: "SUCCESS".to_string(),
            data: Some(data),
        }
    }

    pub fn from_data_message(data: Value, message: impl Into<String>) -> Self {
        Self {
            code: "200".to_string(),
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn new(data: Option<Value>, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data,
        }
    }
}

impl fmt::Display for ApiDataResponseSuccessException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ApiDataResponseSuccessException {}
