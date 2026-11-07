use std::collections::HashMap;
use std::sync::OnceLock;

/// 다국어 메시지 사전(한국어/영어) 조회 및 파라미터 포맷팅 유틸리티
/// OnceLock은 멀티스레드 환경에서 최초 1회만 안전하게 초기화되는 정적 싱글톤 저장소입니다.
pub struct MessagesUtils;

static KO_MESSAGES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
static EN_MESSAGES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

fn ko_map() -> &'static HashMap<&'static str, &'static str> {
    KO_MESSAGES.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("ACCEPTED", "요청이 접수되어 처리중입니다");
        m.insert("BAD_REQUEST", "잘못된 요청입니다");
        m.insert("UNAUTHORIZED", "해당 리소스 권한이 없습니다");
        m.insert("FORBIDDEN", "해당 리소스에 대한 비인가 요청입니다");
        m.insert("NOT_FOUND", "요청한 리소스를 찾을 수 없습니다");
        m.insert("METHOD_NOT_ALLOWED", "허용되지 않는 메소드입니다");
        m.insert("UNSUPPORTED_MEDIA_TYPE", "지원되지 않는 미디어 유형");
        m.insert("INTERNAL_SERVER_ERROR", "내부 서버 오류입니다");
        m.insert("SUCCESS", "정상");
        m.insert("SUCCESS_PRCS", "처리되었습니다");
        m.insert("SUCCESS_SRCH", "조회되었습니다");
        m.insert("SUCCESS_ADD", "저장되었습니다");
        m.insert("SUCCESS_MOD", "수정되었습니다");
        m.insert("SUCCESS_DEL", "삭제되었습니다");
        m.insert("SUCCESS_UPL", "업로드되었습니다");
        m.insert("SUCCESS_APL", "적용되었습니다");
        m.insert("SUCCESS_INIT", "초기화되었습니다");
        m.insert("SUCCESS_PYM", "결제되었습니다");
        m.insert("NO_DATA", "데이터가 없습니다");
        m.insert("NO_DATA_SLCT", "선택된 데이터가 없습니다");
        m.insert("NO_DATA_SRCH", "조회된 데이터가 없습니다");
        m.insert("DEFAULT", "통신에 실패하였습니다. 관리자에게 문의 해주세요");
        m.insert("FIELD_INVALID_FAIL", "필드 유효성 체크 실패");
        m.insert("ERROR_SRCH", "조회에 실패 하였습니다.");
        m.insert("ERROR_ADD", "저장에 실패 하였습니다.");
        m.insert("ERROR_MOD", "수정에 실패 하였습니다.");
        m.insert("ERROR_DEL", "삭제에 실패 하였습니다.");
        m.insert("ERROR_VALID_DATE", "올바른 날짜를 입력해 주세요.");
        m.insert("ERROR_VALID_CUSTOM", "{0} : {1}");
        m.insert(
            "COMMON_VALID_ETC1",
            "참조1값은 수수료율, 숫자(소숫점) 형식만 입력 가능합니다.",
        );
        m.insert(
            "COMMON_VALID_ETC2",
            "참조2값은 수수료 적용일자, 날짜 형식만 입력 가능합니다. ex){0}",
        );
        m.insert("COMMON_VALID_ETC4", "참조4값은 숫자형식만 입력 가능합니다.");
        m
    })
}

fn en_map() -> &'static HashMap<&'static str, &'static str> {
    EN_MESSAGES.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert("ACCEPTED", "accepted");
        m.insert("BAD_REQUEST", "bad request");
        m.insert("UNAUTHORIZED", "unauthorized");
        m.insert("FORBIDDEN", "forbidden");
        m.insert("NOT_FOUND", "not found");
        m.insert("METHOD_NOT_ALLOWED", "method not allowed");
        m.insert("UNSUPPORTED_MEDIA_TYPE", "unsupported media type");
        m.insert("INTERNAL_SERVER_ERROR", "internal server error");
        m.insert("SUCCESS", "success");
        m.insert("SUCCESS_PRCS", "processed");
        m.insert("SUCCESS_SRCH", "searched");
        m.insert("SUCCESS_ADD", "stored");
        m.insert("SUCCESS_MOD", "modified");
        m.insert("SUCCESS_DEL", "deleted");
        m.insert("SUCCESS_UPL", "uploaded");
        m.insert("SUCCESS_APL", "applied");
        m.insert("SUCCESS_INIT", "initialized.");
        m.insert("SUCCESS_PYM", "payment.");
        m.insert("NO_DATA", "empty data");
        m.insert("NO_DATA_SLCT", "no data selected");
        m.insert("NO_DATA_SRCH", "no data retrieved");
        m.insert("DEFAULT", "contact the manager");
        m.insert("FIELD_INVALID_FAIL", "field validation failed");
        m
    })
}

impl MessagesUtils {
    /// 기본 로케일(ko) 기준으로 메시지 코드에 해당하는 문자열 반환
    pub fn get_message(code: &str) -> String {
        Self::get_message_with_args(code, &[])
    }

    /// 메시지 템플릿의 {0}, {1} 위치에 인자(args)를 치환하여 반환
    pub fn get_message_with_args(code: &str, args: &[&str]) -> String {
        let base = ko_map().get(code).copied().unwrap_or(code);
        Self::format_args(base, args)
    }

    /// 지정된 언어 코드(ko / en)에 맞춰 메시지를 반환
    pub fn get_message_with_lang(code: &str, lang: Option<&str>) -> String {
        match lang {
            Some("en") | Some("EN") => en_map().get(code).copied().unwrap_or(code).to_string(),
            _ => Self::get_message(code),
        }
    }

    pub fn get_message_by_code(msg: &str) -> String {
        Self::get_message(msg)
    }

    fn format_args(template: &str, args: &[&str]) -> String {
        let mut out = template.to_string();
        for (idx, val) in args.iter().enumerate() {
            let placeholder = format!("{{{}}}", idx);
            out = out.replace(&placeholder, val);
        }
        out
    }
}
