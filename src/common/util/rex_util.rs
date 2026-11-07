use regex::Regex;

/// 자주 사용하는 정규표현식 상수 및 패턴 검증 유틸리티
pub struct RexUtil;

impl RexUtil {
    /// 소수점 숫자 체크 정규식
    pub const DECIMAL_POINT_PATTERN: &'static str = r"^-?\d*\.?\d+$";
    /// yyyy-MM-dd 날짜 체크 정규식
    pub const DATE_PATTERN: &'static str = r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$";
    /// 숫자형 체크 정규식
    pub const NUMBER_PATTERN: &'static str = r"^\d*$";
    /// 슬래시(/) 및 백슬래시(\) 구분자 정규식
    pub const SLASH_BACKSLASH_PATTERN: &'static str = r"[/\\]";

    pub fn is_decimal(input: &str) -> bool {
        Regex::new(Self::DECIMAL_POINT_PATTERN)
            .map(|r| r.is_match(input))
            .unwrap_or(false)
    }

    pub fn is_date(input: &str) -> bool {
        Regex::new(Self::DATE_PATTERN)
            .map(|r| r.is_match(input))
            .unwrap_or(false)
    }

    pub fn is_number(input: &str) -> bool {
        Regex::new(Self::NUMBER_PATTERN)
            .map(|r| r.is_match(input))
            .unwrap_or(false)
    }

    pub fn replace_slash_backslash(input: &str, replacement: &str) -> String {
        Regex::new(Self::SLASH_BACKSLASH_PATTERN)
            .map(|r| r.replace_all(input, replacement).to_string())
            .unwrap_or_else(|_| input.to_string())
    }
}
