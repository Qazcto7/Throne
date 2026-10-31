use crate::error::{CoreError, Result};
use crate::protocols::*;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use std::collections::HashMap;
use url::Url;

pub fn parse_uri(link: &str) -> Result<(String, OutboundConfig)> {
    let trimmed = link.trim();
    if trimmed.starts_with("vmess://") {
        return parse_vmess(trimmed);
    }
    if trimmed.starts_with("ss://") {
        return parse_shadowsocks(trimmed);
    }

    let url = Url::parse(trimmed).map_err(|e| CoreError::InvalidUri(e.to_string()))?;
    match url.scheme() {
        "vless" => parse_vless(&url),
        "trojan" => parse_trojan(&url),
        "hysteria2" | "hy2" => parse_hysteria2(&url),
        "tuic" => parse_tuic(&url),
        "wireguard" | "wg" => parse_wireguard(&url),
        "socks" | "socks5" => parse_socks(&url),
        "http" | "https" => parse_http(&url),
        other => Err(CoreError::UnsupportedProtocol(other.to_string())),
    }
}

pub fn parse_vless(url: &Url) -> Result<(String, OutboundConfig)> {
    let host = url
        .host_str()
        .ok_or_else(|| CoreError::InvalidUri("Missing host in vless link".to_string()))?
        .to_string();
    let port = url.port().unwrap_or(443);
    let uuid = url.username().to_string();
    let name = url_fragment_or(url, &host);

    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let flow = params.get("flow").cloned().filter(|f| f != "none");
    let packet_encoding = params
        .get("packetEncoding")
        .cloned()
        .or_else(|| Some("xudp".to_string()));

    let security = params.get("security").map(|s| s.as_str()).unwrap_or("none");
    let tls_enabled = security == "tls" || security == "reality";

    let mut tls = TlsConfig {
        enabled: tls_enabled,
        server_name: params.get("sni").cloned(),
        insecure: params.get("allowInsecure").map(|v| v == "1").unwrap_or(false),
        alpn: params
            .get("alpn")
            .map(|a| a.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        reality: None,
    };

    if security == "reality" {
        tls.reality = Some(RealityConfig {
            enabled: true,
            public_key: params.get("pbk").cloned().unwrap_or_default(),
            short_id: params.get("sid").cloned(),
        });
    }

    let transport_type = params.get("type").map(|s| s.as_str()).unwrap_or("tcp");
    let transport = match transport_type {
        "ws" => TransportConfig::Ws {
            path: params.get("path").cloned(),
            headers: params.get("host").map(|h| {
                let mut m = HashMap::new();
                m.insert("Host".to_string(), h.clone());
                m
            }),
        },
        "grpc" => TransportConfig::Grpc {
            service_name: params.get("serviceName").cloned().unwrap_or_default(),
        },
        "httpupgrade" => TransportConfig::HttpUpgrade {
            host: params.get("host").cloned(),
            path: params.get("path").cloned(),
        },
        _ => TransportConfig::Tcp,
    };

    Ok((
        name,
        OutboundConfig::Vless(VlessOutbound {
            server: host,
            server_port: port,
            uuid,
            flow,
            packet_encoding,
            tls,
            transport,
            multiplex: MuxConfig::default(),
        }),
    ))
}

pub fn parse_vmess(link: &str) -> Result<(String, OutboundConfig)> {
    let payload = link
        .trim_start_matches("vmess://")
        .trim();
    
    // Decode base64
    let decoded = STANDARD
        .decode(payload)
        .or_else(|_| URL_SAFE_NO_PAD.decode(payload))
        .map_err(|e| CoreError::InvalidUri(format!("Invalid vmess base64: {e}")))?;

    let json: serde_json::Value = serde_json::from_slice(&decoded)?;

    let host = json["add"].as_str().unwrap_or("").to_string();
    let port = json["port"]
        .as_u64()
        .map(|p| p as u16)
        .or_else(|| json["port"].as_str().and_then(|p| p.parse::<u16>().ok()))
        .unwrap_or(443);
    let uuid = json["id"].as_str().unwrap_or("").to_string();
    let name = json["ps"].as_str().unwrap_or(&host).to_string();
    let alter_id = json["aid"]
        .as_u64()
        .map(|a| a as u32)
        .or_else(|| json["aid"].as_str().and_then(|a| a.parse::<u32>().ok()))
        .unwrap_or(0);
    let security = json["scy"].as_str().unwrap_or("auto").to_string();

    let tls_val = json["tls"].as_str().unwrap_or("");
    let tls = TlsConfig {
        enabled: tls_val == "tls",
        server_name: json["sni"].as_str().map(|s| s.to_string()),
        alpn: json["alpn"]
            .as_str()
            .map(|a| a.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        insecure: false,
        reality: None,
    };

    let net = json["net"].as_str().unwrap_or("tcp");
    let transport = match net {
        "ws" => TransportConfig::Ws {
            path: json["path"].as_str().map(|s| s.to_string()),
            headers: json["host"].as_str().map(|h| {
                let mut m = HashMap::new();
                m.insert("Host".to_string(), h.to_string());
                m
            }),
        },
        "grpc" => TransportConfig::Grpc {
            service_name: json["path"].as_str().unwrap_or("").to_string(),
        },
        _ => TransportConfig::Tcp,
    };

    Ok((
        name,
        OutboundConfig::Vmess(VmessOutbound {
            server: host,
            server_port: port,
            uuid,
            alter_id,
            security,
            tls,
            transport,
            multiplex: MuxConfig::default(),
        }),
    ))
}

pub fn parse_shadowsocks(link: &str) -> Result<(String, OutboundConfig)> {
    let raw = link.trim_start_matches("ss://").trim();
    let (body, fragment) = match raw.split_once('#') {
        Some((b, f)) => (b, Some(f)),
        None => (raw, None),
    };

    let name = fragment
        .map(|f| urlencoding::decode(f).unwrap_or_else(|_| f.into()).to_string())
        .unwrap_or_else(|| "Shadowsocks".to_string());

    // SIP002 format: ss://[base64(method:password)]@host:port
    if let Some((userinfo, hostport)) = body.split_once('@') {
        let decoded_userinfo = if userinfo.contains(':') {
            userinfo.to_string()
        } else {
            let bytes = STANDARD
                .decode(userinfo)
                .or_else(|_| URL_SAFE_NO_PAD.decode(userinfo))
                .map_err(|e| CoreError::InvalidUri(format!("Invalid ss userinfo base64: {e}")))?;
            String::from_utf8_lossy(&bytes).to_string()
        };

        let (method, password) = decoded_userinfo
            .split_once(':')
            .ok_or_else(|| CoreError::InvalidUri("Missing method:password in ss link".to_string()))?;

        let (host, port_str) = hostport
            .split_once(':')
            .ok_or_else(|| CoreError::InvalidUri("Missing host:port in ss link".to_string()))?;

        let port: u16 = port_str
            .split('/')
            .next()
            .unwrap_or(port_str)
            .split('?')
            .next()
            .unwrap_or(port_str)
            .parse()
            .map_err(|_| CoreError::InvalidUri("Invalid ss port".to_string()))?;

        return Ok((
            name,
            OutboundConfig::Shadowsocks(ShadowsocksOutbound {
                server: host.to_string(),
                server_port: port,
                method: method.to_string(),
                password: password.to_string(),
                plugin: None,
                plugin_opts: None,
                multiplex: MuxConfig::default(),
            }),
        ));
    }

    // Legacy format: ss://[base64(method:password@host:port)]
    let decoded = STANDARD
        .decode(body)
        .or_else(|_| URL_SAFE_NO_PAD.decode(body))
        .map_err(|e| CoreError::InvalidUri(format!("Invalid legacy ss base64: {e}")))?;
    let decoded_str = String::from_utf8_lossy(&decoded);

    let (userinfo, hostport) = decoded_str
        .split_once('@')
        .ok_or_else(|| CoreError::InvalidUri("Invalid legacy ss format".to_string()))?;
    let (method, password) = userinfo
        .split_once(':')
        .ok_or_else(|| CoreError::InvalidUri("Missing method:password in ss".to_string()))?;
    let (host, port_str) = hostport
        .split_once(':')
        .ok_or_else(|| CoreError::InvalidUri("Missing host:port in ss".to_string()))?;
    let port: u16 = port_str
        .parse()
        .map_err(|_| CoreError::InvalidUri("Invalid ss port".to_string()))?;

    Ok((
        name,
        OutboundConfig::Shadowsocks(ShadowsocksOutbound {
            server: host.to_string(),
            server_port: port,
            method: method.to_string(),
            password: password.to_string(),
            plugin: None,
            plugin_opts: None,
            multiplex: MuxConfig::default(),
        }),
    ))
}

pub fn parse_trojan(url: &Url) -> Result<(String, OutboundConfig)> {
    let host = url
        .host_str()
        .ok_or_else(|| CoreError::InvalidUri("Missing host in trojan link".to_string()))?
        .to_string();
    let port = url.port().unwrap_or(443);
    let password = url.username().to_string();
    let name = url_fragment_or(url, &host);

    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let tls = TlsConfig {
        enabled: true,
        server_name: params.get("sni").cloned().or_else(|| Some(host.clone())),
        alpn: params
            .get("alpn")
            .map(|a| a.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        insecure: params.get("allowInsecure").map(|v| v == "1").unwrap_or(false),
        reality: None,
    };

    let transport_type = params.get("type").map(|s| s.as_str()).unwrap_or("tcp");
    let transport = match transport_type {
        "ws" => TransportConfig::Ws {
            path: params.get("path").cloned(),
            headers: params.get("host").map(|h| {
                let mut m = HashMap::new();
                m.insert("Host".to_string(), h.clone());
                m
            }),
        },
        "grpc" => TransportConfig::Grpc {
            service_name: params.get("serviceName").cloned().unwrap_or_default(),
        },
        _ => TransportConfig::Tcp,
    };

    Ok((
        name,
        OutboundConfig::Trojan(TrojanOutbound {
            server: host,
            server_port: port,
            password,
            tls,
            transport,
            multiplex: MuxConfig::default(),
        }),
    ))
}

pub fn parse_hysteria2(url: &Url) -> Result<(String, OutboundConfig)> {
    let host = url
        .host_str()
        .ok_or_else(|| CoreError::InvalidUri("Missing host in hysteria2 link".to_string()))?
        .to_string();
    let port = url.port().unwrap_or(443);
    let auth = if url.username().is_empty() {
        None
    } else {
        Some(url.username().to_string())
    };
    let name = url_fragment_or(url, &host);

    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let tls = TlsConfig {
        enabled: true,
        server_name: params.get("sni").cloned().or_else(|| Some(host.clone())),
        alpn: params
            .get("alpn")
            .map(|a| a.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        insecure: params.get("insecure").map(|v| v == "1").unwrap_or(false),
        reality: None,
    };

    let obfs = params.get("obfs-password").map(|obfs_pass| Hysteria2Obfs {
        obfs_type: params
            .get("obfs")
            .cloned()
            .unwrap_or_else(|| "salamander".to_string()),
        password: obfs_pass.clone(),
    });

    Ok((
        name,
        OutboundConfig::Hysteria2(Hysteria2Outbound {
            server: host,
            server_port: port,
            auth,
            obfs,
            tls,
            up_mbps: params.get("up").and_then(|u| u.parse().ok()),
            down_mbps: params.get("down").and_then(|d| d.parse().ok()),
        }),
    ))
}

pub fn parse_tuic(url: &Url) -> Result<(String, OutboundConfig)> {
    let host = url
        .host_str()
        .ok_or_else(|| CoreError::InvalidUri("Missing host in tuic link".to_string()))?
        .to_string();
    let port = url.port().unwrap_or(8443);
    let uuid = url.username().to_string();
    let password = url.password().unwrap_or("").to_string();
    let name = url_fragment_or(url, &host);

    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();

    let tls = TlsConfig {
        enabled: true,
        server_name: params.get("sni").cloned().or_else(|| Some(host.clone())),
        alpn: params
            .get("alpn")
            .map(|a| a.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default(),
        insecure: params.get("allow_insecure").map(|v| v == "1").unwrap_or(false),
        reality: None,
    };

    Ok((
        name,
        OutboundConfig::Tuic(TuicOutbound {
            server: host,
            server_port: port,
            uuid,
            password,
            congestion_control: params
                .get("congestion_control")
                .cloned()
                .unwrap_or_else(|| "bbr".to_string()),
            udp_relay_mode: params
                .get("udp_relay_mode")
                .cloned()
                .unwrap_or_else(|| "native".to_string()),
            zero_rtt_handshake: params.get("zero_rtt_handshake").map(|v| v == "1").unwrap_or(false),
            tls,
        }),
    ))
}

pub fn parse_wireguard(url: &Url) -> Result<(String, OutboundConfig)> {
    let host = url
        .host_str()
        .ok_or_else(|| CoreError::InvalidUri("Missing host in wireguard link".to_string()))?
        .to_string();
    let port = url.port().unwrap_or(51820);
    let private_key = url.username().to_string();
    let name = url_fragment_or(url, &host);

    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    let public_key = params.get("publickey").cloned().unwrap_or_default();
    let local_address = params
        .get("ip")
        .map(|ip| vec![ip.clone()])
        .unwrap_or_else(|| vec!["10.0.0.2/32".to_string()]);

    Ok((
        name,
        OutboundConfig::Wireguard(WireGuardOutbound {
            server: host,
            server_port: port,
            system_interface: false,
            interface_name: None,
            local_address,
            private_key,
            peer_public_key: public_key,
            pre_shared_key: params.get("presharedkey").cloned(),
            reserved: None,
            mtu: params.get("mtu").and_then(|m| m.parse().ok()),
        }),
    ))
}

pub fn parse_socks(url: &Url) -> Result<(String, OutboundConfig)> {
    let host = url
        .host_str()
        .ok_or_else(|| CoreError::InvalidUri("Missing host in socks link".to_string()))?
        .to_string();
    let port = url.port().unwrap_or(1080);
    let name = url_fragment_or(url, &host);

    let username = if url.username().is_empty() {
        None
    } else {
        Some(url.username().to_string())
    };
    let password = url.password().map(|p| p.to_string());

    Ok((
        name,
        OutboundConfig::Socks(SocksOutbound {
            server: host,
            server_port: port,
            version: "5".to_string(),
            username,
            password,
        }),
    ))
}

pub fn parse_http(url: &Url) -> Result<(String, OutboundConfig)> {
    let host = url
        .host_str()
        .ok_or_else(|| CoreError::InvalidUri("Missing host in http link".to_string()))?
        .to_string();
    let port = url.port().unwrap_or(if url.scheme() == "https" { 443 } else { 8080 });
    let name = url_fragment_or(url, &host);

    let username = if url.username().is_empty() {
        None
    } else {
        Some(url.username().to_string())
    };
    let password = url.password().map(|p| p.to_string());

    Ok((
        name,
        OutboundConfig::Http(HttpOutbound {
            server: host,
            server_port: port,
            username,
            password,
            tls: url.scheme() == "https",
        }),
    ))
}

fn url_fragment_or(url: &Url, default: &str) -> String {
    url.fragment()
        .and_then(|f| urlencoding::decode(f).ok().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}
