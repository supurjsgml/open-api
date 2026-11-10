use rand::Rng;
use tracing::info;

use crate::common::exception::ValidException;

/// 0 이상 max_size 미만의 난수 반환
pub fn random_selection(max_size: usize) -> usize {
    if max_size > 0 {
        rand::thread_rng().gen_range(0..max_size)
    } else {
        0
    }
}

/// 슬라이스(리스트)에서 무작위로 요소 하나를 선택하여 복제 반환
pub fn random_selection_list<T: Clone>(list: &[T]) -> Option<T> {
    if list.is_empty() {
        return None;
    }
    let idx = random_selection(list.len());
    list.get(idx).cloned()
}

/// 아이템 목록과 가중치(확률) 목록을 받아 누적 가중치 방식으로 추첨 수행
pub fn weighted_random_selection<T: Clone>(
    items: &[T],
    weights: &[f64],
) -> Result<Option<T>, ValidException> {
    if items.len() != weights.len() {
        return Err(ValidException::new(
            "확률 계산에 실패하였습니다. 다시 시도해 주세요.",
        ));
    }
    if items.is_empty() {
        return Ok(None);
    }

    let total_weight: f64 = weights.iter().sum();
    let random_number = rand::thread_rng().r#gen::<f64>() * total_weight;
    let mut cumulative_weight = 0.0;

    for (item, &weight) in items.iter().zip(weights.iter()) {
        cumulative_weight += weight;
        info!(
            "randomNumber : {}, cumulativeWeight : {}, weight : {}",
            random_number, cumulative_weight, weight
        );
        if random_number < cumulative_weight {
            return Ok(Some(item.clone()));
        }
    }

    Ok(None)
}

/// 수량(qty)과 전체 수량(max_qty)의 백분율(%) 계산
pub fn probability_calculation(qty: i32, max_qty: i32) -> f64 {
    if max_qty == 0 {
        return 0.0;
    }
    (qty as f64 / max_qty as f64) * 100.0
}
