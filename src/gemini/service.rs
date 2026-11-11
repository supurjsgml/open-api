use std::cmp::Ordering;
use std::sync::LazyLock;
use std::time::Duration;

use futures_util::StreamExt;
use regex::Regex;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};

use crate::AppState;
use crate::gemini::dto::{
    GeminiApiRequest, GeminiApiResponse, GeminiModelInfo, GeminiModelListResponse,
    GeminiModelResDTO, GeminiReqDTO, Part,
};

/// 버전 번호 추출용 정규표현식 (예: gemini-3.7-flash -> 3.7)
static VERSION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"gemini-(\d+(?:\.\d+)?)").expect("Invalid regex"));

/// 멀티모달 스트리밍 요청 타임아웃 (180초)
const STREAM_REQUEST_TIMEOUT_SECS: u64 = 180;
/// 프롬프트 미입력 및 이미지 첨부 시 사용할 기본 프롬프트
const DEFAULT_IMAGE_PROMPT: &str = "첨부된 이미지를 분석하고 핵심 내용을 설명해 주세요.";
/// 프롬프트 미입력 및 텍스트 단독 요약 시 사용할 기본 프롬프트
const DEFAULT_TEXT_PROMPT: &str = "다음 웹페이지 내용을 핵심 위주로 요약해 주세요.";

/// Data URL 접두어(`data:...;base64,`)가 포함된 경우 순수 Base64 구간만 잘라 반환합니다.
pub fn strip_base64_prefix_str(data: &str) -> &str {
    let trimmed = data.trim();
    if trimmed.len() >= 5 && trimmed[..5].eq_ignore_ascii_case("data:") {
        if let Some((_, b64)) = trimmed.split_once(";base64,") {
            return b64.trim();
        }
        if let Some((_, b64)) = trimmed.split_once(',') {
            return b64.trim();
        }
    }
    trimmed
}

/// 소유권을 유지한 채 불필요한 재할당 없이 순수 Base64 문자열로 정규화합니다.
fn normalize_base64_owned(data: String) -> String {
    let stripped = strip_base64_prefix_str(&data);
    if stripped.len() == data.len() {
        data
    } else {
        stripped.to_string()
    }
}

/// 요청 DTO의 소유권을 이동 받아 멀티모달 `GeminiApiRequest` 페이로드를 조립합니다.
pub fn build_multimodal_request(req_dto: GeminiReqDTO) -> GeminiApiRequest {
    let GeminiReqDTO {
        text,
        prompt,
        images,
        ..
    } = req_dto;

    let image_list = images.unwrap_or_default();
    let mut parts = Vec::with_capacity(image_list.len() + 1);

    for img in image_list {
        let mime_type = img.mime_type.trim().to_ascii_lowercase();
        let clean_data = normalize_base64_owned(img.data);
        parts.push(Part::inline_data(mime_type, clean_data));
    }

    let prompt_str = prompt.as_deref().map(str::trim).unwrap_or("");
    let text_str = text.as_deref().map(str::trim).unwrap_or("");

    let base_prompt = if !prompt_str.is_empty() {
        prompt_str
    } else if !parts.is_empty() {
        DEFAULT_IMAGE_PROMPT
    } else {
        DEFAULT_TEXT_PROMPT
    };

    let combined_prompt = if !text_str.is_empty() {
        format!("{}\n\n[웹페이지 본문 내용]\n{}", base_prompt, text_str)
    } else {
        base_prompt.to_string()
    };

    parts.push(Part::text(combined_prompt));
    GeminiApiRequest::from_parts(parts)
}

