use std::sync::Arc;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::common::dto::{ApiBodyResponse, HeaderSuccess};
use crate::common::util::api_res_util::ApiResUtil;
use crate::member::dto::MemberReqDTO;
use crate::AppState;

/// `/kafka/send` 쿼리 파라미터
#[derive(Debug, Deserialize)]
pub struct KafkaSendQuery {
    pub msg: Option<String>,
}

/// GET `/kafka/send`
/// 쿼리 파라미터로 전달받은 메시지를 `restApi` 토픽으로 발행하고 결과를 반환합니다.
pub async fn send_msg(
    State(state): State<Arc<AppState>>,
    Query(query): Query<KafkaSendQuery>,
) -> Json<ApiBodyResponse<Option<String>>> {
    let dto = MemberReqDTO {
        id: query.msg.clone(),
        pw: Some("???".to_string()),
        token: None,
    };

    state.kafka_producer.send_message("restApi", &dto).await;

    Json(ApiResUtil::success_with_header(
        query.msg,
        HeaderSuccess::default(),
        "SUCCESS",
    ))
}
