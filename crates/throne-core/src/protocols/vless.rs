use super::common::{MuxConfig, TlsConfig, TransportConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VlessOutbound {
    pub server: String,
    pub server_port: u16,
    pub uuid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_encoding: Option<String>,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default)]
    pub multiplex: MuxConfig,
}

impl Default for VlessOutbound {
    fn default() -> Self {
        Self {
            server: String::new(),
            server_port: 443,
            uuid: String::new(),
            flow: None,
            packet_encoding: Some("xudp".to_string()),
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            transport: TransportConfig::Tcp,
            multiplex: MuxConfig::default(),
        }
    }
}
