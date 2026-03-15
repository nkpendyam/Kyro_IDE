//! RAG (Retrieval-Augmented Generation) Commands
//!
//! Real TF-IDF indexing + BM25 search over source code files

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

/// RAG index status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RagIndexStatus {
    pub indexed_files: u64,
    pub total_chunks: u64,
    pub index_size_mb: f64,
    pub last_indexed: Option<String>,
    pub is_indexing: bool,
}

/// Search result from RAG
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RagSearchResult {
    pub file_path: String,
    pub content: String,
    pub score: f32,
    pub line_start: u32,
    pub line_end: u32,
    pub context: String,
    pub source: String,
    pub graph_score: Option<f32>,
    pub graph_distance: Option<u32>,
    pub neighbors: Vec<String>,
}

/// Index request
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IndexRequest {
    pub path: String,
    pub recursive: bool,
    pub file_types: Option<Vec<String>>,
}

/// Search request
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    pub max_results: Option<u32>,
    pub file_filter: Option<Vec<String>>,
    pub min_score: Option<f32>,
    pub graph_mode: Option<crate::rag::graph_rag::GraphSearchMode>,
}

/// RAG configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RagConfig {
    pub chunk_size: u32,
    pub chunk_overlap: u32,
    pub embedding_model: String,
    pub max_results: u32,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            chunk_size: 512,
            chunk_overlap: 50,
            embedding_model: "tfidf-bm25".to_string(),
            max_results: 10,
        }
    }
}

/// A chunk of source code with metadata
#[derive(Debug, Clone)]
struct CodeChunk {
    file_path: String,
    content: String,
    line_start: u32,
    line_end: u32,
    tokens: Vec<String>,
}

/// RAG State — real inverted index
pub struct RagState {
    pub status: RagIndexStatus,
    pub config: RagConfig,
    pub indexed_paths: Vec<String>,
    chunks: Vec<CodeChunk>,
    /// term -> list of (chunk_index, term_frequency)
    inverted_index: HashMap<String, Vec<(usize, f32)>>,
    /// total documents for IDF calculation
    doc_count: usize,
}

impl Default for RagState {
    fn default() -> Self {
        Self {
            status: RagIndexStatus {
                indexed_files: 0,
                total_chunks: 0,
                index_size_mb: 0.0,
                last_indexed: None,
                is_indexing: false,
            },
            config: RagConfig::default(),
            indexed_paths: Vec::new(),
            chunks: Vec::new(),
            inverted_index: HashMap::new(),
            doc_count: 0,
        }
    }
}

/// Source file extensions to index
const CODE_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "cpp", "h", "hpp", "cs", "rb", "php",
    "swift", "kt", "scala", "vue", "svelte", "html", "css", "scss", "json", "yaml", "yml", "toml",
    "md", "sql", "sh", "bash", "ps1",
];

fn should_index(path: &Path, file_types: &Option<Vec<String>>) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if let Some(types) = file_types {
        types.iter().any(|t| t == ext)
    } else {
        CODE_EXTENSIONS.contains(&ext)
    }
}

fn tokenize(text: &str) -> Vec<String> {
    use rust_stemmers::{Algorithm, Stemmer};
    let stemmer = Stemmer::create(Algorithm::English);
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.len() > 1)
        .map(|w| stemmer.stem(&w.to_lowercase()).to_string())
        .collect()
}

fn chunk_file(file_path: &str, content: &str, chunk_size: u32, overlap: u32) -> Vec<CodeChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let chunk_lines = chunk_size.max(10) as usize;
    let overlap_lines = overlap.min(chunk_size / 2) as usize;
    let mut chunks = Vec::new();
    let mut start = 0usize;

    while start < lines.len() {
        let end = (start + chunk_lines).min(lines.len());
        let chunk_content: String = lines[start..end].join("\n");
        let tokens = tokenize(&chunk_content);
        if !tokens.is_empty() {
            chunks.push(CodeChunk {
                file_path: file_path.to_string(),
                content: chunk_content,
                line_start: start as u32 + 1,
                line_end: end as u32,
                tokens,
            });
        }
        if end >= lines.len() {
            break;
        }
        start = end.saturating_sub(overlap_lines);
    }
    chunks
}

