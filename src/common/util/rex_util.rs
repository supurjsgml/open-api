use regex::Regex;

/// 소수점 숫자 체크 정규식
pub const DECIMAL_POINT_PATTERN: &str = r"^-?\d*\.?\d+$";
/// yyyy-MM-dd 날짜 체크 정규식
pub const DATE_PATTERN: &str = r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$";
/// 숫자형 체크 정규식
pub const NUMBER_PATTERN: &str = r"^\d*$";
/// 슬래시(/) 및 백슬래시(\) 구분자 정규식
pub const SLASH_BACKSLASH_PATTERN: &str = r"[/\\]";

pub fn is_decimal(input: &str) -> bool {
    Regex::new(DECIMAL_POINT_PATTERN)
        .map(|r| r.is_match(input))
        .unwrap_or(false)
}

pub fn is_date(input: &str) -> bool {
    Regex::new(DATE_PATTERN)
        .map(|r| r.is_match(input))
        .unwrap_or(false)
}

pub fn is_number(input: &str) -> bool {
    Regex::new(NUMBER_PATTERN)
        .map(|r| r.is_match(input))
        .unwrap_or(false)
}

pub fn replace_slash_backslash(input: &str, replacement: &str) -> String {
    Regex::new(SLASH_BACKSLASH_PATTERN)
        .map(|r| r.replace_all(input, replacement).to_string())
        .unwrap_or_else(|_| input.to_string())
}
