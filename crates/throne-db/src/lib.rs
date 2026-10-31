pub mod database;
pub mod error;
pub mod repos;

pub use database::Database;
pub use error::{DbError, Result};
pub use repos::*;

#[cfg(test)]
mod tests {
    use super::*;
    use throne_core::Profile;

    #[test]
    fn test_db_in_memory() {
        let db = Database::open_in_memory().expect("Failed to open in-memory db");

        // Groups
        let groups = db.groups.list_all().expect("list groups failed");
        assert!(!groups.is_empty(), "Default group should be seeded");

        // Settings
        let mut settings = db.settings.load_settings().expect("load settings failed");
        assert_eq!(settings.inbound_port, 2080);
        settings.inbound_port = 10800;
        db.settings.save_settings(&settings).expect("save settings failed");
        let updated = db.settings.load_settings().expect("reload settings failed");
        assert_eq!(updated.inbound_port, 10800);

        // Profiles
        let pid = db
            .profiles
            .insert(&Profile {
                name: "Node 1".to_string(),
                profile_type: "vless".to_string(),
                outbound_json: r#"{"type":"vless"}"#.to_string(),
                ..Default::default()
            })
            .expect("insert profile failed");

        let p = db.profiles.get(pid).expect("get profile failed").expect("profile not found");
        assert_eq!(p.name, "Node 1");
    }
}
