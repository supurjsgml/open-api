use std::future::Future;

/// Tokio 경량 비동기 태스크를 사용해 여러 작업을 동시에 실행하고 결과를 모으는 유틸리티
pub struct VirtualThreadUtil;

impl VirtualThreadUtil {
    /// 여러 비동기 클로저(작업)를 경량 태스크(tokio::spawn)에서 동시에 실행하고 모든 결과를 순서대로 반환
    pub async fn run_async_many<F, Fut, T>(tasks: Vec<F>) -> Vec<T>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let mut handles = Vec::with_capacity(tasks.len());
        for task in tasks {
            handles.push(tokio::spawn(async move { task().await }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            if let Ok(output) = handle.await {
                results.push(output);
            }
        }
        results
    }
}
