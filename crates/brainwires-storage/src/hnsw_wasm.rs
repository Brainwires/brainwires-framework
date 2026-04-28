//! In-browser HNSW vector index via `instant-distance`, persisted to OPFS.
//!
//! Compiles only for `wasm32` with the `hnsw-wasm` feature. On native
//! targets this module is empty (use the LanceDB backend instead).
//!
//! Usage from the PWA's WASM crate:
//! ```ignore
//! let index = HnswIndex::new("conversations", 384)?;
//! index.insert(embedding_vec, json_metadata).await?;
//! let results = index.search(&query_vec, 5).await?;
//! ```
//!
//! Persistence: the serialized index is written to OPFS via
//! `navigator.storage.getDirectory()`. Each collection is a separate
//! file under `hnsw-indexes/<name>.bin`.

#![cfg(all(target_arch = "wasm32", feature = "hnsw-wasm"))]

use instant_distance::{Builder, HnswMap, Point, Search};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// A single embedding vector used as a point in the HNSW index.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbeddingPoint {
    /// The raw f32 vector.
    pub vec: Vec<f32>,
}

impl Point for EmbeddingPoint {
    fn distance(&self, other: &Self) -> f32 {
        // Cosine distance = 1 - cosine_similarity.
        // For normalized vectors this is equivalent to (2 - 2*dot) / 2.
        let mut dot = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;
        for (a, b) in self.vec.iter().zip(other.vec.iter()) {
            dot += a * b;
            norm_a += a * a;
            norm_b += b * b;
        }
        let denom = norm_a.sqrt() * norm_b.sqrt();
        if denom < 1e-10 {
            return 1.0;
        }
        1.0 - (dot / denom)
    }
}

/// Metadata stored alongside each vector.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VectorMeta {
    /// Arbitrary JSON metadata (conversation ID, message text, etc.).
    pub json: String,
}

/// A search result with distance and metadata.
#[derive(Clone, Debug)]
pub struct SearchResult {
    /// Cosine distance (0 = identical, 1 = orthogonal, 2 = opposite).
    pub distance: f32,
    /// The stored metadata.
    pub meta: VectorMeta,
    /// The stored embedding.
    pub point: EmbeddingPoint,
}

/// In-browser HNSW vector index. Thread-safe (Mutex) for use from
/// wasm-bindgen exports.
pub struct HnswIndex {
    name: String,
    dim: usize,
    points: Mutex<Vec<EmbeddingPoint>>,
    values: Mutex<Vec<VectorMeta>>,
    index: Mutex<Option<HnswMap<EmbeddingPoint, usize>>>,
}

impl HnswIndex {
    /// Create a new (empty) index for vectors of `dim` dimensions.
    pub fn new(name: &str, dim: usize) -> Self {
        Self {
            name: name.to_string(),
            dim,
            points: Mutex::new(Vec::new()),
            values: Mutex::new(Vec::new()),
            index: Mutex::new(None),
        }
    }

    /// Collection name (used as the OPFS filename).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Number of vectors in the index.
    pub fn len(&self) -> usize {
        self.points.lock().unwrap().len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Insert a vector + metadata. The HNSW graph is rebuilt after
    /// insert (instant-distance doesn't support incremental insert).
    /// For batch inserts, use `insert_batch` to avoid rebuilding per item.
    pub fn insert(&self, embedding: Vec<f32>, meta_json: String) -> Result<(), String> {
        if embedding.len() != self.dim {
            return Err(format!("expected {} dims, got {}", self.dim, embedding.len()));
        }
        let point = EmbeddingPoint { vec: embedding };
        let meta = VectorMeta { json: meta_json };
        {
            let mut pts = self.points.lock().unwrap();
            let mut vals = self.values.lock().unwrap();
            pts.push(point);
            vals.push(meta);
        }
        self.rebuild_index();
        Ok(())
    }

    /// Insert multiple vectors at once, rebuilding the index once.
    pub fn insert_batch(&self, items: Vec<(Vec<f32>, String)>) -> Result<(), String> {
        {
            let mut pts = self.points.lock().unwrap();
            let mut vals = self.values.lock().unwrap();
            for (embedding, meta_json) in items {
                if embedding.len() != self.dim {
                    return Err(format!("expected {} dims, got {}", self.dim, embedding.len()));
                }
                pts.push(EmbeddingPoint { vec: embedding });
                vals.push(VectorMeta { json: meta_json });
            }
        }
        self.rebuild_index();
        Ok(())
    }

    /// Search for the `k` nearest neighbors of `query`.
    pub fn search(&self, query: &[f32], k: usize) -> Result<Vec<SearchResult>, String> {
        if query.len() != self.dim {
            return Err(format!("expected {} dims, got {}", self.dim, query.len()));
        }
        let idx = self.index.lock().unwrap();
        let hnsw = idx.as_ref().ok_or("index not built (no vectors inserted)")?;
        let q = EmbeddingPoint { vec: query.to_vec() };
        let mut search = Search::default();
        let results: Vec<SearchResult> = hnsw
            .search(&q, &mut search)
            .take(k)
            .map(|item| {
                let vals = self.values.lock().unwrap();
                SearchResult {
                    distance: item.distance,
                    point: item.point.clone(),
                    meta: vals.get(*item.value).cloned().unwrap_or(VectorMeta { json: "{}".into() }),
                }
            })
            .collect();
        Ok(results)
    }

    /// Serialize the entire index to bytes (for OPFS persistence).
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        let pts = self.points.lock().unwrap();
        let vals = self.values.lock().unwrap();
        let data = SerializedIndex {
            name: self.name.clone(),
            dim: self.dim,
            points: pts.clone(),
            values: vals.clone(),
        };
        bincode::serialize(&data).map_err(|e| format!("serialize failed: {e}"))
    }

    /// Deserialize an index from bytes (loaded from OPFS).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let data: SerializedIndex =
            bincode::deserialize(bytes).map_err(|e| format!("deserialize failed: {e}"))?;
        let idx = Self {
            name: data.name,
            dim: data.dim,
            points: Mutex::new(data.points),
            values: Mutex::new(data.values),
            index: Mutex::new(None),
        };
        idx.rebuild_index();
        Ok(idx)
    }

    fn rebuild_index(&self) {
        let pts = self.points.lock().unwrap();
        if pts.is_empty() {
            *self.index.lock().unwrap() = None;
            return;
        }
        let values: Vec<usize> = (0..pts.len()).collect();
        let hnsw = Builder::default().build(pts.clone(), values);
        *self.index.lock().unwrap() = Some(hnsw);
    }
}

#[derive(Serialize, Deserialize)]
struct SerializedIndex {
    name: String,
    dim: usize,
    points: Vec<EmbeddingPoint>,
    values: Vec<VectorMeta>,
}
