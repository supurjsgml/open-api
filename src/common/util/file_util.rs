use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
};
use tracing::error;

use crate::common::exception::ValidException;

/// 파일 다운로드 응답 생성, 파일 쓰기 및 이어쓰기 유틸리티
pub struct FileUtil;

impl FileUtil {
    /// 파일을 읽어 다운로드용 HTTP 응답을 구성하고 원본 임시 파일을 삭제
    pub fn file_download(
        file_path_param: &str,
        file: &Path,
    ) -> Result<Response, ValidException> {
        let file_name = file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download.dat");
        let save_file_name = format!("{}/{}", file_path_param.trim_end_matches('/'), file_name);
        let target_path = Path::new(&save_file_name);

        let bytes = fs::read(target_path)
            .map_err(|e| ValidException::new(format!("파일 읽기에 실패하였습니다: {}", e)))?;

        // 한글 등 비-ASCII 파일명을 UTF-8 URL 인코딩 처리
        let mut encoded_name = String::with_capacity(file_name.len());
        for c in file_name.chars() {
            if (c as u32) > ('~' as u32) {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                encoded_name.push_str(&urlencoding::encode(s));
            } else {
                encoded_name.push(c);
            }
        }

        // 다운로드 완료 후 원본 파일 삭제
        if let Err(err) = fs::remove_file(file) {
            error!("delete File Info : {:?}", err);
            return Err(ValidException::new("파일 삭제에 실패하였습니다."));
        }

        let mut headers = HeaderMap::new();
        if let Ok(v) = HeaderValue::from_str(&format!("attachment; filename={};", encoded_name)) {
            headers.insert(header::CONTENT_DISPOSITION, v);
        }
        headers.insert(
            "Content-Transfer-Encoding",
            HeaderValue::from_static("binary"),
        );
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        if let Ok(v) = HeaderValue::from_str(&bytes.len().to_string()) {
            headers.insert(header::CONTENT_LENGTH, v);
        }
        headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache;"));
        headers.insert(header::EXPIRES, HeaderValue::from_static("-1;"));

        let mut resp = Response::new(Body::from(bytes));
        *resp.status_mut() = StatusCode::OK;
        *resp.headers_mut() = headers;
        Ok(resp)
    }

    /// 파일에 전체 문자열을 새로 기록
    pub fn write_file(path: &Path, content: &str) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut f = fs::File::create(path)?;
        f.write_all(content.as_bytes())?;
        f.flush()
    }

    /// 기존 파일 끝에 문자열을 이어붙임
    pub fn append_to_file(path: &Path, content: &str) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut f = OpenOptions::new().create(true).append(true).open(path)?;
        f.write_all(content.as_bytes())?;
        f.flush()
    }
}
