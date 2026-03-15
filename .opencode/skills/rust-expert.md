---
name: rust-expert
description: Use this skill for all Rust code in Kyro IDE — Tauri commands, async Rust, error handling, Cargo dependencies, crate selection, performance optimization, and unsafe code review. Triggers on: any Rust file (.rs), Cargo.toml, "tauri command", "async", "tokio", "error handling", "rust crate".
---

# Rust Expert Skill

## Core rules for Kyro IDE Rust code

### NEVER do this
```rust
// ❌ NEVER — crashes in production
let repo = Repository::open(path).unwrap();
let data = some_option.expect("this won't happen");
```

### ALWAYS do this
```rust
// ✅ CORRECT — proper error propagation
let repo = Repository::open(path).map_err(|e| e.to_string())?;
let data = some_option.ok_or_else(|| "value missing".to_string())?;
```

## Standard Tauri command pattern
```rust
use tauri::command;
use anyhow::Result;

#[tauri::command]
pub async fn command_name(
    param1: String,
    param2: String,
) -> Result<ReturnType, String> {
    // Validate inputs first
    if param1.is_empty() {
        return Err("param1 cannot be empty".to_string());
    }
    
    // Main logic with ? for error propagation
    let result = do_work(&param1, &param2)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_command_name_success() {
        let result = command_name("valid".into(), "input".into()).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_command_name_empty_param() {
        let result = command_name("".into(), "input".into()).await;
        assert!(result.is_err());
    }
}
```

## Path validation (MUST use for all file operations)
```rust
use std::path::{Path, PathBuf};

pub fn validate_workspace_path(
    workspace_root: &str,
    user_path: &str,
) -> Result<PathBuf, String> {
    let root = Path::new(workspace_root)
        .canonicalize()
        .map_err(|e| format!("Invalid workspace: {}", e))?;
    
    let full_path = root.join(user_path)
        .canonicalize()
        .map_err(|e| format!("Invalid path: {}", e))?;
    
    // Prevent path traversal attacks
    if !full_path.starts_with(&root) {
        return Err("Path escapes workspace root".to_string());
    }
    
    Ok(full_path)
}
```

## Key crates for Kyro IDE
```toml
[dependencies]
# Tauri
tauri = { version = "2", features = ["protocol-asset"] }
tauri-plugin-updater = "2"
tauri-plugin-dialog = "2"
tauri-plugin-process = "2"

# Git
git2 = "0.19"

# AI / LLM
llm = "0.1"

# Context / AST
tree-sitter = "0.22"
tree-sitter-typescript = "0.21"
tree-sitter-rust = "0.21"
tree-sitter-python = "0.21"

# Errors
anyhow = "1"
thiserror = "1"

# Async
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Crypto (E2EE)
ring = "0.17"
argon2 = "0.5"
jsonwebtoken = "9"

# HTTP (model downloads)
reqwest = { version = "0.12", features = ["stream", "json"] }

# Hash verification
sha2 = "0.10"
hex = "0.4"
```

## When to search for crates
Use the web-researcher skill + websearch tool before adding any new dependency:
```
websearch: "best rust crate for [task] 2026 crates.io"
webfetch: https://crates.io/search?q=[keyword]
```

Always check: downloads/week, last updated, open issues, license.
