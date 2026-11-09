use std::sync::Arc;

use axum::response::{Html, Redirect};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::AppState;

/// 문서 및 뷰 페이지 라우터를 생성합니다.
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(root_redirect))
        .route("/redoc", get(redoc_page))
        .route("/jsp", get(jsp_page))
        .route("/swagger-ui", get(swagger_ui_page))
        .route("/swagger-ui/index.html", get(swagger_ui_page))
        .route("/v3/api-docs", get(openapi_json))
}

/// 컴파일 타임에 내장되는 Redoc HTML 템플릿 (파일 시스템에 없을 때 폴백으로 사용)
const REDOC_HTML_EMBEDDED: &str = include_str!("../../resources/templates/html/index.html");
/// 컴파일 타임에 내장되는 카멜케이스 변환기 HTML 템플릿
const CONVERT_CAMEL_HTML_EMBEDDED: &str =
    include_str!("../../resources/templates/html/ConvertCamel.html");

/// GET `/`
/// 루트 경로 접속 시 `/redoc` 문서 페이지로 리다이렉트합니다.
pub async fn root_redirect() -> Redirect {
    Redirect::to("/redoc")
}

/// GET `/redoc`
/// Redoc API 명세서 화면(`index.html`)을 렌더링합니다.
pub async fn redoc_page() -> Html<String> {
    let html = tokio::fs::read_to_string("resources/templates/html/index.html")
        .await
        .unwrap_or_else(|_| REDOC_HTML_EMBEDDED.to_string());
    Html(html)
}

/// GET `/jsp`
/// 카멜케이스 변환기 도구 화면(`ConvertCamel.html`)을 렌더링합니다.
pub async fn jsp_page() -> Html<String> {
    let html = tokio::fs::read_to_string("resources/templates/html/ConvertCamel.html")
        .await
        .unwrap_or_else(|_| CONVERT_CAMEL_HTML_EMBEDDED.to_string());
    Html(html)
}

/// GET `/swagger-ui` 및 `/swagger-ui/index.html`
/// OpenAPI 명세(`/v3/api-docs`)를 시각화하는 Swagger UI 화면을 제공합니다.
pub async fn swagger_ui_page() -> Html<&'static str> {
    Html(
        r#"<!DOCTYPE html>
<html lang="ko">
<head>
  <meta charset="UTF-8">
  <title>D.code API - Swagger UI</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    window.onload = () => {
      window.ui = SwaggerUIBundle({
        url: '/v3/api-docs',
        dom_id: '#swagger-ui',
      });
    };
  </script>
</body>
</html>"#,
    )
}