/// 본문·프롬프트·첨부 이미지를 결합하여 Gemini SSE 스트리밍 결과를 비동기 스트림(`ReceiverStream`)으로 반환합니다.
pub async fn get_summary(
    state: &AppState,
    req_dto: GeminiReqDTO,
) -> ReceiverStream<Result<String, std::convert::Infallible>> {
    let (tx, rx) = mpsc::channel(32);

    let model = match req_dto.model.as_deref() {
        Some(m) if !m.trim().is_empty() => m.trim().to_string(),
        _ => "gemini-2.5-flash".to_string(),
    };

    // 실시간 토큰 수신을 위한 SSE(`alt=sse`) 스트리밍 엔드포인트 URL 구성
    let stream_url = format!(
        "{}?alt=sse&key={}",
        state
            .config
            .rest_api
            .gemini
            .generate_content
            .replace("{model}", &model)
            .replace("generateContent", "streamGenerateContent"),
        state.config.keys.gemini_api_key
    );

    let request_body = build_multimodal_request(req_dto);
    let client = state.http_client.clone();

    tokio::spawn(async move {
        let response_result = client
            .post(&stream_url)
            .timeout(Duration::from_secs(STREAM_REQUEST_TIMEOUT_SECS))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

        // 요청 바디 직렬화 및 전송 완료 즉시 대용량 Base64 버퍼 해제
        drop(request_body);

        match response_result {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let err_body = response.text().await.unwrap_or_default();
                    error!(
                        "Gemini API Streaming Error Status: {}, Body: {}",
                        status, err_body
                    );
                    let _ = tx
                        .send(Ok(format!(
                            "ERROR: AI 서비스와의 통신 중 오류가 발생했습니다: HTTP {}",
                            status
                        )))
                        .await;
                    return;
                }

                // 바이트 버퍼(`Vec<u8>`) 단위로 줄바꿈 경계를 분리하여 멀티바이트 UTF-8 깨짐 없이 즉시 파싱
                let mut byte_stream = response.bytes_stream();
                let mut raw_buffer: Vec<u8> = Vec::with_capacity(4096);

                while let Some(chunk_res) = byte_stream.next().await {
                    match chunk_res {
                        Ok(bytes) => {
                            raw_buffer.extend_from_slice(&bytes);
                            consume_sse_lines(&mut raw_buffer, &tx).await;
                        }
                        Err(e) => {
                            error!("Gemini API Stream Chunk Error: {}", e);
                            let _ = tx
                                .send(Ok(format!(
                                    "ERROR: AI 서비스와의 통신 중 오류가 발생했습니다: {}",
                                    e
                                )))
                                .await;
                            return;
                        }
                    }
                }

                // 스트림 종료 후 줄바꿈 없이 남아 있는 잔여 바이트 처리
                if !raw_buffer.is_empty() {
                    flush_sse_buffer(&mut raw_buffer, &tx).await;
                }
            }
            Err(e) => {
                error!("Gemini API Request Error: {}", e);
                let _ = tx
                    .send(Ok(format!(
                        "ERROR: AI 서비스와의 통신 중 오류가 발생했습니다: {}",
                        e
                    )))
                    .await;
            }
        }
    });

    ReceiverStream::new(rx)
}

/// 바이트 버퍼에서 완성된 개행(`\n`) 단위 SSE 라인들을 순차 추출하여 즉시 전송합니다.
pub async fn consume_sse_lines(
    raw_buffer: &mut Vec<u8>,
    tx: &mpsc::Sender<Result<String, std::convert::Infallible>>,
) {
    while let Some(newline_pos) = raw_buffer.iter().position(|&b| b == b'\n') {
        let line = String::from_utf8_lossy(&raw_buffer[..newline_pos])
            .trim()
            .to_string();
        raw_buffer.drain(..=newline_pos);

        if let Some(text) = extract_text_from_sse_line(&line) {
            let _ = tx.send(Ok(text)).await;
        }
    }
}

/// 스트림 종료 시점에 개행 없이 남아 있는 바이트 버퍼를 파싱하여 플러시합니다.
pub async fn flush_sse_buffer(
    raw_buffer: &mut Vec<u8>,
    tx: &mpsc::Sender<Result<String, std::convert::Infallible>>,
) {
    let remaining = String::from_utf8_lossy(raw_buffer);
    let trimmed = remaining.trim();

    if !trimmed.is_empty() {
        if let Some(text) = extract_text_from_sse_line(trimmed) {
            let _ = tx.send(Ok(text)).await;
        } else if let Ok(list) = serde_json::from_str::<Vec<GeminiApiResponse>>(trimmed) {
            for resp in list {
                if let Some(text) = extract_text_from_response(&resp) {
                    let _ = tx.send(Ok(text)).await;
                }
            }
        } else if let Ok(single) = serde_json::from_str::<GeminiApiResponse>(trimmed) {
            if let Some(text) = extract_text_from_response(&single) {
                let _ = tx.send(Ok(text)).await;
            }
        }
    }

    raw_buffer.clear();
}

