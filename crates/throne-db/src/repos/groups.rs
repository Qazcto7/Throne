use crate::error::Result;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use throne_core::Group;

pub struct GroupsRepo {
    conn: Arc<Mutex<Connection>>,
}

impl GroupsRepo {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn insert(&self, g: &Group) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            r#"
            INSERT INTO groups (
                name, url, archive, skip_auto_update, info, sub_last_update, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                g.name,
                g.url,
                g.archive as i32,
                g.skip_auto_update as i32,
                g.info,
                g.sub_last_update,
                now,
                now
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn update(&self, g: &Group) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            r#"
            UPDATE groups SET
                name = ?1, url = ?2, archive = ?3, skip_auto_update = ?4, info = ?5,
                sub_last_update = ?6, updated_at = ?7
            WHERE id = ?8
            "#,
            params![
                g.name,
                g.url,
                g.archive as i32,
                g.skip_auto_update as i32,
                g.info,
                g.sub_last_update,
                now,
                g.id
            ],
        )?;

        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM groups WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get(&self, id: i64) -> Result<Option<Group>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, url, archive, skip_auto_update, info, sub_last_update, created_at, updated_at
            FROM groups WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Self::row_to_group(row)?))
        } else {
            Ok(None)
        }
    }

    pub fn list_all(&self) -> Result<Vec<Group>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, name, url, archive, skip_auto_update, info, sub_last_update, created_at, updated_at
            FROM groups ORDER BY id ASC
            "#,
        )?;

        let rows = stmt.query_map([], Self::row_to_group)?;
        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn update_sub_timestamp(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "UPDATE groups SET sub_last_update = ?1, updated_at = ?2 WHERE id = ?3",
            params![now, now, id],
        )?;
        Ok(())
    }

    fn row_to_group(row: &rusqlite::Row) -> rusqlite::Result<Group> {
        let archive_int: i32 = row.get(3)?;
        let skip_int: i32 = row.get(4)?;
        Ok(Group {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            archive: archive_int != 0,
            skip_auto_update: skip_int != 0,
            info: row.get(5)?,
            sub_last_update: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }
}
