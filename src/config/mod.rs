use std::env;

use crate::common::constants::{
    ApiEndpointConfig, AuthEndpointConfig, BatchConfig, CommonConstants, GeminiEndpointConfig,
    KakaoConfig, KakaoKeyConfig, KeyConfig, RestApiProperties,
};

/// Redis 접속 정보를 담는 구조체
#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub ssl_enabled: bool,
}

impl RedisConfig {
    /// TLS(rediss://) 또는 일반(redis://) 연결 문자열 생성
    pub fn connection_url(&self) -> String {
        let scheme = if self.ssl_enabled { "rediss" } else { "redis" };
        if self.password.is_empty() {
            format!("{}://{}:{}", scheme, self.host, self.port)
        } else {
            format!(
                "{}://default:{}@{}:{}",
                scheme,
                urlencoding::encode(&self.password),
                self.host,
                self.port
            )
        }
    }
}

/// Grafana Loki 로그 전송 설정
#[derive(Debug, Clone)]
pub struct LokiConfig {
    pub url: String,
    pub user: String,
    pub token: String,
}

/// Axiom 비동기 배치 로그 전송 설정
#[derive(Debug, Clone)]
pub struct AxiomConfig {
    pub api_url: String,
    pub dataset: String,
    pub token: String,
    pub app: String,
    pub batch_size: usize,
    pub flush_interval_ms: u64,
    pub max_queue_size: usize,
}

/// Kafka 브로커 및 컨슈머 그룹 설정
#[derive(Debug, Clone)]
pub struct KafkaSettings {
    pub bootstrap_servers: String,
    pub group_id: String,
    pub client_id: String,
}

/// 애플리케이션 전체 환경 설정을 통합 관리하는 구조체
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_name: String,
    pub profile: String,
    pub port: u16,
    pub redis: RedisConfig,
    pub rest_api: RestApiProperties,
    pub keys: KeyConfig,
    pub common_constants: CommonConstants,
    pub admin_emails: Vec<String>,
    pub daily_limit: i64,
    pub kafka: KafkaSettings,
    pub loki: LokiConfig,
    pub axiom: AxiomConfig,
    pub allowed_origins: Vec<String>,
}

