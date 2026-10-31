use super::common::{MuxConfig, TlsConfig, TransportConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrojanOutbound {
    pub server: String,
    pub server_port: u16,
    pub password: String,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default)]
    pub multiplex: MuxConfig,
}

impl Default for TrojanOutbound {
    fn default() -> Self {
        Self {
            server: String::new(),
            server_port: 443,
            password: String::new(),
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
            transport: TransportConfig::Tcp,
            multiplex: MuxConfig::default(),
        }
    }
}
