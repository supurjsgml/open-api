use std::sync::Arc;
use std::time::Instant;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use tower::ServiceExt;

use open_api::common::enums::MessageEnum;
use open_api::common::util::common_util::CommonUtil;
use open_api::common::util::date_utils::DateUtils;
use open_api::common::util::messages_utils::MessagesUtils;
use open_api::common::util::rex_util::RexUtil;
use open_api::config::AppConfig;
use open_api::junit::controller::create_dummy_data;
use open_api::junit::dto::{
    OrderResponseMapper, OrderSeatAssignPriceResponseDto, SeatAssignPriceDto,
};
use open_api::{build_router, AppState};

/// 30,000건 대용량 DTO 리스트 변환 정합성 및 성능 비교 테스트
#[test]
fn test_30000_dto_mapping_equivalence_and_performance() {
    let source_list = create_dummy_data();
    assert_eq!(source_list.len(), 30_000);

    let start_common = Instant::now();
    let common_result =
        CommonUtil::convert_list::<SeatAssignPriceDto, OrderSeatAssignPriceResponseDto>(
            &source_list,
        );
    let elapsed_common = start_common.elapsed();

    let start_mapper = Instant::now();
    let mapper_result =
        OrderResponseMapper::to_order_seat_assign_price_response_dto_list(&source_list);
    let elapsed_mapper = start_mapper.elapsed();

    assert_eq!(common_result.len(), 30_000);
    assert_eq!(mapper_result.len(), 30_000);

    // 첫 번째 항목 필드 검증
    let first = &mapper_result[0];
    assert_eq!(first.product_turn_price_sequence, Some(100_001));
    assert_eq!(first.seat_grade_name.as_deref(), Some("VIP"));

    // JSON 직렬화 시 partnerPatternCode 필드가 제외되는지 검증
    let serialized = serde_json::to_string(first).expect("Failed to serialize DTO");
    assert!(!serialized.contains("partnerPatternCode"));
    assert!(serialized.contains("productTurnPriceSequence"));
    assert!(elapsed_mapper <= elapsed_common);
}

/// 공통 유틸리티(케이스 변환, 페이징, 정규식, 날짜, 메시지) 단위 테스트
#[test]
fn test_common_utilities() {
    assert_eq!(
        CommonUtil::convert_camel_to_snake("productTurnPriceSequence"),
        "PRODUCT_TURN_PRICE_SEQUENCE"
    );
    assert_eq!(
        CommonUtil::convert_snake_to_camel("PRODUCT_TURN_PRICE_SEQUENCE"),
        "productTurnPriceSequence"
    );
    assert_eq!(CommonUtil::str_paging(1, 10), "LIMIT 0, 10");
    assert_eq!(CommonUtil::capitalize("sample"), "Sample");
    assert_eq!(CommonUtil::decapitalize("SampleDTO"), "sampleDTO");

    assert!(RexUtil::is_number("123456"));
    assert!(!RexUtil::is_number("abc-123"));
    assert!(RexUtil::is_date("2026-11-06"));

    let today = DateUtils::current_date_format(Some("yyyyMMdd"));
    assert_eq!(today.len(), 8);

    let ok_msg = MessagesUtils::get_message(MessageEnum::Success.code());
    assert_eq!(ok_msg, "정상");
}

/// 라우터, 보안 미들웨어(Origin 검증 및 비공개 경로 차단), 컨트롤러 통합 테스트
#[tokio::test]
async fn test_http_endpoints_and_security_middleware() {
    let config = AppConfig::from_env();
    let state = Arc::new(AppState::from_config(config));
    let app = build_router(state);

    // 샘플 텍스트 조회 API 검증
    let req = Request::builder()
        .uri("/api/getUrl")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        String::from_utf8_lossy(&body_bytes),
        "뭐에요 왜 깨워요 !!!!!"
    );

    // 헬스체크 API 검증
    let req = Request::builder()
        .uri("/actuator/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 허용되지 않은 Origin 헤더 요청 시 403 차단 검증
    let req = Request::builder()
        .uri("/api/getUrl")
        .header(header::ORIGIN, "https://malicious-site.example.com")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 허용된 Origin 헤더 요청 시 200 정상 통과 검증
    let req = Request::builder()
        .uri("/api/getUrl")
        .header(header::ORIGIN, "https://guney.kro.kr")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 비공개 경로(/api/private/**) 접근 시 403 차단 검증
    let req = Request::builder()
        .uri("/api/private/secret")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 존재하지 않는 경로 요청 시 표준 404 폴백 JSON 응답 검증
    let req = Request::builder()
        .uri("/non-existent-route")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("존재하지 않는 리소스입니다."));
}
