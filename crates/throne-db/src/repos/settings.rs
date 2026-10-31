use crate::error::Result;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use throne_core::Settings;

pub struct SettingsRepo {
    conn: Arc<Mutex<Connection>>,
}

impl SettingsRepo {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn get_value(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_value(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn load_settings(&self) -> Result<Settings> {
        let mut s = Settings::default();

        if let Some(Ok(p)) = self.get_value("inbound_port")?.map(|v| v.parse()) {
            s.inbound_port = p;
        }
        if let Some(v) = self.get_value("inbound_address")? {
            s.inbound_address = v;
        }
        if let Some(v) = self.get_value("enable_tun")? {
            s.enable_tun = v == "true" || v == "1";
        }
        if let Some(v) = self.get_value("tun_interface_name")? {
            s.tun_interface_name = v;
        }
        if let Some(v) = self.get_value("tun_stack")? {
            s.tun_stack = v;
        }
        if let Some(v) = self.get_value("tun_strict_route")? {
            s.tun_strict_route = v == "true" || v == "1";
        }
        if let Some(v) = self.get_value("system_proxy")? {
            s.system_proxy = v == "true" || v == "1";
        }
        if let Some(v) = self.get_value("remote_api_enable")? {
            s.remote_api_enable = v == "true" || v == "1";
        }
        if let Some(Ok(p)) = self.get_value("remote_api_port")?.map(|v| v.parse()) {
            s.remote_api_port = p;
        }
        if let Some(v) = self.get_value("remote_api_secret")? {
            s.remote_api_secret = v;
        }
        if let Some(v) = self.get_value("core_path")? {
            s.core_path = v;
        }
        if let Some(v) = self.get_value("active_profile_id")? {
            s.active_profile_id = v.parse().ok();
        }
        if let Some(v) = self.get_value("active_route_id")? {
            s.active_route_id = v.parse().ok();
        } else if let Some(v) = self.get_value("current_route_id")? {
            s.active_route_id = v.parse().ok();
        }
        if let Some(v) = self.get_value("mux_enabled")? {
            s.mux_enabled = v == "true" || v == "1";
        }
        if let Some(v) = self.get_value("fake_dns")? {
            s.fake_dns = v == "true" || v == "1";
        }
        if let Some(v) = self.get_value("remote_dns")? {
            s.remote_dns = v;
        }
        if let Some(v) = self.get_value("direct_dns")? {
            s.direct_dns = v;
        }
        if let Some(v) = self.get_value("log_level")? {
            s.log_level = v;
        }
        if let Some(v) = self.get_value("byedpi_enable")? {
            s.byedpi_enable = v == "true" || v == "1";
        }
        if let Some(v) = self.get_value("byedpi_path")? {
            s.byedpi_path = v;
        }
        if let Some(v) = self.get_value("byedpi_args")? {
            s.byedpi_args = v;
        }
        if let Some(v) = self.get_value("byedpi_preset")? {
            s.byedpi_preset = v;
        }
        if let Some(v) = self.get_value("dns_predefined_enable")? {
            s.dns_predefined_enable = v == "true" || v == "1";
        }
        if let Some(v) = self.get_value("dns_predefined_rules")?
            && let Ok(rules) = serde_json::from_str::<Vec<String>>(&v)
        {
            s.dns_predefined_rules = rules;
        }

        Ok(s)
    }

    pub fn save_settings(&self, s: &Settings) -> Result<()> {
        self.set_value("inbound_port", &s.inbound_port.to_string())?;
        self.set_value("inbound_address", &s.inbound_address)?;
        self.set_value("enable_tun", if s.enable_tun { "1" } else { "0" })?;
        self.set_value("tun_interface_name", &s.tun_interface_name)?;
        self.set_value("tun_stack", &s.tun_stack)?;
        self.set_value("tun_strict_route", if s.tun_strict_route { "1" } else { "0" })?;
        self.set_value("system_proxy", if s.system_proxy { "1" } else { "0" })?;
        self.set_value("remote_api_enable", if s.remote_api_enable { "1" } else { "0" })?;
        self.set_value("remote_api_port", &s.remote_api_port.to_string())?;
        self.set_value("remote_api_secret", &s.remote_api_secret)?;
        self.set_value("core_path", &s.core_path)?;
        if let Some(id) = s.active_profile_id {
            self.set_value("active_profile_id", &id.to_string())?;
        }
        if let Some(id) = s.active_route_id {
            self.set_value("active_route_id", &id.to_string())?;
            self.set_value("current_route_id", &id.to_string())?;
        }
        self.set_value("mux_enabled", if s.mux_enabled { "1" } else { "0" })?;
        self.set_value("fake_dns", if s.fake_dns { "1" } else { "0" })?;
        self.set_value("remote_dns", &s.remote_dns)?;
        self.set_value("direct_dns", &s.direct_dns)?;
        self.set_value("log_level", &s.log_level)?;
        self.set_value("byedpi_enable", if s.byedpi_enable { "1" } else { "0" })?;
        self.set_value("byedpi_path", &s.byedpi_path)?;
        self.set_value("byedpi_args", &s.byedpi_args)?;
        self.set_value("byedpi_preset", &s.byedpi_preset)?;
        self.set_value("dns_predefined_enable", if s.dns_predefined_enable { "1" } else { "0" })?;
        if let Ok(rules_json) = serde_json::to_string(&s.dns_predefined_rules) {
            self.set_value("dns_predefined_rules", &rules_json)?;
        }
        Ok(())
    }
}
