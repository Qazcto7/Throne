use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteProfile {
    pub id: i64,
    pub name: String,
    pub default_outbound_id: i64, // -1: proxy, -2: direct, -3: block
    pub is_raw: bool,
    pub raw_route: String,
    pub dns_json: String,
    pub custom_dns: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl RouteProfile {
    pub fn default_outbound_tag(&self) -> &'static str {
        match self.default_outbound_id {
            -1 => "proxy",
            -2 => "direct",
            -3 => "block",
            -5 => "warp-bypass",
            _ => "proxy",
        }
    }
}

impl Default for RouteProfile {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Self {
            id: 0,
            name: "Bypass LAN & Direct".to_string(),
            default_outbound_id: -1,
            is_raw: false,
            raw_route: String::new(),
            dns_json: String::new(),
            custom_dns: false,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteRule {
    pub id: i64,
    pub route_profile_id: i64,
    pub rule_order: i32,
    pub name: String,
    pub target: String, // "proxy", "direct", "block"
    pub domains: Vec<String>,
    pub domain_suffixes: Vec<String>,
    pub domain_keywords: Vec<String>,
    pub ip_cidrs: Vec<String>,
    pub rule_sets: Vec<String>,
}

impl Default for RouteRule {
    fn default() -> Self {
        Self {
            id: 0,
            route_profile_id: 0,
            rule_order: 0,
            name: "Rule".to_string(),
            target: "direct".to_string(),
            domains: Vec::new(),
            domain_suffixes: Vec::new(),
            domain_keywords: Vec::new(),
            ip_cidrs: Vec::new(),
            rule_sets: Vec::new(),
        }
    }
}
