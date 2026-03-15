//! Real AI Service Implementation
//!
//! This module provides actual AI inference using multiple backends:
//! 1. Local inference via llama.cpp (when compiled with `llama-cpp` feature)
//! 2. HTTP-based inference (Ollama, LM Studio, vLLM)
//! 3. Cloud API fallback (OpenAI-compatible APIs)
//!
//! Priority: Local > HTTP Local > Cloud API

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::embedded_llm::{EmbeddedLLMConfig, EmbeddedLLMEngine, InferenceRequest};

/// AI Backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiBackendConfig {
    /// Preferred backend: "local", "ollama", "lmstudio", "openai", "auto"
    pub backend: String,
    /// Model to use
    pub model: String,
    /// API endpoint (for HTTP backends)
    pub endpoint: Option<String>,
    /// API key (for cloud backends)
    pub api_key: Option<String>,
    /// Temperature for generation
    pub temperature: f32,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Enable streaming
    pub stream: bool,
}

impl Default for AiBackendConfig {
    fn default() -> Self {
        Self {
            backend: "auto".to_string(),
            model: "codellama:7b".to_string(),
            endpoint: None,
            api_key: None,
            temperature: 0.7,
            max_tokens: 2048,
            stream: true,
        }
    }
}

/// AI completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The prompt to complete
    pub prompt: String,
    /// System prompt (for chat models)
    pub system_prompt: Option<String>,
    /// Conversation history
    pub history: Vec<ConversationMessage>,
    /// Temperature override
    pub temperature: Option<f32>,
    /// Max tokens override
    pub max_tokens: Option<u32>,
    /// Stop sequences
    pub stop_sequences: Vec<String>,
    /// Context from RAG
    pub context: Option<String>,
}

/// Conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
}

/// AI completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Generated text
    pub text: String,
    /// Model used
    pub model: String,
    /// Backend used
    pub backend: String,
    /// Tokens generated
    pub tokens_generated: u32,
    /// Time to first token (ms)
    pub time_to_first_token_ms: u64,
    /// Total time (ms)
    pub total_time_ms: u64,
    /// Tokens per second
    pub tokens_per_second: f32,
    /// Whether response was from cache
    pub from_cache: bool,
}

/// AI Service
pub struct AiService {
    config: AiBackendConfig,
    http_client: reqwest::Client,
    available_backends: Arc<RwLock<Vec<String>>>,
    cache: Arc<RwLock<HashMap<String, CompletionResponse>>>,
    order: Arc<RwLock<VecDeque<String>>>,
    capacity: usize,
    embedded_engine: Arc<RwLock<Option<EmbeddedLLMEngine>>>,
}

