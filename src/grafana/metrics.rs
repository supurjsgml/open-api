use std::env;
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Value, json};
use tracing::warn;

use crate::config::AppConfig;

/// 애플리케이션 시작 시각 (업타임 측정용)
static START_INSTANT: LazyLock<Instant> = LazyLock::new(Instant::now);

/// 애플리케이션 시작 유닉스 타임스탬프 (밀리초)
static START_EPOCH_MS: LazyLock<u64> = LazyLock::new(|| {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
});

/// 프로세스 최대 스레드 수 기록
static PEAK_THREADS: AtomicU64 = AtomicU64::new(0);

/// 이전 시점의 시스템 및 프로세스 CPU 틱(Jiffies) 샘플
#[derive(Debug, Clone, Copy)]
struct CpuSample {
    total_jiffies: u64,
    idle_jiffies: u64,
    proc_jiffies: u64,
}

static PREV_CPU_SAMPLE: LazyLock<Mutex<Option<CpuSample>>> = LazyLock::new(|| Mutex::new(None));

/// 스레드 상태 응답 구조체
#[derive(Debug, Clone, Serialize)]
pub struct ThreadStatusDto {
    pub pod: String,
    pub name: String,
    pub state: String,
    pub daemon: bool,
}

/// 런타임 메트릭 스냅샷 구조체
#[derive(Debug, Clone)]
pub struct RuntimeMetricsSnapshot {
    pub uptime_ms: f64,
    pub start_time_ms: f64,
    pub system_cpu_usage: f64,
    pub process_cpu_usage: f64,
    pub cpu_count: f64,
    pub heap_used_bytes: f64,
    pub heap_max_bytes: f64,
    pub nonheap_used_bytes: f64,
    pub nonheap_max_bytes: f64,
    pub live_threads: f64,
    pub peak_threads: f64,
    pub daemon_threads: f64,
    pub state_runnable: f64,
    pub state_blocked: f64,
    pub state_waiting: f64,
    pub state_timed_waiting: f64,
    pub state_new: f64,
    pub state_terminated: f64,
}

/// 현재 컨테이너(Pod/Replica) 이름을 환경변수에서 추출합니다.
pub fn resolve_pod_name(fallback: Option<&str>) -> String {
    env::var("CONTAINER_APP_REPLICA_NAME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| env::var("POD_NAME").ok().filter(|s| !s.trim().is_empty()))
        .or_else(|| env::var("HOSTNAME").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| fallback.unwrap_or("local-dev").to_string())
}

/// 애플리케이션 시작 후 경과 시간(초)을 반환합니다.
pub fn uptime_seconds() -> f64 {
    START_INSTANT.elapsed().as_secs_f64()
}

/// 현재 프로세스의 스레드 목록과 상태를 수집합니다.
pub fn collect_process_threads(pod_name: &str) -> Vec<ThreadStatusDto> {
    if let Ok(entries) = fs::read_dir("/proc/self/task") {
        let mut threads = Vec::new();
        for entry in entries.flatten() {
            let task_path = entry.path();
            let name = fs::read_to_string(task_path.join("comm"))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|_| "worker".to_string());

            let raw_state = fs::read_to_string(task_path.join("stat"))
                .ok()
                .and_then(|stat| {
                    let close_paren = stat.rfind(')')?;
                    stat.get(close_paren + 2..)?.chars().next()
                })
                .unwrap_or('R');

            let is_main = name == "open-api" || name == "open_api" || name == "main";
            let state = match raw_state {
                'R' => "RUNNABLE",
                'S' if is_main || name.starts_with("tokio-runtime") => "RUNNABLE",
                'S' => "TIMED_WAITING",
                'D' => "BLOCKED",
                'T' | 'I' => "WAITING",
                'Z' | 'X' => "TERMINATED",
                _ => "RUNNABLE",
            };

            threads.push(ThreadStatusDto {
                pod: pod_name.to_string(),
                name,
                state: state.to_string(),
                daemon: !is_main,
            });
        }

        if !threads.is_empty() {
            return threads;
        }
    }

    let worker_count = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    let mut threads = Vec::with_capacity(worker_count + 1);
    threads.push(ThreadStatusDto {
        pod: pod_name.to_string(),
        name: "main".to_string(),
        state: "RUNNABLE".to_string(),
        daemon: false,
    });

    for idx in 0..worker_count {
        threads.push(ThreadStatusDto {
            pod: pod_name.to_string(),
            name: format!("tokio-runtime-worker-{}", idx),
            state: "RUNNABLE".to_string(),
            daemon: true,
        });
    }

    threads
}

