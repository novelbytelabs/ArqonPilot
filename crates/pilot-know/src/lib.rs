use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub id: i64,
    pub created_at: String,
    pub title: String,
    pub context: String,
    pub decision: String,
    pub status: String,
    pub tags: Vec<String>,
}

pub struct KnowStore {
    conn: Connection,
}

impl KnowStore {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed creating know db dir {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("Failed opening know db {}", path.display()))?;
        let store = Self { conn };
        store.init()?;
        Ok(store)
    }

    pub fn default_db_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".pilot").join("know.db")
    }

    fn init(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS decisions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL,
                title TEXT NOT NULL,
                context TEXT NOT NULL,
                decision TEXT NOT NULL,
                status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS decision_tags (
                decision_id INTEGER NOT NULL,
                tag TEXT NOT NULL,
                UNIQUE(decision_id, tag),
                FOREIGN KEY(decision_id) REFERENCES decisions(id) ON DELETE CASCADE
            );
            ",
        )?;
        Ok(())
    }

    pub fn record(
        &self,
        title: &str,
        context: &str,
        decision: &str,
        status: &str,
        tags: &[String],
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO decisions(created_at, title, context, decision, status) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![Utc::now().to_rfc3339(), title, context, decision, status],
        )?;
        let id = self.conn.last_insert_rowid();
        for tag in tags.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
            self.conn.execute(
                "INSERT OR IGNORE INTO decision_tags(decision_id, tag) VALUES (?1, ?2)",
                params![id, tag],
            )?;
        }
        Ok(id)
    }

    pub fn query(&self, q: &str, limit: usize) -> Result<Vec<DecisionRecord>> {
        let pattern = format!("%{}%", q);
        let mut stmt = self.conn.prepare(
            "SELECT id, created_at, title, context, decision, status
             FROM decisions
             WHERE title LIKE ?1 OR context LIKE ?1 OR decision LIKE ?1
             ORDER BY id DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, limit as i64], |row| {
            Ok(DecisionRecord {
                id: row.get(0)?,
                created_at: row.get(1)?,
                title: row.get(2)?,
                context: row.get(3)?,
                decision: row.get(4)?,
                status: row.get(5)?,
                tags: vec![],
            })
        })?;

        let mut out = Vec::new();
        for row in rows {
            let mut rec = row?;
            rec.tags = self.tags_for_decision(rec.id)?;
            out.push(rec);
        }
        Ok(out)
    }

    fn tags_for_decision(&self, id: i64) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag FROM decision_tags WHERE decision_id = ?1 ORDER BY tag")?;
        let rows = stmt.query_map([id], |row| row.get(0))?;
        let mut tags = Vec::new();
        for row in rows {
            tags.push(row?);
        }
        Ok(tags)
    }
}
