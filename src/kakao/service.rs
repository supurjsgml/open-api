use std::collections::HashMap;

use redis::AsyncCommands;
use serde_json::Value;
use tracing::{error, info};

use crate::AppState;
use crate::kakao::dto::KakaoTokenResDTO;
use crate::kakao::template::{Button, KakaoTextTemplate, Link};

const REDIS_KAKAO_REFRESH_TOKEN_KEY: &str = "kakao:token:refresh";
const REDIS_KAKAO_ACCESS_TOKEN_KEY: &str = "kakao:token:access";

/// Redis에서 Refresh Token을 조회하고, 없으면 환경 설정의 기본 토큰으로 초기화합니다.
pub async fn get_refresh_token(state: &AppState) -> String {
    if let Some(ref client) = state.redis_client {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let stored: redis::RedisResult<Option<String>> =
                conn.get(REDIS_KAKAO_REFRESH_TOKEN_KEY).await;
            if let Ok(Some(token)) = stored {
                if !token.trim().is_empty() {
                    return token;
                }
            }

            info!("Redis에 카카오 Refresh Token이 없어 기본 설정값으로 초기화합니다.");
            let default_token = state.config.keys.kakao.refresh_token.clone();
            // 60일(5,184,000초) 만료 시간으로 저장
            let _: redis::RedisResult<()> = conn
                .set_ex(
                    REDIS_KAKAO_REFRESH_TOKEN_KEY,
                    &default_token,
                    60 * 24 * 60 * 60,
                )
                .await;
            return default_token;
        }
    }
    state.config.keys.kakao.refresh_token.clone()
}

/// Redis에서 유효한 Access Token을 조회하고, 만료되었거나 없으면 자동 갱신합니다.
pub async fn get_access_token(state: &AppState) -> Result<String, String> {
    if let Some(ref client) = state.redis_client {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let stored: redis::RedisResult<Option<String>> =
                conn.get(REDIS_KAKAO_ACCESS_TOKEN_KEY).await;
            if let Ok(Some(token)) = stored {
                if !token.trim().is_empty() {
                    return Ok(token);
                }
            }
        }
    }

    info!("Redis에 유효한 카카오 Access Token이 없어 토큰 갱신을 진행합니다.");
    let res = refresh_kakao_token(state).await?;
    res.access_token
        .ok_or_else(|| "카카오 Access Token 발급 실패".to_string())
}

/// 카카오 인증 서버에 Refresh Token을 전달하여 Access Token(및 신규 Refresh Token)을 갱신합니다.
pub async fn refresh_kakao_token(state: &AppState) -> Result<KakaoTokenResDTO, String> {
    let current_refresh_token = get_refresh_token(state).await;

    let mut form_data = HashMap::new();
    form_data.insert("grant_type", "refresh_token".to_string());
    form_data.insert("client_id", state.config.keys.kakao.client_id.clone());
    form_data.insert(
        "client_secret",
        state.config.keys.kakao.client_secret.clone(),
    );
    form_data.insert("refresh_token", current_refresh_token);

    let response = state
        .http_client
        .post(&state.config.rest_api.kakao.auth.token)
        .form(&form_data)
        .send()
        .await
        .map_err(|e| format!("카카오 토큰 갱신 요청 실패: {}", e))?;

    let token_res: HashMap<String, Value> = response
        .json()
        .await
        .map_err(|e| format!("카카오 토큰 응답 파싱 실패: {}", e))?;

    let Some(access_token) = token_res
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
    else {
        error!("카카오 토큰 갱신 실패: {:?}", token_res);
        return Err("카카오 토큰 갱신 실패".to_string());
    };

    let expires_in = token_res
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(21600);

    // 안전 마진 5분(300초)을 차감하여 Redis TTL 설정
    let ttl = if expires_in > 300 {
        expires_in - 300
    } else {
        expires_in
    };

    let mut new_refresh_token: Option<String> = None;

    if let Some(ref client) = state.redis_client {
        if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
            let _: redis::RedisResult<()> = conn
                .set_ex(REDIS_KAKAO_ACCESS_TOKEN_KEY, &access_token, ttl)
                .await;
            info!(
                "카카오 Access Token이 Redis에 캐싱되었습니다. (만료: {}초)",
                ttl
            );

            // 응답에 새로운 refresh_token이 포함된 경우 함께 갱신
            if let Some(rt) = token_res.get("refresh_token").and_then(|v| v.as_str()) {
                let rt_str = rt.to_string();
                let rt_expires = token_res
                    .get("refresh_token_expires_in")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5_184_000);
                let _: redis::RedisResult<()> = conn
                    .set_ex(REDIS_KAKAO_REFRESH_TOKEN_KEY, &rt_str, rt_expires)
                    .await;
                info!("카카오 Refresh Token이 새로 갱신되어 Redis에 저장되었습니다.");
                new_refresh_token = Some(rt_str);
            }
        }
    }

    Ok(KakaoTokenResDTO {
        access_token: Some(access_token),
        new_refresh_token,
    })
}

/// API 서버 내부 에러 발생 시 카카오톡 알림을 전송합니다. (재시작 경로: `/api/lightsail/restart-api`)
pub async fn send_kakao(state: &AppState, msg: &str) {
    if let Err(e) = send_kakao_message(state, msg, "/api/lightsail/restart-api").await {
        error!("카카오톡 전송 중 예외 발생: {}", e);
    }
}

/// 외부 서버(배치 등)에서 요청한 카카오톡 알림을 전송합니다. (재시작 경로: `/api/lightsail/restart`)
pub async fn send_kakao_external(state: &AppState, msg: &str) {
    if let Err(e) = send_kakao_message(state, msg, "/api/lightsail/restart").await {
        error!("카카오톡 전송 중 예외 발생: {}", e);
    }
}

/// 지정된 재시작 경로 버튼을 포함한 카카오톡 나에게 보내기 메시지를 발송합니다.
async fn send_kakao_message(state: &AppState, msg: &str, restart_path: &str) -> Result<(), String> {
    let access_token = get_access_token(state).await?;
    let restart_url = format!("{}{}", state.config.rest_api.base_url, restart_path);

    let link = Link {
        web_url: restart_url.clone(),
        mobile_web_url: restart_url,
    };

    let button = Button {
        title: "서버 재시작 가쥬아".to_string(),
        link: link.clone(),
    };

    let template = KakaoTextTemplate {
        object_type: "text".to_string(),
        text: format!("[에러 발생] : {}", msg),
        link,
        buttons: vec![button],
    };

    let template_json = serde_json::to_string(&template)
        .map_err(|e| format!("카카오 템플릿 직렬화 실패: {}", e))?;

    let mut form_data = HashMap::new();
    form_data.insert("template_object", template_json);

    let response = state
        .http_client
        .post(&state.config.rest_api.kakao.api.memo)
        .header("Authorization", format!("Bearer {}", access_token))
        .form(&form_data)
        .send()
        .await
        .map_err(|e| format!("카카오톡 메시지 전송 요청 실패: {}", e))?;

    let send_res: HashMap<String, Value> = response.json().await.unwrap_or_default();
    info!("카카오톡 전송 결과: {:?}", send_res);
    Ok(())
}
