use serde::{Deserialize, Serialize};

/// 카카오톡 나에게 보내기 텍스트 템플릿 구조체
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KakaoTextTemplate {
    pub object_type: String,
    pub text: String,
    pub link: Link,
    pub buttons: Vec<Button>,
}

/// 템플릿 내부 링크 정보
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Link {
    pub web_url: String,
    pub mobile_web_url: String,
}

/// 템플릿 하단 액션 버튼 정보
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Button {
    pub title: String,
    pub link: Link,
}
