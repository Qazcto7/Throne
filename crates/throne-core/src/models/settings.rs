use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    pub inbound_port: u16,
    pub inbound_address: String,
    pub enable_tun: bool,
    pub tun_interface_name: String,
    pub tun_stack: String,
    pub tun_strict_route: bool,
    pub system_proxy: bool,
    pub remote_api_enable: bool,
    pub remote_api_port: u16,
    pub remote_api_secret: String,
    pub core_path: String,
    pub active_profile_id: Option<i64>,
    pub active_route_id: Option<i64>,
    pub mux_enabled: bool,
    pub fake_dns: bool,
    pub remote_dns: String,
    pub direct_dns: String,
    pub log_level: String,
    pub byedpi_enable: bool,
    pub byedpi_path: String,
    pub byedpi_args: String,
    pub byedpi_preset: String,
    pub dns_predefined_enable: bool,
    pub dns_predefined_rules: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            inbound_port: 2080,
            inbound_address: "127.0.0.1".to_string(),
            enable_tun: false,
            tun_interface_name: "throne-tun".to_string(),
            tun_stack: "system".to_string(),
            tun_strict_route: true,
            system_proxy: false,
            remote_api_enable: true,
            remote_api_port: 9095,
            remote_api_secret: String::new(),
            core_path: "sing-box".to_string(),
            active_profile_id: None,
            active_route_id: None,
            mux_enabled: false,
            fake_dns: false,
            remote_dns: "https://1.1.1.1/dns-query".to_string(),
            direct_dns: "local".to_string(),
            log_level: "info".to_string(),
            byedpi_enable: true,
            byedpi_path: "ciadpi".to_string(),
            byedpi_args: "-s1 -At -d2 -f-1 -r1+s -An".to_string(),
            byedpi_preset: "vodafone".to_string(),
            dns_predefined_enable: true,
            dns_predefined_rules: vec![
                "127.0.0.1 localhost".to_string(),
                "162.159.136.232 discord.com".to_string(),
                "162.159.134.234 gateway.discord.gg".to_string(),
                "162.159.134.233 cdn.discordapp.com".to_string(),
                "162.159.129.233 discordapp.com".to_string(),
                "128.116.44.3 roblox.com".to_string(),
                "128.116.44.3 www.roblox.com".to_string(),
            ],
        }
    }
}
