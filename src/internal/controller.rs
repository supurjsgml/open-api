use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::Value;

use crate::AppState;
use crate::common::dto::ApiBodyResponse;
use crate::common::util::api_res_util;
use crate::internal::dto::{DashboardStatsDTO, PageVisitsDTO, VisitorHitReqDTO};
use crate::internal::service;

/// 대시보드 및 방문자 통계 도메인 라우터를 생성합니다.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/front/dashboardStats", get(get_dashboard_stats))
        .route("/api/front/pageStats", get(get_page_stats))
        .route("/api/front/hit", post(record_hit))
}

/// GET `/api/front/dashboardStats`
/// 주간 방문자 통계 및 트렌드 데이터를 조회합니다.
pub async fn get_dashboard_stats(
    State(state): State<Arc<AppState>>,
) -> Json<ApiBodyResponse<DashboardStatsDTO>> {
    let stats = service::get_dashboard_stats(&state).await;
    Json(api_res_util::success_data(stats))
}

/// GET `/api/front/pageStats`
/// 페이지별 누적 방문 횟수를 조회합니다.
pub async fn get_page_stats(
    State(state): State<Arc<AppState>>,
) -> Json<ApiBodyResponse<PageVisitsDTO>> {
    let stats = service::get_page_visits(&state).await;
    Json(api_res_util::success_data(stats))
}

/// POST `/api/front/hit`
/// 페이지 방문 및 신규 세션 방문자 카운트를 기록합니다.
pub async fn record_hit(
    State(state): State<Arc<AppState>>,
    Json(req_dto): Json<VisitorHitReqDTO>,
) -> Json<ApiBodyResponse<Value>> {
    service::record_hit(&state, req_dto).await;
    Json(api_res_util::success())
}
