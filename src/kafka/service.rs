use serde::Serialize;
use tracing::info;

use crate::member::dto::MemberReqDTO;

/// 메시지 브로커 프로듀서 서비스
#[derive(Clone, Default)]
pub struct KafkaProducerService {
    consumer: KafkaConsumerService,
}

impl KafkaProducerService {
    pub fn new() -> Self {
        Self {
            consumer: KafkaConsumerService::new(),
        }
    }

    /// 지정된 토픽으로 메시지를 발행합니다.
    pub async fn send_message<T: Serialize>(&self, topic: &str, message: &T) {
        let payload = serde_json::to_string(message).unwrap_or_default();
        info!("Kafka send -> topic: {}, payload: {}", topic, payload);

        if topic == "restApi" {
            if let Ok(dto) = serde_json::from_str::<MemberReqDTO>(&payload) {
                self.consumer.rest_api_consume(&dto).await;
            }
        } else if topic == "testJob" {
            self.consumer.quartz_job_consume(&payload).await;
        }
    }
}

/// 메시지 브로커 컨슈머 서비스
#[derive(Clone, Default)]
pub struct KafkaConsumerService;

impl KafkaConsumerService {
    pub fn new() -> Self {
        Self
    }

    /// `restApi` 토픽 메시지 수신 핸들러
    pub async fn rest_api_consume(&self, dto: &MemberReqDTO) {
        info!("Received Message in group restApi: {:?}", dto);
    }

    /// `testJob` 토픽 메시지 수신 핸들러
    pub async fn quartz_job_consume(&self, message: &str) {
        info!("Received Message in group testJob: {}", message);
    }
}
