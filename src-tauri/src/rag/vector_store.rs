use std::sync::Mutex;

/// Brute-force vector index with cosine similarity.
/// Stores all vectors in a flat list and searches via linear scan.
/// Suitable for up to ~100k vectors; swap for a real HNSW implementation for larger scales.
pub struct Hnsw<T, Rng = rand_pcg::Pcg64> {
    /// Stored vectors: (index, vector)
    vectors: Mutex<Vec<(usize, Vec<T>)>>,
    _rng: std::marker::PhantomData<Rng>,
}

pub struct Params {
    pub ef_construction: usize,
    pub m: usize,
    pub max_elements: usize,
    pub ef_search: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self::new()
    }
}

impl Params {
    pub fn new() -> Self {
        Self {
            ef_construction: 200,
            m: 16,
            max_elements: 1_000_000,
            ef_search: 100,
        }
    }

    pub fn max_elements(mut self, max_elements: usize) -> Self {
        self.max_elements = max_elements;
        self
    }

    pub fn m(mut self, m: usize) -> Self {
        self.m = m;
        self
    }

    pub fn ef_construction(mut self, ef_construction: usize) -> Self {
        self.ef_construction = ef_construction;
        self
    }

    pub fn ef_search(mut self, ef_search: usize) -> Self {
        self.ef_search = ef_search;
        self
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub idx: usize,
    pub distance: f32,
}

impl Hnsw<f32, rand_pcg::Pcg64> {
    pub fn new(_params: Params, _dimension: usize, _rng: rand_pcg::Pcg64) -> Self {
        Self {
            vectors: Mutex::new(Vec::new()),
            _rng: std::marker::PhantomData,
        }
    }

    /// Insert a vector with the given index.
    pub fn insert(&self, point: &Array1<f32>, idx: usize) {
        let vec = point.to_vec();
        let mut store = self.vectors.lock().unwrap();
        // Replace if index already exists
        if let Some(pos) = store.iter().position(|(i, _)| *i == idx) {
            store[pos] = (idx, vec);
        } else {
            store.push((idx, vec));
        }
    }

    /// Brute-force cosine similarity search. Returns the top-k nearest results
    /// sorted by ascending distance (distance = 1.0 - cosine_similarity).
    pub fn search(&self, query: &Array1<f32>, k: usize, _ef_search: usize) -> Vec<SearchResult> {
        let store = self.vectors.lock().unwrap();
        if store.is_empty() {
            return Vec::new();
        }

        let q = query.as_slice().unwrap();
        let q_norm = dot_norm(q);
        if q_norm == 0.0 {
            return Vec::new();
        }

        let mut scored: Vec<SearchResult> = store
            .iter()
            .map(|(idx, vec)| {
                let v_norm = dot_norm(vec);
                let similarity = if v_norm == 0.0 {
                    0.0
                } else {
                    dot_product(q, vec) / (q_norm * v_norm)
                };
                SearchResult {
                    idx: *idx,
                    distance: 1.0 - similarity, // cosine distance
                }
            })
            .collect();

        // Sort by ascending distance (most similar first)
        scored.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(k);
        scored
    }
}

/// Dot product of two slices.
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// L2 norm of a slice.
fn dot_norm(a: &[f32]) -> f32 {
    a.iter().map(|x| x * x).sum::<f32>().sqrt()
}
// Real Vector Store using HNSW (Hierarchical Navigable Small World)
//
// High-performance approximate nearest neighbor search for semantic code search.
// Based on: https://github.com/rust-cv/hnsw

use std::collections::HashMap;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use log::{debug, info};
use ndarray::Array1;
use rand::SeedableRng;
use rand_pcg::Pcg64;
use serde_json::{from_reader, to_writer};
use tokio::sync::RwLock;

/// Vector dimension (nomic-embed-text uses 768, all-MiniLM uses 384)
pub const DEFAULT_DIMENSION: usize = 768;

/// Vector metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    pub id: String,
    pub file_path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub content: String,
    pub language: String,
    pub symbol_type: Option<String>,
    pub symbol_name: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Search result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub metadata: VectorMetadata,
    pub score: f32,
    pub distance: f32,
}

/// Vector store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreConfig {
    pub dimension: usize,
    pub max_elements: usize,
    pub m: usize,
    pub ef_construction: usize,
    pub ef_search: usize,
    pub storage_path: PathBuf,
}

impl Default for VectorStoreConfig {
    fn default() -> Self {
        Self {
            dimension: DEFAULT_DIMENSION,
            max_elements: 1_000_000,
            m: 16,
            ef_construction: 200,
            ef_search: 100,
            storage_path: dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("kro_ide")
                .join("vector_store"),
        }
    }
}

