/// 파일 생성 기본 경로 상수를 보관하는 구조체
#[derive(Debug, Clone)]
pub struct CommonConstants {
    pub file_path: String,
}

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub base_url: String,
}

#[derive(Debug, Clone)]
pub struct ApiEndpointConfig {
    pub base_url: String,
    pub memo: String,
}

#[derive(Debug, Clone)]
pub struct AuthEndpointConfig {
    pub base_url: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct KakaoConfig {
    pub v2: String,
    pub api: ApiEndpointConfig,
    pub auth: AuthEndpointConfig,
}

#[derive(Debug, Clone)]
pub struct GeminiEndpointConfig {
    pub base_url: String,
    pub generate_content: String,
}

/// 외부 연동 서비스(배치, 카카오, 제미나이) 엔드포인트 URL 설정 구조체
#[derive(Debug, Clone)]
pub struct RestApiProperties {
    pub base_url: String,
    pub batch: BatchConfig,
    pub kakao: KakaoConfig,
    pub gemini: GeminiEndpointConfig,
}

#[derive(Debug, Clone)]
pub struct KakaoKeyConfig {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
}

/// 외부 API 인증 키 설정 구조체
#[derive(Debug, Clone)]
pub struct KeyConfig {
    pub kakao: KakaoKeyConfig,
    pub gemini_api_key: String,
}
