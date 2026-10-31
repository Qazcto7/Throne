use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub id: i64,
    #[serde(rename = "type")]
    pub profile_type: String,
    pub name: String,
    pub gid: i64,
    pub latency: i32,
    pub dl_speed: Option<String>,
    pub ul_speed: Option<String>,
    pub test_country: Option<String>,
    pub ip_out: Option<String>,
    pub outbound_json: String,
    pub traffic_dl: i64,
    pub traffic_up: i64,
    pub endpoint_json: String,
    pub latency_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for Profile {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Self {
            id: 0,
            profile_type: "vless".to_string(),
            name: "New Profile".to_string(),
            gid: 0,
            latency: 0,
            dl_speed: None,
            ul_speed: None,
            test_country: None,
            ip_out: None,
            outbound_json: "{}".to_string(),
            traffic_dl: 0,
            traffic_up: 0,
            endpoint_json: "{}".to_string(),
            latency_at: 0,
            created_at: now,
            updated_at: now,
        }
    }
}