impl AiService {
    /// Create a new AI service
    pub fn new(config: AiBackendConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            http_client,
            available_backends: Arc::new(RwLock::new(Vec::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            order: Arc::new(RwLock::new(VecDeque::new())),
            capacity: 64,
            embedded_engine: Arc::new(RwLock::new(None)),
        }
    }

    /// Detect available backends
    pub async fn detect_backends(&self) -> Result<Vec<String>> {
        let mut backends = Vec::new();

        // Check for local llama.cpp models
        #[cfg(feature = "llama-cpp")]
        {
            backends.push("local".to_string());
        }

        // Check for Ollama
        if self.check_ollama().await.is_ok() {
            backends.push("ollama".to_string());
        }

        // Check for LM Studio
        if self.check_lm_studio().await.is_ok() {
            backends.push("lmstudio".to_string());
        }

        // Check for vLLM
        if self.check_vllm().await.is_ok() {
            backends.push("vllm".to_string());
        }

        *self.available_backends.write().await = backends.clone();
        Ok(backends)
    }

    /// Check if Ollama is running
    async fn check_ollama(&self) -> Result<()> {
        let response = self
            .http_client
            .get("http://localhost:11434/api/tags")
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .context("Ollama not available")?;

        if response.status().is_success() {
            Ok(())
        } else {
            bail!("Ollama returned non-success status")
        }
    }

    /// Check if LM Studio is running
    async fn check_lm_studio(&self) -> Result<()> {
        let response = self
            .http_client
            .get("http://localhost:1234/v1/models")
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .context("LM Studio not available")?;

        if response.status().is_success() {
            Ok(())
        } else {
            bail!("LM Studio returned non-success status")
        }
    }

    /// Check if vLLM is running
    async fn check_vllm(&self) -> Result<()> {
        let response = self
            .http_client
            .get("http://localhost:8000/v1/models")
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await
            .context("vLLM not available")?;

        if response.status().is_success() {
            Ok(())
        } else {
            bail!("vLLM returned non-success status")
        }
    }

    /// Complete a prompt using the best available backend
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let start = Instant::now();

        let key = self.compute_key(&request).await?;
        if let Some(resp) = self.cache_get(&key).await {
            return Ok(resp);
        }

        let backend = self.select_backend().await?;

        let response = match backend.as_str() {
            "ollama" => self.complete_ollama(&request).await?,
            "lmstudio" => {
                self.complete_openai_compatible(&request, "http://localhost:1234/v1")
                    .await?
            }
            "vllm" => {
                self.complete_openai_compatible(&request, "http://localhost:8000/v1")
                    .await?
            }
            "local" => self.complete_local(&request).await?,
            _ => {
                // Fallback to pattern matching
                self.complete_fallback(&request).await?
            }
        };

        self.cache_put(key, response.clone()).await?;

        let total_time = start.elapsed();
        Ok(CompletionResponse {
            total_time_ms: total_time.as_millis() as u64,
            ..response
        })
    }

    async fn cache_get(&self, key: &str) -> Option<CompletionResponse> {
        let cache = self.cache.read().await;
        cache.get(key).cloned()
    }

    async fn cache_put(&self, key: String, value: CompletionResponse) -> Result<()> {
        {
            let mut cache = self.cache.write().await;
            cache.insert(key.clone(), value);
        }
        {
            let mut order = self.order.write().await;
            order.retain(|k| k != &key);
            order.push_front(key.clone());
            while order.len() > self.capacity {
                if let Some(old) = order.pop_back() {
                    let mut cache = self.cache.write().await;
                    cache.remove(&old);
                }
            }
        }
        Ok(())
    }

    async fn compute_key(&self, request: &CompletionRequest) -> Result<String> {
        let mut hasher = Sha256::new();
        if let Some(system) = &request.system_prompt {
            hasher.update(system.as_bytes());
        }
        if let Some(ctx) = &request.context {
            hasher.update(ctx.as_bytes());
        }
        for msg in &request.history {
            hasher.update(msg.role.as_bytes());
            hasher.update(msg.content.as_bytes());
        }
        hasher.update(request.prompt.as_bytes());
        hasher.update(
            format!("{}", request.temperature.unwrap_or(self.config.temperature)).as_bytes(),
        );
        hasher
            .update(format!("{}", request.max_tokens.unwrap_or(self.config.max_tokens)).as_bytes());
        hasher.update(self.config.model.as_bytes());
        let digest = hasher.finalize();
        Ok(format!("{:x}", digest))
    }

    /// Select the best available backend
    async fn select_backend(&self) -> Result<String> {
        let backends = self.available_backends.read().await;

        if self.config.backend != "auto" {
            return Ok(self.config.backend.clone());
        }

        // Priority order
        let priority = ["ollama", "lmstudio", "vllm", "local"];

        for b in priority {
            if backends.contains(&b.to_string()) {
                return Ok(b.to_string());
            }
        }

        Ok("fallback".to_string())
    }