/// 단일 SSE 라인(`data: {...}`)에서 텍스트 조각을 추출합니다.
fn extract_text_from_sse_line(line: &str) -> Option<String> {
    let payload = line.strip_prefix("data:")?.trim();
    if payload.is_empty() || payload == "[DONE]" {
        return None;
    }
    let resp = serde_json::from_str::<GeminiApiResponse>(payload).ok()?;
    extract_text_from_response(&resp)
}

/// Gemini 응답 구조체에서 첫 번째 후보의 텍스트 파트들을 결합하여 추출합니다.
fn extract_text_from_response(response: &GeminiApiResponse) -> Option<String> {
    let candidates = response.candidates.as_ref()?;
    let first_candidate = candidates.first()?;
    let content = first_candidate.content.as_ref()?;
    let parts = content.parts.as_ref()?;

    let mut combined = String::new();
    for part in parts {
        if let Some(ref t) = part.text {
            combined.push_str(t);
        }
    }

    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}

/// 사용 가능한 Gemini 모델 목록을 조회하고 우선순위에 따라 정렬하여 반환합니다.
pub async fn get_model_list(state: &AppState) -> Vec<GeminiModelResDTO> {
    let list_url = format!(
        "{}?key={}",
        state.config.rest_api.gemini.base_url, state.config.keys.gemini_api_key
    );
    info!(
        "Fetching Gemini models from: {}",
        state.config.rest_api.gemini.base_url
    );

    let response = match state.http_client.get(&list_url).send().await {
        Ok(resp) if resp.status().is_success() => resp,
        Ok(resp) => {
            error!(
                "Failed to fetch models from Gemini API: HTTP {}",
                resp.status()
            );
            return fallback_models();
        }
        Err(e) => {
            error!("Failed to fetch models from Gemini API: {}", e);
            return fallback_models();
        }
    };

    let body: GeminiModelListResponse = match response.json().await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to parse Gemini models response: {}", e);
            return fallback_models();
        }
    };

    let Some(models) = body.models else {
        return Vec::new();
    };

    let mut filtered: Vec<GeminiModelResDTO> = models
        .into_iter()
        .filter(is_valid_generation_model)
        .map(|m| {
            let raw_name = m.name.unwrap_or_default();
            let id = raw_name.replace("models/", "");
            let display_name = m.display_name.unwrap_or_else(|| id.clone());
            let raw_desc = m.description.unwrap_or_default();
            let description = get_friendly_description(&id, &raw_desc);
            GeminiModelResDTO {
                id,
                display_name,
                description,
                is_default: false,
            }
        })
        .collect();

    // 우선순위에 따라 정렬 (Flash 모델 우선 -> 최신 버전 우선 -> 정식 버전 우선 -> 이름순)
    filtered.sort_by(compare_models);

    // 첫 번째 모델을 기본 모델로 설정
    if let Some(first) = filtered.first_mut() {
        first.is_default = true;
    }

    filtered
}

/// 텍스트 생성(`generateContent`)을 지원하는 모델인지 확인합니다.
fn is_valid_generation_model(model: &GeminiModelInfo) -> bool {
    let Some(ref name) = model.name else {
        return false;
    };
    let Some(ref methods) = model.supported_generation_methods else {
        return false;
    };
    name.starts_with("models/gemini-") && methods.iter().any(|m| m == "generateContent")
}

