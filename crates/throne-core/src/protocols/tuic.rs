use super::common::TlsConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TuicOutbound {
    pub server: String,
    pub server_port: u16,
    pub uuid: String,
    pub password: String,
    #[serde(default = "default_tuic_cc")]
    pub congestion_control: String,
    #[serde(default = "default_tuic_relay")]
    pub udp_relay_mode: String,
    #[serde(default)]
    pub zero_rtt_handshake: bool,
    #[serde(default)]
    pub tls: TlsConfig,
}

fn default_tuic_cc() -> String {
    "bbr".to_string()
}

fn default_tuic_relay() -> String {
    "native".to_string()
}

impl Default for TuicOutbound {
    fn default() -> Self {
        Self {
            server: String::new(),
            server_port: 8443,
            uuid: String::new(),
            password: String::new(),
            congestion_control: "bbr".to_string(),
            udp_relay_mode: "native".to_string(),
            zero_rtt_handshake: false,
            tls: TlsConfig {
                enabled: true,
                ..Default::default()
            },
        }
    }
}
