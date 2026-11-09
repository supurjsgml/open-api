use std::{collections::HashMap, time::Duration};

use chrono::Utc;
use reqwest::Client;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::{Event, Subscriber, warn};
use tracing_subscriber::{Layer, layer::Context};

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

        let url = config.url.trim().to_string();
        let user = config.user.trim().to_string();
        let token = config.token.trim().to_string();
        let replica = resolve_pod_name(None);
        let batch_size = 50usize;
        let flush_interval = Duration::from_millis(1000);
        let (tx, mut rx) = mpsc::channel::<LogEntry>(2048);

        let client = Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        tokio::spawn(async move {
            let mut buffer: Vec<LogEntry> = Vec::with_capacity(batch_size);
            let mut ticker = tokio::time::interval(flush_interval);

            loop {
                tokio::select! {
                    maybe_entry = rx.recv() => {
                        match maybe_entry {
                            Some(entry) => {
                                buffer.push(entry);
                                if buffer.len() >= batch_size {
                                    let batch = std::mem::replace(&mut buffer, Vec::with_capacity(batch_size));
                                    Self::send_batch(&client, &url, &user, &token, &replica, batch).await;
                                }
                            }
                            None => {
                                if !buffer.is_empty() {
                                    let batch = std::mem::take(&mut buffer);
                                    Self::send_batch(&client, &url, &user, &token, &replica, batch).await;
                                }
                                break;
                            }
                        }
                    }
                    _ = ticker.tick() => {
                        if !buffer.is_empty() {
                            let batch = std::mem::replace(&mut buffer, Vec::with_capacity(batch_size));
                            Self::send_batch(&client, &url, &user, &token, &replica, batch).await;
                        }
                    }
                }
            }
        });

        Self { sender: Some(tx) }
    }

    pub fn append(&self, entry: LogEntry) {
        if let Some(tx) = &self.sender {
            let _ = tx.try_send(entry);
        }
    }

    async fn send_batch(
        client: &Client,
        url: &str,
        user: &str,
        token: &str,
        replica: &str,
        batch: Vec<LogEntry>,
    ) {
        let mut grouped: HashMap<String, Vec<[String; 2]>> = HashMap::new();

        for entry in batch {
            let ts_ns = (entry.timestamp_ms as i128 * 1_000_000).to_string();
            let formatted_time = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms)
                .unwrap_or_else(Utc::now)
                .format("%Y-%m-%d %H:%M:%S%.3f");
            let line = format!(
                "{} [{}] {:<5} {} - {}",
                formatted_time, entry.thread, entry.level, entry.logger, entry.message
            );
            grouped
                .entry(entry.level)
                .or_default()
                .push([ts_ns, line]);
        }

        let streams: Vec<Value> = grouped
            .into_iter()
            .map(|(level, values)| {
                json!({
                    "stream": {
                        "app": "rest-api",
                        "replica": replica,
                        "level": level
                    },
                    "values": values
                })
            })
            .collect();

        let payload = json!({ "streams": streams });

        match client
            .post(url)
            .basic_auth(user, Some(token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if !resp.status().is_success() => {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                eprintln!("Loki 로그 전송 실패: status={}, body={}", status, body);
            }
            Err(err) => {
                eprintln!("Loki 로그 전송 중 오류 발생: {}", err);
            }
            _ => {}
        }
    }
}

/// `tracing` 이벤트에서 `message` 필드 문자열을 추출하는 방문자(Visitor) 구조체
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

/// 모든 `tracing` 로그(info!, warn!, error!)를 가로채어 Axiom 및 Loki로 자동 전달하는 구독 레이어
pub struct RemoteLogLayer {
    axiom: AxiomBatchSender,
    loki: LokiSender,
}

impl RemoteLogLayer {
    pub fn new(axiom: AxiomBatchSender, loki: LokiSender) -> Self {
        Self { axiom, loki }
    }
}

impl<S: Subscriber> Layer<S> for RemoteLogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        // 로그 전송기 자체에서 발생하는 내부 로그는 무한 루프 방지를 위해 제외
        if metadata.target().starts_with("reqwest") || metadata.target().starts_with("hyper") {
            return;
        }

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        if visitor.message.is_empty() {
            return;
        }

        let thread_name = std::thread::current()
            .name()
            .unwrap_or("tokio-worker")
            .to_string();

        let entry = LogEntry {
            timestamp_ms: Utc::now().timestamp_millis(),
            level: metadata.level().to_string(),
            logger: metadata.target().to_string(),
            thread: thread_name,
            message: visitor.message,
            exception: None,
        };

        self.axiom.append(&entry);
        self.loki.append(entry);
    }
}
