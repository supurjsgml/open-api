use std::env;

/// 현재 컨테이너(Pod/Replica) 이름을 환경변수에서 추출합니다.
pub fn resolve_pod_name(fallback: Option<&str>) -> String {
    env::var("CONTAINER_APP_REPLICA_NAME")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            env::var("HOSTNAME")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
        .unwrap_or_else(|| fallback.unwrap_or("local-dev").to_string())
}
