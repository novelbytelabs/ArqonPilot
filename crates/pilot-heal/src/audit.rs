use crate::parser_rust::TestFailure;
use crate::r#loop::HealOutcome;
use anyhow::Result;
use chrono::Utc;
use pilot_oracle::hash::compute_hash;
use rusqlite::{params, Connection};
use uuid::Uuid;

pub struct AuditLog {
    conn: Connection,
}

impl AuditLog {
    pub fn open(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let audit = Self { conn };
        audit.init_db()?;
        Ok(audit)
    }

    /// Initialize the healing_attempts table if it doesn't exist
    fn init_db(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS healing_attempts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                file_path TEXT NOT NULL,
                error_msg TEXT NOT NULL,
                prompt_hash TEXT NOT NULL,
                diff_hash TEXT NOT NULL,
                outcome TEXT NOT NULL
            )",
            [],
        )?;
        Ok(())
    }

    pub fn log_attempt(
        &self,
        failure: &TestFailure,
        prompt: &str,
        fix: &str,
        outcome: &HealOutcome,
    ) -> Result<()> {
        let run_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().to_rfc3339();
        let prompt_hash = compute_hash(prompt);
        let diff_hash = compute_hash(fix);
        let outcome_str = format!("{:?}", outcome);

        self.conn.execute(
            "INSERT INTO healing_attempts (run_id, timestamp, file_path, error_msg, prompt_hash, diff_hash, outcome)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                run_id,
                timestamp,
                failure.file_path,
                failure.error_message,
                prompt_hash,
                diff_hash,
                outcome_str
            ],
        )?;

        Ok(())
    }
}
