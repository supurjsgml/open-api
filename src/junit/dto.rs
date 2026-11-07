use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 좌석 배정 가격 원본 DTO
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SeatAssignPriceDto {
    pub product_turn_price_sequence: Option<i64>,
    pub product_price_sequence: Option<i64>,
    pub seat_grade_code: Option<String>,
    pub seat_grade_name: Option<String>,
    pub product_sale_price: Option<f32>,
    pub preferred_rank: Option<i32>,
    pub product_price_classification_name: Option<String>,
    pub product_price_classification_description: Option<String>,
    pub price_type_code: Option<String>,
    pub price_type_name: Option<String>,
    pub personal_document_requirement_yn: Option<String>,
    pub partner_pattern_code: Option<String>,
    pub pay_code: Option<String>,
    pub code: Option<String>,
    pub product_sale_card_quantity: Option<String>,
}

/// 주문 좌석 배정 가격 응답 DTO
/// `partner_pattern_code` 필드는 JSON 응답 직렬화 시 제외됩니다.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OrderSeatAssignPriceResponseDto {
    pub product_turn_price_sequence: Option<i64>,
    pub product_price_sequence: Option<i64>,
    pub seat_grade_code: Option<String>,
    pub seat_grade_name: Option<String>,
    pub product_sale_price: Option<f32>,
    pub preferred_rank: Option<i32>,
    pub product_price_classification_name: Option<String>,
    pub product_price_classification_description: Option<String>,
    pub price_type_code: Option<String>,
    pub price_type_name: Option<String>,
    pub personal_document_requirement_yn: Option<String>,
    #[serde(skip_serializing)]
    pub partner_pattern_code: Option<String>,
    pub pay_code: Option<String>,
    pub code: Option<String>,
    pub product_sale_card_quantity: Option<String>,
}

impl From<&SeatAssignPriceDto> for OrderSeatAssignPriceResponseDto {
    fn from(src: &SeatAssignPriceDto) -> Self {
        Self {
            product_turn_price_sequence: src.product_turn_price_sequence,
            product_price_sequence: src.product_price_sequence,
            seat_grade_code: src.seat_grade_code.clone(),
            seat_grade_name: src.seat_grade_name.clone(),
            product_sale_price: src.product_sale_price,
            preferred_rank: src.preferred_rank,
            product_price_classification_name: src.product_price_classification_name.clone(),
            product_price_classification_description: src
                .product_price_classification_description
                .clone(),
            price_type_code: src.price_type_code.clone(),
            price_type_name: src.price_type_name.clone(),
            personal_document_requirement_yn: src.personal_document_requirement_yn.clone(),
            partner_pattern_code: src.partner_pattern_code.clone(),
            pay_code: src.pay_code.clone(),
            code: src.code.clone(),
            product_sale_card_quantity: src.product_sale_card_quantity.clone(),
        }
    }
}

/// 컴파일 타임 필드 매핑 변환기
pub struct OrderResponseMapper;

impl OrderResponseMapper {
    /// 슬라이스의 각 요소를 직접 필드 복사로 고속 변환합니다.
    pub fn to_order_seat_assign_price_response_dto_list(
        source: &[SeatAssignPriceDto],
    ) -> Vec<OrderSeatAssignPriceResponseDto> {
        source
            .iter()
            .map(OrderSeatAssignPriceResponseDto::from)
            .collect()
    }
}
