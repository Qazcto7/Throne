use crate::error::{CoreError, Result};
use crate::protocols::*;
use serde_yaml::{Mapping, Value};

fn get_val<'a>(map: &'a Mapping, key: &str) -> Option<&'a Value> {
    map.get(Value::from(key))
}

fn get_str<'a>(map: &'a Mapping, key: &str) -> Option<&'a str> {
    get_val(map, key).and_then(|v| v.as_str())
}

pub fn parse_clash(yaml_str: &str) -> Result<Vec<(String, OutboundConfig)>> {
    let root: Value = serde_yaml::from_str(yaml_str).map_err(CoreError::YamlError)?;
    let proxies = root
        .get("proxies")
        .and_then(|p| p.as_sequence())
        .ok_or_else(|| CoreError::ConfigGenError("No 'proxies' section found in YAML".to_string()))?;

    let mut results = Vec::new();

    for proxy in proxies {
        if let Some(map) = proxy.as_mapping() {
            let name = get_str(map, "name").unwrap_or("Proxy").to_string();
            let ptype = get_str(map, "type").unwrap_or("").to_lowercase();
            let server = get_str(map, "server").unwrap_or("").to_string();
            let port = get_val(map, "port")
                .and_then(|v| v.as_u64())
                .unwrap_or(443) as u16;

            match ptype.as_str() {
                "ss" => {
                    let cipher = get_str(map, "cipher").unwrap_or("aes-256-gcm").to_string();
                    let password = get_str(map, "password").unwrap_or("").to_string();

                    results.push((
                        name,
                        OutboundConfig::Shadowsocks(ShadowsocksOutbound {
                            server,
                            server_port: port,
                            method: cipher,
                            password,
                            plugin: None,
                            plugin_opts: None,
                            multiplex: MuxConfig::default(),
                        }),
                    ));
                }
                "trojan" => {
                    let password = get_str(map, "password").unwrap_or("").to_string();
                    let sni = get_str(map, "sni").map(|s| s.to_string());

                    results.push((
                        name,
                        OutboundConfig::Trojan(TrojanOutbound {
                            server,
                            server_port: port,
                            password,
                            tls: TlsConfig {
                                enabled: true,
                                server_name: sni,
                                ..Default::default()
                            },
                            transport: TransportConfig::Tcp,
                            multiplex: MuxConfig::default(),
                        }),
                    ));
                }
                "vmess" => {
                    let uuid = get_str(map, "uuid").unwrap_or("").to_string();
                    let alter_id = get_val(map, "alterId")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    let cipher = get_str(map, "cipher").unwrap_or("auto").to_string();
                    let tls = get_val(map, "tls")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let sni = get_str(map, "servername").map(|s| s.to_string());

                    results.push((
                        name,
                        OutboundConfig::Vmess(VmessOutbound {
                            server,
                            server_port: port,
                            uuid,
                            alter_id,
                            security: cipher,
                            tls: TlsConfig {
                                enabled: tls,
                                server_name: sni,
                                ..Default::default()
                            },
                            transport: TransportConfig::Tcp,
                            multiplex: MuxConfig::default(),
                        }),
                    ));
                }
                "vless" => {
                    let uuid = get_str(map, "uuid").unwrap_or("").to_string();
                    let flow = get_str(map, "flow").map(|s| s.to_string());
                    let tls = get_val(map, "tls")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    let sni = get_str(map, "servername").map(|s| s.to_string());

                    results.push((
                        name,
                        OutboundConfig::Vless(VlessOutbound {
                            server,
                            server_port: port,
                            uuid,
                            flow,
                            packet_encoding: Some("xudp".to_string()),
                            tls: TlsConfig {
                                enabled: tls,
                                server_name: sni,
                                ..Default::default()
                            },
                            transport: TransportConfig::Tcp,
                            multiplex: MuxConfig::default(),
                        }),
                    ));
                }
                _ => {}
            }
        }
    }

    Ok(results)
}