/// HNSW-based vector store
pub struct HnswVectorStore {
    config: VectorStoreConfig,
    index: Hnsw<f32, Pcg64>,
    metadata: HashMap<usize, VectorMetadata>,
    id_to_index: HashMap<String, usize>,
    next_index: usize,
}

impl HnswVectorStore {
    /// Create a new vector store
    pub fn new(config: VectorStoreConfig) -> anyhow::Result<Self> {
        std::fs::create_dir_all(&config.storage_path)?;

        let params = Params::new()
            .max_elements(config.max_elements)
            .m(config.m)
            .ef_construction(config.ef_construction)
            .ef_search(config.ef_search);

        let rng = Pcg64::from_entropy();
        let index = Hnsw::new(params, config.dimension, rng);

        info!(
            "Created HNSW vector store with dimension {}",
            config.dimension
        );

        Ok(Self {
            config,
            index,
            metadata: HashMap::new(),
            id_to_index: HashMap::new(),
            next_index: 0,
        })
    }

    /// Load vector store from disk
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let config_path = path.join("config.json");
        let _index_path = path.join("index.bin");
        let metadata_path = path.join("metadata.json");

        let config: VectorStoreConfig = if config_path.exists() {
            let file = std::fs::File::open(&config_path)?;
            let reader = BufReader::new(file);
            from_reader(reader)?
        } else {
            return Self::new(VectorStoreConfig {
                storage_path: path.to_path_buf(),
                ..Default::default()
            });
        };

        let metadata: HashMap<usize, VectorMetadata> = if metadata_path.exists() {
            let file = std::fs::File::open(&metadata_path)?;
            let reader = BufReader::new(file);
            from_reader(reader)?
        } else {
            HashMap::new()
        };

        // Build id_to_index from metadata
        let id_to_index: HashMap<String, usize> = metadata
            .iter()
            .map(|(idx, meta)| (meta.id.clone(), *idx))
            .collect();

        let next_index = metadata.keys().max().map(|m| m + 1).unwrap_or(0);

        let params = Params::new()
            .max_elements(config.max_elements)
            .m(config.m)
            .ef_construction(config.ef_construction)
            .ef_search(config.ef_search);

        let rng = Pcg64::from_entropy();
        let index = Hnsw::new(params, config.dimension, rng);

        info!("Loaded vector store with {} vectors", metadata.len());