/// Linux `/proc` 파일시스템에서 CPU 틱 샘플을 읽어옵니다.
fn read_cpu_sample() -> Option<CpuSample> {
    let stat_content = fs::read_to_string("/proc/stat").ok()?;
    let cpu_line = stat_content.lines().find(|l| l.starts_with("cpu "))?;
    let parts: Vec<u64> = cpu_line
        .split_whitespace()
        .skip(1)
        .filter_map(|v| v.parse::<u64>().ok())
        .collect();

    if parts.len() < 4 {
        return None;
    }

    let total_jiffies: u64 = parts.iter().sum();
    let idle_jiffies = parts[3] + parts.get(4).copied().unwrap_or(0);

    let self_stat = fs::read_to_string("/proc/self/stat").ok()?;
    let close_paren = self_stat.rfind(')')?;
    let after_comm = self_stat.get(close_paren + 2..)?;
    let proc_parts: Vec<&str> = after_comm.split_whitespace().collect();
    let utime = proc_parts.get(11)?.parse::<u64>().ok()?;
    let stime = proc_parts.get(12)?.parse::<u64>().ok()?;

    Some(CpuSample {
        total_jiffies,
        idle_jiffies,
        proc_jiffies: utime + stime,
    })
}

/// 이전 샘플과의 차이(Delta)를 기반으로 시스템 및 프로세스 CPU 사용률(0.0 ~ 1.0)을 계산합니다.
fn sample_cpu_usage() -> (f64, f64) {
    let Some(curr) = read_cpu_sample() else {
        return (0.01, 0.005);
    };

    let mut guard = match PREV_CPU_SAMPLE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    let usage = if let Some(prev) = *guard {
        let delta_total = curr.total_jiffies.saturating_sub(prev.total_jiffies);
        let delta_idle = curr.idle_jiffies.saturating_sub(prev.idle_jiffies);
        let delta_proc = curr.proc_jiffies.saturating_sub(prev.proc_jiffies);

        if delta_total > 0 {
            let sys = ((delta_total.saturating_sub(delta_idle)) as f64 / delta_total as f64)
                .clamp(0.0, 1.0);
            let proc_cpu = (delta_proc as f64 / delta_total as f64).clamp(0.0, 1.0);
            (sys, proc_cpu)
        } else {
            (0.0, 0.0)
        }
    } else {
        (0.0, 0.0)
    };

    *guard = Some(curr);
    usage
}

/// 프로세스 메모리 사용량(RSS 및 코드/라이브러리 영역)과 컨테이너 메모리 한도를 바이트 단위로 조회합니다.
fn read_memory_usage() -> (f64, f64, f64) {
    let default_max_bytes = 1_073_741_824.0;
    let max_bytes = fs::read_to_string("/sys/fs/cgroup/memory.max")
        .or_else(|_| fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes"))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .filter(|&v| v > 0 && v <= 68_719_476_736)
        .map(|v| v as f64)
        .unwrap_or(default_max_bytes);

    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        let mut rss_kb = 0u64;
        let mut nonheap_kb = 0u64;

        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("VmRSS:") {
                rss_kb = rest
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0);
            } else if let Some(rest) = line
                .strip_prefix("VmExe:")
                .or_else(|| line.strip_prefix("VmLib:"))
                .or_else(|| line.strip_prefix("VmPTE:"))
            {
                let val = rest
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0);
                nonheap_kb += val;
            }
        }

        if rss_kb > 0 {
            return (
                (rss_kb * 1024) as f64,
                (nonheap_kb.max(1024) * 1024) as f64,
                max_bytes,
            );
        }
    }

    (26_214_400.0, 8_388_608.0, max_bytes)
}