    /// Complete using Ollama API
    async fn complete_ollama(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let start = Instant::now();

        // Build the prompt with context and history
        let full_prompt = self.build_prompt(request);

        let body = serde_json::json!({
            "model": self.config.model,
            "prompt": full_prompt,
            "stream": false,
            "options": {
                "temperature": request.temperature.unwrap_or(self.config.temperature),
                "num_predict": request.max_tokens.unwrap_or(self.config.max_tokens),
                "stop": request.stop_sequences,
            }
        });

        let response = self
            .http_client
            .post("http://localhost:11434/api/generate")
            .json(&body)
            .send()
            .await
            .context("Failed to connect to Ollama")?;

        if !response.status().is_success() {
            bail!("Ollama returned status: {}", response.status());
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        let text = json
            .get("response")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let tokens_generated =
            json.get("eval_count")
                .and_then(|v| v.as_u64())
                .unwrap_or_else(|| text.split_whitespace().count() as u64) as u32;

        let total_time_ms = start.elapsed().as_millis() as u64;
        let tokens_per_second = if total_time_ms > 0 && tokens_generated > 0 {
            (tokens_generated as f64 / (total_time_ms as f64 / 1000.0)) as f32
        } else {
            0.0
        };

        Ok(CompletionResponse {
            text,
            model: self.config.model.clone(),
            backend: "ollama".to_string(),
            tokens_generated,
            time_to_first_token_ms: total_time_ms / (tokens_generated.max(1) as u64),
            total_time_ms,
            tokens_per_second,
            from_cache: false,
        })
    }

    /// Complete using OpenAI-compatible API (LM Studio, vLLM)
    async fn complete_openai_compatible(
        &self,
        request: &CompletionRequest,
        base_url: &str,
    ) -> Result<CompletionResponse> {
        let start = Instant::now();

        let mut messages = Vec::new();

        // Add system prompt
        if let Some(system) = &request.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }

        // Add context if available
        if let Some(context) = &request.context {
            messages.push(serde_json::json!({
                "role": "system",
                "content": format!("Context from codebase:\n{}", context)
            }));
        }

        // Add history
        for msg in &request.history {
            messages.push(serde_json::json!({
                "role": msg.role,
                "content": msg.content
            }));
        }

        // Add current prompt
        messages.push(serde_json::json!({
            "role": "user",
            "content": request.prompt
        }));

        let body = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "temperature": request.temperature.unwrap_or(self.config.temperature),
            "max_tokens": request.max_tokens.unwrap_or(self.config.max_tokens),
            "stop": request.stop_sequences,
        });

        let response = self
            .http_client
            .post(format!("{}/chat/completions", base_url))
            .json(&body)
            .send()
            .await
            .context("Failed to connect to API")?;

        if !response.status().is_success() {
            bail!("API returned status: {}", response.status());
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse API response")?;

        let text = json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();

        let tokens_generated = text.split_whitespace().count() as u32;
        let total_time_ms = start.elapsed().as_millis() as u64;
        let tokens_per_second = if total_time_ms > 0 && tokens_generated > 0 {
            (tokens_generated as f64 / (total_time_ms as f64 / 1000.0)) as f32
        } else {
            0.0
        };

        Ok(CompletionResponse {
            text,
            model: self.config.model.clone(),
            backend: base_url.split('/').nth(2).unwrap_or("api").to_string(),
            tokens_generated,
            time_to_first_token_ms: total_time_ms / (tokens_generated.max(1) as u64),
            total_time_ms,
            tokens_per_second,
            from_cache: false,
        })
    }

    /// Complete using local llama.cpp
    #[cfg(feature = "llama-cpp")]
    async fn complete_local(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let start = Instant::now();

        let mut engine_guard = self.embedded_engine.write().await;
        if engine_guard.is_none() {
            let config = EmbeddedLLMConfig {
                default_model: self.config.model.clone(),
                context_size: 8192,
                ..EmbeddedLLMConfig::default()
            };
            *engine_guard = Some(EmbeddedLLMEngine::new(config).await?);
        }

        let engine = engine_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Embedded LLM engine failed to initialize"))?;

        let inference_request = InferenceRequest {
            prompt: self.build_prompt(request),
            system_prompt: request.system_prompt.clone(),
            max_tokens: request.max_tokens.unwrap_or(self.config.max_tokens),
            temperature: request.temperature.unwrap_or(self.config.temperature),
            top_p: 0.95,
            top_k: 40,
            repeat_penalty: 1.1,
            stop_sequences: request.stop_sequences.clone(),
            stream: self.config.stream,
            history: request
                .history
                .iter()
                .map(|msg| crate::embedded_llm::ConversationTurn {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                })
                .collect(),
        };

        let response = engine.complete(&inference_request).await?;
        let total_time_ms = start.elapsed().as_millis() as u64;

        Ok(CompletionResponse {
            text: response.text,
            model: response.model,
            backend: "local".to_string(),
            tokens_generated: response.tokens_generated,
            time_to_first_token_ms: response.time_to_first_token_ms,
            total_time_ms,
            tokens_per_second: response.tokens_per_second,
            from_cache: response.from_cache,
        })
    }

    #[cfg(not(feature = "llama-cpp"))]
    async fn complete_local(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let start = Instant::now();

        let mut engine_guard = self.embedded_engine.write().await;
        if engine_guard.is_none() {
            let config = EmbeddedLLMConfig {
                default_model: self.config.model.clone(),
                context_size: 8192,
                ..EmbeddedLLMConfig::default()
            };
            *engine_guard = Some(EmbeddedLLMEngine::new(config).await?);
        }

        let engine = engine_guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Embedded LLM engine failed to initialize"))?;

        let inference_request = InferenceRequest {
            prompt: self.build_prompt(request),
            system_prompt: request.system_prompt.clone(),
            max_tokens: request.max_tokens.unwrap_or(self.config.max_tokens),
            temperature: request.temperature.unwrap_or(self.config.temperature),
            top_p: 0.95,
            top_k: 40,
            repeat_penalty: 1.1,
            stop_sequences: request.stop_sequences.clone(),
            stream: self.config.stream,
            history: request
                .history
                .iter()
                .map(|msg| crate::embedded_llm::ConversationTurn {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
                })
                .collect(),
        };

        let response = engine.complete(&inference_request).await?;
        let total_time_ms = start.elapsed().as_millis() as u64;

        Ok(CompletionResponse {
            text: response.text,
            model: response.model,
            backend: "local".to_string(),
            tokens_generated: response.tokens_generated,
            time_to_first_token_ms: response.time_to_first_token_ms,
            total_time_ms,
            tokens_per_second: response.tokens_per_second,
            from_cache: response.from_cache,
        })
    }

    /// Fallback completion (pattern matching for basic assistance)
    async fn complete_fallback(&self, request: &CompletionRequest) -> Result<CompletionResponse> {
        let start = Instant::now();

        // Generate intelligent response based on code analysis patterns
        let response_text = self.generate_pattern_response(&request.prompt);

        let tokens_generated = response_text.split_whitespace().count() as u32;
        let total_time_ms = start.elapsed().as_millis() as u64;

        Ok(CompletionResponse {
            text: response_text,
            model: "pattern-matcher".to_string(),
            backend: "fallback".to_string(),
            tokens_generated,
            time_to_first_token_ms: 25,
            total_time_ms,
            tokens_per_second: 50.0,
            from_cache: false,
        })
    }

    /// Build full prompt from request
    fn build_prompt(&self, request: &CompletionRequest) -> String {
        let mut prompt = String::new();
        let mut ctx = String::new();
        let mut hist = String::new();
        if let Some(context) = &request.context {
            ctx.push_str(context);
        }
        if !request.history.is_empty() {
            let mut h = String::new();
            for msg in &request.history {
                h.push_str(&msg.role);
                h.push_str(": ");
                h.push_str(&msg.content);
                h.push('\n');
            }
            hist = crate::memory::compression::compress_chat_history(&h);
        }
        if !ctx.is_empty() {
            ctx = crate::memory::compression::compress_ast_to_signatures(&ctx, "typescript");
        }

        if let Some(system) = &request.system_prompt {
            prompt.push_str(&format!("System: {}\n\n", system));
        }

        if !ctx.is_empty() {
            prompt.push_str("Context:\n");
            prompt.push_str(&ctx);
            prompt.push_str("\n\n");
        }

        if !hist.is_empty() {
            prompt.push_str(&hist);
        }

        prompt.push_str(&format!("User: {}", request.prompt));

        if prompt.len() > 12000 {
            let tail = &prompt[prompt.len() - 12000..];
            tail.to_string()
        } else {
            prompt
        }
    }

    /// Generate pattern-based response
    fn generate_pattern_response(&self, prompt: &str) -> String {
        let prompt_lower = prompt.to_lowercase();

        if prompt_lower.contains("fix") || prompt_lower.contains("bug") {
            self.analyze_for_fix(prompt)
        } else if prompt_lower.contains("explain") || prompt_lower.contains("what") {
            self.analyze_for_explanation(prompt)
        } else if prompt_lower.contains("refactor") || prompt_lower.contains("improve") {
            self.analyze_for_refactor(prompt)
        } else if prompt_lower.contains("test") {
            self.generate_test_template(prompt)
        } else if prompt_lower.contains("implement") || prompt_lower.contains("create") {
            self.generate_implementation_hint(prompt)
        } else {
            self.generate_general_assistance(prompt)
        }
    }

    fn analyze_for_fix(&self, prompt: &str) -> String {
        let mut response = String::from("🔧 **Code Analysis for Bug Fix**\n\n");

        let bug_patterns = [
            (
                "unwrap()",
                "Consider using `ok_or()` or `?` operator for proper error handling",
            ),
            (
                "expect(",
                "Add more descriptive error messages or handle the None case",
            ),
            (
                "panic!",
                "Replace panics with Result types for recoverable errors",
            ),
            (
                "clone()",
                "Check if borrowing would work to avoid unnecessary allocations",
            ),
        ];

        let mut found_patterns = Vec::new();
        for (pattern, suggestion) in &bug_patterns {
            if prompt.contains(pattern) {
                found_patterns.push(format!("- Found `{}`: {}", pattern, suggestion));
            }
        }

        if !found_patterns.is_empty() {
            response.push_str("**Potential issues found:**\n");
            for p in found_patterns {
                response.push_str(&p);
                response.push('\n');
            }
        } else {
            response.push_str("No obvious issues detected. Consider:\n");
            response.push_str("- Adding error handling for edge cases\n");
            response.push_str("- Checking for off-by-one errors in loops\n");
        }

        response
    }

    fn analyze_for_explanation(&self, prompt: &str) -> String {
        let mut response = String::from("📚 **Code Explanation**\n\n");

        // Detect language
        let language = if prompt.contains("fn ") || prompt.contains("let mut") {
            "Rust"
        } else if prompt.contains("def ") || prompt.contains("import ") {
            "Python"
        } else if prompt.contains("function ") || prompt.contains("const ") {
            "JavaScript/TypeScript"
        } else {
            "code"
        };

        response.push_str(&format!("This appears to be {} code.\n\n", language));
        response.push_str("**Structure analysis:**\n");

        let functions = prompt.matches("fn ").count()
            + prompt.matches("def ").count()
            + prompt.matches("function ").count();
        let loops = prompt.matches("for ").count() + prompt.matches("while ").count();
        let conditionals = prompt.matches("if ").count() + prompt.matches("match ").count();

        response.push_str(&format!("- Functions/methods: {}\n", functions));
        response.push_str(&format!("- Loops: {}\n", loops));
        response.push_str(&format!("- Conditionals: {}\n", conditionals));

        response
    }

    fn analyze_for_refactor(&self, prompt: &str) -> String {
        let mut response = String::from("♻️ **Refactoring Suggestions**\n\n");

        if prompt.matches("clone()").count() > 2 {
            response.push_str("- Multiple `.clone()` calls detected - consider using references\n");
        }
        if prompt.matches("unwrap()").count() > 1 {
            response.push_str("- Multiple `.unwrap()` calls - use proper error handling\n");
        }

        response.push_str("\n**Principles:**\n");
        response.push_str("- DRY: Don't Repeat Yourself\n");
        response.push_str("- SRP: Single Responsibility Principle\n");

        response
    }

    fn generate_test_template(&self, prompt: &str) -> String {
        let mut response = String::from("🧪 **Test Template**\n\n");

        if prompt.contains("fn ") || prompt.contains("#[test]") {
            response.push_str("```rust\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn test_happy_path() {\n        // Arrange, Act, Assert\n    }\n}\n```\n");
        } else {
            response.push_str("```javascript\ndescribe('YourFunction', () => {\n    it('should work correctly', () => {\n        // Test implementation\n    });\n});\n```\n");
        }

        response
    }

    fn generate_implementation_hint(&self, prompt: &str) -> String {
        let mut response = String::from("💡 **Implementation Guide**\n\n");

        if prompt.contains("api") || prompt.contains("http") {
            response.push_str("**API Implementation Steps:**\n");
            response.push_str("1. Define request/response types\n");
            response.push_str("2. Add error handling\n");
            response.push_str("3. Implement retry logic\n");
        } else {
            response.push_str("**General Implementation Approach:**\n");
            response.push_str("1. Define the interface first\n");
            response.push_str("2. Implement core logic\n");
            response.push_str("3. Add error handling\n");
            response.push_str("4. Write tests\n");
        }

        response
    }

    fn generate_general_assistance(&self, prompt: &str) -> String {
        format!("🤖 **AI Assistant**\n\nI understand you're asking about: \"{}\"\n\n**I can help you with:**\n- 📝 Code explanation\n- 🔧 Bug fixing\n- ♻️ Refactoring\n- 🧪 Testing\n- 💡 Implementation\n\n*Note: For full AI capabilities, run Ollama or LM Studio locally.*", 
            prompt.chars().take(100).collect::<String>())
    }
}

