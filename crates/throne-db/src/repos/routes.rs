use crate::error::Result;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use throne_core::{RouteProfile, RouteRule};

pub struct RoutesRepo {
    conn: Arc<Mutex<Connection>>,
}

impl RoutesRepo {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn insert_profile(&self, rp: &RouteProfile) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            r#"
            INSERT INTO route_profiles (
                name, default_outbound_id, is_raw, raw_route, dns_json, custom_dns, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                rp.name,
                rp.default_outbound_id,
                rp.is_raw as i32,
                rp.raw_route,
                rp.dns_json,
                rp.custom_dns as i32,
                now,
                now
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn get_profile(&self, id: i64) -> Result<Option<RouteProfile>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, default_outbound_id, is_raw, raw_route, dns_json, custom_dns, created_at, updated_at
            FROM route_profiles WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let is_raw_i: i32 = row.get(3)?;
            let custom_dns_i: i32 = row.get(6)?;
            Ok(Some(RouteProfile {
                id: row.get(0)?,
                name: row.get(1)?,
                default_outbound_id: row.get(2)?,
                is_raw: is_raw_i != 0,
                raw_route: row.get(4)?,
                dns_json: row.get(5)?,
                custom_dns: custom_dns_i != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_profiles(&self) -> Result<Vec<RouteProfile>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, default_outbound_id, is_raw, raw_route, dns_json, custom_dns, created_at, updated_at
            FROM route_profiles ORDER BY id ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let is_raw_i: i32 = row.get(3)?;
            let custom_dns_i: i32 = row.get(6)?;
            Ok(RouteProfile {
                id: row.get(0)?,
                name: row.get(1)?,
                default_outbound_id: row.get(2)?,
                is_raw: is_raw_i != 0,
                raw_route: row.get(4)?,
                dns_json: row.get(5)?,
                custom_dns: custom_dns_i != 0,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn insert_rule(&self, r: &RouteRule) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let domains_json = serde_json::to_string(&r.domains)?;
        let suffixes_json = serde_json::to_string(&r.domain_suffixes)?;
        let keywords_json = serde_json::to_string(&r.domain_keywords)?;
        let ip_json = serde_json::to_string(&r.ip_cidrs)?;
        let rule_sets_json = serde_json::to_string(&r.rule_sets)?;

        conn.execute(
            r#"
            INSERT INTO route_rules (
                route_profile_id, rule_order, name, target,
                domains_json, domain_suffixes_json, domain_keywords_json, ip_cidrs_json, rule_sets_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                r.route_profile_id,
                r.rule_order,
                r.name,
                r.target,
                domains_json,
                suffixes_json,
                keywords_json,
                ip_json,
                rule_sets_json
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn list_rules_for_profile(&self, profile_id: i64) -> Result<Vec<RouteRule>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, route_profile_id, rule_order, name, target,
                   domains_json, domain_suffixes_json, domain_keywords_json, ip_cidrs_json, rule_sets_json
            FROM route_rules WHERE route_profile_id = ?1 ORDER BY rule_order ASC
            "#,
        )?;

        let rows = stmt.query_map(params![profile_id], |row| {
            let id: i64 = row.get(0)?;
            let route_profile_id: i64 = row.get(1)?;
            let rule_order: i32 = row.get(2)?;
            let name: String = row.get(3)?;
            let target: String = row.get(4)?;
            let d_str: String = row.get(5)?;
            let s_str: String = row.get(6)?;
            let k_str: String = row.get(7)?;
            let ip_str: String = row.get(8)?;
            let rs_str: String = row.get(9)?;

            let domains = serde_json::from_str(&d_str).unwrap_or_default();
            let domain_suffixes = serde_json::from_str(&s_str).unwrap_or_default();
            let domain_keywords = serde_json::from_str(&k_str).unwrap_or_default();
            let ip_cidrs = serde_json::from_str(&ip_str).unwrap_or_default();
            let rule_sets = serde_json::from_str(&rs_str).unwrap_or_default();

            Ok(RouteRule {
                id,
                route_profile_id,
                rule_order,
                name,
                target,
                domains,
                domain_suffixes,
                domain_keywords,
                ip_cidrs,
                rule_sets,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }
}
