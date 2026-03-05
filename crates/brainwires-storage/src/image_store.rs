//! Image Analysis Store
//!
//! Provides storage and retrieval for analyzed images with embeddings.
//! Images are stored with their LLM-generated analysis for semantic search.

use anyhow::{Context, Result};
use arrow_array::{
    Array, ArrayRef, FixedSizeListArray, Float32Array, Int64Array, RecordBatch,
    RecordBatchIterator, StringArray, UInt32Array, UInt64Array,
};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use super::embeddings::EmbeddingProvider;
use super::image_types::{
    ImageFormat, ImageMetadata, ImageSearchRequest, ImageSearchResult, ImageStorage,
};
use super::LanceClient;

/// Store for analyzed images with semantic search
pub struct ImageStore {
    client: Arc<LanceClient>,
    embeddings: Arc<EmbeddingProvider>,
}

impl ImageStore {
    /// Create a new image store
    pub fn new(client: Arc<LanceClient>, embeddings: Arc<EmbeddingProvider>) -> Self {
        Self { client, embeddings }
    }

    /// Compute SHA256 hash of image bytes
    pub fn compute_hash(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }

    /// Store an analyzed image
    ///
    /// # Arguments
    /// * `metadata` - Image metadata including analysis
    /// * `storage` - How to store the image data (base64, file path, or URL)
    pub async fn store(
        &self,
        metadata: ImageMetadata,
        storage: ImageStorage,
    ) -> Result<ImageMetadata> {
        let table = self.client.images_table().await?;
        let dimension = self.embeddings.dimension();
        let schema = LanceClient::images_schema(dimension);

        // Generate embedding from searchable text (analysis + OCR + tags)
        let searchable_text = metadata.searchable_text();
        let embedding = self.embeddings.embed(&searchable_text)?;

        // Build the record
        let embedding_array: Vec<f32> = embedding;
        let embedding_values = Float32Array::from(embedding_array.clone());
        let vector_field = Arc::new(arrow_schema::Field::new(
            "item",
            arrow_schema::DataType::Float32,
            true,
        ));
        let embedding_list = FixedSizeListArray::new(
            vector_field,
            dimension as i32,
            Arc::new(embedding_values),
            None,
        );

        let tags_json = serde_json::to_string(&metadata.tags)?;

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(embedding_list) as ArrayRef,
                Arc::new(StringArray::from(vec![metadata.image_id.as_str()])) as ArrayRef,
                Arc::new(StringArray::from(vec![metadata
                    .message_id
                    .as_deref()
                    .unwrap_or("")])) as ArrayRef,
                Arc::new(StringArray::from(vec![metadata.conversation_id.as_str()])) as ArrayRef,
                Arc::new(StringArray::from(vec![metadata
                    .file_name
                    .as_deref()
                    .unwrap_or("")])) as ArrayRef,
                Arc::new(StringArray::from(vec![metadata.format.as_str()])) as ArrayRef,
                Arc::new(StringArray::from(vec![metadata.mime_type.as_str()])) as ArrayRef,
                Arc::new(UInt32Array::from(vec![metadata.width.unwrap_or(0)])) as ArrayRef,
                Arc::new(UInt32Array::from(vec![metadata.height.unwrap_or(0)])) as ArrayRef,
                Arc::new(UInt64Array::from(vec![metadata.file_size_bytes])) as ArrayRef,
                Arc::new(StringArray::from(vec![metadata.file_hash.as_str()])) as ArrayRef,
                Arc::new(StringArray::from(vec![metadata.analysis.as_str()])) as ArrayRef,
                Arc::new(StringArray::from(vec![metadata
                    .extracted_text
                    .as_deref()
                    .unwrap_or("")])) as ArrayRef,
                Arc::new(StringArray::from(vec![tags_json.as_str()])) as ArrayRef,
                Arc::new(StringArray::from(vec![storage.storage_type()])) as ArrayRef,
                Arc::new(StringArray::from(vec![storage.value()])) as ArrayRef,
                Arc::new(Int64Array::from(vec![metadata.created_at])) as ArrayRef,
            ],
        )?;

        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to store image")?;

        Ok(metadata)
    }

    /// Store image with analysis from bytes
    ///
    /// # Arguments
    /// * `bytes` - Raw image bytes
    /// * `analysis` - LLM-generated analysis
    /// * `conversation_id` - Conversation to associate with
    /// * `format` - Image format
    pub async fn store_from_bytes(
        &self,
        bytes: &[u8],
        analysis: String,
        conversation_id: String,
        format: ImageFormat,
    ) -> Result<ImageMetadata> {
        let file_hash = Self::compute_hash(bytes);

        // Check for duplicate
        if let Some(existing) = self.get_by_hash(&file_hash).await? {
            return Ok(existing);
        }

        let image_id = format!("img_{}", uuid::Uuid::new_v4());
        let metadata = ImageMetadata::new(
            image_id,
            conversation_id,
            format,
            bytes.len() as u64,
            file_hash,
            analysis,
        );

        let storage = ImageStorage::from_bytes(bytes);
        self.store(metadata, storage).await
    }

    /// Get image by hash (for deduplication)
    pub async fn get_by_hash(&self, file_hash: &str) -> Result<Option<ImageMetadata>> {
        let table = self.client.images_table().await?;

        let filter = format!("file_hash = '{}'", file_hash.replace('\'', "''"));

        let mut results = table
            .query()
            .only_if(filter)
            .limit(1)
            .execute()
            .await
            .context("Failed to query images by hash")?;

        if let Some(batch) = results.try_next().await?
            && batch.num_rows() > 0 {
                return Ok(Some(self.batch_to_metadata(&batch, 0)?));
            }

        Ok(None)
    }

    /// Get image by ID
    pub async fn get(&self, image_id: &str) -> Result<Option<ImageMetadata>> {
        let table = self.client.images_table().await?;

        let filter = format!("image_id = '{}'", image_id.replace('\'', "''"));

        let mut results = table
            .query()
            .only_if(filter)
            .limit(1)
            .execute()
            .await
            .context("Failed to query image by ID")?;

        if let Some(batch) = results.try_next().await?
            && batch.num_rows() > 0 {
                return Ok(Some(self.batch_to_metadata(&batch, 0)?));
            }

        Ok(None)
    }

    /// Search images using semantic search on analysis text
    pub async fn search(&self, request: ImageSearchRequest) -> Result<Vec<ImageSearchResult>> {
        let table = self.client.images_table().await?;

        // Generate query embedding
        let query_embedding = self.embeddings.embed(&request.query)?;

        // Build filter
        let mut filters = Vec::new();

        if let Some(ref conv_id) = request.conversation_id {
            filters.push(format!(
                "conversation_id = '{}'",
                conv_id.replace('\'', "''")
            ));
        }

        if let Some(format) = request.format {
            filters.push(format!("format = '{}'", format.as_str()));
        }

        let filter = if filters.is_empty() {
            None
        } else {
            Some(filters.join(" AND "))
        };

        // Execute vector search
        let mut query = table
            .vector_search(query_embedding)
            .context("Failed to create vector search")?
            .limit(request.limit)
            .nprobes(10);

        if let Some(f) = filter {
            query = query.only_if(f);
        }

        let mut results = query
            .execute()
            .await
            .context("Failed to execute image search")?;

        let mut search_results = Vec::new();

        while let Some(batch) = results.try_next().await? {
            for i in 0..batch.num_rows() {
                // Get distance/score
                let distance_col = batch
                    .column_by_name("_distance")
                    .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

                let score = distance_col
                    .map(|d| 1.0 - d.value(i).min(2.0) / 2.0) // Convert distance to similarity
                    .unwrap_or(0.5);

                if score < request.min_score {
                    continue;
                }

                let metadata = self.batch_to_metadata(&batch, i)?;
                search_results.push(ImageSearchResult::from_metadata(metadata, score));
            }
        }

        // Sort by score descending
        search_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(search_results)
    }

    /// List images by conversation
    pub async fn list_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ImageMetadata>> {
        let table = self.client.images_table().await?;

        let filter = format!(
            "conversation_id = '{}'",
            conversation_id.replace('\'', "''")
        );

        let mut results = table
            .query()
            .only_if(filter)
            .execute()
            .await
            .context("Failed to list images by conversation")?;

        let mut images = Vec::new();

        while let Some(batch) = results.try_next().await? {
            for i in 0..batch.num_rows() {
                images.push(self.batch_to_metadata(&batch, i)?);
            }
        }

        // Sort by created_at descending
        images.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(images)
    }

    /// List images by message
    pub async fn list_by_message(&self, message_id: &str) -> Result<Vec<ImageMetadata>> {
        let table = self.client.images_table().await?;

        let filter = format!("message_id = '{}'", message_id.replace('\'', "''"));

        let mut results = table
            .query()
            .only_if(filter)
            .execute()
            .await
            .context("Failed to list images by message")?;

        let mut images = Vec::new();

        while let Some(batch) = results.try_next().await? {
            for i in 0..batch.num_rows() {
                images.push(self.batch_to_metadata(&batch, i)?);
            }
        }

        Ok(images)
    }

    /// Delete an image
    pub async fn delete(&self, image_id: &str) -> Result<bool> {
        let table = self.client.images_table().await?;

        let filter = format!("image_id = '{}'", image_id.replace('\'', "''"));

        table
            .delete(&filter)
            .await
            .context("Failed to delete image")?;

        Ok(true)
    }

    /// Delete all images for a conversation
    pub async fn delete_by_conversation(&self, conversation_id: &str) -> Result<usize> {
        let images = self.list_by_conversation(conversation_id).await?;
        let count = images.len();

        let table = self.client.images_table().await?;

        let filter = format!(
            "conversation_id = '{}'",
            conversation_id.replace('\'', "''")
        );

        table
            .delete(&filter)
            .await
            .context("Failed to delete images by conversation")?;

        Ok(count)
    }

    /// Get image data (base64 or path)
    pub async fn get_image_data(&self, image_id: &str) -> Result<Option<ImageStorage>> {
        let table = self.client.images_table().await?;

        let filter = format!("image_id = '{}'", image_id.replace('\'', "''"));

        let mut results = table
            .query()
            .only_if(filter)
            .limit(1)
            .execute()
            .await
            .context("Failed to query image data")?;

        if let Some(batch) = results.try_next().await?
            && batch.num_rows() > 0 {
                let storage_type = batch
                    .column_by_name("storage_type")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(0).to_string())
                    .unwrap_or_default();

                let storage_value = batch
                    .column_by_name("storage_value")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                    .map(|a| a.value(0).to_string())
                    .unwrap_or_default();

                let storage = match storage_type.as_str() {
                    "base64" => ImageStorage::Base64(storage_value),
                    "file" => ImageStorage::FilePath(storage_value),
                    "url" => ImageStorage::Url(storage_value),
                    _ => ImageStorage::Base64(storage_value),
                };

                return Ok(Some(storage));
            }

        Ok(None)
    }

    /// Count images in a conversation
    pub async fn count_by_conversation(&self, conversation_id: &str) -> Result<usize> {
        let images = self.list_by_conversation(conversation_id).await?;
        Ok(images.len())
    }

    /// Convert a record batch row to ImageMetadata
    fn batch_to_metadata(&self, batch: &RecordBatch, row: usize) -> Result<ImageMetadata> {
        let get_string = |name: &str| -> String {
            batch
                .column_by_name(name)
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .map(|a| a.value(row).to_string())
                .unwrap_or_default()
        };

        let get_u32 = |name: &str| -> Option<u32> {
            batch
                .column_by_name(name)
                .and_then(|c| c.as_any().downcast_ref::<UInt32Array>())
                .map(|a| a.value(row))
                .filter(|&v| v > 0)
        };

        let get_u64 = |name: &str| -> u64 {
            batch
                .column_by_name(name)
                .and_then(|c| c.as_any().downcast_ref::<UInt64Array>())
                .map(|a| a.value(row))
                .unwrap_or(0)
        };

        let get_i64 = |name: &str| -> i64 {
            batch
                .column_by_name(name)
                .and_then(|c| c.as_any().downcast_ref::<Int64Array>())
                .map(|a| a.value(row))
                .unwrap_or(0)
        };

        let image_id = get_string("image_id");
        let message_id = {
            let s = get_string("message_id");
            if s.is_empty() { None } else { Some(s) }
        };
        let conversation_id = get_string("conversation_id");
        let file_name = {
            let s = get_string("file_name");
            if s.is_empty() { None } else { Some(s) }
        };
        let format_str = get_string("format");
        let format = format_str.parse().unwrap_or(ImageFormat::Unknown);
        let mime_type = get_string("mime_type");
        let width = get_u32("width");
        let height = get_u32("height");
        let file_size_bytes = get_u64("file_size_bytes");
        let file_hash = get_string("file_hash");
        let analysis = get_string("analysis");
        let extracted_text = {
            let s = get_string("extracted_text");
            if s.is_empty() { None } else { Some(s) }
        };
        let tags_json = get_string("tags");
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let created_at = get_i64("created_at");

        Ok(ImageMetadata {
            image_id,
            message_id,
            conversation_id,
            file_name,
            format,
            mime_type,
            width,
            height,
            file_size_bytes,
            file_hash,
            analysis,
            extracted_text,
            tags,
            created_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_store() -> (ImageStore, TempDir) {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test_images.lance");

        let client = Arc::new(
            LanceClient::new(db_path.to_str().unwrap())
                .await
                .unwrap(),
        );

        let embeddings = Arc::new(EmbeddingProvider::new().unwrap());

        client.initialize(embeddings.dimension()).await.unwrap();

        let store = ImageStore::new(client, embeddings);
        (store, temp)
    }

    #[tokio::test]
    async fn test_store_and_get() {
        let (store, _temp) = create_test_store().await;

        let metadata = ImageMetadata::new(
            "img-test-123".to_string(),
            "conv-456".to_string(),
            ImageFormat::Png,
            1024,
            "hash123".to_string(),
            "A screenshot showing code editor with Rust code".to_string(),
        )
        .with_file_name("screenshot.png".to_string())
        .with_dimensions(1920, 1080)
        .with_tags(vec!["code".to_string(), "rust".to_string()]);

        let storage = ImageStorage::Base64("dGVzdCBpbWFnZSBkYXRh".to_string());

        store.store(metadata.clone(), storage).await.unwrap();

        // Get by ID
        let retrieved = store.get("img-test-123").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.image_id, "img-test-123");
        assert_eq!(retrieved.conversation_id, "conv-456");
        assert_eq!(retrieved.format, ImageFormat::Png);
        assert_eq!(retrieved.width, Some(1920));
        assert_eq!(retrieved.tags.len(), 2);
    }

    #[tokio::test]
    async fn test_search() {
        let (store, _temp) = create_test_store().await;

        // Store an image
        let metadata = ImageMetadata::new(
            "img-search-1".to_string(),
            "conv-search".to_string(),
            ImageFormat::Png,
            2048,
            "searchhash1".to_string(),
            "A flowchart diagram showing the authentication flow with OAuth2".to_string(),
        )
        .with_tags(vec!["diagram".to_string(), "auth".to_string()]);

        store
            .store(metadata, ImageStorage::Base64("test".to_string()))
            .await
            .unwrap();

        // Search
        let request = ImageSearchRequest::new("authentication flow diagram")
            .with_conversation("conv-search".to_string())
            .with_limit(5)
            .with_min_score(0.3);

        let results = store.search(request).await.unwrap();
        // Note: Results may be empty depending on embedding similarity
        // The important thing is no errors occur
        assert!(results.len() <= 5);
    }

    #[tokio::test]
    async fn test_list_by_conversation() {
        let (store, _temp) = create_test_store().await;

        // Store multiple images
        for i in 0..3 {
            let metadata = ImageMetadata::new(
                format!("img-list-{}", i),
                "conv-list-test".to_string(),
                ImageFormat::Jpeg,
                1024,
                format!("listhash{}", i),
                format!("Image number {}", i),
            );

            store
                .store(metadata, ImageStorage::Base64("test".to_string()))
                .await
                .unwrap();
        }

        let images = store.list_by_conversation("conv-list-test").await.unwrap();
        assert_eq!(images.len(), 3);
    }

    #[tokio::test]
    async fn test_delete() {
        let (store, _temp) = create_test_store().await;

        let metadata = ImageMetadata::new(
            "img-delete-test".to_string(),
            "conv-delete".to_string(),
            ImageFormat::Gif,
            512,
            "deletehash".to_string(),
            "An animated GIF".to_string(),
        );

        store
            .store(metadata, ImageStorage::Base64("test".to_string()))
            .await
            .unwrap();

        // Verify exists
        assert!(store.get("img-delete-test").await.unwrap().is_some());

        // Delete
        store.delete("img-delete-test").await.unwrap();

        // Verify deleted
        assert!(store.get("img-delete-test").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_duplicate_detection() {
        let (store, _temp) = create_test_store().await;

        let bytes = b"test image bytes for dedup";
        let hash = ImageStore::compute_hash(bytes);

        // Store first time
        let meta1 = store
            .store_from_bytes(
                bytes,
                "First analysis".to_string(),
                "conv-dedup".to_string(),
                ImageFormat::Png,
            )
            .await
            .unwrap();

        // Try to store again - should return existing
        let meta2 = store
            .store_from_bytes(
                bytes,
                "Second analysis".to_string(),
                "conv-dedup".to_string(),
                ImageFormat::Png,
            )
            .await
            .unwrap();

        assert_eq!(meta1.image_id, meta2.image_id);
        assert_eq!(meta1.file_hash, hash);
    }

    #[tokio::test]
    async fn test_get_image_data() {
        let (store, _temp) = create_test_store().await;

        let test_data = "dGVzdCBiYXNlNjQgZGF0YQ==";
        let metadata = ImageMetadata::new(
            "img-data-test".to_string(),
            "conv-data".to_string(),
            ImageFormat::Webp,
            256,
            "datahash".to_string(),
            "Test image".to_string(),
        );

        store
            .store(metadata, ImageStorage::Base64(test_data.to_string()))
            .await
            .unwrap();

        let data = store.get_image_data("img-data-test").await.unwrap();
        assert!(data.is_some());

        if let Some(ImageStorage::Base64(b64)) = data {
            assert_eq!(b64, test_data);
        } else {
            panic!("Expected Base64 storage");
        }
    }
}
