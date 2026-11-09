use std::sync::Arc;

use axum::routing::get;
use axum::{Json, Router};

use crate::AppState;
use crate::common::util::common_util::CommonUtil;
use crate::junit::dto::{
    OrderResponseMapper, OrderSeatAssignPriceResponseDto, SeatAssignPriceDto,
};

/// 대용량 DTO 변환 테스트 도메인 라우터를 생성합니다.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/test/commonUtil", get(test_common_util))
        .route("/test/mapStruct", get(test_map_struct))
}

/// 성능 비교 테스트를 위한 30,000건의 더미 좌석 가격 데이터를 생성합니다.
pub fn create_dummy_data() -> Vec<SeatAssignPriceDto> {
    let mut list = Vec::with_capacity(30_000);
    for i in 1..=30_000 {
        let grade_name = if i == 1 {
            "VIP"
        } else if i == 2 {
            "R"
        } else {
            "S"
        };

        list.push(SeatAssignPriceDto {
            product_turn_price_sequence: Some(100_000 + i as i64),
            product_price_sequence: Some(1_000 + i as i64),
            seat_grade_code: Some(format!("00{}", i)),
            seat_grade_name: Some(grade_name.to_string()),
            product_sale_price: Some(10_000.0 * i as f32),
            preferred_rank: Some(i),
            product_price_classification_name: Some(format!("분류{}", i)),
            product_price_classification_description: Some(format!("상세설명{}", i)),
            price_type_code: Some("D".to_string()),
            price_type_name: Some("할인가".to_string()),
            personal_document_requirement_yn: Some("N".to_string()),
            partner_pattern_code: Some("1".to_string()),
            pay_code: Some("10".to_string()),
            code: Some("30".to_string()),
            product_sale_card_quantity: Some("5".to_string()),
        });
    }
    list
}

/// GET `/test/commonUtil`
/// `CommonUtil::convert_list` (Serde 기반 범용 변환) 방식으로 30,000건의 DTO 리스트를 변환합니다.
pub async fn test_common_util() -> Json<Vec<OrderSeatAssignPriceResponseDto>> {
    let source_list = create_dummy_data();
    let result = CommonUtil::convert_list::<SeatAssignPriceDto, OrderSeatAssignPriceResponseDto>(
        &source_list,
    );
    Json(result)
}

/// GET `/test/mapStruct`
/// `OrderResponseMapper` (컴파일 타임 직접 필드 매핑) 방식으로 30,000건의 DTO 리스트를 변환합니다.
pub async fn test_map_struct() -> Json<Vec<OrderSeatAssignPriceResponseDto>> {
    let source_list = create_dummy_data();
    let result = OrderResponseMapper::to_order_seat_assign_price_response_dto_list(&source_list);
    Json(result)
}
