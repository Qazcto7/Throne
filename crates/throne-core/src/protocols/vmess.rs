use super::common::{MuxConfig, TlsConfig, TransportConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VmessOutbound {
    pub server: String,
    pub server_port: u16,
    pub uuid: String,
    #[serde(default)]
    pub alter_id: u32,
    #[serde(default = "default_vmess_security")]
    pub security: String,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub transport: TransportConfig,
    #[serde(default)]
    pub multiplex: MuxConfig,
}

fn default_vmess_security() -> String {
    "auto".to_string()
}

impl Default for VmessOutbound {
    fn default() -> Self {
        Self {
            server: String::new(),
            server_port: 443,
            uuid: String::new(),
            alter_id: 0,
            security: "auto".to_string(),
            tls: TlsConfig::default(),
            transport: TransportConfig::Tcp,
            multiplex: MuxConfig::default(),
        }
    }
}
