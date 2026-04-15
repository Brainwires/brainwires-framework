//! SQLite-backed memory store.

use std::sync::{Arc, Mutex};

use anyhow::Context;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use uuid::Uuid;

use crate::types::{ListMemoriesQuery, Memory, SearchMemoriesRequest, SearchResult};

// ── Store ─────────────────────────────────────────────────────────────────────

/// Thread-safe SQLite memory store.
#[derive(Clone)]
pub struct MemoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl MemoryStore {
    /// Open (or create) the database at the given path.
    pub fn open(path: &str) -> anyhow::Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open memory DB at {path}"))?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             PRAGMA synchronous=NORMAL;",
        )
        .context("Failed to configure SQLite pragmas")?;

        ensure_schema(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Open an in-memory database (useful for tests).
    pub fn in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        ensure_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    // ── Write ops ────────────────────────────────────────────────────────────

    /// Insert a new memory record. Returns the created [`Memory`].
    pub fn add(
        &self,
        user_id: &str,
        agent_id: Option<&str>,
        session_id: Option<&str>,
        content: &str,
        metadata: &serde_json::Value,
    ) -> anyhow::Result<Memory> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let meta_str = serde_json::to_string(metadata)?;

        let conn = self.conn.lock().expect("memory store lock poisoned");
        conn.execute(
            "INSERT INTO memories
               (id, user_id, agent_id, session_id, content, metadata, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id.to_string(),
                user_id,
                agent_id,
                session_id,
                content,
                meta_str,
                now.timestamp_millis(),
                now.timestamp_millis(),
            ],
        )
        .context("Failed to insert memory")?;

        Ok(Memory {
            id,
            memory: content.to_string(),
            user_id: user_id.to_string(),
            agent_id: agent_id.map(str::to_string),
            session_id: session_id.map(str::to_string),
            metadata: metadata.clone(),
            created_at: now,
            updated_at: now,
            categories: None,
        })
    }

    /// Retrieve a single memory by ID.
    pub fn get(&self, id: Uuid) -> anyhow::Result<Option<Memory>> {
        let conn = self.conn.lock().expect("memory store lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, user_id, agent_id, session_id, content, metadata, created_at, updated_at
               FROM memories WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id.to_string()])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row_to_memory(row)?))
        } else {
            Ok(None)
        }
    }

    /// List memories with optional filters and pagination.
    pub fn list(&self, query: &ListMemoriesQuery) -> anyhow::Result<(Vec<Memory>, u64)> {
        let conn = self.conn.lock().expect("memory store lock poisoned");

        // Build a dynamic WHERE clause.
        let mut conditions: Vec<String> = vec!["1=1".to_string()];
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(uid) = &query.user_id {
            conditions.push(format!("user_id = ?{}", binds.len() + 1));
            binds.push(Box::new(uid.clone()));
        }
        if let Some(aid) = &query.agent_id {
            conditions.push(format!("agent_id = ?{}", binds.len() + 1));
            binds.push(Box::new(aid.clone()));
        }
        if let Some(sid) = &query.session_id {
            conditions.push(format!("session_id = ?{}", binds.len() + 1));
            binds.push(Box::new(sid.clone()));
        }

        let where_clause = conditions.join(" AND ");

        // Count total.
        let count_sql = format!("SELECT COUNT(*) FROM memories WHERE {where_clause}");
        let bind_refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
        let total: i64 = conn.query_row(&count_sql, bind_refs.as_slice(), |r| r.get(0))?;
        let total = total as u64;

        // Paginate.
        let offset = (query.page.saturating_sub(1)) as u64 * query.page_size as u64;
        let data_sql = format!(
            "SELECT id, user_id, agent_id, session_id, content, metadata, created_at, updated_at
               FROM memories WHERE {where_clause}
               ORDER BY created_at DESC
               LIMIT ?{} OFFSET ?{}",
            binds.len() + 1,
            binds.len() + 2,
        );
        binds.push(Box::new(query.page_size as i64));
        binds.push(Box::new(offset as i64));

        let bind_refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&data_sql)?;
        let memories = stmt
            .query_map(bind_refs.as_slice(), |row| {
                row_to_memory(row).map_err(anyhow_to_rusqlite)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok((memories, total))
    }

    /// Substring search across memory content.
    pub fn search(&self, req: &SearchMemoriesRequest) -> anyhow::Result<Vec<SearchResult>> {
        let conn = self.conn.lock().expect("memory store lock poisoned");

        let pattern = format!("%{}%", req.query.replace('%', "\\%").replace('_', "\\_"));
        let mut conditions = vec!["content LIKE ?1 ESCAPE '\\'".to_string()];
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(pattern)];

        if let Some(uid) = &req.user_id {
            conditions.push(format!("user_id = ?{}", binds.len() + 1));
            binds.push(Box::new(uid.clone()));
        }
        if let Some(aid) = &req.agent_id {
            conditions.push(format!("agent_id = ?{}", binds.len() + 1));
            binds.push(Box::new(aid.clone()));
        }

        let where_clause = conditions.join(" AND ");
        let sql = format!(
            "SELECT id, user_id, agent_id, session_id, content, metadata, created_at, updated_at
               FROM memories WHERE {where_clause}
               ORDER BY updated_at DESC
               LIMIT ?{}",
            binds.len() + 1,
        );
        binds.push(Box::new(req.limit as i64));

        let bind_refs: Vec<&dyn rusqlite::ToSql> = binds.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let results = stmt
            .query_map(bind_refs.as_slice(), |row| {
                row_to_memory(row).map_err(anyhow_to_rusqlite)
            })?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|m| {
                // Simple score: full-word match = 1.0, substring = 0.7
                let score = if m.memory.to_lowercase().contains(&req.query.to_lowercase()) {
                    0.7
                } else {
                    0.5
                };
                SearchResult { memory: m, score }
            })
            .collect();

        Ok(results)
    }

    /// Update memory content. Returns the updated record.
    pub fn update(&self, id: Uuid, content: &str) -> anyhow::Result<Option<Memory>> {
        let now = Utc::now();
        let conn = self.conn.lock().expect("memory store lock poisoned");

        let rows_affected = conn.execute(
            "UPDATE memories SET content = ?1, updated_at = ?2 WHERE id = ?3",
            params![content, now.timestamp_millis(), id.to_string()],
        )?;

        if rows_affected == 0 {
            return Ok(None);
        }

        let mut stmt = conn.prepare(
            "SELECT id, user_id, agent_id, session_id, content, metadata, created_at, updated_at
               FROM memories WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id.to_string()])?;
        Ok(rows
            .next()?
            .map(|r| row_to_memory(r).expect("row valid after update")))
    }

    /// Delete a memory by ID. Returns true if a row was deleted.
    pub fn delete(&self, id: Uuid) -> anyhow::Result<bool> {
        let conn = self.conn.lock().expect("memory store lock poisoned");
        let rows = conn.execute(
            "DELETE FROM memories WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(rows > 0)
    }

    /// Delete all memories belonging to a user.
    pub fn delete_all_for_user(&self, user_id: &str) -> anyhow::Result<u64> {
        let conn = self.conn.lock().expect("memory store lock poisoned");
        let rows = conn.execute("DELETE FROM memories WHERE user_id = ?1", params![user_id])?;
        Ok(rows as u64)
    }
}