/// 현재 시점의 런타임 메트릭 스냅샷을 생성합니다.
pub fn collect_runtime_metrics(pod_name: &str) -> RuntimeMetricsSnapshot {
    let uptime_ms = START_INSTANT.elapsed().as_millis() as f64;
    let start_time_ms = *START_EPOCH_MS as f64;
    let (system_cpu_usage, process_cpu_usage) = sample_cpu_usage();
    let cpu_count = std::thread::available_parallelism()
        .map(|n| n.get() as f64)
        .unwrap_or(1.0);

    let (heap_used_bytes, nonheap_used_bytes, heap_max_bytes) = read_memory_usage();
    let nonheap_max_bytes = 268_435_456.0;

    let threads = collect_process_threads(pod_name);
    let live_count = threads.len() as u64;
    let prev_peak = PEAK_THREADS.fetch_max(live_count, Ordering::Relaxed);
    let peak_count = prev_peak.max(live_count);
    let daemon_count = threads.iter().filter(|t| t.daemon).count() as u64;

    let mut state_runnable = 0.0;
    let mut state_blocked = 0.0;
    let mut state_waiting = 0.0;
    let mut state_timed_waiting = 0.0;
    let mut state_new = 0.0;
    let mut state_terminated = 0.0;

    for t in &threads {
        match t.state.as_str() {
            "RUNNABLE" => state_runnable += 1.0,
            "BLOCKED" => state_blocked += 1.0,
            "WAITING" => state_waiting += 1.0,
            "TIMED_WAITING" => state_timed_waiting += 1.0,
            "NEW" => state_new += 1.0,
            "TERMINATED" => state_terminated += 1.0,
            _ => state_runnable += 1.0,
        }
    }

    RuntimeMetricsSnapshot {
        uptime_ms,
        start_time_ms,
        system_cpu_usage,
        process_cpu_usage,
        cpu_count,
        heap_used_bytes,
        heap_max_bytes,
        nonheap_used_bytes,
        nonheap_max_bytes,
        live_threads: live_count as f64,
        peak_threads: peak_count as f64,
        daemon_threads: daemon_count as f64,
        state_runnable,
        state_blocked,
        state_waiting,
        state_timed_waiting,
        state_new,
        state_terminated,
    }
}

/// OTLP 속성 키-값 JSON 객체를 생성합니다.
fn kv_attr(key: &str, value: &str) -> Value {
    json!({
        "key": key,
        "value": { "stringValue": value }
    })
}

/// 단일 Gauge 데이터포인트를 생성합니다.
fn gauge_dp(time_nano: &str, value: f64, attrs: Vec<Value>) -> Value {
    json!({
        "timeUnixNano": time_nano,
        "asDouble": value,
        "attributes": attrs
    })
}

/// OTLP Gauge 메트릭 항목을 생성합니다.
fn build_gauge_metric(name: &str, unit: Option<&str>, data_points: Vec<Value>) -> Value {
    let mut obj = json!({
        "name": name,
        "gauge": {
            "dataPoints": data_points
        }
    });
    if let Some(u) = unit {
        if !u.is_empty() {
            obj["unit"] = Value::String(u.to_string());
        }
    }
    obj
}

