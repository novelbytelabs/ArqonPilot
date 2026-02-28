use pilot::oracle::vector_store::VectorStore;
use tempfile::tempdir;

#[tokio::test]
async fn test_vector_search() {
    let dir = tempdir().unwrap();
    let uri = dir.path().to_str().unwrap();

    // 1. Init
    let mut store: VectorStore = VectorStore::new(uri).await.expect("Failed to init store");
    store
        .create_table_if_not_exists()
        .await
        .expect("Failed to create table");

    // 2. Insert one real vector embedding
    let mut embedding = vec![0.0_f32; 384];
    embedding[0] = 1.0;
    embedding[5] = 0.5;
    store
        .add_embeddings(
            vec![42],
            vec![embedding.clone()],
            vec!["fn demo() {}".to_string()],
        )
        .await
        .expect("Failed to add embeddings");

    // 3. Search with the same vector and verify that inserted id is returned
    let hits = store
        .search(embedding, 3)
        .await
        .expect("Failed to search embeddings");
    assert!(!hits.is_empty(), "Expected at least one vector hit");
    assert!(
        hits.iter().any(|(id, _)| *id == 42),
        "Expected inserted id 42 in vector search hits: {:?}",
        hits
    );
}
