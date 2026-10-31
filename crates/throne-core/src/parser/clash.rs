use crate::error::{CoreError, Result};
use crate::protocols::*;
use serde_yaml::Value;

pub fn parse_clash(yaml_str: &str) -> Result<Vec<(String, OutboundConfig)>> {
    let root: Value = serde_yaml::from_str(yaml_str).map_err(CoreError::YamlError)?;
    let proxies = root
        .get("proxies")
        .and_then(|p| p.as_sequence())
        .ok_or_else(|| CoreError::ConfigGenError("No 'proxies' section found in YAML".to_string()))?;

    let mut results = Vec::new();

    for proxy in proxies {
        if let Some(map) = proxy.as_mapping() {
            let name = map
                .get(&Value::String("name".to_string()))
                .and_then(|v| v.as_str())
                .unwrap_or("Proxy")
                .to_string();

            let ptype = map
                .get(&Value::String("type".to_string()))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_lowercase();

            let server = map
                .get(&Value::String("server".to_string()))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let port = map
                .get(&Value::String("port".to_string()))
                .and_then(|v| v.as_u64())
                .unwrap_or(443) as u16;

            match ptype.as_str() {
                "ss" => {
                    let cipher = map
                        .get(&Value::String("cipher".to_string()))
                        .and_then(|v| v.as_str())
                        .unwrap_or("aes-256-gcm")
                        .to_string();
                    let password = map
                        .get(&Value::String("password".to_string()))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

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
                    let password = map
                        .get(&Value::String("password".to_string()))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let sni = map
                        .get(&Value::String("sni".to_string()))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

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
                    let uuid = map
                        .get(&Value::String("uuid".to_string()))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let alter_id = map
                        .get(&Value::String("alterId".to_string()))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32;
                    let cipher = map
                        .get(&Value::String("cipher".to_string()))
                        .and_then(|v| v.as_str())
                        .unwrap_or("auto")
                        .to_string();
                    let tls = map
                        .get(&Value::String("tls".to_string()))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let sni = map
                        .get(&Value::String("servername".to_string()))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

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
                    let uuid = map
                        .get(&Value::String("uuid".to_string()))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let flow = map
                        .get(&Value::String("flow".to_string()))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let tls = map
                        .get(&Value::String("tls".to_string()))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    let sni = map
                        .get(&Value::String("servername".to_string()))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

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