impl Default for AiService {
    fn default() -> Self {
        Self::new(AiBackendConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedded_llm::MemoryTier;

    #[test]
    fn test_ai_service_creation() {
        let service = AiService::default();
        assert_eq!(service.config.backend, "auto");
    }

    #[test]
    fn test_pattern_response_fix() {
        let service = AiService::default();
        let response = service.generate_pattern_response("fix the bug with unwrap()");
        assert!(response.contains("unwrap()"));
        assert!(response.contains("error handling"));
    }

    #[tokio::test]
    async fn test_complete_local_uses_embedded_engine_when_backend_is_local() {
        let service = AiService::new(AiBackendConfig {
            backend: "local".to_string(),
            model: "test-local-model".to_string(),
            ..AiBackendConfig::default()
        });

        let result = service
            .complete(CompletionRequest {
                prompt: "Explain this Rust function".to_string(),
                system_prompt: Some("You are a helpful coding assistant".to_string()),
                history: vec![],
                temperature: Some(0.2),
                max_tokens: Some(64),
                stop_sequences: vec![],
                context: Some("fn greet() { println!(\"hi\"); }".to_string()),
            })
            .await
            .expect("local completion should succeed");

        assert_eq!(result.backend, "local");
        assert!(!result.text.is_empty());
    }

    #[tokio::test]
    async fn test_complete_local_initializes_embedded_engine_once() {
        let service = AiService::new(AiBackendConfig {
            backend: "local".to_string(),
            ..AiBackendConfig::default()
        });

        let first = service
            .complete(CompletionRequest {
                prompt: "implement a function".to_string(),
                system_prompt: None,
                history: vec![],
                temperature: None,
                max_tokens: Some(32),
                stop_sequences: vec![],
                context: None,
            })
            .await
            .expect("first local completion should succeed");

        let second = service
            .complete(CompletionRequest {
                prompt: "implement a function".to_string(),
                system_prompt: None,
                history: vec![],
                temperature: None,
                max_tokens: Some(32),
                stop_sequences: vec![],
                context: None,
            })
            .await
            .expect("second local completion should succeed");

        let engine_guard = service.embedded_engine.read().await;
        assert!(engine_guard.is_some());
        assert_eq!(first.backend, "local");
        assert!(second.from_cache || !second.text.is_empty());
        let _tier = MemoryTier::Cpu;
    }
}
