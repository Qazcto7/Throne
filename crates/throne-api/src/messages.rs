use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    #[serde(default)]
    pub id: Option<u64>,
    pub method: String,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ApiResponse {
    pub fn success(id: Option<u64>, val: serde_json::Value) -> Self {
        Self {
            id,
            result: Some(val),
            error: None,
        }
    }

    pub fn err(id: Option<u64>, err_msg: impl Into<String>) -> Self {
        Self {
            id,
            result: None,
            error: Some(err_msg.into()),
        }
    }
}
