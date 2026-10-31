use std::sync::Arc;
use throne_core::parser::{parse_clash, parse_subscription};
use throne_core::Profile;
use throne_db::Database;
use tracing::info;

pub struct SubUpdater {
    db: Arc<Database>,
    client: reqwest::Client,
}

impl SubUpdater {
    pub fn new(db: Arc<Database>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Throne/1.4.0 (Rust)")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap_or_default();

        Self { db, client }
    }

    pub async fn update_group(&self, group_id: i64) -> anyhow::Result<usize> {
        let group = self
            .db
            .groups
            .get(group_id)?
            .ok_or_else(|| anyhow::anyhow!("Group {} not found", group_id))?;

        let url = group
            .url
            .ok_or_else(|| anyhow::anyhow!("Group {} has no subscription URL", group_id))?;

        info!("Updating subscription for group '{}' from {}", group.name, url);

        let resp = self.client.get(&url).send().await?.text().await?;

        // Try clash first or standard subscription
        let nodes = if resp.contains("proxies:") {
            parse_clash(&resp)?
        } else {
            parse_subscription(&resp)?
        };

        let count = nodes.len();
        info!("Parsed {} nodes from subscription", count);

        // Delete existing profiles in this group
        self.db.profiles.delete_by_group(group_id)?;

        for (name, config) in nodes {
            let outbound_json = serde_json::to_string(&config)?;
            let profile = Profile {
                name,
                gid: group_id,
                profile_type: config.protocol_name().to_string(),
                outbound_json,
                ..Default::default()
            };
            self.db.profiles.insert(&profile)?;
        }

        self.db.groups.update_sub_timestamp(group_id)?;
        Ok(count)
    }
}
