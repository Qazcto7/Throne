use crate::error::{CoreError, Result};
use crate::models::{Profile, RouteProfile, RouteRule, Settings};
use crate::protocols::*;
use serde_json::{json, Map, Value};

pub struct SingBoxGenerator<'a> {
    settings: &'a Settings,
    profile: &'a Profile,
    route_profile: Option<&'a RouteProfile>,
    route_rules: &'a [RouteRule],
}

impl<'a> SingBoxGenerator<'a> {
    pub fn new(
        settings: &'a Settings,
        profile: &'a Profile,
        route_profile: Option<&'a RouteProfile>,
        route_rules: &'a [RouteRule],
    ) -> Self {
        Self {
            settings,
            profile,
            route_profile,
            route_rules,
        }
    }

    pub fn generate(&self) -> Result<Value> {
        let mut root = Map::new();

        // 1. Log section
        root.insert(
            "log".to_string(),
            json!({
                "level": self.settings.log_level,
                "timestamp": true
            }),
        );

        // 2. DNS section
        root.insert("dns".to_string(), self.build_dns()?);

        // 3. Inbounds
        root.insert("inbounds".to_string(), self.build_inbounds()?);

        // 4. Outbounds
        root.insert("outbounds".to_string(), self.build_outbounds()?);

        // 5. Route
        root.insert("route".to_string(), self.build_route()?);

        Ok(Value::Object(root))
    }

    fn build_dns(&self) -> Result<Value> {
        let mut servers = Vec::new();

        // Remote DNS
        servers.push(json!({
            "tag": "dns-remote",
            "address": self.settings.remote_dns,
            "detour": "proxy"
        }));

        // Direct DNS
        let direct_addr = if self.settings.direct_dns == "local" {
            "local"
        } else {
            &self.settings.direct_dns
        };
        servers.push(json!({
            "tag": "dns-direct",
            "address": direct_addr,
            "detour": "direct"
        }));

        let mut dns_rules = Vec::new();
        dns_rules.push(json!({
            "outbound": "any",
            "server": "dns-direct"
        }));

        Ok(json!({
            "servers": servers,
            "rules": dns_rules,
            "final": "dns-remote",
            "strategy": "prefer_ipv4"
        }))
    }

    fn build_inbounds(&self) -> Result<Value> {
        let mut inbounds = Vec::new();

        // Mixed inbound (HTTP + SOCKS5)
        inbounds.push(json!({
            "type": "mixed",
            "tag": "mixed-in",
            "listen": self.settings.inbound_address,
            "listen_port": self.settings.inbound_port,
            "sniff": true,
            "sniff_override_destination": true
        }));

        // TUN inbound if enabled
        if self.settings.enable_tun {
            inbounds.push(json!({
                "type": "tun",
                "tag": "tun-in",
                "interface_name": self.settings.tun_interface_name,
                "inet4_address": "172.19.0.1/30",
                "auto_route": true,
                "strict_route": self.settings.tun_strict_route,
                "stack": self.settings.tun_stack,
                "sniff": true,
                "sniff_override_destination": true
            }));
        }

        Ok(Value::Array(inbounds))
    }

    fn build_outbounds(&self) -> Result<Value> {
        let mut outbounds = Vec::new();

        // Parse profile's outbound JSON
        let outbound_cfg: OutboundConfig = serde_json::from_str(&self.profile.outbound_json)
            .map_err(|e| CoreError::ConfigGenError(format!("Failed to parse profile outbound_json: {e}")))?;

        let mut proxy_outbound = self.convert_outbound(&outbound_cfg)?;
        proxy_outbound["tag"] = json!("proxy");
        outbounds.push(proxy_outbound);

        // Direct outbound
        outbounds.push(json!({
            "type": "direct",
            "tag": "direct"
        }));

        // Block outbound
        outbounds.push(json!({
            "type": "block",
            "tag": "block"
        }));

        // DNS outbound
        outbounds.push(json!({
            "type": "dns",
            "tag": "dns-out"
        }));

        Ok(Value::Array(outbounds))
    }

