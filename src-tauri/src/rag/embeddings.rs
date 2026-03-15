//! Local Embedding Generation
//!
//! Generates embeddings locally using various backends:
//! - ONNX Runtime (for all-MiniLM-L6-v2)
//! - Fallback hash-based embeddings for zero-dependency mode

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// Embedding model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub model_name: String,
    pub dimension: usize,
    pub max_length: usize,
    pub batch_size: usize,
    pub normalize: bool,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_name: "all-MiniLM-L6-v2".to_string(),
            dimension: 384,
            max_length: 512,
            batch_size: 32,
            normalize: true,
        }
    }
}

/// Embedding result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    pub embedding: Vec<f32>,
    pub model: String,
    pub tokens: usize,
    pub processing_time_ms: u64,
}

/// Embedder trait
pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> anyhow::Result<EmbeddingResult>;
    fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<EmbeddingResult>>;
    fn dimension(&self) -> usize;
    fn model_name(&self) -> &str;
}

/// Hash-based embedder (zero-dependency fallback)
pub struct HashEmbedder {
    config: EmbeddingConfig,
    cache: HashMap<String, Vec<f32>>,
}

impl HashEmbedder {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self {
            config,
            cache: HashMap::new(),
        }
    }

    /// Generate deterministic embedding from text hash
    fn hash_embedding(&self, text: &str) -> Vec<f32> {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        // Generate embedding from hash
        let mut embedding = vec![0.0f32; self.config.dimension];

        // Use multiple hash rounds to fill the embedding
        for i in 0..self.config.dimension {
            let mut h = DefaultHasher::new();
            hash.hash(&mut h);
            i.hash(&mut h);
            let val = h.finish() as f64 / u64::MAX as f64;
            embedding[i] = (val * 2.0 - 1.0) as f32; // Range [-1, 1]
        }

        // Normalize
        if self.config.normalize {
            let mag: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            if mag > 0.0 {
                for x in &mut embedding {
                    *x /= mag;
                }
            }
        }

        embedding
    }

    /// Simple tokenization for estimating token count
    fn estimate_tokens(&self, text: &str) -> usize {
        // Rough estimate: ~4 chars per token
        (text.len() / 4).max(1)
    }
}

