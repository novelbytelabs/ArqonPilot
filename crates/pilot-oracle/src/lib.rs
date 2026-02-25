pub mod edges;
pub mod embed;
pub mod graph;
pub mod hash;
pub mod parser;
pub mod parser_py;
pub mod schema;
pub mod store;
pub mod vector_store;

use anyhow::Result;
use ignore::WalkBuilder; // Add 'ignore' crate for .gitignore support
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path; // Add 'indicatif'

pub use store::OracleStore;
pub use vector_store::VectorStore;
pub mod query;

pub async fn scan_codebase(root: &Path) -> Result<()> {
    println!("Scanning codebase at {:?}", root);

    // 1. Init Stores
    let db_path = root.join(".pilot/graph.db");
    let mut store = OracleStore::open(db_path)?;

    let vector_path = root.join(".pilot/vectors.lance");
    let vector_uri = vector_path.to_str().unwrap();
    let mut vector_store = VectorStore::new(vector_uri).await?;
    vector_store.create_table_if_not_exists().await?;

    // 2. Walk Files
    let walker = WalkBuilder::new(root)
        // .hidden(false) // config overrides
        .build();

    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner());

    let mut graph_builder = graph::GraphBuilder::new()?;
    let mut edge_builder = edges::EdgeBuilder::new()?;
    let mut embedding_model = embed::MiniLM::new()?;

    // Batch vectors for efficient insertion
    let mut pending_ids: Vec<i64> = Vec::new();
    let mut pending_vectors: Vec<Vec<f32>> = Vec::new();
    let mut pending_texts: Vec<String> = Vec::new();
    const BATCH_SIZE: usize = 50;

    for result in walker {
        match result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy();
                        if ext_str == "rs" || ext_str == "py" {
                            pb.set_message(format!("Processing {:?}", path.file_name().unwrap()));

                            let content = std::fs::read_to_string(path)?;
                            let relative_path = path.strip_prefix(root)?.to_string_lossy();

                            // 3. Extract Nodes
                            let nodes = graph_builder.extract_nodes(&relative_path, &content);
                            for node in &nodes {
                                let node_id = store.insert_node(node)?;

                                // Create text for embedding: name + docstring
                                let embed_text = if let Some(ref doc) = node.docstring {
                                    format!("{} {}: {}", node.node_type, node.name, doc)
                                } else {
                                    format!("{} {}", node.node_type, node.name)
                                };

                                // Embed and queue for batch insert
                                if let Ok(vec) = embedding_model.embed(&embed_text) {
                                    pending_ids.push(node_id);
                                    pending_vectors.push(vec);
                                    pending_texts.push(embed_text);
                                }

                                // Flush batch if full
                                if pending_ids.len() >= BATCH_SIZE {
                                    vector_store
                                        .add_embeddings(
                                            std::mem::take(&mut pending_ids),
                                            std::mem::take(&mut pending_vectors),
                                            std::mem::take(&mut pending_texts),
                                        )
                                        .await?;
                                }
                            }

                            // 4. Extract Edges
                            let edges = edge_builder.extract_edges(&relative_path, &content);
                            for edge in edges {
                                store.insert_edge(&edge)?;
                            }
                        }
                    }
                }
            }
            Err(err) => eprintln!("Error walking path: {}", err),
        }
    }

    // Flush any remaining vectors
    if !pending_ids.is_empty() {
        vector_store
            .add_embeddings(pending_ids, pending_vectors, pending_texts)
            .await?;
    }

    pb.finish_with_message("Scan complete.");
    Ok(())
}
