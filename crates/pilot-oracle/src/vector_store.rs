use anyhow::Result;
use arrow_array::{
    ArrayRef, FixedSizeListArray, Float32Array, Int64Array, RecordBatch, RecordBatchIterator,
    StringArray,
};
use arrow_schema::FieldRef;
use futures::StreamExt;
use lancedb::arrow::arrow_schema::{DataType, Field, Schema};
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::{connect, Connection, Table};
use std::sync::Arc;

/// Vector dimension for MiniLM embeddings
const VECTOR_DIM: i32 = 384;

pub struct VectorStore {
    conn: Connection,
    table: Option<Table>,
}

impl VectorStore {
    pub async fn new(uri: &str) -> Result<Self> {
        let conn = connect(uri).execute().await?;
        let table = conn.open_table("code_vectors").execute().await.ok();
        Ok(Self { conn, table })
    }

    fn schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    VECTOR_DIM,
                ),
                false,
            ),
            Field::new("text", DataType::Utf8, false),
        ]))
    }

    pub async fn create_table_if_not_exists(&mut self) -> Result<()> {
        if self.table.is_some() {
            return Ok(());
        }

        let table = self
            .conn
            .create_empty_table("code_vectors", Self::schema())
            .execute()
            .await?;
        self.table = Some(table);
        Ok(())
    }

    /// Add embeddings to the vector store
    ///
    /// # Arguments
    /// * `ids` - Node IDs from the graph database
    /// * `vectors` - 384-dimensional embedding vectors
    /// * `texts` - Source text for each vector
    pub async fn add_embeddings(
        &mut self,
        ids: Vec<i64>,
        vectors: Vec<Vec<f32>>,
        texts: Vec<String>,
    ) -> Result<()> {
        self.create_table_if_not_exists().await?;

        let table = self
            .table
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Table not initialized"))?;

        if ids.is_empty() {
            return Ok(());
        }

        // Build Arrow arrays
        let id_array: ArrayRef = Arc::new(Int64Array::from(ids));
        let text_array: ArrayRef = Arc::new(StringArray::from(texts));

        // Build FixedSizeListArray for vectors
        // Flatten all vectors into a single Float32Array
        let flat_values: Vec<f32> = vectors.iter().flatten().copied().collect();
        let values_array = Float32Array::from(flat_values);

        // Create the list field
        let list_field: FieldRef = Arc::new(Field::new("item", DataType::Float32, true));
        let vector_array: ArrayRef = Arc::new(FixedSizeListArray::new(
            list_field,
            VECTOR_DIM,
            Arc::new(values_array),
            None,
        ));

        // Create RecordBatch
        let schema = Self::schema();
        let batch = RecordBatch::try_new(schema.clone(), vec![id_array, vector_array, text_array])?;

        // Insert via RecordBatchIterator
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        table.add(batches).execute().await?;

        Ok(())
    }

    /// Search for similar vectors
    ///
    /// # Arguments
    /// * `query_vec` - 384-dimensional query vector
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    /// Vector of (id, similarity_score) tuples
    pub async fn search(&self, query_vec: Vec<f32>, limit: usize) -> Result<Vec<(i64, f32)>> {
        let table = self
            .table
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Table not initialized"))?;

        // Perform vector search
        let mut results = table
            .vector_search(query_vec)?
            .limit(limit)
            .execute()
            .await?;

        // Extract IDs and distances
        let mut hits = Vec::new();

        while let Some(batch_result) = results.next().await {
            let batch: RecordBatch = batch_result?;

            // Get the ID column
            if let Some(id_col) = batch.column_by_name("id") {
                if let Some(id_array) = id_col.as_any().downcast_ref::<Int64Array>() {
                    // LanceDB returns _distance column for L2 distance
                    let dist_col = batch.column_by_name("_distance");

                    for i in 0..id_array.len() {
                        let id = id_array.value(i);

                        // Convert distance to similarity score
                        // L2 distance: lower is better, so we use 1/(1+d)
                        let score = if let Some(d) = &dist_col {
                            if let Some(dist_array) = d.as_any().downcast_ref::<Float32Array>() {
                                let distance = dist_array.value(i);
                                1.0 / (1.0 + distance)
                            } else {
                                0.0
                            }
                        } else {
                            0.0
                        };

                        hits.push((id, score));
                    }
                }
            }
        }

        Ok(hits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_store_schema() {
        let schema = VectorStore::schema();
        assert_eq!(schema.fields().len(), 3);
        assert_eq!(schema.field(0).name(), "id");
        assert_eq!(schema.field(1).name(), "vector");
        assert_eq!(schema.field(2).name(), "text");
    }
}
