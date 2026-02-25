use rusqlite::{Connection, Result};

pub const SCHEMA_V1: &str = r#"
    -- Graph Nodes
    CREATE TABLE IF NOT EXISTS nodes (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        path TEXT NOT NULL,
        type TEXT NOT NULL,
        name TEXT NOT NULL,
        start_line INTEGER NOT NULL,
        end_line INTEGER NOT NULL,
        signature_hash TEXT NOT NULL,
        docstring TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_nodes_path ON nodes(path);
    CREATE INDEX IF NOT EXISTS idx_nodes_name ON nodes(name);
    CREATE UNIQUE INDEX IF NOT EXISTS idx_nodes_unique ON nodes(path, type, name, start_line);

    -- Graph Edges
    CREATE TABLE IF NOT EXISTS edges (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        source_id INTEGER NOT NULL,
        target_id INTEGER NOT NULL,
        type TEXT NOT NULL,
        FOREIGN KEY(source_id) REFERENCES nodes(id) ON DELETE CASCADE,
        FOREIGN KEY(target_id) REFERENCES nodes(id) ON DELETE CASCADE
    );

    CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_id);
    CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_id);

    -- Healing Audit Log
    CREATE TABLE IF NOT EXISTS healing_attempts (
        run_id TEXT PRIMARY KEY,
        timestamp TEXT NOT NULL,
        file_path TEXT NOT NULL,
        error_msg TEXT NOT NULL,
        prompt_hash TEXT NOT NULL,
        diff_hash TEXT NOT NULL,
        outcome TEXT NOT NULL
    );
"#;

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA_V1)?;
    Ok(())
}
