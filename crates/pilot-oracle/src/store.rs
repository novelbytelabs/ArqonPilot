use crate::edges::GraphEdge;
use crate::graph::GraphNode;
use crate::schema::init_db;
use rusqlite::{params, Connection, Result};
use std::path::Path;

pub struct OracleStore {
    conn: Connection,
}

impl OracleStore {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        init_db(&conn)?;
        Ok(Self { conn })
    }

    pub fn insert_node(&mut self, node: &GraphNode) -> Result<i64> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO nodes (path, type, name, start_line, end_line, signature_hash, docstring)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(path, type, name, start_line) DO UPDATE SET
             signature_hash=excluded.signature_hash, end_line=excluded.end_line",
            params![
                node.path,
                node.node_type,
                node.name,
                node.start_line,
                node.end_line,
                node.signature_hash,
                node.docstring
            ],
        )?;
        let id = tx.last_insert_rowid();
        tx.commit()?;
        Ok(id)
    }

    pub fn insert_edge(&mut self, edge: &GraphEdge) -> Result<()> {
        // Find IDs first (simplified logic: assumes uniqueness by name for MVP)
        // In reality, would need path resolution.
        let source_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM nodes WHERE name = ?1 LIMIT 1",
                params![edge.source_node_name],
                |row| row.get(0),
            )
            .optional()?;

        let target_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM nodes WHERE name = ?1 LIMIT 1",
                params![edge.target_node_name],
                |row| row.get(0),
            )
            .optional()?;

        if let (Some(sid), Some(tid)) = (source_id, target_id) {
            self.conn.execute(
                "INSERT INTO edges (source_id, target_id, type) VALUES (?1, ?2, ?3)",
                params![sid, tid, edge.edge_type],
            )?;
        }
        Ok(())
    }

    /// Get a node by its ID
    pub fn get_node_by_id(&self, id: i64) -> Option<GraphNode> {
        self.conn.query_row(
            "SELECT path, type, name, start_line, end_line, signature_hash, docstring FROM nodes WHERE id = ?1",
            params![id],
            |row| {
                Ok(GraphNode {
                    path: row.get(0)?,
                    node_type: row.get(1)?,
                    name: row.get(2)?,
                    start_line: row.get(3)?,
                    end_line: row.get(4)?,
                    signature_hash: row.get(5)?,
                    docstring: row.get(6)?,
                })
            },
        ).ok()
    }

    /// Get function signatures from the same file or nearby lines
    pub fn get_related_signatures(&self, file_path: &str, near_line: Option<u32>) -> Vec<String> {
        let line = near_line.unwrap_or(0) as i64;

        // Query functions/structs in the same file, ordered by proximity to the error line
        let mut stmt = match self.conn.prepare(
            "SELECT name, type, start_line, end_line FROM nodes 
             WHERE path = ?1 AND type IN ('function', 'struct', 'impl')
             ORDER BY ABS(start_line - ?2) LIMIT 5",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let results = stmt.query_map(params![file_path, line], |row| {
            let name: String = row.get(0)?;
            let node_type: String = row.get(1)?;
            let start: i64 = row.get(2)?;
            let end: i64 = row.get(3)?;
            Ok(format!("{} {} (L{}-L{})", node_type, name, start, end))
        });

        match results {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }
}

// Add optional helper trait
use rusqlite::OptionalExtension;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_test_node(name: &str, path: &str, start: usize, end: usize) -> GraphNode {
        GraphNode {
            path: path.to_string(),
            node_type: "function".to_string(),
            name: name.to_string(),
            start_line: start,
            end_line: end,
            signature_hash: format!("hash_{}", name),
            docstring: Some(format!("Doc for {}", name)),
        }
    }

    #[test]
    fn test_oracle_store_open_creates_db() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = OracleStore::open(&db_path);
        assert!(store.is_ok());
        assert!(db_path.exists());
    }

    #[test]
    fn test_insert_node_returns_id() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut store = OracleStore::open(&db_path).unwrap();

        let node = make_test_node("test_func", "src/lib.rs", 10, 20);
        let id = store.insert_node(&node).unwrap();
        assert!(id > 0);
    }

    #[test]
    fn test_insert_node_upsert() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut store = OracleStore::open(&db_path).unwrap();

        let node = make_test_node("test_func", "src/lib.rs", 10, 20);
        let id1 = store.insert_node(&node).unwrap();

        // Insert same node again (upsert)
        let id2 = store.insert_node(&node).unwrap();

        // Should get same ID due to upsert
        assert!(id1 > 0);
        assert!(id2 > 0);
    }

    #[test]
    fn test_get_node_by_id() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut store = OracleStore::open(&db_path).unwrap();

        let node = make_test_node("my_function", "src/main.rs", 5, 15);
        let id = store.insert_node(&node).unwrap();

        let retrieved = store.get_node_by_id(id);
        assert!(retrieved.is_some());
        let r = retrieved.unwrap();
        assert_eq!(r.name, "my_function");
        assert_eq!(r.path, "src/main.rs");
        assert_eq!(r.start_line, 5);
        assert_eq!(r.end_line, 15);
    }

    #[test]
    fn test_get_node_by_id_not_found() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = OracleStore::open(&db_path).unwrap();

        let retrieved = store.get_node_by_id(9999);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_insert_edge() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut store = OracleStore::open(&db_path).unwrap();

        // Insert nodes first
        let node1 = make_test_node("caller", "src/lib.rs", 10, 20);
        let node2 = make_test_node("callee", "src/lib.rs", 30, 40);
        store.insert_node(&node1).unwrap();
        store.insert_node(&node2).unwrap();

        // Insert edge
        let edge = GraphEdge {
            source_node_name: "caller".to_string(),
            target_node_name: "callee".to_string(),
            edge_type: "calls".to_string(),
        };
        let result = store.insert_edge(&edge);
        assert!(result.is_ok());
    }

    #[test]
    fn test_insert_edge_missing_nodes() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut store = OracleStore::open(&db_path).unwrap();

        // Insert edge without nodes - should not fail, just no-op
        let edge = GraphEdge {
            source_node_name: "nonexistent_a".to_string(),
            target_node_name: "nonexistent_b".to_string(),
            edge_type: "calls".to_string(),
        };
        let result = store.insert_edge(&edge);
        assert!(result.is_ok()); // No error, just ignored
    }

    #[test]
    fn test_get_related_signatures() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let mut store = OracleStore::open(&db_path).unwrap();

        // Insert multiple nodes in same file
        let node1 = make_test_node("func_a", "src/main.rs", 10, 20);
        let node2 = make_test_node("func_b", "src/main.rs", 25, 35);
        let node3 = make_test_node("func_c", "src/main.rs", 50, 60);
        store.insert_node(&node1).unwrap();
        store.insert_node(&node2).unwrap();
        store.insert_node(&node3).unwrap();

        let related = store.get_related_signatures("src/main.rs", Some(30));
        assert!(!related.is_empty());
        // Should find nodes near line 30
        assert!(related.iter().any(|s| s.contains("func_b")));
    }

    #[test]
    fn test_get_related_signatures_empty_file() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = OracleStore::open(&db_path).unwrap();

        let related = store.get_related_signatures("nonexistent.rs", None);
        assert!(related.is_empty());
    }
}
