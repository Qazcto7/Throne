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
                default_outbound_id INTEGER NOT NULL DEFAULT -1,
                is_raw INTEGER NOT NULL DEFAULT 0,
                raw_route TEXT NOT NULL DEFAULT '',
                dns_json TEXT NOT NULL DEFAULT '',
                custom_dns INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS route_rules (
                route_profile_id INTEGER NOT NULL,
                rule_order INTEGER NOT NULL,
                name TEXT NOT NULL DEFAULT '',
                outbound_id INTEGER NOT NULL DEFAULT -2,
                domain_json TEXT DEFAULT '[]',
                domain_suffix_json TEXT DEFAULT '[]',
                domain_keyword_json TEXT DEFAULT '[]',
                ip_cidr_json TEXT DEFAULT '[]',
                process_name_json TEXT DEFAULT '[]',
                rule_set_json TEXT DEFAULT '[]',
                PRIMARY KEY (route_profile_id, rule_order),
                FOREIGN KEY(route_profile_id) REFERENCES route_profiles(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;

        // Run migrations for existing databases
        if !Self::column_exists(&conn, "profiles", "latency_at") {
            let _ = conn.execute("ALTER TABLE profiles ADD COLUMN latency_at INTEGER NOT NULL DEFAULT 0", []);
        }
        if !Self::column_exists(&conn, "profiles", "endpoint_json") {
            let _ = conn.execute("ALTER TABLE profiles ADD COLUMN endpoint_json TEXT NOT NULL DEFAULT '{}'", []);
        }
        if !Self::column_exists(&conn, "route_profiles", "dns_json") {
            let _ = conn.execute("ALTER TABLE route_profiles ADD COLUMN dns_json TEXT NOT NULL DEFAULT ''", []);
        }
        if !Self::column_exists(&conn, "route_profiles", "custom_dns") {
            let _ = conn.execute("ALTER TABLE route_profiles ADD COLUMN custom_dns INTEGER NOT NULL DEFAULT 0", []);
        }
        if !Self::column_exists(&conn, "route_rules", "outbound_id") {
            let _ = conn.execute("ALTER TABLE route_rules ADD COLUMN outbound_id INTEGER NOT NULL DEFAULT -2", []);
        }
        if !Self::column_exists(&conn, "route_rules", "domain_json") {
            let _ = conn.execute("ALTER TABLE route_rules ADD COLUMN domain_json TEXT DEFAULT '[]'", []);
        }
        if !Self::column_exists(&conn, "route_rules", "domain_suffix_json") {
            let _ = conn.execute("ALTER TABLE route_rules ADD COLUMN domain_suffix_json TEXT DEFAULT '[]'", []);
        }
        if !Self::column_exists(&conn, "route_rules", "domain_keyword_json") {
            let _ = conn.execute("ALTER TABLE route_rules ADD COLUMN domain_keyword_json TEXT DEFAULT '[]'", []);
        }
        if !Self::column_exists(&conn, "route_rules", "ip_cidr_json") {
            let _ = conn.execute("ALTER TABLE route_rules ADD COLUMN ip_cidr_json TEXT DEFAULT '[]'", []);
        }
        if !Self::column_exists(&conn, "route_rules", "process_name_json") {
            let _ = conn.execute("ALTER TABLE route_rules ADD COLUMN process_name_json TEXT DEFAULT '[]'", []);
        }
        if !Self::column_exists(&conn, "route_rules", "rule_set_json") {
            let _ = conn.execute("ALTER TABLE route_rules ADD COLUMN rule_set_json TEXT DEFAULT '[]'", []);
        }

        Ok(())
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> bool {
        let pragma_sql = format!("PRAGMA table_info({})", table);
        let Ok(mut stmt) = conn.prepare(&pragma_sql) else {
            return false;
        };
        let Ok(mut rows) = stmt.query([]) else {
            return false;
        };
        while let Ok(Some(row)) = rows.next() {
            if let Ok(name) = row.get::<_, String>(1)
                && name == column
            {
                return true;
            }
        }
        false
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
                default_outbound_id: -1,
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