fn build_inverted_index(chunks: &[CodeChunk]) -> HashMap<String, Vec<(usize, f32)>> {
    let mut index: HashMap<String, Vec<(usize, f32)>> = HashMap::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let mut term_counts: HashMap<&str, u32> = HashMap::new();
        for token in &chunk.tokens {
            *term_counts.entry(token.as_str()).or_insert(0) += 1;
        }
        let total = chunk.tokens.len() as f32;
        for (term, count) in term_counts {
            let tf = count as f32 / total;
            index.entry(term.to_string()).or_default().push((i, tf));
        }
    }
    index
}

/// BM25 scoring
fn bm25_score(
    query_tokens: &[String],
    inverted_index: &HashMap<String, Vec<(usize, f32)>>,
    doc_count: usize,
    chunk_idx: usize,
    avg_dl: f32,
    doc_len: f32,
) -> f32 {
    let k1: f32 = 1.2;
    let b: f32 = 0.75;
    let mut score = 0.0f32;
    for token in query_tokens {
        if let Some(postings) = inverted_index.get(token) {
            let df = postings.len() as f32;
            let idf = ((doc_count as f32 - df + 0.5) / (df + 0.5) + 1.0).ln();
            if let Some((_, tf)) = postings.iter().find(|(idx, _)| *idx == chunk_idx) {
                let numerator = tf * (k1 + 1.0);
                let denominator = tf + k1 * (1.0 - b + b * doc_len / avg_dl);
                score += idf * numerator / denominator;
            }
        }
    }
    score
}

// ============ Tauri Commands ============

#[tauri::command]
pub async fn get_rag_status(
    state: State<'_, Arc<RwLock<RagState>>>,
) -> Result<RagIndexStatus, String> {
    let rag = state.read().await;
    Ok(rag.status.clone())
}

#[tauri::command]
pub async fn index_project(
    request: IndexRequest,
    state: State<'_, Arc<RwLock<RagState>>>,
) -> Result<RagIndexStatus, String> {
    // Collect files first (outside the lock)
    let root = PathBuf::from(&request.path);
    if !root.exists() {
        return Err(format!("Path does not exist: {}", request.path));
    }

    {
        let mut rag = state.write().await;
        rag.status.is_indexing = true;
    }

    let mut file_paths: Vec<PathBuf> = Vec::new();
    if request.recursive {
        for entry in walkdir::WalkDir::new(&root)
            .follow_links(true)
            .max_depth(20)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() && should_index(path, &request.file_types) {
                // Skip common non-source directories
                let path_str = path.to_string_lossy();
                if !path_str.contains("node_modules")
                    && !path_str.contains(".git")
                    && !path_str.contains("target")
                    && !path_str.contains("dist")
                    && !path_str.contains("build")
                {
                    file_paths.push(path.to_path_buf());
                }
            }
        }
    } else {
        // Read only immediate directory
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && should_index(&path, &request.file_types) {
                    file_paths.push(path);
                }
            }
        }
    }

    // Read and chunk all files
    let chunk_size;
    let chunk_overlap;
    {
        let rag = state.read().await;
        chunk_size = rag.config.chunk_size;
        chunk_overlap = rag.config.chunk_overlap;
    }

    let mut all_chunks: Vec<CodeChunk> = Vec::new();
    let mut total_size: u64 = 0;
    let mut indexed_count: u64 = 0;

    for path in &file_paths {
        match tokio::fs::read_to_string(path).await {
            Ok(content) => {
                total_size += content.len() as u64;
                let file_str = path.to_string_lossy().to_string();
                let chunks = chunk_file(&file_str, &content, chunk_size, chunk_overlap);
                all_chunks.extend(chunks);
                indexed_count += 1;
            }
            Err(_) => continue, // Skip binary/unreadable files
        }
    }

    // Build inverted index
    let _inverted_index = build_inverted_index(&all_chunks);
    let _doc_count = all_chunks.len();
    let _total_chunks = all_chunks.len() as u64;

    // Update state
    let mut rag = state.write().await;
    rag.chunks.extend(all_chunks);
    // Rebuild full index since we may have merged with existing chunks
    rag.inverted_index = build_inverted_index(&rag.chunks);
    rag.doc_count = rag.chunks.len();
    rag.status.indexed_files += indexed_count;
    rag.status.total_chunks = rag.chunks.len() as u64;
    rag.status.index_size_mb = total_size as f64 / (1024.0 * 1024.0);
    rag.status.last_indexed = Some(chrono::Utc::now().to_rfc3339());
    rag.status.is_indexing = false;
    if !rag.indexed_paths.contains(&request.path) {
        rag.indexed_paths.push(request.path);
    }

    Ok(rag.status.clone())
}

