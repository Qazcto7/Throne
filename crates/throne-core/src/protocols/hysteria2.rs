use super::common::TlsConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Hysteria2Obfs {
    #[serde(rename = "type")]
    pub obfs_type: String, // "salamander"
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hysteria2Outbound {
    pub server: String,
    pub server_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfs: Option<Hysteria2Obfs>,
    pub tls: TlsConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up_mbps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down_mbps: Option<u32>,
}

impl Default for Hysteria2Outbound {
    fn default() -> Self {
        Self {
            server: String::new(),
            server_port: 443,
            auth: None,
            obfs: None,
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            up_mbps: None,
            down_mbps: None,
        }
    }
}
