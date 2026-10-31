use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TlsConfig {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(default)]
    pub insecure: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub alpn: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reality: Option<RealityConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RealityConfig {
    pub enabled: bool,
    pub public_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TransportConfig {
    Tcp,
    Ws {
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<std::collections::HashMap<String, String>>,
    },
    Grpc {
        #[serde(rename = "service_name")]
        service_name: String,
    },
    HttpUpgrade {
        #[serde(skip_serializing_if = "Option::is_none")]
        host: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
    },
}

impl Default for TransportConfig {
    fn default() -> Self {
        TransportConfig::Tcp
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MuxConfig {
    pub enabled: bool,
    #[serde(default = "default_mux_protocol")]
    pub protocol: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

fn default_mux_protocol() -> String {
    "h2mux".to_string()
}

fn default_max_connections() -> u32 {
    4
}
