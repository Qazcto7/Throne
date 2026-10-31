use crate::error::Result;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use throne_core::Profile;

pub struct ProfilesRepo {
    conn: Arc<Mutex<Connection>>,
}

impl ProfilesRepo {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn insert(&self, p: &Profile) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            r#"
            INSERT INTO profiles (
                type, name, gid, latency, dl_speed, ul_speed, test_country, ip_out,
                outbound_json, traffic_dl, traffic_up, endpoint_json, latency_at, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            "#,
            params![
                p.profile_type,
                p.name,
                p.gid,
                p.latency,
                p.dl_speed,
                p.ul_speed,
                p.test_country,
                p.ip_out,
                p.outbound_json,
                p.traffic_dl,
                p.traffic_up,
                p.endpoint_json,
                p.latency_at,
                now,
                now
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn update(&self, p: &Profile) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            r#"
            UPDATE profiles SET
                type = ?1, name = ?2, gid = ?3, latency = ?4, dl_speed = ?5, ul_speed = ?6,
                test_country = ?7, ip_out = ?8, outbound_json = ?9, traffic_dl = ?10, traffic_up = ?11,
                endpoint_json = ?12, latency_at = ?13, updated_at = ?14
            WHERE id = ?15
            "#,
            params![
                p.profile_type,
                p.name,
                p.gid,
                p.latency,
                p.dl_speed,
                p.ul_speed,
                p.test_country,
                p.ip_out,
                p.outbound_json,
                p.traffic_dl,
                p.traffic_up,
                p.endpoint_json,
                p.latency_at,
                now,
                p.id
            ],
        )?;

        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM profiles WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn delete_by_group(&self, gid: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM profiles WHERE gid = ?1", params![gid])?;
        Ok(())
    }

    pub fn get(&self, id: i64) -> Result<Option<Profile>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, type, name, gid, latency, dl_speed, ul_speed, test_country, ip_out,
                   outbound_json, traffic_dl, traffic_up, endpoint_json, latency_at, created_at, updated_at
            FROM profiles WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_profile(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn list_all(&self) -> Result<Vec<Profile>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, type, name, gid, latency, dl_speed, ul_speed, test_country, ip_out,
                   outbound_json, traffic_dl, traffic_up, endpoint_json, latency_at, created_at, updated_at
            FROM profiles ORDER BY id ASC
            "#,
        )?;

        let rows = stmt.query_map([], |row| Self::row_to_profile(row))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn list_by_group(&self, gid: i64) -> Result<Vec<Profile>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, type, name, gid, latency, dl_speed, ul_speed, test_country, ip_out,
                   outbound_json, traffic_dl, traffic_up, endpoint_json, latency_at, created_at, updated_at
            FROM profiles WHERE gid = ?1 ORDER BY id ASC
            "#,
        )?;

        let rows = stmt.query_map(params![gid], |row| Self::row_to_profile(row))?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_latency(&self, id: i64, latency: i32) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "UPDATE profiles SET latency = ?1, latency_at = ?2 WHERE id = ?3",
            params![latency, now, id],
        )?;
        Ok(())
    }

    fn row_to_profile(row: &rusqlite::Row) -> rusqlite::Result<Profile> {
        Ok(Profile {
            id: row.get(0)?,
            profile_type: row.get(1)?,
            name: row.get(2)?,
            gid: row.get(3)?,
            latency: row.get(4)?,
            dl_speed: row.get(5)?,
            ul_speed: row.get(6)?,
            test_country: row.get(7)?,
            ip_out: row.get(8)?,
            outbound_json: row.get(9)?,
            traffic_dl: row.get(10)?,
            traffic_up: row.get(11)?,
            endpoint_json: row.get(12)?,
            latency_at: row.get(13)?,
            created_at: row.get(14)?,
            updated_at: row.get(15)?,
        })
    }
}
