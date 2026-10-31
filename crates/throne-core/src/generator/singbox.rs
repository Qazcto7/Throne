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
        let mut rules = Vec::new();

        // 1. Predefined hosts for DPI/DNS bypass (e.g. Discord, Roblox)
        if self.settings.dns_predefined_enable {
            let mut predefined_map: std::collections::BTreeMap<String, Vec<String>> =
                std::collections::BTreeMap::new();

            for rule in &self.settings.dns_predefined_rules {
                let trimmed = rule.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let mut parts = trimmed.split_whitespace();
                if let (Some(ip), Some(domain)) = (parts.next(), parts.next()) {
                    if domain == "localhost" {
                        continue;
                    }
                    predefined_map
                        .entry(domain.to_string())
                        .or_default()
                        .push(ip.to_string());
                }
            }

            // Ensure essential Discord and Roblox predefined wildcard/subdomain mappings
            let defaults = [
                ("discord.com", "162.159.136.232"),
                ("updates.discord.com", "162.159.137.232"),
                ("gateway.discord.gg", "162.159.134.234"),
                ("cdn.discordapp.com", "162.159.134.233"),
                ("discordapp.com", "162.159.129.233"),
                ("status.discord.com", "162.159.128.233"),
                ("discord.gg", "162.159.135.232"),
                ("*.discord.com", "162.159.136.232"),
                ("*.discordapp.com", "162.159.134.233"),
                ("*.discord.gg", "162.159.134.234"),
                ("*.discordapp.net", "162.159.129.233"),
                ("roblox.com", "128.116.44.3"),
                ("www.roblox.com", "128.116.44.3"),
                ("*.roblox.com", "128.116.44.3"),
            ];
            for (dom, ip) in defaults {
                predefined_map
                    .entry(dom.to_string())
                    .or_insert_with(|| vec![ip.to_string()]);
            }

            if !predefined_map.is_empty() {
                servers.push(json!({
                    "tag": "dns-hosts",
                    "type": "hosts",
                    "predefined": predefined_map
                }));

                rules.push(json!({
                    "domain_suffix": [
                        "discord.com",
                        "discordapp.com",
                        "discord.gg",
                        "discordapp.net",
                        "discordcdn.com",
                        "discord.media",
                        "roblox.com"
                    ],
                    "server": "dns-hosts"
                }));
            }
        }

        // Remote DNS (routes through proxy outbound so ByeDPI unblocks it)
        servers.push(Self::parse_dns_server("dns-remote", &self.settings.remote_dns, Some("proxy")));

        // Direct DNS
        servers.push(Self::parse_dns_server("dns-direct", &self.settings.direct_dns, None));

        let mut dns_obj = json!({
            "servers": servers,
            "final": "dns-remote",
            "strategy": "ipv4_only"
        });

        if !rules.is_empty() {
            dns_obj["rules"] = Value::Array(rules);
        }

        Ok(dns_obj)
    }

    fn parse_dns_server(tag: &str, addr: &str, detour: Option<&str>) -> Value {
        let mut obj = Map::new();
        obj.insert("tag".to_string(), json!(tag));
        if let Some(d) = detour
            && d != "direct"
        {
            obj.insert("detour".to_string(), json!(d));
        }

        if addr == "local" || addr.is_empty() {
            obj.insert("type".to_string(), json!("local"));
        } else if let Some(stripped) = addr.strip_prefix("https://") {
            obj.insert("type".to_string(), json!("https"));
            if let Some((host, path)) = stripped.split_once('/') {
                obj.insert("server".to_string(), json!(host));
                obj.insert("path".to_string(), json!(format!("/{}", path)));
            } else {
                obj.insert("server".to_string(), json!(stripped));
            }
        } else if let Some(stripped) = addr.strip_prefix("tls://") {
            obj.insert("type".to_string(), json!("tls"));
            let (host, port) = stripped.split_once(':').unwrap_or((stripped, "853"));
            obj.insert("server".to_string(), json!(host));
            if let Ok(p) = port.parse::<u16>() {
                obj.insert("server_port".to_string(), json!(p));
            }
        } else if let Some(stripped) = addr.strip_prefix("tcp://") {
            obj.insert("type".to_string(), json!("tcp"));
            let (host, port) = stripped.split_once(':').unwrap_or((stripped, "53"));
            obj.insert("server".to_string(), json!(host));
            if let Ok(p) = port.parse::<u16>() {
                obj.insert("server_port".to_string(), json!(p));
            }
        } else if let Some(stripped) = addr.strip_prefix("udp://") {
            obj.insert("type".to_string(), json!("udp"));
            let (host, port) = stripped.split_once(':').unwrap_or((stripped, "53"));
            obj.insert("server".to_string(), json!(host));
            if let Ok(p) = port.parse::<u16>() {
                obj.insert("server_port".to_string(), json!(p));
            }
        } else {
            // Plain IP or host
            obj.insert("type".to_string(), json!("udp"));
            let (host, port) = addr.split_once(':').unwrap_or((addr, "53"));
            obj.insert("server".to_string(), json!(host));
            if let Ok(p) = port.parse::<u16>() {
                obj.insert("server_port".to_string(), json!(p));
            }
        }

        Value::Object(obj)
    }

    fn build_inbounds(&self) -> Result<Value> {
        let mut inbounds = Vec::new();

        // Mixed inbound (HTTP + SOCKS5)
        inbounds.push(json!({
            "type": "mixed",
            "tag": "mixed-in",
            "listen": self.settings.inbound_address,
            "listen_port": self.settings.inbound_port
        }));

        // DNS direct inbound (matching classic Throne core_dns_in_port 55331)
        inbounds.push(json!({
            "type": "direct",
            "tag": "dns-in",
            "listen": "127.0.0.1",
            "listen_port": 55331
        }));

        // TUN inbound if enabled
        if self.settings.enable_tun {
            let tun_obj = json!({
                "type": "tun",
                "tag": "tun-in",
                "interface_name": self.settings.tun_interface_name,
                "address": ["172.19.0.1/24"],
                "auto_route": true,
                "strict_route": false,
                "stack": self.settings.tun_stack,
                "dns_mode": "disabled",
                "route_exclude_address": ["127.0.0.0/8"]
            });
            inbounds.push(tun_obj);
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
                    if let Some(r) = v.tls.reality.as_ref().filter(|r| r.enabled) {
                        let mut reality_obj = json!({
                            "enabled": true,
                            "public_key": r.public_key
                        });
                        if let Some(ref sid) = r.short_id {
                            reality_obj["short_id"] = json!(sid);
                        }
                        tls_obj["reality"] = reality_obj;
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

        // 1. ciadpi / ByeDPI process bypass to direct physical interface
        // to prevent routing loop when TUN is active
        rules.push(json!({
            "process_name": ["ciadpi"],
            "outbound": "direct"
        }));
        if !self.settings.byedpi_path.is_empty() {
            rules.push(json!({
                "process_path": [self.settings.byedpi_path],
                "outbound": "direct"
            }));
        }

        // 2. Sniff inbound connections
        rules.push(json!({
            "action": "sniff"
        }));

        // 3. Inbound DNS traffic hijack to internal DNS (both port 53 and protocol dns)
        rules.push(json!({
            "port": 53,
            "action": "hijack-dns"
        }));
        rules.push(json!({
            "protocol": "dns",
            "action": "hijack-dns"
        }));

        // 4. Resolve domains to IPv4 before sending to outbound proxy
        rules.push(json!({
            "action": "resolve",
            "strategy": "ipv4_only"
        }));

        // 5. Private IP ranges bypass
        rules.push(json!({
            "ip_is_private": true,
            "outbound": "direct"
        }));

        // 6. Essential Discord and Roblox routing to proxy outbound (ByeDPI bypass)
        rules.push(json!({
            "process_name": [
                "Discord",
                "discord",
                "updater_bootstrap",
                "DiscordCanary",
                "DiscordPTB",
                "RobloxPlayerBeta.exe",
                "RobloxStudioBeta.exe"
            ],
            "outbound": "proxy"
        }));
        rules.push(json!({
            "domain_suffix": [
                "discord.com",
                "discordapp.com",
                "discord.gg",
                "discordapp.net",
                "discordcdn.com",
                "discord.media",
                "roblox.com",
                "rbxcdn.com",
                "rbxinfra.com"
            ],
            "outbound": "proxy"
        }));

        // 7. Custom rules from RouteProfile
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
            if !r.process_names.is_empty() {
                rule_obj["process_name"] = json!(r.process_names);
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
            .map(|rp| rp.default_outbound_tag())
            .unwrap_or("proxy");

        Ok(json!({
            "rules": rules,
            "final": final_outbound,
            "auto_detect_interface": true,
            "default_domain_resolver": "dns-remote",
            "find_process": true
        }))
    }
}