/// GET `/v3/api-docs`
/// Redoc 및 Swagger UI가 참조하는 OpenAPI 3.0.1 표준 JSON 명세를 반환합니다.
pub async fn openapi_json() -> Json<Value> {
    Json(json!({
        "openapi": "3.0.1",
        "info": {
            "title": "D.code API",
            "description": "Rest Api 명세서",
            "version": "1.0.0"
        },
        "servers": [
            {
                "url": "https://guney-api.kro.kr",
                "description": "Default Server URL"
            },
            {
                "url": "http://localhost:8080",
                "description": "Local Server URL"
            }
        ],
        "security": [
            { "JWT": [] }
        ],
        "tags": [
            { "name": "Redocly Sample", "description": "Redocly Sample API" },
            { "name": "Gemini", "description": "Google Gemini AI 요약 및 모델 조회 API" },
            { "name": "Front Internal", "description": "대시보드 및 페이지 방문 통계 API" },
            { "name": "Kakao", "description": "카카오톡 알림 및 토큰 관리 API" },
            { "name": "Member", "description": "잡코리아 로그인 및 스케줄러 연동 API" },
            { "name": "Lightsail", "description": "서버 컨테이너 원격 재시작 API" },
            { "name": "Common", "description": "공통 파일 업로드 및 유틸리티 API" },
            { "name": "Kafka", "description": "메시지 브로커 연동 API" },
            { "name": "Test", "description": "대용량 DTO 변환 성능 비교 API" }
        ],
        "paths": {
            "/api/getUrl": {
                "get": {
                    "tags": ["Redocly Sample"],
                    "summary": "샘플 조회",
                    "description": "샘플 조회 설명입니다.",
                    "operationId": "getUrl",
                    "responses": {
                        "200": {
                            "description": "OK",
                            "content": {
                                "text/plain;charset=UTF-8": {
                                    "schema": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/add": {
                "post": {
                    "tags": ["Redocly Sample"],
                    "summary": "샘플 등록",
                    "description": "샘플 등록입니다.",
                    "operationId": "addUrl",
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SampleDTO" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/api/mod": {
                "put": {
                    "tags": ["Redocly Sample"],
                    "summary": "샘플 수정",
                    "description": "샘플 수정입니다.",
                    "operationId": "modUrl",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/api/del": {
                "delete": {
                    "tags": ["Redocly Sample"],
                    "summary": "샘플 삭제",
                    "description": "샘플 삭제입니다.",
                    "operationId": "delUrl",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/api/gemini/summarize": {
                "post": {
                    "tags": ["Gemini"],
                    "summary": "웹페이지 본문 AI 스트리밍 요약",
                    "operationId": "summarize",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/GeminiReqDTO" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "스트리밍 요약 텍스트",
                            "content": {
                                "text/plain;charset=UTF-8": {
                                    "schema": { "type": "string" }
                                }
                            }
                        }
                    }
                }
            },
            "/api/gemini/models": {
                "get": {
                    "tags": ["Gemini"],
                    "summary": "사용 가능한 Gemini 모델 목록 조회",
                    "operationId": "getModels",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/api/front/dashboardStats": {
                "get": {
                    "tags": ["Front Internal"],
                    "summary": "주간 방문자 통계 조회",
                    "operationId": "getDashboardStats",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/api/front/pageStats": {
                "get": {
                    "tags": ["Front Internal"],
                    "summary": "페이지별 방문 횟수 조회",
                    "operationId": "getPageStats",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/api/front/hit": {
                "post": {
                    "tags": ["Front Internal"],
                    "summary": "방문자 히트 기록",
                    "operationId": "recordHit",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/VisitorHitReqDTO" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/api/kakao/refresh-token": {
                "post": {
                    "tags": ["Kakao"],
                    "summary": "카카오 토큰 수동 갱신",
                    "operationId": "refreshKakaoToken",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/api/kakao/send": {
                "post": {
                    "tags": ["Kakao"],
                    "summary": "카카오톡 에러 알림 메시지 전송",
                    "operationId": "sendKakaoMessage",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/KakaoSendReqDTO" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/member/jobKorea/login": {
                "post": {
                    "tags": ["Member"],
                    "summary": "잡코리아 로그인 및 스케줄러 등록",
                    "operationId": "jobKoreaLogin",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/MemberReqDTO" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/member/jobKorea/logout": {
                "post": {
                    "tags": ["Member"],
                    "summary": "잡코리아 스케줄러 일시중지(로그아웃)",
                    "operationId": "jobKoreaLogout",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/MemberReqDTO" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/api/lightsail/restart": {
                "get": {
                    "tags": ["Lightsail"],
                    "summary": "배치 서버(job-batch) 원격 재시작",
                    "operationId": "restartServer",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/api/lightsail/restart-api": {
                "get": {
                    "tags": ["Lightsail"],
                    "summary": "API 서버(rest-api) 원격 재시작",
                    "operationId": "restartApiServer",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/common/upload": {
                "post": {
                    "tags": ["Common"],
                    "summary": "멀티파트 파일 업로드",
                    "operationId": "uploadFiles",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/kafka/send": {
                "get": {
                    "tags": ["Kafka"],
                    "summary": "Kafka 메시지 발행 테스트",
                    "operationId": "sendMsg",
                    "parameters": [
                        {
                            "name": "msg",
                            "in": "query",
                            "required": false,
                            "schema": { "type": "string" }
                        }
                    ],
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/test/commonUtil": {
                "get": {
                    "tags": ["Test"],
                    "summary": "CommonUtil 30,000건 DTO 변환 테스트",
                    "operationId": "testCommonUtil",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            },
            "/test/mapStruct": {
                "get": {
                    "tags": ["Test"],
                    "summary": "컴파일 타임 매퍼 30,000건 DTO 변환 테스트",
                    "operationId": "testMapStruct",
                    "responses": {
                        "200": { "description": "OK" }
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "JWT": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }
            },
            "schemas": {
                "SampleDTO": {
                    "type": "object",
                    "properties": {
                        "sample": {
                            "type": "string",
                            "description": "샘플",
                            "default": "sample Data"
                        }
                    }
                },
                "GeminiReqDTO": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string" },
                        "prompt": { "type": "string" },
                        "model": { "type": "string" }
                    }
                },
                "VisitorHitReqDTO": {
                    "type": "object",
                    "properties": {
                        "pageName": { "type": "string" },
                        "isNewSession": { "type": "boolean" }
                    }
                },
                "KakaoSendReqDTO": {
                    "type": "object",
                    "properties": {
                        "msg": { "type": "string" }
                    }
                },
                "MemberReqDTO": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "pw": { "type": "string" },
                        "token": { "type": "string" }
                    }
                }
            }
        }
    }))
}
