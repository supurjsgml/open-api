use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::AppState;
use crate::common::dto::ApiBodyResponse;
use crate::common::util::api_res_util;
use crate::gemini::dto::{GeminiModelResDTO, GeminiReqDTO};
use crate::gemini::{rate_limit, service};

/// Gemini AI 도메인 라우터를 생성합니다.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/gemini/summarize", post(summarize))
        .route("/api/gemini/models", get(get_models))
}

/// Google OAuth2 토큰 검증 결과 구조체
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenInfo {
    pub user_id: Option<String>,
    pub sub: Option<String>,
    pub email: Option<String>,
    pub error: Option<String>,
}

/// 인메모리 토큰 캐시 엔트리 (유효기간 10분)
#[derive(Debug, Clone)]
pub struct CachedToken {
    pub token_info: TokenInfo,
    pub expires_at: Instant,
}

impl CachedToken {
    pub fn new(token_info: TokenInfo, ttl_ms: u64) -> Self {
        Self {
            token_info,
            expires_at: Instant::now() + Duration::from_millis(ttl_ms),
        }
    }

    pub fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// 토큰 검증 캐시 저장소 타입 별칭
pub type TokenCache = Arc<RwLock<HashMap<String, CachedToken>>>;

/// POST `/api/gemini/summarize`
/// Google OAuth2 액세스 토큰 검증 및 일일 호출 제한 확인 후 스트리밍 요약을 반환합니다.
pub async fn summarize(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req_dto): Json<GeminiReqDTO>,
) -> Response {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let Some(auth_str) = auth_header.filter(|h| h.starts_with("Bearer ")) else {
        warn!("[Gemini Auth] Missing or invalid Authorization header");
        return plain_text_response("ERROR: 인증 토큰이 누락되었습니다. 구글 로그인이 필요합니다.");
    };

    let token = auth_str.trim_start_matches("Bearer ").trim().to_string();

    // Google OAuth2 토큰 유효성 검증 (캐시 우선 조회)
    let Some(token_info) = verify_google_token(&state, &token).await else {
        warn!("[Gemini Auth] Invalid Google access token provided");
        return plain_text_response(
            "ERROR: 유효하지 않은 구글 인증 토큰입니다. 다시 로그인해 주세요.",
        );
    };

    // 사용자 고유 식별자 결정 (sub -> user_id -> email 순서)
    let sub_id = token_info
        .sub
        .as_deref()
        .or(token_info.user_id.as_deref())
        .or(token_info.email.as_deref());

    // 일일 호출 횟수 제한 검사
    let allowed = rate_limit::check_limit(&state, token_info.email.as_deref(), sub_id).await;

    if !allowed {
        return plain_text_response(
            "ERROR: 일일 요약 제한 횟수(50회)를 초과했습니다. 내일 다시 이용해 주세요.",
        );
    }

    // 비동기 텍스트 스트림 생성 및 반환
    let stream = service::get_summary(&state, req_dto).await;
    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(body)
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// GET `/api/gemini/models`
/// 사용 가능한 Gemini 모델 리스트를 반환합니다.
pub async fn get_models(
    State(state): State<Arc<AppState>>,
) -> Json<ApiBodyResponse<Vec<GeminiModelResDTO>>> {
    let models = service::get_model_list(&state).await;
    Json(api_res_util::success_data_message(models, "SUCCESS"))
}

/// Google OAuth2 TokenInfo 엔드포인트를 통해 액세스 토큰을 검증하고 10분간 캐싱합니다.
async fn verify_google_token(state: &AppState, token: &str) -> Option<TokenInfo> {
    // 캐시에 유효한 토큰 정보가 있는지 먼저 확인
    {
        let cache = state.token_cache.read().await;
        if let Some(cached) = cache.get(token) {
            if !cached.is_expired() {
                info!("[Gemini Auth] Token verified from local cache");
                return Some(cached.token_info.clone());
            }
        }
    }

    // 만료된 항목 정리 및 Google 서버에 토큰 검증 요청
    {
        let mut cache = state.token_cache.write().await;
        cache.retain(|_, v| !v.is_expired());
    }

    let url = format!(
        "https://oauth2.googleapis.com/tokeninfo?access_token={}",
        token
    );

    let resp = match state.http_client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            error!("[Gemini Auth] Token verification failed with error: {}", e);
            return None;
        }
    };

    if !resp.status().is_success() {
        warn!(
            "[Gemini Auth] Google tokeninfo returned error status: {}",
            resp.status()
        );
        return None;
    }

    let info: TokenInfo = match resp.json().await {
        Ok(i) => i,
        Err(e) => {
            error!("[Gemini Auth] Failed to parse tokeninfo JSON: {}", e);
            return None;
        }
    };

    let is_valid = info.error.is_none()
        && (info.user_id.is_some() || info.sub.is_some() || info.email.is_some());

    if is_valid {
        info!(
            "[Gemini Auth] Successfully verified user - email: {:?}, sub: {:?}, user_id: {:?}",
            info.email, info.sub, info.user_id
        );
        // 10분(600,000ms) 동안 인메모리 캐시에 저장
        let mut cache = state.token_cache.write().await;
        cache.insert(token.to_string(), CachedToken::new(info.clone(), 600_000));
        Some(info)
    } else {
        warn!("[Gemini Auth] Token info response was invalid");
        None
    }
}

/// 단순 텍스트 응답 생성 헬퍼 함수
fn plain_text_response(msg: &str) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(msg.to_string()))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
