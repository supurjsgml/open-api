use std::sync::Arc;

use axum::extract::State;
use axum::Json;
use serde_json::Value;

use crate::common::dto::ApiBodyResponse;
use crate::common::util::api_res_util::ApiResUtil;
use crate::internal::dto::{DashboardStatsDTO, PageVisitsDTO, VisitorHitReqDTO};
use crate::AppState;

/// GET `/api/front/dashboardStats`
/// 주간 방문자 통계 및 트렌드 데이터를 조회합니다.
pub async fn get_dashboard_stats(
    State(state): State<Arc<AppState>>,
) -> Json<ApiBodyResponse<DashboardStatsDTO>> {
    let stats = state.internal_service.get_dashboard_stats().await;
    Json(ApiResUtil::success_data(stats))
}

/// GET `/api/front/pageStats`
/// 페이지별 누적 방문 횟수를 조회합니다.
pub async fn get_page_stats(
    State(state): State<Arc<AppState>>,
) -> Json<ApiBodyResponse<PageVisitsDTO>> {
    let stats = state.internal_service.get_page_visits().await;
    Json(ApiResUtil::success_data(stats))
}

/// POST `/api/front/hit`
/// 페이지 방문 및 신규 세션 방문자 카운트를 기록합니다.
pub async fn record_hit(
    State(state): State<Arc<AppState>>,
    Json(req_dto): Json<VisitorHitReqDTO>,
) -> Json<ApiBodyResponse<Value>> {
    state.internal_service.record_hit(req_dto).await;
    Json(ApiResUtil::success())
}
