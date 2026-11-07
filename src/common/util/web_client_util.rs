use std::{collections::HashMap, future::Future, sync::Arc, time::Duration};

use futures_util::{StreamExt, stream};
use reqwest::{Client, Method};
use serde::{Serialize, de::DeserializeOwned};
use tracing::error;

use crate::common::{enums::MessageEnum, exception::ValidException};

/// 외부 HTTP API 호출(GET, POST, Form, 병렬 요청)을 담당하는 클라이언트 유틸리티
#[derive(Clone)]
pub struct WebClientUtil {
    client: Client,
}

impl Default for WebClientUtil {
    fn default() -> Self {
        Self::new()
    }
}

impl WebClientUtil {
    /// 커넥션 타임아웃 60초, 응답 타임아웃 60초가 적용된 HTTP 클라이언트 생성
    pub fn new() -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(60))
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    pub fn raw_client(&self) -> &Client {
        &self.client
    }

    /// GET 비동기 요청 후 JSON 응답을 대상 타입으로 역직렬화
    pub async fn get_async<T: DeserializeOwned>(&self, url: &str) -> Result<T, String> {
        let resp = self
            .client
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        resp.json::<T>().await.map_err(|e| e.to_string())
    }

    /// POST 비동기 JSON 바디 전송 후 응답 역직렬화
    pub async fn post_async<V: Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        request_body: &V,
    ) -> Result<T, String> {
        let resp = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(request_body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        resp.json::<T>().await.map_err(|e| e.to_string())
    }

    /// 바디 없는 POST 비동기 호출
    pub async fn post_empty_async(&self, url: &str) -> Result<(), String> {
        self.client
            .post(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// application/x-www-form-urlencoded 폼 데이터 POST 전송 후 JSON 역직렬화
    pub async fn post_form_async<T: DeserializeOwned>(
        &self,
        url: &str,
        form_data: &[(String, String)],
        headers: Option<&HashMap<String, String>>,
    ) -> Result<T, String> {
        let mut req = self.client.post(url).form(form_data);
        if let Some(hdrs) = headers {
            for (k, v) in hdrs {
                req = req.header(k, v);
            }
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;
        resp.json::<T>().await.map_err(|e| e.to_string())
    }

    /// Form 데이터 POST 전송 후 문자열 응답을 그대로 반환
    pub async fn post_form_text_async(
        &self,
        url: &str,
        form_data: &[(String, String)],
        headers: Option<&HashMap<String, String>>,
    ) -> Result<String, String> {
        let mut req = self.client.post(url).form(form_data);
        if let Some(hdrs) = headers {
            for (k, v) in hdrs {
                req = req.header(k, v);
            }
        }

        let resp = req.send().await.map_err(|e| e.to_string())?;
        resp.text().await.map_err(|e| e.to_string())
    }

    /// N개의 비동기 API 호출을 동시에 병렬 실행하여 결과를 리스트로 합침
    pub async fn get_async_many<Fut, T>(futures: Vec<Fut>) -> Vec<T>
    where
        Fut: Future<Output = T>,
    {
        futures_util::future::join_all(futures).await
    }

    /// 동시 실행 개수(max_concurrency)를 제한하며 병렬 실행
    pub async fn get_async_many_with_limit<Fut, T>(
        futures: Vec<Fut>,
        max_concurrency: usize,
    ) -> Vec<T>
    where
        Fut: Future<Output = T>,
    {
        stream::iter(futures)
            .buffer_unordered(max_concurrency.max(1))
            .collect::<Vec<T>>()
            .await
    }

    /// 지정된 HTTP 메서드로 요청을 보내고 응답을 역직렬화
    pub async fn exchange<O: Serialize, C: DeserializeOwned>(
        &self,
        url: &str,
        method: Method,
        body: Option<&O>,
    ) -> Result<C, ValidException> {
        let mut req = self
            .client
            .request(method, url)
            .header("Content-Type", "application/json");

        if let Some(b) = body {
            req = req.json(b);
        }

        let resp = req.send().await.map_err(|e| {
            error!("RestUtil exchange ERROR : {}", e);
            ValidException::from_enum(MessageEnum::InternalServerError)
        })?;

        resp.json::<C>().await.map_err(|e| {
            error!("RestUtil exchange parse ERROR : {}", e);
            ValidException::from_enum(MessageEnum::InternalServerError)
        })
    }
}

pub type SharedWebClient = Arc<WebClientUtil>;
