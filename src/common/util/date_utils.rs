use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use tracing::error;

/// 날짜/시간 조회, 포맷 변환, 상대 시간("방금", "N분전" 등) 계산을 담당하는 유틸리티
pub struct DateUtils;

impl DateUtils {
    pub const YYYYMMDD_TIME_STR: &'static str = "%Y-%m-%d %H:%M:%S";
    pub const YYYYMMDD_STR: &'static str = "%Y-%m-%d";
    pub const YYYY_MM_DD_TIME: &'static str = "%Y-%m-%d %H:%M:%S";
    pub const YYYY_MM_DD: &'static str = "%Y-%m-%d";
    pub const YYYYMMDDTIME: &'static str = "%Y%m%d %H:%M:%S";
    pub const YYYYMMDD: &'static str = "%Y%m%d";

    /// 현재 로컬 날짜/시간 반환
    pub fn current_date_time() -> NaiveDateTime {
        Local::now().naive_local()
    }

    /// 현재 날짜/시간을 지정된 포맷(없으면 yyyy-MM-dd HH:mm:ss)으로 반환
    pub fn current_date_time_format(format: Option<&str>) -> String {
        let pattern = Self::normalize_pattern(format.unwrap_or(Self::YYYY_MM_DD_TIME));
        Local::now().naive_local().format(&pattern).to_string()
    }

    /// yyyy-MM-dd HH:mm:ss 문자열을 NaiveDateTime으로 파싱
    pub fn local_date_time_parse(date: &str) -> Option<NaiveDateTime> {
        if date.trim().is_empty() {
            return None;
        }
        NaiveDateTime::parse_from_str(date, Self::YYYY_MM_DD_TIME).ok()
    }

    /// 오늘 날짜의 자정(00:00:00) 시각 반환
    pub fn cur_local_date_time_start() -> NaiveDateTime {
        let today = Local::now().date_naive();
        today.and_hms_opt(0, 0, 0).unwrap_or_default()
    }

    /// 오늘 날짜의 마지막 시각(23:59:59.999999999) 반환
    pub fn cur_local_date_time_end() -> NaiveDateTime {
        let today = Local::now().date_naive();
        let max_time =
            NaiveTime::from_hms_nano_opt(23, 59, 59, 999_999_999).unwrap_or(NaiveTime::MIN);
        NaiveDateTime::new(today, max_time)
    }

    /// 오늘 날짜(NaiveDate) 반환
    pub fn current_date() -> NaiveDate {
        Local::now().date_naive()
    }

    pub fn current_date_format(format: Option<&str>) -> String {
        let pattern = Self::normalize_pattern(format.unwrap_or(Self::YYYY_MM_DD));
        Local::now().date_naive().format(&pattern).to_string()
    }

    /// Epoch 밀리초 타임스탬프를 로컬 날짜/시간으로 변환
    pub fn to_local_date_time_from_millis(time_ms: i64) -> Option<NaiveDateTime> {
        let dt_utc = DateTime::from_timestamp_millis(time_ms)?;
        Some(dt_utc.with_timezone(&Local).naive_local())
    }

    /// yyyy-MM-dd 문자열을 NaiveDate로 파싱
    pub fn local_date_parse(date: &str) -> Option<NaiveDate> {
        if date.trim().is_empty() {
            return None;
        }
        NaiveDate::parse_from_str(date, Self::YYYY_MM_DD).ok()
    }

    pub fn local_date_time_to_string(date: Option<NaiveDateTime>) -> Option<String> {
        date.map(|d| d.format(Self::YYYY_MM_DD_TIME).to_string())
    }

    pub fn local_date_to_string(date: Option<NaiveDate>) -> Option<String> {
        date.map(|d| d.format(Self::YYYY_MM_DD).to_string())
    }

    pub fn local_date_time_to_string_with_format(
        date: Option<NaiveDateTime>,
        format: &str,
    ) -> Option<String> {
        let pattern = Self::normalize_pattern(format);
        date.map(|d| d.format(&pattern).to_string())
    }

    /// 등록 시각 문자열과 현재 시각의 차이를 계산하여 "방금", "N분전", "N시간전", "N일전", "N개월전", "N년전" 반환
    pub fn timediff(reg_dt_str: &str) -> String {
        let parsed = NaiveDateTime::parse_from_str(reg_dt_str, "%Y-%m-%d %H:%M").or_else(|_| {
            NaiveDateTime::parse_from_str(reg_dt_str, Self::YYYY_MM_DD_TIME)
        });

        match parsed {
            Ok(reg_dt) => {
                let current_dt = Local::now().naive_local();
                let seconds_diff = (current_dt - reg_dt).num_seconds();

                if seconds_diff < 60 {
                    "방금".to_string()
                } else if seconds_diff < 3600 {
                    format!("{}분전", seconds_diff / 60)
                } else if seconds_diff < 86400 {
                    format!("{}시간전", seconds_diff / 3600)
                } else if seconds_diff < 2_592_000 {
                    format!("{}일전", seconds_diff / 86400)
                } else if seconds_diff < 31_104_000 {
                    format!("{}개월전", seconds_diff / 2_592_000)
                } else {
                    format!("{}년전", seconds_diff / 31_104_000)
                }
            }
            Err(e) => {
                error!(">>>>>>>ERR-TIMEDIFF:{}", e);
                String::new()
            }
        }
    }

    /// 한국 표준시(Asia/Seoul) 기준 현재 시각 반환
    pub fn kst_now() -> DateTime<chrono_tz::Tz> {
        chrono_tz::Asia::Seoul.from_utc_datetime(&chrono::Utc::now().naive_utc())
    }

    /// 날짜 포맷(yyyy-MM-dd HH:mm:ss)을 chrono 포맷(%Y-%m-%d %H:%M:%S)으로 정규화
    fn normalize_pattern(pattern: &str) -> String {
        if pattern.contains('%') {
            return pattern.to_string();
        }
        pattern
            .replace("yyyy", "%Y")
            .replace("MM", "%m")
            .replace("dd", "%d")
            .replace("HH", "%H")
            .replace("mm", "%M")
            .replace("ss", "%S")
    }
}
