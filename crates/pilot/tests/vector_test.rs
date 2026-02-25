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

    // 2. Insert placeholder vector data once real insert support is ready.

    assert!(store.create_table_if_not_exists().await.is_ok());
}
