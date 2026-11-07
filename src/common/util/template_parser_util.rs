use std::collections::HashMap;
use std::sync::OnceLock;

use regex::{Captures, Regex};
use serde::Serialize;
use serde_json::Value;

/// 문자열 템플릿 내의 `{필드명}` 또는 별칭 플레이스홀더를 DTO 필드 값으로 치환하는 유틸리티
pub struct TemplateParserUtil;

static PLACEHOLDER_REGEX: OnceLock<Regex> = OnceLock::new();

fn placeholder_regex() -> &'static Regex {
    PLACEHOLDER_REGEX.get_or_init(|| Regex::new(r"\{(.+?)\}").expect("valid regex"))
}

impl TemplateParserUtil {
    /// 기본 필드명 기준으로 `{key}` 플레이스홀더를 치환
    pub fn parse<T: Serialize>(template: Option<&str>, dto: Option<&T>) -> String {
        let Some(tpl) = template else {
            return String::new();
        };
        let Some(data) = dto else {
            return tpl.to_string();
        };

        Self::parse_with_aliases(tpl, data, &HashMap::new())
    }

    /// 별칭(aliases) 매핑과 기본 필드명을 모두 지원하여 `{key}` 플레이스홀더를 치환
    pub fn parse_with_aliases<T: Serialize>(
        template: &str,
        dto: &T,
        aliases: &HashMap<String, String>,
    ) -> String {
        let Ok(Value::Object(map)) = serde_json::to_value(dto) else {
            return template.to_string();
        };

        placeholder_regex()
            .replace_all(template, |caps: &Captures| {
                let key = &caps[1];
                let target_field = aliases.get(key).map(|s| s.as_str()).unwrap_or(key);
                match map.get(target_field) {
                    Some(Value::String(s)) => s.clone(),
                    Some(Value::Number(n)) => n.to_string(),
                    Some(Value::Bool(b)) => b.to_string(),
                    _ => String::new(),
                }
            })
            .to_string()
    }
}