#[tauri::command]
pub async fn semantic_search(
    request: SearchRequest,
    state: State<'_, Arc<RwLock<RagState>>>,
) -> Result<Vec<RagSearchResult>, String> {
    let rag = state.read().await;

    if rag.chunks.is_empty() {
        return Ok(vec![]);
    }

    let max = request.max_results.unwrap_or(rag.config.max_results) as usize;
    let min_score = request.min_score.unwrap_or(0.01);
    let query_tokens = tokenize(&request.query);

    if query_tokens.is_empty() {
        return Ok(vec![]);
    }

    // Calculate average document length
    let avg_dl = rag
        .chunks
        .iter()
        .map(|c| c.tokens.len() as f32)
        .sum::<f32>()
        / rag.doc_count.max(1) as f32;

    // Score all chunks
    let mut scored: Vec<(usize, f32)> = rag
        .chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let score = bm25_score(
                &query_tokens,
                &rag.inverted_index,
                rag.doc_count,
                i,
                avg_dl,
                chunk.tokens.len() as f32,
            );
            (i, score)
        })
        .filter(|(_, score)| *score > min_score)
        .collect();

    // Apply file filter
    if let Some(ref filters) = request.file_filter {
        scored.retain(|(i, _)| {
            let path = &rag.chunks[*i].file_path;
            filters.iter().any(|f| path.contains(f))
        });
    }

    // Sort by score descending
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(max);

    let results: Vec<RagSearchResult> = scored
        .into_iter()
        .map(|(i, score)| {
            let chunk = &rag.chunks[i];
            // Extract 3 lines of context around first query term match
            let lines: Vec<&str> = chunk.content.lines().collect();
            let context_line = lines
                .iter()
                .position(|l| {
                    let lower = l.to_lowercase();
                    request
                        .query
                        .split_whitespace()
                        .any(|q| lower.contains(&q.to_lowercase()))
                })
                .unwrap_or(0);
            let ctx_start = context_line.saturating_sub(1);
            let ctx_end = (context_line + 2).min(lines.len());
            let context = lines[ctx_start..ctx_end].join("\n");

            RagSearchResult {
                file_path: chunk.file_path.clone(),
                content: chunk.content.clone(),
                score,
                line_start: chunk.line_start,
                line_end: chunk.line_end,
                context,
                source: "direct".to_string(),
                graph_score: None,
                graph_distance: None,
                neighbors: Vec::new(),
            }
        })
        .collect();

    Ok(results)
}

#[tauri::command]
pub async fn clear_rag_index(state: State<'_, Arc<RwLock<RagState>>>) -> Result<(), String> {
    let mut rag = state.write().await;
    rag.status = RagIndexStatus {
        indexed_files: 0,
        total_chunks: 0,
        index_size_mb: 0.0,
        last_indexed: None,
        is_indexing: false,
    };
    rag.chunks.clear();
    rag.inverted_index.clear();
    rag.doc_count = 0;
    rag.indexed_paths.clear();
    Ok(())
}