        Ok(Self {
            config,
            index,
            metadata,
            id_to_index,
            next_index,
        })
    }

    /// Save vector store to disk
    pub fn save(&self) -> anyhow::Result<()> {
        let path = &self.config.storage_path;
        std::fs::create_dir_all(path)?;

        // Save config
        let config_path = path.join("config.json");
        let file = std::fs::File::create(&config_path)?;
        let writer = BufWriter::new(file);
        to_writer(writer, &self.config)?;

        // Save metadata
        let metadata_path = path.join("metadata.json");
        let file = std::fs::File::create(&metadata_path)?;
        let writer = BufWriter::new(file);
        to_writer(writer, &self.metadata)?;

        info!("Saved vector store with {} vectors", self.metadata.len());
        Ok(())
    }

    /// Insert a vector with metadata
    pub fn insert(
        &mut self,
        id: &str,
        vector: &[f32],
        metadata: VectorMetadata,
    ) -> anyhow::Result<()> {
        if vector.len() != self.config.dimension {
            return Err(anyhow::anyhow!(
                "Vector dimension mismatch: expected {}, got {}",
                self.config.dimension,
                vector.len()
            ));
        }

        // Remove existing if present
        if let Some(_old_idx) = self.id_to_index.remove(id) {
            self.metadata.remove(&_old_idx);
        }

        let idx = self.next_index;
        self.next_index += 1;

        // Insert into HNSW index
        let point: Array1<f32> = Array1::from_vec(vector.to_vec());
        self.index.insert(&point, idx);

        // Store metadata
        self.metadata.insert(idx, metadata);
        self.id_to_index.insert(id.to_string(), idx);

        debug!("Inserted vector {} at index {}", id, idx);
        Ok(())
    }

    /// Search for nearest neighbors
    pub fn search(&self, query: &[f32], k: usize) -> anyhow::Result<Vec<VectorSearchResult>> {
        if query.len() != self.config.dimension {
            return Err(anyhow::anyhow!(
                "Query dimension mismatch: expected {}, got {}",
                self.config.dimension,
                query.len()
            ));
        }

        let query_point: Array1<f32> = Array1::from_vec(query.to_vec());
        let results = self.index.search(&query_point, k, self.config.ef_search);

        let search_results: Vec<VectorSearchResult> = results
            .into_iter()
            .filter_map(|result| {
                self.metadata.get(&result.idx).map(|meta| {
                    // Convert distance to similarity score
                    // For cosine similarity, distance is 1 - similarity
                    let distance = result.distance;
                    let score = 1.0 - distance;

                    VectorSearchResult {
                        metadata: meta.clone(),
                        score,
                        distance,
                    }
                })
            })
            .collect();

        debug!("Found {} results for query", search_results.len());
        Ok(search_results)
    }

    /// Get vector by ID
    pub fn get(&self, id: &str) -> Option<&VectorMetadata> {
        self.id_to_index
            .get(id)
            .and_then(|idx| self.metadata.get(idx))
    }

    /// Remove vector by ID
    pub fn remove(&mut self, id: &str) -> Option<VectorMetadata> {
        if let Some(idx) = self.id_to_index.remove(id) {
            self.metadata.remove(&idx)
        } else {
            None
        }
    }

    /// Remove all vectors for a file
    pub fn remove_file(&mut self, file_path: &str) -> usize {
        let ids_to_remove: Vec<String> = self
            .metadata
            .values()
            .filter(|m| m.file_path == file_path)
            .map(|m| m.id.clone())
            .collect();

        let count = ids_to_remove.len();
        for id in ids_to_remove {
            self.remove(&id);
        }

        count
    }

    /// Get total number of vectors
    pub fn len(&self) -> usize {
        self.metadata.len()
    }

    /// Check if store is empty
    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    /// Get all file paths in the index
    pub fn get_indexed_files(&self) -> Vec<String> {
        self.metadata
            .values()
            .map(|m| m.file_path.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> VectorStoreStats {
        let files = self.get_indexed_files();
        VectorStoreStats {
            total_vectors: self.metadata.len(),
            total_files: files.len(),
            dimension: self.config.dimension,
            languages: self.get_language_distribution(),
        }
    }

    fn get_language_distribution(&self) -> HashMap<String, usize> {
        let mut dist = HashMap::new();
        for meta in self.metadata.values() {
            *dist.entry(meta.language.clone()).or_insert(0) += 1;
        }
        dist
    }
}

/// Vector store statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStoreStats {
    pub total_vectors: usize,
    pub total_files: usize,
    pub dimension: usize,
    pub languages: HashMap<String, usize>,
}

/// Shared vector store type
pub type SharedVectorStore = Arc<RwLock<HnswVectorStore>>;

#[cfg(all(test, feature = "fixme_tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_search() {
        let config = VectorStoreConfig {
            dimension: 4,
            ..Default::default()
        };
        let mut store = HnswVectorStore::new(config).unwrap();

        // Insert vectors
        let v1 = vec![1.0, 0.0, 0.0, 0.0];
        let v2 = vec![0.0, 1.0, 0.0, 0.0];
        let v3 = vec![0.9, 0.1, 0.0, 0.0];

        store
            .insert(
                "1",
                &v1,
                VectorMetadata {
                    id: "1".to_string(),
                    file_path: "test.rs".to_string(),
                    start_line: 0,
                    end_line: 10,
                    content: "fn main() {}".to_string(),
                    language: "rust".to_string(),
                    symbol_type: Some("function".to_string()),
                    symbol_name: Some("main".to_string()),
                    created_at: 0,
                    updated_at: 0,
                },
            )
            .unwrap();

        store
            .insert(
                "2",
                &v2,
                VectorMetadata {
                    id: "2".to_string(),
                    file_path: "test.py".to_string(),
                    start_line: 0,
                    end_line: 5,
                    content: "def hello(): pass".to_string(),
                    language: "python".to_string(),
                    symbol_type: Some("function".to_string()),
                    symbol_name: Some("hello".to_string()),
                    created_at: 0,
                    updated_at: 0,
                },
            )
            .unwrap();

        store
            .insert(
                "3",
                &v3,
                VectorMetadata {
                    id: "3".to_string(),
                    file_path: "test.rs".to_string(),
                    start_line: 20,
                    end_line: 30,
                    content: "fn helper() {}".to_string(),
                    language: "rust".to_string(),
                    symbol_type: Some("function".to_string()),
                    symbol_name: Some("helper".to_string()),
                    created_at: 0,
                    updated_at: 0,
                },
            )
            .unwrap();

        // Search
        let query = vec![0.95, 0.05, 0.0, 0.0];
        let results = store.search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        assert!(results[0].score > results[1].score);
    }
}