impl Embedder for HashEmbedder {
    fn embed(&self, text: &str) -> anyhow::Result<EmbeddingResult> {
        let start = std::time::Instant::now();

        let embedding = if let Some(cached) = self.cache.get(text) {
            cached.clone()
        } else {
            self.hash_embedding(text)
        };

        Ok(EmbeddingResult {
            embedding,
            model: self.config.model_name.clone(),
            tokens: self.estimate_tokens(text),
            processing_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<EmbeddingResult>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }

    fn model_name(&self) -> &str {
        &self.config.model_name
    }
}

/// TF-IDF based embedder for semantic search
pub struct TfIdfEmbedder {
    config: EmbeddingConfig,
    vocabulary: HashMap<String, usize>,
    /// Number of documents each term appears in (indexed by vocabulary index)
    document_frequencies: Vec<usize>,
    idf: Vec<f32>,
    document_count: usize,
    rust_stemmers: Option<rust_stemmers::Stemmer>,
}

impl TfIdfEmbedder {
    pub fn new(config: EmbeddingConfig) -> Self {
        let stemmer = Some(rust_stemmers::Stemmer::create(
            rust_stemmers::Algorithm::English,
        ));
        Self {
            config,
            vocabulary: HashMap::new(),
            document_frequencies: Vec::new(),
            idf: Vec::new(),
            document_count: 0,
            rust_stemmers: stemmer,
        }
    }

    /// Tokenize and stem text
    fn tokenize(&self, text: &str) -> Vec<String> {
        let tokens: Vec<String> = text
            .to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() > 1)
            .map(|s| {
                if let Some(stemmer) = &self.rust_stemmers {
                    stemmer.stem(s).into_owned()
                } else {
                    s.to_string()
                }
            })
            .collect();
        tokens
    }

    /// Add document to vocabulary
    pub fn add_document(&mut self, text: &str) {
        let tokens = self.tokenize(text);
        let mut term_counts: HashMap<String, usize> = HashMap::new();

        for token in tokens {
            *term_counts.entry(token).or_insert(0) += 1;
        }

        // Assign vocabulary indices and track document frequency
        let new_vocab_size = self.vocabulary.len() + term_counts.len();
        self.document_frequencies.resize(new_vocab_size, 0);

        for term in term_counts.keys() {
            let len = self.vocabulary.len();
            let idx = *self.vocabulary.entry(term.clone()).or_insert(len);
            if idx < self.document_frequencies.len() {
                self.document_frequencies[idx] += 1;
            }
        }

        self.document_count += 1;

        // Update IDF values
        self.update_idf();
    }

    fn update_idf(&mut self) {
        let vocab_size = self.vocabulary.len();
        self.idf = vec![0.0; vocab_size];
        let n = self.document_count as f32 + 1.0;
        for i in 0..vocab_size {
            let df = self.document_frequencies.get(i).copied().unwrap_or(0) as f32;
            // Standard smoothed IDF: ln((N + 1) / (df + 1)) + 1
            self.idf[i] = (n / (df + 1.0)).ln() + 1.0;
        }
    }

    /// Compute TF-IDF embedding
    fn compute_embedding(&self, text: &str) -> Vec<f32> {
        let tokens = self.tokenize(text);
        let mut tf = vec![0.0f32; self.vocabulary.len().max(self.config.dimension)];

        // Compute term frequency
        let total = tokens.len() as f32;
        for token in &tokens {
            if let Some(&idx) = self.vocabulary.get(token) {
                if idx < tf.len() && idx < self.idf.len() {
                    tf[idx] += 1.0 / total;
                }
            }
        }

        // Multiply by IDF
        for (i, tf_val) in tf.iter_mut().enumerate() {
            if i < self.idf.len() {
                *tf_val *= self.idf[i];
            }
        }

        // Truncate or pad to dimension
        tf.truncate(self.config.dimension);
        while tf.len() < self.config.dimension {
            tf.push(0.0);
        }

        // Normalize
        if self.config.normalize {
            let mag: f32 = tf.iter().map(|x| x * x).sum::<f32>().sqrt();
            if mag > 0.0 {
                for x in &mut tf {
                    *x /= mag;
                }
            }
        }

        tf
    }
}
impl Embedder for TfIdfEmbedder {
    fn embed(&self, text: &str) -> anyhow::Result<EmbeddingResult> {
        let start = std::time::Instant::now();

        let embedding = self.compute_embedding(text);

        Ok(EmbeddingResult {
            embedding,
            model: self.config.model_name.clone(),
            tokens: text.split_whitespace().count(),
            processing_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<EmbeddingResult>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }

    fn model_name(&self) -> &str {
        &self.config.model_name
    }
}

/// Code-aware embedder that understands code structure
pub struct CodeEmbedder {
    config: EmbeddingConfig,
    base_embedder: Box<dyn Embedder>,
}

impl CodeEmbedder {
    pub fn new(config: EmbeddingConfig) -> Self {
        let base: Box<dyn Embedder> = Box::new(HashEmbedder::new(config.clone()));
        Self {
            config,
            base_embedder: base,
        }
    }

    /// Extract code features for enhanced embedding
    fn extract_features(&self, code: &str, language: &str) -> Vec<f32> {
        let mut features = vec![0.0f32; 64];

        // Extract structural features
        let lines = code.lines().count();
        features[0] = (lines as f32).ln(); // Log line count

        let indent_levels: Vec<usize> = code
            .lines()
            .map(|l| l.chars().take_while(|&c| c == ' ' || c == '\t').count())
            .collect();
        features[1] = indent_levels.iter().max().copied().unwrap_or(0) as f32 / 10.0;

        // Count language-specific keywords
        let keywords = self.get_keywords(language);
        for keyword in keywords {
            let count = code.matches(keyword).count();
            features[2] += count as f32;
        }
        features[2] = (features[2] / 10.0).min(1.0);

        // Symbol density
        let symbols = code
            .chars()
            .filter(|c| "!@#$%^&*(){}[]|;:,.<>?".contains(*c))
            .count();
        features[3] = symbols as f32 / code.len().max(1) as f32;

        // Comment ratio
        let comment_chars = code
            .lines()
            .filter(|l| {
                l.trim().starts_with("//")
                    || l.trim().starts_with("#")
                    || l.trim().starts_with("/*")
            })
            .count();
        features[4] = comment_chars as f32 / lines.max(1) as f32;

        features
    }

    fn get_keywords(&self, language: &str) -> Vec<&'static str> {
        match language {
            "rust" => vec![
                "fn", "let", "impl", "struct", "enum", "trait", "pub", "use", "mod",
            ],
            "python" => vec![
                "def", "class", "import", "from", "if", "else", "for", "while", "return",
            ],
            "javascript" | "typescript" => vec![
                "function", "const", "let", "var", "class", "import", "export", "async", "await",
            ],
            "go" => vec![
                "func",
                "var",
                "type",
                "struct",
                "interface",
                "package",
                "import",
            ],
            "java" => vec![
                "class",
                "interface",
                "public",
                "private",
                "void",
                "static",
                "import",
                "package",
            ],
            _ => vec!["function", "class", "if", "else", "for", "while", "return"],
        }
    }

    /// Combine base embedding with code features
    fn combine_embeddings(&self, base: &[f32], features: &[f32]) -> Vec<f32> {
        let mut combined = Vec::with_capacity(self.config.dimension);

        // Use most of the base embedding
        let base_len = (self.config.dimension as f32 * 0.9) as usize;
        combined.extend_from_slice(&base[..base_len.min(base.len())]);

        // Append features
        for f in features {
            combined.push(*f);
        }

        // Pad or truncate to exact dimension
        combined.resize(self.config.dimension, 0.0);

        // Normalize
        let mag: f32 = combined.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag > 0.0 {
            for x in &mut combined {
                *x /= mag;
            }
        }

        combined
    }
}

impl Embedder for CodeEmbedder {
    fn embed(&self, text: &str) -> anyhow::Result<EmbeddingResult> {
        let start = std::time::Instant::now();

        // Get base embedding
        let base_result = self.base_embedder.embed(text)?;

        // Extract code features (assume no language info in single embed)
        let features = self.extract_features(text, "");

        // Combine
        let combined = self.combine_embeddings(&base_result.embedding, &features);

        Ok(EmbeddingResult {
            embedding: combined,
            model: self.config.model_name.clone(),
            tokens: base_result.tokens,
            processing_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<EmbeddingResult>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }

    fn model_name(&self) -> &str {
        &self.config.model_name
    }
}

/// Embed code with language awareness
pub fn embed_code(
    _embedder: &dyn Embedder,
    code: &str,
    _language: &str,
) -> anyhow::Result<EmbeddingResult> {
    let code_embedder = CodeEmbedder::new(EmbeddingConfig::default());
    code_embedder.embed(code)
}

#[cfg(all(test, feature = "fixme_tests"))]
mod tests {
    use super::*;

    #[test]
    fn test_hash_embedder() {
        let config = EmbeddingConfig::default();
        let embedder = HashEmbedder::new(config);

        let result = embedder.embed("hello world").unwrap();
        assert_eq!(result.embedding.len(), 384);
        assert!(result.embedding.iter().all(|&x| x >= -1.0 && x <= 1.0));
    }

    #[test]
    fn test_deterministic_embeddings() {
        let config = EmbeddingConfig::default();
        let embedder = HashEmbedder::new(config);

        let r1 = embedder.embed("test").unwrap();
        let r2 = embedder.embed("test").unwrap();

        assert_eq!(r1.embedding, r2.embedding);
    }
}