// ── Schema ────────────────────────────────────────────────────────────────────

fn ensure_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memories (
            id          TEXT    PRIMARY KEY,
            user_id     TEXT    NOT NULL,
            agent_id    TEXT,
            session_id  TEXT,
            content     TEXT    NOT NULL,
            metadata    TEXT    NOT NULL DEFAULT '{}',
            created_at  INTEGER NOT NULL,
            updated_at  INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_memories_user ON memories (user_id);
         CREATE INDEX IF NOT EXISTS idx_memories_agent ON memories (agent_id);
         CREATE INDEX IF NOT EXISTS idx_memories_session ON memories (session_id);",
    )
    .context("Failed to create memories schema")
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn anyhow_to_rusqlite(e: anyhow::Error) -> rusqlite::Error {
    rusqlite::Error::InvalidParameterName(e.to_string())
}

// ── Row deserialisation ───────────────────────────────────────────────────────

fn row_to_memory(row: &rusqlite::Row<'_>) -> anyhow::Result<Memory> {
    let id_str: String = row.get(0)?;
    let id = Uuid::parse_str(&id_str).context("Invalid UUID in DB")?;
    let user_id: String = row.get(1)?;
    let agent_id: Option<String> = row.get(2)?;
    let session_id: Option<String> = row.get(3)?;
    let content: String = row.get(4)?;
    let meta_str: String = row.get(5)?;
    let created_ms: i64 = row.get(6)?;
    let updated_ms: i64 = row.get(7)?;

    let metadata: serde_json::Value =
        serde_json::from_str(&meta_str).unwrap_or(serde_json::Value::Object(Default::default()));

    let created_at = DateTime::<Utc>::from_timestamp_millis(created_ms).unwrap_or_else(Utc::now);
    let updated_at = DateTime::<Utc>::from_timestamp_millis(updated_ms).unwrap_or_else(Utc::now);

    Ok(Memory {
        id,
        memory: content,
        user_id,
        agent_id,
        session_id,
        metadata,
        created_at,
        updated_at,
        categories: None,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> MemoryStore {
        MemoryStore::in_memory().unwrap()
    }

    #[test]
    fn add_and_get() {
        let s = store();
        let m = s
            .add(
                "user1",
                None,
                None,
                "the sky is blue",
                &serde_json::Value::Null,
            )
            .unwrap();
        let fetched = s.get(m.id).unwrap().unwrap();
        assert_eq!(fetched.memory, "the sky is blue");
        assert_eq!(fetched.user_id, "user1");
    }

    #[test]
    fn list_with_user_filter() {
        let s = store();
        s.add("u1", None, None, "mem A", &serde_json::Value::Null)
            .unwrap();
        s.add("u1", None, None, "mem B", &serde_json::Value::Null)
            .unwrap();
        s.add("u2", None, None, "mem C", &serde_json::Value::Null)
            .unwrap();

        let q = ListMemoriesQuery {
            user_id: Some("u1".to_string()),
            agent_id: None,
            session_id: None,
            page: 1,
            page_size: 50,
        };
        let (mems, total) = s.list(&q).unwrap();
        assert_eq!(total, 2);
        assert_eq!(mems.len(), 2);
    }

    #[test]
    fn search_returns_matches() {
        let s = store();
        s.add(
            "u1",
            None,
            None,
            "rust programming is fast",
            &serde_json::Value::Null,
        )
        .unwrap();
        s.add("u1", None, None, "python is easy", &serde_json::Value::Null)
            .unwrap();

        let req = SearchMemoriesRequest {
            query: "rust".to_string(),
            user_id: Some("u1".to_string()),
            agent_id: None,
            limit: 10,
        };
        let results = s.search(&req).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].memory.memory.contains("rust"));
    }

    #[test]
    fn update_memory() {
        let s = store();
        let m = s
            .add("u1", None, None, "original", &serde_json::Value::Null)
            .unwrap();
        let updated = s.update(m.id, "updated content").unwrap().unwrap();
        assert_eq!(updated.memory, "updated content");
    }

    #[test]
    fn delete_memory() {
        let s = store();
        let m = s
            .add("u1", None, None, "to delete", &serde_json::Value::Null)
            .unwrap();
        assert!(s.delete(m.id).unwrap());
        assert!(s.get(m.id).unwrap().is_none());
        assert!(!s.delete(m.id).unwrap());
    }

    #[test]
    fn delete_all_for_user() {
        let s = store();
        s.add("u1", None, None, "a", &serde_json::Value::Null)
            .unwrap();
        s.add("u1", None, None, "b", &serde_json::Value::Null)
            .unwrap();
        s.add("u2", None, None, "c", &serde_json::Value::Null)
            .unwrap();

        let deleted = s.delete_all_for_user("u1").unwrap();
        assert_eq!(deleted, 2);

        let q = ListMemoriesQuery {
            user_id: Some("u2".to_string()),
            agent_id: None,
            session_id: None,
            page: 1,
            page_size: 50,
        };
        let (mems, _) = s.list(&q).unwrap();
        assert_eq!(mems.len(), 1);
    }
}
