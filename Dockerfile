# Rust 빌드 스테이지
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /app

# 빌드 캐시 최적화를 위해 매니페스트와 소스 복사 후 릴리즈 빌드 수행
COPY Cargo.toml ./
COPY src ./src
COPY resources ./resources

RUN cargo build --release

# 경량 런타임 스테이지
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    curl \
    && rm -rf /var/lib/apt/lists/*

ENV TZ=Asia/Seoul
WORKDIR /app

COPY --from=builder /app/target/release/open-api /app/open-api
COPY --from=builder /app/resources /app/resources

EXPOSE 8080

ENTRYPOINT ["/app/open-api"]