/// Grafana Cloud OTLP 게이트웨이 규격의 메트릭 JSON 페이로드를 생성합니다.
pub fn build_otlp_metrics_payload(app_name: &str, pod_name: &str) -> Value {
    let snap = collect_runtime_metrics(pod_name);
    let now_nano = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string());
    let start_nano = ((*START_EPOCH_MS as u128) * 1_000_000).to_string();

    let base_attrs = || {
        vec![
            kv_attr("application", app_name),
            kv_attr("instance", pod_name),
            kv_attr("pod", pod_name),
        ]
    };

    let mem_attrs = |area: &str, id: &str| {
        let mut a = base_attrs();
        a.push(kv_attr("area", area));
        a.push(kv_attr("id", id));
        a
    };

    let state_attrs = |state: &str| {
        let mut a = base_attrs();
        a.push(kv_attr("state", state));
        a
    };

    let metrics = vec![
        build_gauge_metric(
            "process.uptime",
            Some("milliseconds"),
            vec![gauge_dp(&now_nano, snap.uptime_ms, base_attrs())],
        ),
        build_gauge_metric(
            "process.start.time",
            Some("milliseconds"),
            vec![gauge_dp(&now_nano, snap.start_time_ms, base_attrs())],
        ),
        build_gauge_metric(
            "system.cpu.usage",
            None,
            vec![gauge_dp(&now_nano, snap.system_cpu_usage, base_attrs())],
        ),
        build_gauge_metric(
            "process.cpu.usage",
            None,
            vec![gauge_dp(&now_nano, snap.process_cpu_usage, base_attrs())],
        ),
        build_gauge_metric(
            "system.cpu.count",
            None,
            vec![gauge_dp(&now_nano, snap.cpu_count, base_attrs())],
        ),
        build_gauge_metric(
            "jvm.memory.used",
            Some("bytes"),
            vec![
                gauge_dp(
                    &now_nano,
                    snap.heap_used_bytes,
                    mem_attrs("heap", "G1 Old Gen"),
                ),
                gauge_dp(&now_nano, 0.0, mem_attrs("heap", "G1 Eden Space")),
                gauge_dp(
                    &now_nano,
                    snap.nonheap_used_bytes,
                    mem_attrs("nonheap", "Metaspace"),
                ),
            ],
        ),
        build_gauge_metric(
            "jvm.memory.committed",
            Some("bytes"),
            vec![
                gauge_dp(
                    &now_nano,
                    snap.heap_used_bytes,
                    mem_attrs("heap", "G1 Old Gen"),
                ),
                gauge_dp(&now_nano, 0.0, mem_attrs("heap", "G1 Eden Space")),
                gauge_dp(
                    &now_nano,
                    snap.nonheap_used_bytes,
                    mem_attrs("nonheap", "Metaspace"),
                ),
            ],
        ),
        build_gauge_metric(
            "jvm.memory.max",
            Some("bytes"),
            vec![
                gauge_dp(
                    &now_nano,
                    snap.heap_max_bytes,
                    mem_attrs("heap", "G1 Old Gen"),
                ),
                gauge_dp(&now_nano, 0.0, mem_attrs("heap", "G1 Eden Space")),
                gauge_dp(
                    &now_nano,
                    snap.nonheap_max_bytes,
                    mem_attrs("nonheap", "Metaspace"),
                ),
            ],
        ),
        build_gauge_metric(
            "jvm.gc.live.data.size",
            Some("bytes"),
            vec![gauge_dp(&now_nano, snap.heap_used_bytes, base_attrs())],
        ),
        build_gauge_metric(
            "jvm.gc.max.data.size",
            Some("bytes"),
            vec![gauge_dp(&now_nano, snap.heap_max_bytes, base_attrs())],
        ),
        build_gauge_metric(
            "jvm.gc.overhead",
            Some("percent"),
            vec![gauge_dp(&now_nano, 0.0, base_attrs())],
        ),
        json!({
            "name": "jvm.gc.cpu.time",
            "unit": "ns",
            "sum": {
                "aggregationTemporality": 2,
                "isMonotonic": true,
                "dataPoints": [
                    {
                        "startTimeUnixNano": start_nano,
                        "timeUnixNano": now_nano,
                        "asDouble": 0.0,
                        "attributes": base_attrs()
                    }
                ]
            }
        }),
        build_gauge_metric(
            "jvm.threads.live",
            Some("threads"),
            vec![gauge_dp(&now_nano, snap.live_threads, base_attrs())],
        ),
        build_gauge_metric(
            "jvm.threads.peak",
            Some("threads"),
            vec![gauge_dp(&now_nano, snap.peak_threads, base_attrs())],
        ),
        build_gauge_metric(
            "jvm.threads.daemon",
            Some("threads"),
            vec![gauge_dp(&now_nano, snap.daemon_threads, base_attrs())],
        ),
        build_gauge_metric(
            "jvm.threads.states",
            Some("threads"),
            vec![
                gauge_dp(&now_nano, snap.state_runnable, state_attrs("runnable")),
                gauge_dp(&now_nano, snap.state_blocked, state_attrs("blocked")),
                gauge_dp(&now_nano, snap.state_waiting, state_attrs("waiting")),
                gauge_dp(
                    &now_nano,
                    snap.state_timed_waiting,
                    state_attrs("timed-waiting"),
                ),
                gauge_dp(&now_nano, snap.state_new, state_attrs("new")),
                gauge_dp(&now_nano, snap.state_terminated, state_attrs("terminated")),
            ],
        ),
        build_gauge_metric(
            "jvm.classes.loaded",
            Some("classes"),
            vec![gauge_dp(&now_nano, 0.0, base_attrs())],
        ),
    ];

    json!({
        "resourceMetrics": [
            {
                "resource": {
                    "attributes": [
                        kv_attr("service.name", app_name),
                        kv_attr("telemetry.sdk.name", "io.micrometer"),
                        kv_attr("telemetry.sdk.language", "rust")
                    ]
                },
                "scopeMetrics": [
                    {
                        "metrics": metrics
                    }
                ]
            }
        ]
    })
}

