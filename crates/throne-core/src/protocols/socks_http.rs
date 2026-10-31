use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SocksOutbound {
    pub server: String,
    pub server_port: u16,
    #[serde(default = "default_socks_version")]
    pub version: String, // "5"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

fn default_socks_version() -> String {
    "5".to_string()
}

impl Default for SocksOutbound {
    fn default() -> Self {
        Self {
            server: String::new(),
            server_port: 1080,
            version: "5".to_string(),
            username: None,
            password: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HttpOutbound {
    pub server: String,
    pub server_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(default)]
    pub tls: bool,
}

impl Default for HttpOutbound {
    fn default() -> Self {
        Self {
            server: String::new(),
            server_port: 8080,
            username: None,
            password: None,
            tls: false,
        }
    }
}
