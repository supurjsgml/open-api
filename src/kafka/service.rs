use serde::Serialize;
use tracing::info;

use crate::member::dto::MemberReqDTO;

/// 지정된 토픽으로 메시지를 발행합니다.
pub async fn send_message<T: Serialize>(topic: &str, message: &T) {
    let payload = serde_json::to_string(message).unwrap_or_default();
    info!("Kafka send -> topic: {}, payload: {}", topic, payload);

    if topic == "restApi" {
        if let Ok(dto) = serde_json::from_str::<MemberReqDTO>(&payload) {
            rest_api_consume(&dto).await;
        }
    } else if topic == "testJob" {
        quartz_job_consume(&payload).await;
    }
}

/// `restApi` 토픽 메시지 수신 핸들러
pub async fn rest_api_consume(dto: &MemberReqDTO) {
    info!("Received Message in group restApi: {:?}", dto);
}

/// `testJob` 토픽 메시지 수신 핸들러
pub async fn quartz_job_consume(message: &str) {
    info!("Received Message in group testJob: {}", message);
}
