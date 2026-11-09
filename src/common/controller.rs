use std::{collections::HashMap, sync::Arc};

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use tracing::error;

use crate::{
    AppState,
    common::{
        core::AppError,
        dto::{ApiBodyResponse, FileGenReqDTO, HeaderSuccess},
        enums::MessageEnum,
        util::api_res_util::ApiResUtil,
    },
};

/// 공통 템플릿 생성 및 조회 라우터를 생성합니다.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/com/genFile", post(gen_file))
        .route("/com/getFileInfo", post(get_file_info))
}

/// POST `/com/genFile` - 보일러플레이트 패키지 및 소스 파일 생성 핸들러
pub async fn gen_file(
    State(state): State<Arc<AppState>>,
    Json(file_gen_req): Json<FileGenReqDTO>,
) -> Result<Response, AppError> {
    // 필수 입력값(camelStr) 누락 여부 검증
    if let Err(valid_msg) = file_gen_req.validate() {
        return Err(AppError::MethodArgumentNotValid(valid_msg));
    }

    if let Err(e) = state.common_service.create_package(&file_gen_req) {
        error!("CommonController failDownload ERROR : {}", e.message);
    }

    Ok(StatusCode::OK.into_response())
}

/// POST `/com/getFileInfo` - 템플릿 치환 결과 Map 조회 핸들러
pub async fn get_file_info(
    State(state): State<Arc<AppState>>,
    Json(file_gen_req): Json<FileGenReqDTO>,
) -> Json<ApiBodyResponse<HashMap<String, String>>> {
    let info_map = state.common_service.get_file_info(&file_gen_req);
    let resp = ApiResUtil::success_with_header(
        info_map,
        HeaderSuccess::default(),
        MessageEnum::Success.code(),
    );
    Json(resp)
}
