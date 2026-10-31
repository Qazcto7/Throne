use super::common::MuxConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShadowsocksOutbound {
    pub server: String,
    pub server_port: u16,
    pub method: String,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_opts: Option<String>,
    pub multiplex: MuxConfig,
}

impl Default for ShadowsocksOutbound {
    fn default() -> Self {
        Self {
            server: String::new(),
            server_port: 8388,
            method: "aes-256-gcm".to_string(),
            password: String::new(),
            plugin: None,
            plugin_opts: None,
            multiplex: MuxConfig::default(),
        }
    }
}