    fn convert_outbound(&self, cfg: &OutboundConfig) -> Result<Value> {
        match cfg {
            OutboundConfig::Vless(v) => {
                let mut obj = json!({
                    "type": "vless",
                    "server": v.server,
                    "server_port": v.server_port,
                    "uuid": v.uuid
                });

                if let Some(ref flow) = v.flow {
                    obj["flow"] = json!(flow);
                }
                if let Some(ref pe) = v.packet_encoding {
                    obj["packet_encoding"] = json!(pe);
                }

                if v.tls.enabled {
                    let mut tls_obj = json!({
                        "enabled": true,
                        "insecure": v.tls.insecure
                    });
                    if let Some(ref sni) = v.tls.server_name {
                        tls_obj["server_name"] = json!(sni);
                    }
                    if !v.tls.alpn.is_empty() {
                        tls_obj["alpn"] = json!(v.tls.alpn);
                    }
                    if let Some(ref r) = v.tls.reality {
                        if r.enabled {
                            let mut reality_obj = json!({
                                "enabled": true,
                                "public_key": r.public_key
                            });
                            if let Some(ref sid) = r.short_id {
                                reality_obj["short_id"] = json!(sid);
                            }
                            tls_obj["reality"] = reality_obj;
                        }
                    }
                    obj["tls"] = tls_obj;
                }

                match &v.transport {
                    TransportConfig::Ws { path, headers } => {
                        let mut ws = json!({
                            "type": "ws"
                        });
                        if let Some(p) = path {
                            ws["path"] = json!(p);
                        }
                        if let Some(h) = headers {
                            ws["headers"] = json!(h);
                        }
                        obj["transport"] = ws;
                    }
                    TransportConfig::Grpc { service_name } => {
                        obj["transport"] = json!({
                            "type": "grpc",
                            "service_name": service_name
                        });
                    }
                    TransportConfig::HttpUpgrade { host, path } => {
                        let mut hu = json!({ "type": "httpupgrade" });
                        if let Some(h) = host {
                            hu["host"] = json!(h);
                        }
                        if let Some(p) = path {
                            hu["path"] = json!(p);
                        }
                        obj["transport"] = hu;
                    }
                    TransportConfig::Tcp => {}
                }

                Ok(obj)
            }
            OutboundConfig::Vmess(v) => {
                let mut obj = json!({
                    "type": "vmess",
                    "server": v.server,
                    "server_port": v.server_port,
                    "uuid": v.uuid,
                    "alter_id": v.alter_id,
                    "security": v.security
                });
                if v.tls.enabled {
                    let mut tls_obj = json!({
                        "enabled": true,
                        "insecure": v.tls.insecure
                    });
                    if let Some(ref sni) = v.tls.server_name {
                        tls_obj["server_name"] = json!(sni);
                    }
                    obj["tls"] = tls_obj;
                }
                Ok(obj)
            }
            OutboundConfig::Shadowsocks(s) => Ok(json!({
                "type": "shadowsocks",
                "server": s.server,
                "server_port": s.server_port,
                "method": s.method,
                "password": s.password
            })),
            OutboundConfig::Trojan(t) => {
                let mut obj = json!({
                    "type": "trojan",
                    "server": t.server,
                    "server_port": t.server_port,
                    "password": t.password
                });
                if t.tls.enabled {
                    let mut tls_obj = json!({
                        "enabled": true,
                        "insecure": t.tls.insecure
                    });
                    if let Some(ref sni) = t.tls.server_name {
                        tls_obj["server_name"] = json!(sni);
                    }
                    obj["tls"] = tls_obj;
                }
                Ok(obj)
            }
            OutboundConfig::Hysteria2(h) => {
                let mut obj = json!({
                    "type": "hysteria2",
                    "server": h.server,
                    "server_port": h.server_port
                });
                if let Some(ref auth) = h.auth {
                    obj["password"] = json!(auth);
                }
                if let Some(ref obfs) = h.obfs {
                    obj["obfs"] = json!({
                        "type": obfs.obfs_type,
                        "password": obfs.password
                    });
                }
                if h.tls.enabled {
                    let mut tls_obj = json!({
                        "enabled": true,
                        "insecure": h.tls.insecure
                    });
                    if let Some(ref sni) = h.tls.server_name {
                        tls_obj["server_name"] = json!(sni);
                    }
                    obj["tls"] = tls_obj;
                }
                Ok(obj)
            }
            OutboundConfig::Tuic(t) => {
                let mut obj = json!({
                    "type": "tuic",
                    "server": t.server,
                    "server_port": t.server_port,
                    "uuid": t.uuid,
                    "password": t.password,
                    "congestion_control": t.congestion_control,
                    "udp_relay_mode": t.udp_relay_mode,
                    "zero_rtt_handshake": t.zero_rtt_handshake
                });
                if t.tls.enabled {
                    let mut tls_obj = json!({
                        "enabled": true,
                        "insecure": t.tls.insecure
                    });
                    if let Some(ref sni) = t.tls.server_name {
                        tls_obj["server_name"] = json!(sni);
                    }
                    obj["tls"] = tls_obj;
                }
                Ok(obj)
            }
            OutboundConfig::Wireguard(w) => Ok(json!({
                "type": "wireguard",
                "server": w.server,
                "server_port": w.server_port,
                "local_address": w.local_address,
                "private_key": w.private_key,
                "peer_public_key": w.peer_public_key,
                "mtu": w.mtu
            })),
            OutboundConfig::Socks(s) => Ok(json!({
                "type": "socks",
                "server": s.server,
                "server_port": s.server_port,
                "version": s.version
            })),
            OutboundConfig::Http(h) => Ok(json!({
                "type": "http",
                "server": h.server,
                "server_port": h.server_port,
                "tls": {
                    "enabled": h.tls
                }
            })),
        }
    }

    fn build_route(&self) -> Result<Value> {
        let mut rules = Vec::new();

        // 1. Inbound protocol DNS traffic route to dns-out
        rules.push(json!({
            "protocol": "dns",
            "outbound": "dns-out"
        }));

        // 2. Private IP ranges bypass
        rules.push(json!({
            "ip_is_private": true,
            "outbound": "direct"
        }));

        // 3. Custom rules from RouteProfile
        for r in self.route_rules {
            let mut rule_obj = json!({
                "outbound": r.target
            });
            let mut has_conditions = false;

            if !r.domains.is_empty() {
                rule_obj["domain"] = json!(r.domains);
                has_conditions = true;
            }
            if !r.domain_suffixes.is_empty() {
                rule_obj["domain_suffix"] = json!(r.domain_suffixes);
                has_conditions = true;
            }
            if !r.domain_keywords.is_empty() {
                rule_obj["domain_keyword"] = json!(r.domain_keywords);
                has_conditions = true;
            }
            if !r.ip_cidrs.is_empty() {
                rule_obj["ip_cidr"] = json!(r.ip_cidrs);
                has_conditions = true;
            }
            if !r.rule_sets.is_empty() {
                rule_obj["rule_set"] = json!(r.rule_sets);
                has_conditions = true;
            }

            if has_conditions {
                rules.push(rule_obj);
            }
        }

        let final_outbound = self
            .route_profile
            .map(|rp| rp.default_outbound.as_str())
            .unwrap_or("proxy");

        Ok(json!({
            "rules": rules,
            "final": final_outbound,
            "auto_detect_interface": true
        }))
    }
}
