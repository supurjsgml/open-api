use std::collections::HashMap;

use chrono::{Duration, Utc};
use chrono_tz::Asia::Seoul;
use redis::AsyncCommands;
use tracing::{error, info};

use crate::internal::dto::{DashboardStatsDTO, PageVisitsDTO, VisitorHitReqDTO};

const VISITOR_KEY_PREFIX: &str = "visitor_count:";
const PAGE_VISITS_KEY: &str = "page_visits";

/// 대시보드 및 방문자 통계 서비스
#[derive(Clone)]
pub struct InternalService {
    redis_client: Option<redis::Client>,
}

impl InternalService {
    pub fn new(redis_client: Option<redis::Client>) -> Self {
        Self { redis_client }
    }

    /// 최근 7일간의 일별 방문자 수와 증감 추이를 조회합니다.
    pub async fn get_dashboard_stats(&self) -> DashboardStatsDTO {
        let today_kst = Utc::now().with_timezone(&Seoul).date_naive();

        let mut keys = Vec::with_capacity(7);
        let mut days = Vec::with_capacity(7);

        for i in (0..=6).rev() {
            let date = today_kst - Duration::days(i);
            keys.push(format!("{}{}", VISITOR_KEY_PREFIX, date.format("%Y-%m-%d")));
            if i == 0 {
                days.push("오늘".to_string());
            } else {
                days.push(date.format("%d").to_string());
            }
        }

        // Redis MGET 명령어로 7일치 방문자 데이터를 한 번의 네트워크 왕복으로 조회
        let raw_counts: Vec<Option<String>> = match self.get_mget(&keys).await {
            Some(list) => list,
            None => vec![None; 7],
        };

        let mut sparkline_values = Vec::with_capacity(7);
        let mut this_week_total: i64 = 0;

        for idx in 0..7 {
            let count = raw_counts
                .get(idx)
                .and_then(|opt| opt.as_deref())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            sparkline_values.push(count);
            this_week_total += count;
        }

        let today_count = *sparkline_values.last().unwrap_or(&0);
        let (trend, trend_direction) = if this_week_total > 0 {
            let avg = this_week_total as f64 / 7.0;
            if avg > 0.0 {
                let percent = ((today_count as f64 - avg) / avg) * 100.0;
                let abs_percent = percent.abs().round() as i64;
                let dir = if percent >= 0.0 { "up" } else { "down" };
                (Some(format!("{}%", abs_percent)), Some(dir.to_string()))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        DashboardStatsDTO {
            label: "주간 방문자 (1주)".to_string(),
            value: Self::format_with_commas(this_week_total),
            description: None,
            trend,
            trend_direction,
            sparkline_values,
            days,
        }
    }

    /// 페이지별 누적 방문 횟수를 Redis Hash(`page_visits`)에서 조회합니다.
    pub async fn get_page_visits(&self) -> PageVisitsDTO {
        let entries = self.get_hgetall(PAGE_VISITS_KEY).await;

        let parse_val = |key: &str| -> i64 {
            entries
                .get(key)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0)
        };

        PageVisitsDTO {
            main: parse_val("Main"),
            camel: parse_val("Camel"),
            translate: parse_val("Translate"),
            diff: parse_val("Diff"),
            jasypt: parse_val("Jasypt"),
        }
    }

    /// 페이지 방문 수 및 신규 세션인 경우 오늘자 일일 방문자 수를 1 증가시킵니다.
    pub async fn record_hit(&self, req_dto: VisitorHitReqDTO) {
        let page_name = req_dto
            .page_name
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Main".to_string());

        let Some(ref client) = self.redis_client else {
            return;
        };

        let Ok(mut conn) = client.get_multiplexed_async_connection().await else {
            error!("Failed to connect to Redis for record_hit");
            return;
        };

        // HINCRBY로 해당 페이지 방문 카운트 1 증가
        let _: redis::RedisResult<i64> = conn.hincr(PAGE_VISITS_KEY, &page_name, 1).await;

        // 신규 세션이면 오늘 날짜(KST) 기준 방문자 수 1 증가
        if req_dto.is_new_session {
            let today_str = Utc::now()
                .with_timezone(&Seoul)
                .format("%Y-%m-%d")
                .to_string();
            let visitor_key = format!("{}{}", VISITOR_KEY_PREFIX, today_str);
            let _: redis::RedisResult<i64> = conn.incr(&visitor_key, 1).await;
            info!("New session visitor count incremented for {}", today_str);
        }
    }

    /// Redis MGET 헬퍼 함수
    async fn get_mget(&self, keys: &[String]) -> Option<Vec<Option<String>>> {
        let client = self.redis_client.as_ref()?;
        let mut conn = client.get_multiplexed_async_connection().await.ok()?;
        conn.mget(keys).await.ok()
    }

    /// Redis HGETALL 헬퍼 함수
    async fn get_hgetall(&self, key: &str) -> HashMap<String, String> {
        let Some(ref client) = self.redis_client else {
            return HashMap::new();
        };
        let Ok(mut conn) = client.get_multiplexed_async_connection().await else {
            return HashMap::new();
        };
        conn.hgetall(key).await.unwrap_or_default()
    }

    /// 정수를 천 단위 콤마 문자열(예: 48,250)로 포맷팅합니다.
    fn format_with_commas(n: i64) -> String {
        let is_negative = n < 0;
        let s = n.abs().to_string();
        let mut result = String::new();
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();

        for (i, ch) in chars.into_iter().enumerate() {
            if i > 0 && (len - i) % 3 == 0 {
                result.push(',');
            }
            result.push(ch);
        }

        if is_negative {
            format!("-{}", result)
        } else {
            result
        }
    }
}