#[tauri::command]
pub async fn get_rag_config(state: State<'_, Arc<RwLock<RagState>>>) -> Result<RagConfig, String> {
    let rag = state.read().await;
    Ok(rag.config.clone())
}

#[tauri::command]
pub async fn set_rag_config(
    config: RagConfig,
    state: State<'_, Arc<RwLock<RagState>>>,
) -> Result<RagConfig, String> {
    let mut rag = state.write().await;
    rag.config = config;
    Ok(rag.config.clone())
}

#[tauri::command]
pub async fn get_indexed_paths(
    state: State<'_, Arc<RwLock<RagState>>>,
) -> Result<Vec<String>, String> {
    let rag = state.read().await;
    Ok(rag.indexed_paths.clone())
}

#[tauri::command]
pub async fn remove_indexed_path(
    path: String,
    state: State<'_, Arc<RwLock<RagState>>>,
) -> Result<(), String> {
    let mut rag = state.write().await;
    // Remove chunks from that path
    rag.chunks.retain(|c| !c.file_path.starts_with(&path));
    rag.inverted_index = build_inverted_index(&rag.chunks);
    rag.doc_count = rag.chunks.len();
    rag.status.total_chunks = rag.chunks.len() as u64;
    rag.indexed_paths.retain(|p| p != &path);
    Ok(())
}

/// Graph-enhanced search: BM25 + dependency graph neighbors re-ranked by centrality
#[tauri::command]
pub async fn graph_enhanced_semantic_search(
    request: SearchRequest,
    rag_state: State<'_, Arc<RwLock<RagState>>>,
) -> Result<Vec<crate::rag::graph_rag::GraphSearchResult>, String> {
    use crate::rag::graph_rag::{graph_enhanced_search, GraphSearchMode};

    // 1. Run standard BM25 search
    let rag = rag_state.read().await;
    if rag.chunks.is_empty() {
        return Ok(vec![]);
    }

    let max = request.max_results.unwrap_or(rag.config.max_results) as usize;
    let min_score = request.min_score.unwrap_or(0.01);
    let query_tokens = tokenize(&request.query);
    if query_tokens.is_empty() {
        return Ok(vec![]);
    }

    let avg_dl = rag
        .chunks
        .iter()
        .map(|c| c.tokens.len() as f32)
        .sum::<f32>()
        / rag.doc_count.max(1) as f32;

    let mut scored: Vec<(usize, f32)> = rag
        .chunks
        .iter()
        .enumerate()
        .map(|(i, chunk)| {
            let score = bm25_score(
                &query_tokens,
                &rag.inverted_index,
                rag.doc_count,
                i,
                avg_dl,
                chunk.tokens.len() as f32,
            );
            (i, score)
        })
        .filter(|(_, score)| *score > min_score)
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(max * 2); // get extra for graph re-ranking

    let bm25_results: Vec<(String, f32, String, u32, u32, String)> = scored
        .into_iter()
        .map(|(i, score)| {
            let chunk = &rag.chunks[i];
            (
                chunk.file_path.clone(),
                score,
                chunk.content.clone(),
                chunk.line_start,
                chunk.line_end,
                String::new(),
            )
        })
        .collect();

    // 2. Enhance with dependency graph from RepoWiki engine
    let wiki_arc = super::repowiki::get_wiki_engine();
    let wiki_guard = wiki_arc.lock().await;
    let graph = match wiki_guard.as_ref() {
        Some(engine) => engine.graph.clone(),
        None => crate::repowiki::DependencyGraph::default(),
    };
    drop(wiki_guard);

    let mode = request.graph_mode.unwrap_or(GraphSearchMode::Local);
    let results = graph_enhanced_search(bm25_results, &graph, max, mode);
    Ok(results)
}
