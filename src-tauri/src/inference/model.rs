//! Model Loading and Management
//!
//! Support for GGUF and safetensors model formats

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Model format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelFormat {
    GGUF,
    Safetensors,
    PyTorch,
}

/// Model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub architecture: String,
    pub context_length: usize,
    pub embedding_length: usize,
    pub num_layers: usize,
    pub num_heads: usize,
    pub vocab_size: usize,
    pub quantization: Option<String>,
}

/// Detect model format from path
pub fn detect_format(path: &Path) -> Result<ModelFormat> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension {
        "gguf" => Ok(ModelFormat::GGUF),
        "safetensors" => Ok(ModelFormat::Safetensors),
        "bin" | "pt" | "pth" => Ok(ModelFormat::PyTorch),
        _ => anyhow::bail!("Unknown model format: {}", extension),
    }
}

/// Load model metadata from file
pub fn load_metadata(path: &Path) -> Result<ModelMetadata> {
    let format = detect_format(path)?;

    match format {
        ModelFormat::GGUF => load_gguf_metadata(path),
        ModelFormat::Safetensors => load_safetensors_metadata(path),
        ModelFormat::PyTorch => load_pytorch_metadata(path),
    }
}

fn load_gguf_metadata(path: &Path) -> Result<ModelMetadata> {
    use std::io::{BufReader, Read};

    let file = std::fs::File::open(path)?;
    let mut r = BufReader::new(file);

    // Magic: "GGUF" (4 bytes)
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)?;
    anyhow::ensure!(&magic == b"GGUF", "Not a GGUF file: bad magic");

    // version u32 LE
    let mut buf4 = [0u8; 4];
    r.read_exact(&mut buf4)?;
    let _version = u32::from_le_bytes(buf4);

    // tensor_count u64 LE
    let mut buf8 = [0u8; 8];
    r.read_exact(&mut buf8)?;
    let _tensor_count = u64::from_le_bytes(buf8);

    // metadata_count u64 LE
    r.read_exact(&mut buf8)?;
    let metadata_count = u64::from_le_bytes(buf8);

    // GGUF value type IDs
    const GGUF_TYPE_UINT8: u32 = 0;
    const GGUF_TYPE_INT8: u32 = 1;
    const GGUF_TYPE_UINT16: u32 = 2;
    const GGUF_TYPE_INT16: u32 = 3;
    const GGUF_TYPE_UINT32: u32 = 4;
    const GGUF_TYPE_INT32: u32 = 5;
    const GGUF_TYPE_FLOAT32: u32 = 6;
    const GGUF_TYPE_BOOL: u32 = 7;
    const GGUF_TYPE_STRING: u32 = 8;
    const GGUF_TYPE_ARRAY: u32 = 9;
    const GGUF_TYPE_UINT64: u32 = 10;
    const GGUF_TYPE_INT64: u32 = 11;
    const GGUF_TYPE_FLOAT64: u32 = 12;

    // Read a GGUF string: u64 length + UTF-8 bytes
    let read_str = |r: &mut BufReader<std::fs::File>| -> Result<String> {
        let mut len8 = [0u8; 8];
        r.read_exact(&mut len8)?;
        let len = u64::from_le_bytes(len8) as usize;
        let mut bytes = vec![0u8; len];
        r.read_exact(&mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    };

    let mut architecture = String::from("llama");
    let mut context_length: usize = 4096;
    let mut embedding_length: usize = 4096;
    let mut num_layers: usize = 32;
    let mut num_heads: usize = 32;
    let mut vocab_size: usize = 32000;
    let mut quantization: Option<String> = None;

    for _ in 0..metadata_count {
        let key = match read_str(&mut r) {
            Ok(k) => k,
            Err(_) => break,
        };

        let mut type_buf = [0u8; 4];
        if r.read_exact(&mut type_buf).is_err() {
            break;
        }
        let val_type = u32::from_le_bytes(type_buf);

        match val_type {
            GGUF_TYPE_UINT8 => { let mut b = [0u8; 1]; let _ = r.read_exact(&mut b); }
            GGUF_TYPE_INT8  => { let mut b = [0u8; 1]; let _ = r.read_exact(&mut b); }
            GGUF_TYPE_UINT16 | GGUF_TYPE_INT16 => { let mut b = [0u8; 2]; let _ = r.read_exact(&mut b); }
            GGUF_TYPE_UINT32 | GGUF_TYPE_INT32 | GGUF_TYPE_FLOAT32 => {
                let mut b = [0u8; 4];
                if r.read_exact(&mut b).is_err() { break; }
                let val = u32::from_le_bytes(b);
                match key.as_str() {
                    "llama.context_length" | "phi2.context_length" | "gpt2.context_length"
                    | "mpt.context_length" | "falcon.context_length" | "bloom.context_length"
                    | "rwkv.context_length" | "whisper.context_length"
                    => context_length = val as usize,
                    "llama.embedding_length" | "phi2.embedding_length" | "gpt2.embedding_length"
                    | "mpt.embedding_length" | "falcon.embedding_length"
                    => embedding_length = val as usize,
                    "llama.block_count" | "phi2.block_count" | "gpt2.block_count"
                    | "mpt.n_layers" | "falcon.block_count"
                    => num_layers = val as usize,
                    "llama.attention.head_count" | "phi2.attention.head_count"
                    | "gpt2.attention.head_count"
                    => num_heads = val as usize,
                    _ => {}
                }
            }
            GGUF_TYPE_BOOL => { let mut b = [0u8; 1]; let _ = r.read_exact(&mut b); }
            GGUF_TYPE_STRING => {
                let val = match read_str(&mut r) { Ok(v) => v, Err(_) => break };
                match key.as_str() {
                    "general.architecture" => architecture = val,
                    "general.quantization_version" | "general.file_type" => {
                        quantization = Some(val);
                    }
                    _ => {}
                }
            }
            GGUF_TYPE_UINT64 | GGUF_TYPE_INT64 | GGUF_TYPE_FLOAT64 => {
                let mut b = [0u8; 8];
                if r.read_exact(&mut b).is_err() { break; }
                if key == "tokenizer.ggml.tokens" {
                    // this would be array type, handled below
                }
            }
            GGUF_TYPE_ARRAY => {
                // array: element_type u32, count u64, then count elements
                let mut elem_type_buf = [0u8; 4];
                if r.read_exact(&mut elem_type_buf).is_err() { break; }
                let elem_type = u32::from_le_bytes(elem_type_buf);
                let mut count_buf = [0u8; 8];
                if r.read_exact(&mut count_buf).is_err() { break; }
                let count = u64::from_le_bytes(count_buf) as usize;

                if key == "tokenizer.ggml.tokens" {
                    vocab_size = count;
                }

                // Skip all elements
                let elem_size: Option<usize> = match elem_type {
                    GGUF_TYPE_UINT8 | GGUF_TYPE_INT8 | GGUF_TYPE_BOOL => Some(1),
                    GGUF_TYPE_UINT16 | GGUF_TYPE_INT16 => Some(2),
                    GGUF_TYPE_UINT32 | GGUF_TYPE_INT32 | GGUF_TYPE_FLOAT32 => Some(4),
                    GGUF_TYPE_UINT64 | GGUF_TYPE_INT64 | GGUF_TYPE_FLOAT64 => Some(8),
                    _ => None,
                };

                match elem_size {
                    Some(sz) => {
                        let total = sz * count;
                        let mut skip = vec![0u8; total];
                        if r.read_exact(&mut skip).is_err() { break; }
                    }
                    None if elem_type == GGUF_TYPE_STRING => {
                        for _ in 0..count {
                            if read_str(&mut r).is_err() { break; }
                        }
                    }
                    _ => break, // Nested array or unknown — stop parsing
                }
            }
            _ => break, // Unknown type — stop parsing to avoid misalignment
        }
    }

    // Derive quantization from filename if not found in metadata
    if quantization.is_none() {
        let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
        for tag in &["q4_k_m", "q4_k_s", "q5_k_m", "q8_0", "q4_0", "q5_0", "f16", "bf16"] {
            if fname.contains(tag) {
                quantization = Some(tag.to_uppercase());
                break;
            }
        }
    }

    Ok(ModelMetadata {
        architecture,
        context_length,
        embedding_length,
        num_layers,
        num_heads,
        vocab_size,
        quantization,
    })
}

