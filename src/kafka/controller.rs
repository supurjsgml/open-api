use std::sync::Arc;

use axum::extract::Query;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use crate::AppState;
use crate::common::dto::{ApiBodyResponse, HeaderSuccess};
use crate::common::util::api_res_util;
use crate::kafka::service;
use crate::member::dto::MemberReqDTO;

/// 메시지 브로커 도메인 라우터를 생성합니다.
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/kafka/send", get(send_msg))
}

/// `/kafka/send` 쿼리 파라미터
#[derive(Debug, Deserialize)]
pub struct KafkaSendQuery {
    pub msg: Option<String>,
}

/// GET `/kafka/send`
/// 쿼리 파라미터로 전달받은 메시지를 `restApi` 토픽으로 발행하고 결과를 반환합니다.
pub async fn send_msg(
    Query(query): Query<KafkaSendQuery>,
) -> Json<ApiBodyResponse<Option<String>>> {
    let dto = MemberReqDTO {
        id: query.msg.clone(),
        pw: Some("???".to_string()),
        token: None,
    };

    service::send_message("restApi", &dto).await;

    Json(api_res_util::success_with_header(
        query.msg,
        HeaderSuccess::default(),
        "SUCCESS",
    ))
}