/// 모델 정렬 비교 함수
fn compare_models(m1: &GeminiModelResDTO, m2: &GeminiModelResDTO) -> Ordering {
    let id1 = m1.id.to_lowercase();
    let id2 = m2.id.to_lowercase();

    // Flash 모델 여부를 최우선으로 비교
    let is_flash1 = id1.contains("flash");
    let is_flash2 = id2.contains("flash");
    if is_flash1 != is_flash2 {
        return if is_flash1 {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }

    // 버전 번호 비교 (높은 버전이 앞으로)
    let v1 = extract_version(&id1);
    let v2 = extract_version(&id2);
    if (v1 - v2).abs() > f64::EPSILON {
        return v2.partial_cmp(&v1).unwrap_or(Ordering::Equal);
    }

    // 정식(Stable) 버전 우선 (preview, exp가 아닌 모델 우선)
    let is_stable1 = !id1.contains("preview") && !id1.contains("exp");
    let is_stable2 = !id2.contains("preview") && !id2.contains("exp");
    if is_stable1 != is_stable2 {
        return if is_stable1 {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }

    id1.cmp(&id2)
}

/// 모델 ID에서 버전 숫자(예: 3.7, 2.5)를 추출합니다.
fn extract_version(model_id: &str) -> f64 {
    if let Some(caps) = VERSION_REGEX.captures(model_id) {
        if let Some(matched) = caps.get(1) {
            return matched.as_str().parse::<f64>().unwrap_or(0.0);
        }
    }
    0.0
}

/// 모델 ID에 맞는 한국어 친화적 설명을 반환합니다.
fn get_friendly_description(id: &str, raw_description: &str) -> String {
    let lower_id = id.to_lowercase();
    if lower_id.contains("3.7-flash") {
        "최신 3.7 세대 초고속 모델 (가장 빠르고 정확함)".to_string()
    } else if lower_id.contains("3.5-flash") {
        "검증된 3.5 세대 고성능 모델 (속도와 품질의 조화)".to_string()
    } else if lower_id.contains("3.1-pro") {
        "고도의 추론과 복잡한 문맥 이해에 특화된 모델".to_string()
    } else if lower_id.contains("2.5-flash") {
        "안정적인 성능의 2.5 세대 경량 모델".to_string()
    } else if lower_id.contains("2.0-flash-lite") {
        "초경량·초고속 응답에 최적화된 경제적 모델".to_string()
    } else if lower_id.contains("2.0-flash") {
        "빠른 응답 속도를 제공하는 2.0 세대 표준 모델".to_string()
    } else if lower_id.contains("1.5-flash") {
        "대량의 텍스트 요약에 강점이 있는 베스트셀러 모델".to_string()
    } else if lower_id.contains("1.5-pro") {
        "정교한 분석과 긴 문맥 처리에 적합한 고성능 모델".to_string()
    } else if lower_id.contains("flash") {
        "빠른 속도와 효율성을 제공하는 경량화 모델".to_string()
    } else if lower_id.contains("pro") {
        "복잡한 작업과 고성능 추론을 위한 전문가용 모델".to_string()
    } else if !raw_description.is_empty() {
        raw_description.to_string()
    } else {
        "구글의 차세대 멀티모달 AI 모델".to_string()
    }
}

/// API 호출 실패 시 반환할 기본 모델 목록
fn fallback_models() -> Vec<GeminiModelResDTO> {
    vec![
        GeminiModelResDTO {
            id: "gemini-3.7-flash".to_string(),
            display_name: "Gemini 3.7 Flash".to_string(),
            description: "최신 초고속·고효율 모델 (권장)".to_string(),
            is_default: true,
        },
        GeminiModelResDTO {
            id: "gemini-3.5-flash".to_string(),
            display_name: "Gemini 3.5 Flash".to_string(),
            description: "안정적인 고성능 요약 및 분석".to_string(),
            is_default: false,
        },
        GeminiModelResDTO {
            id: "gemini-3.1-pro-preview".to_string(),
            display_name: "Gemini 3.1 Pro Preview".to_string(),
            description: "복잡한 추론 및 정밀 요약에 최적화".to_string(),
            is_default: false,
        },
        GeminiModelResDTO {
            id: "gemini-2.5-flash".to_string(),
            display_name: "Gemini 2.5 Flash".to_string(),
            description: "검증된 범용 경량 모델".to_string(),
            is_default: false,
        },
    ]
}
