use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use reqwest::Client;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::warn;

use crate::{
    config::{AxiomConfig, LokiConfig},
    grafana::metrics::resolve_pod_name,
};

/// 외부 로그 수집기(Axiom, Loki)로 전송할 단일 로그 이벤트 데이터
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp_ms: i64,
    pub level: String,
    pub logger: String,
    pub thread: String,
    pub message: String,
    pub exception: Option<String>,
}

/// Axiom 비동기 JSON 배치 전송기
/// 비동기 채널(mpsc)과 백그라운드 태스크를 사용하여 메인 요청 지연을 방지하고 배치 단위로 묶어 전송합니다.
#[derive(Clone)]
pub struct AxiomBatchSender {
    sender: Option<mpsc::Sender<MapEntry>>,
}

type MapEntry = HashMap<String, Value>;

impl AxiomBatchSender {
    pub fn start(config: &AxiomConfig) -> Self {
        if config.token.trim().is_empty() {
            return Self { sender: None };
        }

        let replica = resolve_pod_name(None);
        let max_queue_size = config.max_queue_size.max(1);
        let batch_size = config.batch_size.max(1);
        let flush_interval = Duration::from_millis(config.flush_interval_ms.max(100));
        let full_url = if config.api_url.ends_with('/') {
            format!("{}{}/ingest", config.api_url, config.dataset)
        } else {
            format!("{}/{}/ingest", config.api_url, config.dataset)
        };
        let token = config.token.clone();
        let app_name = config.app.clone();

        let (tx, mut rx) = mpsc::channel::<MapEntry>(max_queue_size);

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        // 백그라운드에서 큐에 쌓인 로그를 일정 개수(batch_size) 또는 일정 주기(flush_interval)마다 전송
        tokio::spawn(async move {
            let mut buffer: Vec<MapEntry> = Vec::with_capacity(batch_size);
            let mut ticker = tokio::time::interval(flush_interval);

            loop {
                tokio::select! {
                    maybe_item = rx.recv() => {
                        match maybe_item {
                            Some(mut item) => {
                                item.entry("app".to_string()).or_insert_with(|| Value::String(app_name.clone()));
                                item.entry("replica".to_string()).or_insert_with(|| Value::String(replica.clone()));
                                buffer.push(item);
                                if buffer.len() >= batch_size {
                                    let batch = std::mem::replace(&mut buffer, Vec::with_capacity(batch_size));
                                    Self::send_batch(&client, &full_url, &token, batch).await;
                                }
                            }
                            None => {
                                // 채널이 닫히면(서버 종료 시) 남은 버퍼를 모두 플러시하고 루프를 종료
                                if !buffer.is_empty() {
                                    let batch = std::mem::take(&mut buffer);
                                    Self::send_batch(&client, &full_url, &token, batch).await;
                                }
                                break;
                            }
                        }
                    }
                    _ = ticker.tick() => {
                        if !buffer.is_empty() {
                            let batch = std::mem::replace(&mut buffer, Vec::with_capacity(batch_size));
                            Self::send_batch(&client, &full_url, &token, batch).await;
                        }
                    }
                }
            }
        });

        Self { sender: Some(tx) }
    }

    pub fn append(&self, entry: &LogEntry) {
        let Some(tx) = &self.sender else {
            return;
        };

        let mut map = HashMap::new();
        let iso_time = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
            .unwrap_or_else(Utc::now)
            .to_rfc3339();
        map.insert("_time".to_string(), Value::String(iso_time));
        map.insert("level".to_string(), Value::String(entry.level.clone()));
        map.insert("logger".to_string(), Value::String(entry.logger.clone()));
        map.insert("thread".to_string(), Value::String(entry.thread.clone()));
        map.insert("message".to_string(), Value::String(entry.message.clone()));

        if let Some(ex) = &entry.exception {
            map.insert("exception".to_string(), Value::String(ex.clone()));
        }

        let _ = tx.try_send(map);
    }

    async fn send_batch(client: &Client, full_url: &str, token: &str, batch: Vec<MapEntry>) {
        if let Err(err) = client
            .post(full_url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&batch)
            .send()
            .await
        {
            warn!("Axiom 로그 전송 중 오류 발생: {}", err);
        }
    }
}

/// Grafana Loki 비동기 로그 전송기
#[derive(Clone)]
pub struct LokiSender {
    sender: Option<mpsc::Sender<LogEntry>>,
}

impl LokiSender {
    pub fn start(config: &LokiConfig) -> Self {
        if config.token.trim().is_empty() || config.user.trim().is_empty() {
            return Self { sender: None };
        }

        let url = config.url.clone();
        let user = config.user.clone();
        let token = config.token.clone();
        let replica = resolve_pod_name(None);
        let (tx, mut rx) = mpsc::channel::<LogEntry>(1024);

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        tokio::spawn(async move {
            while let Some(entry) = rx.recv().await {
                let ts_ns = (entry.timestamp_ms as i128 * 1_000_000).to_string();
                let line = format!(
                    "[{}] {:<5} {} - {}",
                    entry.thread, entry.level, entry.logger, entry.message
                );
                let payload = json!({
                    "streams": [
                        {
                            "stream": {
                                "app": "rest-api",
                                "replica": replica,
                                "level": entry.level
                            },
                            "values": [
                                [ts_ns, line]
                            ]
                        }
                    ]
                });

                let _ = client
                    .post(&url)
                    .basic_auth(&user, Some(&token))
                    .json(&payload)
                    .send()
                    .await;
            }
        });

        Self { sender: Some(tx) }
    }

    pub fn append(&self, entry: LogEntry) {
        if let Some(tx) = &self.sender {
            let _ = tx.try_send(entry);
        }
    }
}
