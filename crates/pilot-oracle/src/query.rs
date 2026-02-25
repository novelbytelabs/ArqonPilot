use crate::embed::MiniLM;
use crate::store::OracleStore;
use crate::vector_store::VectorStore;
use anyhow::Result;
use std::fs;

pub struct QueryEngine {
    store: OracleStore,
    vector_store: VectorStore,
    model: MiniLM,
    root: std::path::PathBuf,
}

#[derive(Debug)]
pub struct QueryResult {
    pub name: String,
    pub path: String,
    pub score: f32,
    pub snippet: String,
}

impl QueryEngine {
    pub async fn new(db_path: &str, vector_uri: &str) -> Result<Self> {
        let store = OracleStore::open(db_path)?;
        let vector_store = VectorStore::new(vector_uri).await?;
        let model = MiniLM::new()?;
        let root = std::env::current_dir()?;

        Ok(Self {
            store,
            vector_store,
            model,
            root,
        })
    }

    pub async fn query(&mut self, text: &str) -> Result<Vec<QueryResult>> {
        // 1. Embed query
        let vec = self.model.embed(text)?;

        // 2. Vector Search
        let hits = self.vector_store.search(vec, 5).await?;

        // 3. Enrich with Graph Data
        let mut results = Vec::new();
        for (id, score) in hits {
            if let Some(node) = self.store.get_node_by_id(id) {
                // Read snippet from file
                let snippet = self.get_snippet(&node.path, node.start_line, node.end_line);

                results.push(QueryResult {
                    name: node.name,
                    path: node.path,
                    score,
                    snippet,
                });
            }
        }

        Ok(results)
    }

    fn get_snippet(&self, path: &str, start: usize, end: usize) -> String {
        let file_path = self.root.join(path);
        if let Ok(content) = fs::read_to_string(&file_path) {
            let lines: Vec<&str> = content.lines().collect();
            let start = start.min(lines.len());
            let end = end.min(lines.len());
            lines[start..=end.min(start + 10)].join("\n")
        } else {
            String::new()
        }
    }
}