/// Grafana Cloud OTLP 엔드포인트로 주기적으로 메트릭을 푸시하는 백그라운드 태스크를 시작합니다.
pub fn start_otlp_metrics_exporter(http_client: reqwest::Client, config: &AppConfig) {
    if !config.grafana_otlp.enabled || config.grafana_otlp.token.trim().is_empty() {
        return;
    }

    let url = config.grafana_otlp.url.clone();
    let raw_token = config.grafana_otlp.token.trim().to_string();
    let auth_header = if raw_token.starts_with("Basic ") || raw_token.starts_with("Bearer ") {
        raw_token
    } else {
        format!("Basic {}", raw_token)
    };
    let step_secs = config.grafana_otlp.step_secs.max(10);
    let app_name = config.app_name.clone();
    let pod_name = resolve_pod_name(Some("unknown-pod"));

    // 초기 CPU 틱 샘플 기준점 기록
    let _ = sample_cpu_usage();

    tokio::spawn(async move {
        // 컨테이너 기동 직후 대시보드에 파드명이 빠르게 노출되도록 5초 후 첫 푸시 수행
        tokio::time::sleep(Duration::from_secs(5)).await;
        let mut interval = tokio::time::interval(Duration::from_secs(step_secs));

        loop {
            interval.tick().await;
            let payload = build_otlp_metrics_payload(&app_name, &pod_name);

            let res = http_client
                .post(&url)
                .header("Authorization", &auth_header)
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await;

            match res {
                Ok(resp) if !resp.status().is_success() => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    warn!(
                        "Grafana OTLP metrics push failed: status={}, body={}",
                        status, body
                    );
                }
                Err(e) => {
                    warn!("Grafana OTLP metrics push error: {}", e);
                }
                _ => {}
            }
        }
    });
}
