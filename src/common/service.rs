use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use tracing::{error, info};

use crate::{
    AppState,
    common::{
        dto::FileGenReqDTO,
        enums::TemplatesEnum,
        exception::{DataCustomException, ValidException},
        util::{common_util, file_util, rex_util},
    },
};

/// 지정된 경로에 controller, service, impl, repository, dto 디렉터리와 소스 파일을 생성
pub fn create_package(state: &AppState, req: &FileGenReqDTO) -> Result<(), ValidException> {
    let raw_file_nm = req.file_nm.as_deref().unwrap_or("Sample");
    let file_nm = common_util::capitalize(raw_file_nm);
    let dto_nm = format!(
        "{}{}",
        common_util::capitalize(&file_nm),
        TemplatesEnum::Dto.file_nm()
    );

    let file_path_param = match req.file_path.as_deref() {
        Some(p) if !p.trim().is_empty() => p.to_string(),
        _ => state.config.common_constants.file_path.clone(),
    };

    let pack_path = resolve_pack_path(&file_path_param);
    let file_var_name = common_util::decapitalize(&file_nm);
    let dto_var_name = common_util::decapitalize(&dto_nm);

    for tpl in TemplatesEnum::values() {
        let sub_folder = if tpl.folder_nm() == TemplatesEnum::Implement.folder_nm() {
            format!(
                "{}/{}",
                TemplatesEnum::Service.folder_nm(),
                tpl.folder_nm()
            )
        } else {
            tpl.folder_nm().to_string()
        };

        let dir_path = PathBuf::from(format!(
            "{}/{}",
            file_path_param.trim_end_matches('/'),
            sub_folder
        ));

        if !dir_path.exists() {
            fs::create_dir_all(&dir_path).map_err(|e| {
                error!("fileDownload ERROR : {}", e);
                ValidException::new(DataCustomException::new("file Save Error").to_string())
            })?;
        }

        let target_file = dir_path.join(format!("{}{}.java", file_nm, tpl.file_nm()));
        if target_file.exists() {
            info!("filePath : {}", target_file.display());
            return Err(ValidException::new("파일 생성에 실패하였습니다."));
        }

        let rendered = render_vm_template(
            *tpl,
            &pack_path,
            &file_nm,
            &file_var_name,
            &req.method_nm,
            &dto_nm,
            &dto_var_name,
        );

        file_util::write_file(&target_file, &rendered).map_err(|e| {
            error!("fileDownload ERROR : {}", e);
            ValidException::new(DataCustomException::new("file Save Error").to_string())
        })?;
    }

    Ok(())
}

/// 5종의 템플릿 렌더링 결과를 폴더명 키(controller, service, impl, repository, dto)를 가진 Map으로 반환
pub fn get_file_info(state: &AppState, req: &FileGenReqDTO) -> HashMap<String, String> {
    let mut result = HashMap::new();

    let raw_file_nm = req.file_nm.as_deref().unwrap_or("Sample");
    let file_nm = common_util::capitalize(raw_file_nm);
    let dto_nm = format!(
        "{}{}",
        common_util::capitalize(&file_nm),
        TemplatesEnum::Dto.file_nm()
    );

    let file_path_param = match req.file_path.as_deref() {
        Some(p) if !p.trim().is_empty() => p.to_string(),
        _ => state.config.common_constants.file_path.clone(),
    };

    let pack_path = resolve_pack_path(&file_path_param);
    let file_var_name = common_util::decapitalize(&file_nm);
    let dto_var_name = common_util::decapitalize(&dto_nm);

    for tpl in TemplatesEnum::values() {
        let rendered = render_vm_template(
            *tpl,
            &pack_path,
            &file_nm,
            &file_var_name,
            &req.method_nm,
            &dto_nm,
            &dto_var_name,
        );
        result.insert(tpl.folder_nm().to_string(), rendered);
    }

    result
}

/// 경로 문자열에 "com"이 포함되어 있으면 그 위치부터 잘라 패키지 경로(com.xxx.yyy)로 변환하고, 없으면 "com.example" 반환
fn resolve_pack_path(file_path_param: &str) -> String {
    if let Some(idx) = file_path_param.find("com") {
        let sub = &file_path_param[idx..];
        rex_util::replace_slash_backslash(sub.trim_end_matches(['/', '\\']), ".")
    } else {
        "com.example".to_string()
    }
}

/// resources/templates/vm/*.vm 파일을 읽거나 내장 템플릿을 사용해 템플릿 변수(${packPath} 등)를 치환
fn render_vm_template(
    tpl: TemplatesEnum,
    pack_path: &str,
    file_nm: &str,
    file_var_name: &str,
    method_nm: &str,
    dto_nm: &str,
    dto_var_name: &str,
) -> String {
    let disk_path = Path::new("resources").join(tpl.vm_path());
    let raw_template =
        fs::read_to_string(&disk_path).unwrap_or_else(|_| builtin_vm_template(tpl).to_string());

    raw_template
        .replace("${packPath}", pack_path)
        .replace("${fileNm}", file_nm)
        .replace("${fileVarName}", file_var_name)
        .replace("${methodNm}", method_nm)
        .replace("${dtoNm}", dto_nm)
        .replace("${dtoVarName}", dto_var_name)
}

fn builtin_vm_template(tpl: TemplatesEnum) -> &'static str {
    match tpl {
        TemplatesEnum::Controller => include_str!("../../resources/templates/vm/controller.vm"),
        TemplatesEnum::Service => include_str!("../../resources/templates/vm/service.vm"),
        TemplatesEnum::Implement => include_str!("../../resources/templates/vm/implement.vm"),
        TemplatesEnum::Repository => include_str!("../../resources/templates/vm/repository.vm"),
        TemplatesEnum::Dto => include_str!("../../resources/templates/vm/dto.vm"),
    }
}
