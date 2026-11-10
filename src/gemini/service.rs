use std::cmp::Ordering;
use std::sync::LazyLock;

use futures_util::StreamExt;
use regex::Regex;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info};

use crate::AppState;
use crate::gemini::dto::{
    GeminiApiRequest, GeminiApiResponse, GeminiModelInfo, GeminiModelListResponse,
    GeminiModelResDTO, GeminiReqDTO,
};

/// 버전 번호 추출용 정규표현식 (예: gemini-3.7-flash -> 3.7)
static VERSION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"gemini-(\d+(?:\.\d+)?)").expect("Invalid regex"));

/// 본문과 프롬프트를 결합하여 Gemini 스트리밍 요약 결과를 비동기 스트림(`ReceiverStream`)으로 반환합니다.
pub async fn get_summary(
    state: &AppState,
    req_dto: GeminiReqDTO,
) -> ReceiverStream<Result<String, std::convert::Infallible>> {
    let (tx, rx) = mpsc::channel(32);

    let model = match req_dto.model.as_deref() {
        Some(m) if !m.trim().is_empty() => m.to_string(),
        _ => "gemini-1.5-flash".to_string(),
    };

    // 스트리밍 엔드포인트 URL 구성
    let stream_url = format!(
        "{}?key={}",
        state
            .config
            .rest_api
            .gemini
            .generate_content
            .replace("{model}", &model)
            .replace("generateContent", "streamGenerateContent"),
        state.config.keys.gemini_api_key
    );

    let combined_prompt = format!(
        "{}\n\n[웹페이지 본문 내용]\n{}",
        req_dto.prompt.unwrap_or_default(),
        req_dto.text.unwrap_or_default()
    );

    let request_body = GeminiApiRequest::from_text(&combined_prompt);
    let client = state.http_client.clone();

    tokio::spawn(async move {
        let response_result = client
            .post(&stream_url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await;

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

                // 바이트 스트림에서 JSON 응답 청크를 수집 및 파싱하여 텍스트 조각을 전송합니다.
                let mut byte_stream = response.bytes_stream();
                let mut buffer = String::new();

                while let Some(chunk_res) = byte_stream.next().await {
                    match chunk_res {
                        Ok(bytes) => {
                            let chunk_str = String::from_utf8_lossy(&bytes);
                            buffer.push_str(&chunk_str);

                            // 단일 객체 또는 배열 형태의 JSON 청크에서 텍스트 추출 시도
                            extract_and_send_texts(&mut buffer, &tx).await;
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

                // 스트림 종료 후 남은 버퍼 처리
                if !buffer.trim().is_empty() {
                    flush_remaining_buffer(&buffer, &tx).await;
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

/// 스트리밍 버퍼에서 완성된 GeminiApiResponse JSON 객체들을 파싱하여 채널로 전송합니다.
async fn extract_and_send_texts(
    buffer: &mut String,
    tx: &mpsc::Sender<Result<String, std::convert::Infallible>>,
) {
    if let Ok(list) = serde_json::from_str::<Vec<GeminiApiResponse>>(buffer) {
        for resp in list {
            if let Some(text) = extract_text_from_response(&resp) {
                let _ = tx.send(Ok(text)).await;
            }
        }
        buffer.clear();
        return;
    }

    if let Ok(single) = serde_json::from_str::<GeminiApiResponse>(buffer) {
        if let Some(text) = extract_text_from_response(&single) {
            let _ = tx.send(Ok(text)).await;
        }
        buffer.clear();
    }
}

/// 스트림 완료 시점에 남아 있는 버퍼에서 텍스트를 추출합니다.
async fn flush_remaining_buffer(
    buffer: &str,
    tx: &mpsc::Sender<Result<String, std::convert::Infallible>>,
) {
    let trimmed = buffer.trim();
    if let Ok(list) = serde_json::from_str::<Vec<GeminiApiResponse>>(trimmed) {
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

/// Gemini 응답 구조체에서 첫 번째 후보의 텍스트를 추출합니다.
fn extract_text_from_response(response: &GeminiApiResponse) -> Option<String> {
    let candidates = response.candidates.as_ref()?;
    let first_candidate = candidates.first()?;
    let content = first_candidate.content.as_ref()?;
    let parts = content.parts.as_ref()?;
    let first_part = parts.first()?;
    first_part.text.clone()
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
