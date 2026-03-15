//! Model Manager for KRO_IDE
//!
//! Handles model discovery, download, and lifecycle management

use super::*;
use crate::inference::model as model_meta;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Model specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    /// Model name/identifier
    pub name: String,
    /// Path to GGUF file
    pub path: String,
    /// Model size in bytes
    pub size_bytes: u64,
    /// Quantization type
    pub quantization: Quantization,
    /// Context size the model was trained on
    pub trained_context: u32,
    /// Parameter count (e.g., 7_000_000_000 for 7B)
    pub parameters: u64,
    /// Model architecture
    pub architecture: String,
    /// HuggingFace repo ID if applicable
    pub hf_repo: Option<String>,
    /// SHA256 hash for verification
    pub sha256: Option<String>,
    /// Download URL (direct GGUF link)
    pub url: Option<String>,
    /// Recommended memory tier
    pub min_memory_tier: MemoryTier,
}

/// Quantization type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quantization {
    /// 4-bit, medium quality (recommended)
    Q4_K_M,
    /// 4-bit, small
    Q4_K_S,
    /// 5-bit, medium quality
    Q5_K_M,
    /// 8-bit, high quality
    Q8_0,
    /// FP16, full precision
    F16,
    /// Unknown
    Unknown,
}

impl std::fmt::Display for Quantization {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Q4_K_M => write!(f, "Q4_K_M"),
            Self::Q4_K_S => write!(f, "Q4_K_S"),
            Self::Q5_K_M => write!(f, "Q5_K_M"),
            Self::Q8_0 => write!(f, "Q8_0"),
            Self::F16 => write!(f, "F16"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Pre-configured models for KRO_IDE
pub const BUILTIN_MODELS: &[(&str, &str, &str, u64, MemoryTier)] = &[
    // (name, hf_repo, url, size_bytes, min_tier)
    (
        "phi-2b-q4_k_m",
        "TheBloke/phi-2-GGUF",
        "https://huggingface.co/TheBloke/phi-2-GGUF/resolve/main/phi-2.Q4_K_M.gguf",
        1_500_000_000,
        MemoryTier::Cpu
    ),
    (
        "tinyllama-1.1b-q4_k_m",
        "TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF",
        "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf",
        800_000_000,
        MemoryTier::Cpu
    ),
    (
        "qwen2.5-coder-7b-q4_k_m",
        "Qwen/Qwen2.5-Coder-7B-Instruct-GGUF",
        "https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct-GGUF/resolve/main/qwen2.5-coder-7b-instruct-q4_k_m.gguf",
        4_800_000_000,
        MemoryTier::Medium8GB
    ),
];

/// Model manager
pub struct ModelManager {
    model_paths: Vec<String>,
    available_models: HashMap<String, ModelSpec>,
}

impl ModelManager {
    pub fn new(model_paths: Vec<String>) -> Result<Self> {
        let mut manager = Self {
            model_paths,
            available_models: HashMap::new(),
        };
        manager.scan_models()?;
        Ok(manager)
    }

    /// Scan model directories for available GGUF files
    fn scan_models(&mut self) -> Result<()> {
        let paths: Vec<String> = self.model_paths.clone();
        for path_str in &paths {
            let expanded = shellexpand::tilde(path_str).into_owned();
            let path = PathBuf::from(expanded);

            if path.exists() {
                self.scan_directory(&path)?;
            }
        }

        // Add builtin model specs if not found
        for (name, hf_repo, url, size, tier) in BUILTIN_MODELS {
            if !self.available_models.contains_key(*name) {
                self.available_models.insert(
                    name.to_string(),
                    ModelSpec {
                        name: name.to_string(),
                        path: String::new(), // Not downloaded yet
                        size_bytes: *size,
                        quantization: Quantization::Q4_K_M,
                        trained_context: 4096,
                        parameters: *size * 2, // Rough estimate
                        architecture: "llama".to_string(),
                        hf_repo: Some(hf_repo.to_string()),
                        sha256: None,
                        url: Some(url.to_string()),
                        min_memory_tier: *tier,
                    },
                );
            }
        }

        log::info!("Found {} models", self.available_models.len());
        Ok(())
    }

    /// Scan a directory for GGUF files
    fn scan_directory(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recurse into subdirectories
                self.scan_directory(&path)?;
            } else if let Some(ext) = path.extension() {
                if ext == "gguf" || ext == "GGUF" {
                    if let Ok(spec) = self.parse_gguf(&path) {
                        self.available_models.insert(spec.name.clone(), spec);
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse GGUF file metadata
    fn parse_gguf(&self, path: &Path) -> Result<ModelSpec> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let metadata = std::fs::metadata(path)?;
        let size_bytes = metadata.len();

        // Parse quantization from filename
        let quantization = self.detect_quantization(&name);

        let parsed_meta = model_meta::load_metadata(path).ok();
        let trained_context = parsed_meta
            .as_ref()
            .map(|m| m.context_length as u32)
            .unwrap_or(4096);
        let parameters = parsed_meta
            .as_ref()
            .map(|m| m.num_layers as u64 * m.embedding_length as u64 * 12)
            .unwrap_or(0);
        let architecture = parsed_meta
            .as_ref()
            .map(|m| m.architecture.clone())
            .unwrap_or_else(|| "llama".to_string());

        // Estimate tier from size
        let tier = MemoryTier::from_vram(size_bytes * 2); // Account for KV cache

        Ok(ModelSpec {
            name,
            path: path.to_string_lossy().to_string(),
            size_bytes,
            quantization,
            trained_context,
            parameters,
            architecture,
            hf_repo: None,
            sha256: None,
            url: None,
            min_memory_tier: tier,
        })
    }

    /// Detect quantization from model name
    fn detect_quantization(&self, name: &str) -> Quantization {
        let lower = name.to_lowercase();

        if lower.contains("q4_k_m") || lower.contains("q4km") {
            Quantization::Q4_K_M
        } else if lower.contains("q4_k_s") || lower.contains("q4ks") {
            Quantization::Q4_K_S
        } else if lower.contains("q5_k_m") || lower.contains("q5km") {
            Quantization::Q5_K_M
        } else if lower.contains("q8_0") || lower.contains("q80") {
            Quantization::Q8_0
        } else if lower.contains("f16") || lower.contains("fp16") {
            Quantization::F16
        } else {
            Quantization::Unknown
        }
    }

    /// Get model specification
    pub fn get_spec(&self, name: &str) -> Result<ModelSpec> {
        self.available_models
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", name))
    }

    /// List all available models
    pub fn list_models(&self) -> Vec<&ModelSpec> {
        self.available_models.values().collect()
    }

    /// List models compatible with a memory tier
    pub fn list_compatible_models(&self, tier: MemoryTier) -> Vec<&ModelSpec> {
        self.available_models
            .values()
            .filter(|m| m.min_memory_tier <= tier)
            .collect()
    }

    /// Check if a model is downloaded
    pub fn is_downloaded(&self, name: &str) -> bool {
        self.available_models
            .get(name)
            .map(|m| !m.path.is_empty() && Path::new(&m.path).exists())
            .unwrap_or(false)
    }

    /// Download a model (async, with progress callback)
    pub async fn download_model<F>(&mut self, name: &str, progress: F) -> Result<PathBuf>
    where
        F: Fn(f32) + Send + 'static,
    {
        // Need to clone spec to avoid borrow issues
        let spec = self.get_spec(name)?.clone();

        // Determine URL
        let url = if let Some(u) = &spec.url {
            u.clone()
        } else if let Some(hf_repo) = &spec.hf_repo {
            format!(
                "https://huggingface.co/{}/resolve/main/{}.gguf",
                hf_repo, name
            )
        } else {
            anyhow::bail!("No URL or HuggingFace repo for model: {}", name);
        };

        // Determine destination path
        let dest_dir = if self.model_paths.is_empty() {
            PathBuf::from("models")
        } else {
            PathBuf::from(shellexpand::tilde(&self.model_paths[0]).into_owned())
        };

        if !dest_dir.exists() {
            std::fs::create_dir_all(&dest_dir)?;
        }

        let dest_path = dest_dir.join(format!("{}.gguf", name));

        log::info!("Downloading model from {} to {:?}", url, dest_path);

        // Use reqwest for download
        let response = reqwest::get(&url).await?;
        let total_size = response.content_length().unwrap_or(spec.size_bytes);

        use futures_util::StreamExt;
        use std::io::Write;

        // Use std::fs for synchronous write in async loop (or tokio::fs)
        // Using tokio::fs is better
        let mut file = tokio::fs::File::create(&dest_path).await?;
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut last_reported = 0.0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            use tokio::io::AsyncWriteExt;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let p = downloaded as f32 / total_size as f32;
                if p - last_reported >= 0.01 {
                    progress(p);
                    last_reported = p;
                }
            }
        }

        log::info!("Model {} downloaded to {:?}", name, dest_path);

        // Update the spec with the new path
        if let Some(s) = self.available_models.get_mut(name) {
            s.path = dest_path.to_string_lossy().to_string();
        }

        Ok(dest_path)
    }

    /// Delete a downloaded model
    pub fn delete_model(&self, name: &str) -> Result<()> {
        let spec = self.get_spec(name)?;

        if !spec.path.is_empty() {
            std::fs::remove_file(&spec.path)?;
            log::info!("Deleted model: {}", name);
        }

        Ok(())
    }
}

impl std::ops::Index<&str> for ModelManager {
    type Output = ModelSpec;

    fn index(&self, index: &str) -> &Self::Output {
        &self.available_models[index]
    }
}
