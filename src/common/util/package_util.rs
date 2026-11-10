use crate::common::util::common_util;

/// 기본 컨트롤러 보일러플레이트 소스 코드 문자열을 포맷팅해 생성하는 유틸리티
pub fn create_package(pack_path: &str, file_nm: &str, dto_nm: &str) -> String {
    let service = format!("{}Service", common_util::decapitalize(file_nm));
    let low_upper_dto_nm = common_util::decapitalize(dto_nm);

    format!(
        "package {pack_path}.controller;\n\n\
import org.springframework.http.MediaType;\n\
import org.springframework.web.bind.annotation.PostMapping;\n\
import org.springframework.web.bind.annotation.RequestMapping;\n\
import org.springframework.web.bind.annotation.RequestBody;\n\
import org.springframework.web.bind.annotation.RestController;\n\n\
import {pack_path}.dto.{dto_nm};\n\
import {pack_path}.service.{file_nm}Service;\n\n\
import io.swagger.v3.oas.annotations.Operation;\n\
import io.swagger.v3.oas.annotations.tags.Tag;\n\
import lombok.RequiredArgsConstructor;\n\
import lombok.extern.slf4j.Slf4j;\n\n\
@Slf4j\n\
@Tag(name = \"\", description = \"\")\n\
@RequiredArgsConstructor\n\
@RestController\n\
@RequestMapping(value = \"/\")\n\
public class {file_nm}Controller {{\n\n\
    private final {file_nm}Service {service};\n    \n\
    @Operation(summary = \"\", description = \"\")\n\
    @PostMapping(value = \"/\", produces = MediaType.APPLICATION_JSON_VALUE)\n\
    public void name(@RequestBody {dto_nm} {low_upper_dto_nm}){{\n\
        {service}.fileDownload({low_upper_dto_nm});\n\
    }}\n\n\
}}\n"
    )
}
