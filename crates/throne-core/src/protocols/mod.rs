pub mod common;
pub mod hysteria2;
pub mod shadowsocks;
pub mod socks_http;
pub mod trojan;
pub mod tuic;
pub mod vless;
pub mod vmess;
pub mod wireguard;

pub use common::*;
pub use hysteria2::*;
pub use shadowsocks::*;
pub use socks_http::*;
pub use trojan::*;
pub use tuic::*;
pub use vless::*;
pub use vmess::*;
pub use wireguard::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum OutboundConfig {
    Vless(VlessOutbound),
    Vmess(VmessOutbound),
    Shadowsocks(ShadowsocksOutbound),
    Trojan(TrojanOutbound),
    Hysteria2(Hysteria2Outbound),
    Tuic(TuicOutbound),
    Wireguard(WireGuardOutbound),
    Socks(SocksOutbound),
    Http(HttpOutbound),
}

impl OutboundConfig {
    pub fn protocol_name(&self) -> &'static str {
        match self {
            Self::Vless(_) => "vless",
            Self::Vmess(_) => "vmess",
            Self::Shadowsocks(_) => "shadowsocks",
            Self::Trojan(_) => "trojan",
            Self::Hysteria2(_) => "hysteria2",
            Self::Tuic(_) => "tuic",
            Self::Wireguard(_) => "wireguard",
            Self::Socks(_) => "socks",
            Self::Http(_) => "http",
        }
    }

    pub fn server_and_port(&self) -> (&str, u16) {
        match self {
            Self::Vless(c) => (&c.server, c.server_port),
            Self::Vmess(c) => (&c.server, c.server_port),
            Self::Shadowsocks(c) => (&c.server, c.server_port),
            Self::Trojan(c) => (&c.server, c.server_port),
            Self::Hysteria2(c) => (&c.server, c.server_port),
            Self::Tuic(c) => (&c.server, c.server_port),
            Self::Wireguard(c) => (&c.server, c.server_port),
            Self::Socks(c) => (&c.server, c.server_port),
            Self::Http(c) => (&c.server, c.server_port),
        }
    }
}