fn load_safetensors_metadata(path: &Path) -> Result<ModelMetadata> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;

    // First 8 bytes: header length (u64 LE)
    let mut len_buf = [0u8; 8];
    file.read_exact(&mut len_buf)?;
    let header_len = u64::from_le_bytes(len_buf) as usize;

    anyhow::ensure!(header_len < 100 * 1024 * 1024, "Safetensors header too large");

    let mut header_bytes = vec![0u8; header_len];
    file.read_exact(&mut header_bytes)?;
    let header: serde_json::Value = serde_json::from_slice(&header_bytes)
        .context("Failed to parse safetensors JSON header")?;

    // Extract model info from __metadata__ if present
    let meta = header.get("__metadata__");
    let architecture = meta
        .and_then(|m| m.get("model_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Infer embedding_length from the first weight tensor's shape
    let embedding_length = header
        .as_object()
        .and_then(|obj| {
            obj.iter()
                .filter(|(k, _)| k.as_str() != "__metadata__")
                .filter_map(|(_, v)| {
                    v.get("shape")?.as_array().and_then(|s| {
                        s.last()?.as_u64().map(|n| n as usize)
                    })
                })
                .next()
        })
        .unwrap_or(2048);

    // Count transformer layers from tensor names
    let num_layers = header
        .as_object()
        .and_then(|obj| {
            obj.keys()
                .filter_map(|k| {
                    // Match patterns like "model.layers.31...."
                    let parts: Vec<&str> = k.split('.').collect();
                    parts.iter().position(|&p| p == "layers").and_then(|i| {
                        parts.get(i + 1)?.parse::<usize>().ok()
                    })
                })
                .max()
                .map(|m| m + 1)
        })
        .unwrap_or(24);

    Ok(ModelMetadata {
        architecture,
        context_length: 2048,
        embedding_length,
        num_layers,
        num_heads: 16,
        vocab_size: 32000,
        quantization: None,
    })
}

fn load_pytorch_metadata(path: &Path) -> Result<ModelMetadata> {
    // PyTorch .bin/.pt/.pth files use pickle format — non-trivial to parse without a full
    // implementation. Use file size as a heuristic for parameter count.
    let size_bytes = std::fs::metadata(path)?.len();
    let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();

    // Rough param estimate: FP16 = 2 bytes/param, FP32 = 4 bytes/param
    let params_estimate = size_bytes / 2;

    let num_layers = match params_estimate {
        p if p > 30_000_000_000 => 80, // 70B+
        p if p > 10_000_000_000 => 40, // 13B
        p if p > 5_000_000_000  => 32, // 7B
        p if p > 2_000_000_000  => 24, // 3B
        _                        => 12, // smaller
    };

    let architecture = if fname.contains("llama") { "llama" }
        else if fname.contains("mistral") { "mistral" }
        else if fname.contains("phi") { "phi" }
        else if fname.contains("falcon") { "falcon" }
        else { "unknown" };

    Ok(ModelMetadata {
        architecture: architecture.to_string(),
        context_length: 2048,
        embedding_length: 4096,
        num_layers,
        num_heads: 32,
        vocab_size: 32000,
        quantization: None,
    })
}
