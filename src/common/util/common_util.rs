use std::collections::HashMap;

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use url::Url;

/// SQL 페이징 LIMIT 문자열 생성
pub fn str_paging(cur_page: i32, page_size: i32) -> String {
    let per_page = if page_size == -1 || page_size >= 500 {
        500
    } else {
        page_size
    };
    let start_num = if cur_page == 0 {
        0
    } else {
        (cur_page - 1) * page_size
    };

    if start_num >= 0 && per_page > 0 {
        format!("LIMIT {}, {}", start_num, per_page)
    } else {
        String::new()
    }
}

/// lowerCamelCase 문자열을 UPPER_SNAKE_CASE로 변환
pub fn convert_camel_to_snake(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            for upper in ch.to_uppercase() {
                result.push(upper);
            }
        } else {
            for upper in ch.to_uppercase() {
                result.push(upper);
            }
        }
    }
    result
}

/// UPPER_SNAKE_CASE 문자열을 lowerCamelCase로 변환
pub fn convert_snake_to_camel(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = false;

    for (i, ch) in s.chars().enumerate() {
        if ch == '_' {
            if i > 0 {
                capitalize_next = true;
            }
        } else if capitalize_next {
            for upper in ch.to_uppercase() {
                result.push(upper);
            }
            capitalize_next = false;
        } else {
            for lower in ch.to_lowercase() {
                result.push(lower);
            }
        }
    }
    result
}

/// 리스트 내 객체들을 대상 타입(D)으로 일괄 변환
pub fn convert_list<S, D>(source_list: &[S]) -> Vec<D>
where
    S: Serialize,
    D: DeserializeOwned,
{
    if source_list.is_empty() {
        return Vec::new();
    }
    source_list
        .iter()
        .filter_map(|item| convert_object(item))
        .collect()
}

/// 단일 구조체를 JSON Value를 거쳐 대상 타입(D)으로 변환
pub fn convert_object<S, D>(source: &S) -> Option<D>
where
    S: Serialize,
    D: DeserializeOwned,
{
    let val = serde_json::to_value(source).ok()?;
    if val.is_null() {
        return None;
    }
    serde_json::from_value(val).ok()
}

/// JSON 문자열을 대상 구조체(D)로 역직렬화
pub fn read_value<D>(string_data: &str) -> Option<D>
where
    D: DeserializeOwned,
{
    if string_data.trim().is_empty() {
        return None;
    }
    serde_json::from_str(string_data).ok()
}

/// 구조체 프로퍼티 복사
pub fn copy_bean<S, D>(source: &S) -> Option<D>
where
    S: Serialize,
    D: DeserializeOwned,
{
    convert_object(source)
}

/// Map의 모든 키를 UPPER_SNAKE_CASE로 변환하여 새 Map으로 반환
pub fn convert_map(input: &HashMap<String, Value>) -> HashMap<String, Value> {
    let mut return_map = HashMap::with_capacity(input.len());
    for (k, v) in input {
        return_map.insert(convert_camel_to_snake(k), v.clone());
    }
    return_map
}

pub fn map_param_convert(input: &HashMap<String, Value>) -> HashMap<String, Value> {
    convert_map(input)
}

pub fn get_query_param_with_kwd<T: Serialize>(
    url_str: &str,
    _kwd: &str,
    obj: &T,
) -> Result<String, String> {
    get_query_param(url_str, obj)
}

/// DTO 필드를 URL 쿼리 파라미터(?key=value)로 붙여서 완성된 URL 문자열 반환
pub fn get_query_param<T: Serialize>(url_str: &str, obj: &T) -> Result<String, String> {
    let mut parsed = Url::parse(url_str).map_err(|e| e.to_string())?;
    let val = serde_json::to_value(obj).map_err(|e| e.to_string())?;

    if let Value::Object(map) = val {
        let mut pairs = parsed.query_pairs_mut();
        for (k, v) in map {
            match v {
                Value::Null => {}
                Value::String(s) if !s.is_empty() => {
                    pairs.append_pair(&k, &s);
                }
                Value::Number(n) => {
                    pairs.append_pair(&k, &n.to_string());
                }
                Value::Bool(b) => {
                    pairs.append_pair(&k, &b.to_string());
                }
                _ => {}
            }
        }
    }

    Ok(parsed.to_string())
}

/// 지정된 소수점 자릿수(len)만큼 포맷팅
pub fn decimal_format(d: f64, len: usize) -> String {
    format!("{:.*}", len, d)
}

/// 문자열의 첫 글자를 소문자로 변환
pub fn decapitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

/// 문자열의 첫 글자를 대문자로 변환
pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// 문자열이 None이거나 공백이거나 "null" 문자열인지 확인
pub fn string_is_blank_or_null(s: Option<&str>) -> bool {
    match s {
        None => true,
        Some(val) => {
            let trimmed = val.trim();
            trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null")
        }
    }
}
