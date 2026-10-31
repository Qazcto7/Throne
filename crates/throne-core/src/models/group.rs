use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub url: Option<String>,
    pub archive: bool,
    pub skip_auto_update: bool,
    pub info: Option<String>,
    pub sub_last_update: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Default for Group {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Self {
            id: 0,
            name: "Default Group".to_string(),
            url: None,
            archive: false,
            skip_auto_update: false,
            info: None,
            sub_last_update: 0,
            created_at: now,
            updated_at: now,
        }
    }
}
