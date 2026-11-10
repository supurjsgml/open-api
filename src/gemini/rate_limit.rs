use chrono::{Duration, Local, NaiveTime};
use redis::AsyncCommands;
use tracing::{error, info};

use crate::AppState;

/// 일일 최대 요약 요청 제한 횟수
const DAILY_LIMIT: i64 = 50;
/// Redis 키 접두사
const KEY_PREFIX: &str = "rate:limit:";

/// 이메일과 Google 고유 식별자(sub)를 기반으로 일일 제한 초과 여부를 검사합니다.
/// 제한 이내이면 `true`, 제한을 초과했거나 식별자가 없으면 `false`를 반환합니다.
pub async fn check_limit(
    state: &AppState,
    email: Option<&str>,
    google_sub_id: Option<&str>,
) -> bool {
    // 관리자 이메일 목록에 포함된 경우 횟수 제한 없이 즉시 통과
    if let Some(em) = email {
        if state.config.admin_emails.iter().any(|admin| admin == em) {
            info!(
                "[RateLimit] Admin bypass triggered for email: {} (sub: {:?})",
                em, google_sub_id
            );
            return true;
        }
    }

    let sub_id = match google_sub_id {
        Some(s) if !s.trim().is_empty() => s,
        _ => return false,
    };

    let Some(ref client) = state.redis_client else {
        // Redis 클라이언트가 설정되지 않은 환경에서는 서비스 중단을 막기 위해 통과 처리 (Fail-open)
        return true;
    };

    let today = Local::now().format("%Y%m%d").to_string();
    let key = format!("{}{}:{}", KEY_PREFIX, today, sub_id);

    let Ok(mut conn) = client.get_multiplexed_async_connection().await else {
        error!(
            "[RateLimit] Redis error occurred for sub: {}. Falling back to allow.",
            sub_id
        );
        return true;
    };

    // Redis INCR 명령어로 카운트를 1 증가시키고 현재 값을 가져옵니다.
    let count: redis::RedisResult<i64> = conn.incr(&key, 1).await;
    match count {
        Ok(current_count) => {
            // 오늘 최초 요청인 경우 자정까지의 남은 시간(초)을 계산하여 만료 시간(TTL) 설정
            if current_count == 1 {
                let now = Local::now();
                if let Some(tomorrow) = (now + Duration::days(1))
                    .date_naive()
                    .and_time(NaiveTime::MIN)
                    .and_local_timezone(Local)
                    .single()
                {
                    let seconds_until_midnight = (tomorrow - now).num_seconds().max(1) as u64;
                    let _: redis::RedisResult<()> =
                        conn.expire(&key, seconds_until_midnight as i64).await;
                }
            }

            if current_count > DAILY_LIMIT {
                info!(
                    "[RateLimit] User {} (email: {:?}) exceeded daily limit of {}",
                    sub_id, email, DAILY_LIMIT
                );
                return false;
            }
            true
        }
        Err(e) => {
            error!(
                "[RateLimit] Redis error occurred for sub: {}. Falling back to allow. Error: {}",
                sub_id, e
            );
            true
        }
    }
}