impl AppConfig {
    /// 환경변수 및 실행 프로필(local, dev, prod)별 기본값을 읽어 설정 객체를 초기화
    pub fn from_env() -> Self {
        // 루트 경로에 .env 파일이 있으면 환경변수로 로드하고, 없으면 무시
        let _ = dotenvy::dotenv();

        // 활성 프로필 결정 (환경변수가 없으면 local 사용)
        let profile = env::var("SPRING_PROFILES_ACTIVE")
            .or_else(|_| env::var("APP_PROFILE"))
            .unwrap_or_else(|_| "local".to_string());

        let port = env::var("port")
            .or_else(|_| env::var("PORT"))
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(8080);

        let redis_host = env::var("REDIS_HOST")
            .unwrap_or_else(|_| "localhost".to_string());
        let redis_port = env::var("REDIS_PORT")
            .ok()
            .and_then(|v| v.parse::<u16>().ok())
            .unwrap_or(6379);
        let redis_password = env::var("REDIS_TOKEN").unwrap_or_default();
        let redis_ssl = env::var("REDIS_SSL_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(true);

        // 프로필(local / dev / prod)에 따른 기본 도메인 및 업로드 경로 분기
        let (base_url, batch_base_url, default_file_path) = match profile.as_str() {
            "prod" => (
                "https://guney-api.kro.kr".to_string(),
                "https://guney-batch.kro.kr".to_string(),
                "/app/upload/".to_string(),
            ),
            "dev" => (
                "https://guney.kro.kr".to_string(),
                "https://guney-batch.kro.kr".to_string(),
                "/app/upload/".to_string(),
            ),
            _ => (
                "http://localhost:8080".to_string(),
                "http://localhost:8081".to_string(),
                "D:/app/upload/".to_string(),
            ),
        };

        let base_url = env::var("REST_API_BASE_URL").unwrap_or(base_url);
        let batch_base_url = env::var("REST_API_BATCH_BASE_URL").unwrap_or(batch_base_url);
        let file_path = env::var("FILE_PATH").unwrap_or(default_file_path);

        let kakao_api_base = "https://kapi.kakao.com".to_string();
        let kakao_v2 = "/v2".to_string();
        let kakao_memo = format!("{}{}/api/talk/memo/default/send", kakao_api_base, kakao_v2);
        let kakao_auth_base = "https://kauth.kakao.com".to_string();
        let kakao_token_url = format!("{}/oauth/token", kakao_auth_base);

        let gemini_base = "https://generativelanguage.googleapis.com/v1beta/models".to_string();
        let gemini_generate = format!("{}/{{model}}:generateContent", gemini_base);

        let rest_api = RestApiProperties {
            base_url,
            batch: BatchConfig {
                base_url: batch_base_url,
            },
            kakao: KakaoConfig {
                v2: kakao_v2,
                api: ApiEndpointConfig {
                    base_url: kakao_api_base,
                    memo: kakao_memo,
                },
                auth: AuthEndpointConfig {
                    base_url: kakao_auth_base,
                    token: kakao_token_url,
                },
            },
            gemini: GeminiEndpointConfig {
                base_url: gemini_base,
                generate_content: gemini_generate,
            },
        };

        // local 프로필일 때는 기본 키 값을 폴백으로 사용
        let is_local = profile == "local";
        let kakao_client_id = env::var("KAKAO_CLIENT_ID")
            .unwrap_or_else(|_| if is_local { "뭐게요".to_string() } else { String::new() });
        let kakao_client_secret = env::var("KAKAO_CLIENT_SECRET")
            .unwrap_or_else(|_| if is_local { "뭘까".to_string() } else { String::new() });
        let kakao_refresh_token = env::var("KAKAO_REFRESH_TOKEN").unwrap_or_else(|_| {
            if is_local {
                "궁굼하지롱".to_string()
            } else {
                String::new()
            }
        });
        let gemini_api_key = env::var("GEMINI_API_KEY").unwrap_or_default();

        // 콤마(,)로 구분된 관리자 이메일 목록 파싱 (일일 한도 제한 면제 대상)
        let admin_emails = env::var("APP_ADMIN_EMAILS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();

        let daily_limit = env::var("APP_DAILY_LIMIT")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(50);

        // CORS 및 Origin 검증 시 허용할 도메인 및 크롬 확장 프로그램 목록
        let allowed_origins = vec![
            "https://guney.kro.kr".to_string(),
            "https://guney-api.kro.kr".to_string(),
            "https://xn--v69a93jfng.xn--yq5b.xn--3e0b707e".to_string(),
            "http://xn--v69a93jfng.xn--hk3b17f.xn--3e0b707e".to_string(),
            "https://xn--v69a93jfng.xn--hk3b17f.xn--3e0b707e".to_string(),
            "chrome-extension://ilaafkabnbkdgaieedckdnfkmapocimc".to_string(),
            "chrome-extension://chjbcemdkiommdpeklplkbfpemefejcp".to_string(),
            "chrome-extension://hecddnbbnkjpjajhmdoldmimmpjghaic".to_string(),
            "chrome-extension://cigmfccgmaeohgblgnpfcheefkpockeo".to_string(),
            "http://localhost:8080".to_string(),
        ];

        Self {
            app_name: "rest-api".to_string(),
            profile,
            port,
            redis: RedisConfig {
                host: redis_host,
                port: redis_port,
                password: redis_password,
                ssl_enabled: redis_ssl,
            },
            rest_api,
            keys: KeyConfig {
                kakao: KakaoKeyConfig {
                    client_id: kakao_client_id,
                    client_secret: kakao_client_secret,
                    refresh_token: kakao_refresh_token,
                },
                gemini_api_key,
            },
            common_constants: CommonConstants { file_path },
            admin_emails,
            daily_limit,
            kafka: KafkaSettings {
                bootstrap_servers: env::var("KAFKA_BOOTSTRAP_SERVERS")
                    .unwrap_or_else(|_| "localhost:9092".to_string()),
                group_id: "guney-consumer".to_string(),
                client_id: "api-server".to_string(),
            },
            loki: LokiConfig {
                url: env::var("LOKI_URL").unwrap_or_else(|_| {
                    "https://logs-prod-030.grafana.net/loki/api/v1/push".to_string()
                }),
                user: env::var("LOKI_USER").unwrap_or_default(),
                token: env::var("LOKI_TOKEN").unwrap_or_default(),
            },
            axiom: AxiomConfig {
                api_url: env::var("AXIOM_API_URL")
                    .unwrap_or_else(|_| "https://api.axiom.co/v1/datasets/".to_string()),
                dataset: env::var("AXIOM_DATASET").unwrap_or_else(|_| "rest-api".to_string()),
                token: env::var("AXIOM_TOKEN").unwrap_or_default(),
                app: "rest-api".to_string(),
                batch_size: 50,
                flush_interval_ms: 1000,
                max_queue_size: 2048,
            },
            allowed_origins,
        }
    }
}
