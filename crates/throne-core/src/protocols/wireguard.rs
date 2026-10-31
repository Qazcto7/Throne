use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WireGuardOutbound {
    pub server: String,
    pub server_port: u16,
    pub system_interface: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface_name: Option<String>,
    pub local_address: Vec<String>,
    pub private_key: String,
    pub peer_public_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_shared_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserved: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mtu: Option<u32>,
}

impl Default for WireGuardOutbound {
    fn default() -> Self {
        Self {
            server: String::new(),
            server_port: 51820,
            system_interface: false,
            interface_name: None,
            local_address: vec!["10.0.0.2/32".to_string()],
            private_key: String::new(),
            peer_public_key: String::new(),
            pre_shared_key: None,
            reserved: None,
            mtu: Some(1420),
        }
    }
}
