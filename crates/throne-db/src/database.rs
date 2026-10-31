use crate::error::Result;
use crate::repos::{GroupsRepo, ProfilesRepo, RoutesRepo, SettingsRepo};
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};
use throne_core::{Group, RouteProfile, RouteRule};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    pub profiles: Arc<ProfilesRepo>,
    pub groups: Arc<GroupsRepo>,
    pub routes: Arc<RoutesRepo>,
    pub settings: Arc<SettingsRepo>,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self> {
        // Performance pragmas
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            "#,
        )?;

        let conn_arc = Arc::new(Mutex::new(conn));
        let db = Self {
            profiles: Arc::new(ProfilesRepo::new(Arc::clone(&conn_arc))),
            groups: Arc::new(GroupsRepo::new(Arc::clone(&conn_arc))),
            routes: Arc::new(RoutesRepo::new(Arc::clone(&conn_arc))),
            settings: Arc::new(SettingsRepo::new(Arc::clone(&conn_arc))),
            conn: conn_arc,
        };

        db.migrate()?;
        db.seed_defaults()?;

        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS groups (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT,
                archive INTEGER NOT NULL DEFAULT 0,
                skip_auto_update INTEGER NOT NULL DEFAULT 0,
                info TEXT,
                sub_last_update INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS profiles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                type TEXT NOT NULL,
                name TEXT,
                gid INTEGER NOT NULL DEFAULT 0,
                latency INTEGER NOT NULL DEFAULT 0,
                dl_speed TEXT,
                ul_speed TEXT,
                test_country TEXT,
                ip_out TEXT,
                outbound_json TEXT NOT NULL,
                traffic_dl INTEGER NOT NULL DEFAULT 0,
                traffic_up INTEGER NOT NULL DEFAULT 0,
                endpoint_json TEXT NOT NULL DEFAULT '{}',
                latency_at INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_profiles_gid ON profiles(gid);
            CREATE INDEX IF NOT EXISTS idx_profiles_name ON profiles(name);

            CREATE TABLE IF NOT EXISTS route_profiles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                default_outbound TEXT NOT NULL DEFAULT 'proxy',
                is_raw INTEGER NOT NULL DEFAULT 0,
                raw_route TEXT NOT NULL DEFAULT '',
                dns_json TEXT NOT NULL DEFAULT '',
                custom_dns INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS route_rules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                route_profile_id INTEGER NOT NULL,
                rule_order INTEGER NOT NULL,
                name TEXT NOT NULL DEFAULT '',
                target TEXT NOT NULL DEFAULT 'direct',
                domains_json TEXT NOT NULL DEFAULT '[]',
                domain_suffixes_json TEXT NOT NULL DEFAULT '[]',
                domain_keywords_json TEXT NOT NULL DEFAULT '[]',
                ip_cidrs_json TEXT NOT NULL DEFAULT '[]',
                rule_sets_json TEXT NOT NULL DEFAULT '[]',
                FOREIGN KEY(route_profile_id) REFERENCES route_profiles(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;

        Ok(())
    }

    fn seed_defaults(&self) -> Result<()> {
        // Seed default group if empty
        let groups = self.groups.list_all()?;
        if groups.is_empty() {
            self.groups.insert(&Group {
                id: 1,
                name: "Default".to_string(),
                ..Default::default()
            })?;
        }

        // Seed default routing profiles if empty
        let route_profiles = self.routes.list_profiles()?;
        if route_profiles.is_empty() {
            let pid = self.routes.insert_profile(&RouteProfile {
                name: "Bypass LAN & Private".to_string(),
                default_outbound: "proxy".to_string(),
                ..Default::default()
            })?;

            self.routes.insert_rule(&RouteRule {
                route_profile_id: pid,
                rule_order: 1,
                name: "Private IPs".to_string(),
                target: "direct".to_string(),
                ip_cidrs: vec![
                    "10.0.0.0/8".to_string(),
                    "172.16.0.0/12".to_string(),
                    "192.168.0.0/16".to_string(),
                    "127.0.0.0/8".to_string(),
                ],
                ..Default::default()
            })?;
        }

        Ok(())
    }
}
