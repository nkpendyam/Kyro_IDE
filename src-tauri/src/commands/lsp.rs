//! LSP commands for KYRO IDE with AI-powered completion

use crate::lsp::completion_engine::{
    AiCompletionEngine, CompletionContext, CompletionStats, CompletionTriggerKind,
    PERFORMANCE_BUDGET_MS,
};
use crate::lsp::{CompletionItem, Diagnostic, Import, MolecularLsp, Symbol};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, State};
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolsResponse {
    pub symbols: Vec<Symbol>,
    pub language: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportsResponse {
    pub imports: Vec<Import>,
    pub language: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionsResponse {
    pub completions: Vec<CompletionItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiagnosticsResponse {
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnhancedCompletionsResponse {
    pub items: Vec<ScoredCompletionItem>,
    pub total_latency_ms: u64,
    pub sources_used: Vec<String>,
    pub performance_warning: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScoredCompletionItem {
    pub label: String,
    pub kind: String,
    pub detail: Option<String>,
    pub documentation: Option<String>,
    pub insert_text: Option<String>,
    pub score: f32,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub file_path: String,
    pub language: String,
    pub code: String,
    pub line: usize,
    pub column: usize,
    pub trigger_kind: String, // "invoked", "trigger_character", "incomplete"
    pub prefix: String,
}

#[command]
pub async fn detect_language(
    lsp: State<'_, Arc<Mutex<MolecularLsp>>>,
    path: String,
) -> Result<String, String> {
    let lsp = lsp.lock().await;
    Ok(lsp.detect_language(&path))
}

#[command]
pub async fn extract_symbols(
    lsp: State<'_, Arc<Mutex<MolecularLsp>>>,
    language: String,
    code: String,
) -> Result<SymbolsResponse, String> {
    let lsp = lsp.lock().await;
    let symbols = lsp.extract_symbols(&language, &code);
    Ok(SymbolsResponse { symbols, language })
}

#[command]
pub async fn extract_imports(
    lsp: State<'_, Arc<Mutex<MolecularLsp>>>,
    language: String,
    code: String,
) -> Result<ImportsResponse, String> {
    let lsp = lsp.lock().await;
    let imports = lsp.extract_imports(&language, &code);
    Ok(ImportsResponse { imports, language })
}

#[command]
pub async fn get_completions(
    lsp: State<'_, Arc<Mutex<MolecularLsp>>>,
    language: String,
    code: String,
    line: usize,
    col: usize,
) -> Result<CompletionsResponse, String> {
    let lsp = lsp.lock().await;
    let completions = lsp.get_completions(&language, &code, line, col);
    Ok(CompletionsResponse { completions })
}

#[command]
pub async fn get_diagnostics(
    lsp: State<'_, Arc<Mutex<MolecularLsp>>>,
    language: String,
    code: String,
) -> Result<DiagnosticsResponse, String> {
    let lsp = lsp.lock().await;
    let diagnostics = lsp.get_diagnostics(&language, &code);
    Ok(DiagnosticsResponse { diagnostics })
}

#[command]
pub async fn lsp_list_supported_languages(
    lsp: State<'_, Arc<Mutex<MolecularLsp>>>,
) -> Result<Vec<String>, String> {
    let lsp = lsp.lock().await;
    Ok(lsp.list_languages())
}

/// Enhanced AI-powered completion endpoint
///
/// Flow:
/// 1. User types: fn fib(n: u32) -> u32 {
/// 2. Monaco detects completion request
/// 3. KYRO routes to molecular_lsp.getCompletions
/// 4. Molecular LSP processes in parallel:
///    - Symbol table (1ms): locals in scope
///    - Tree-sitter patterns (5ms): common patterns
///    - WASM molecule (10ms): language-specific logic
///    - AI hints (50ms): neural suggestions
/// 5. Results merged by confidence then recency
/// 6. Returned to Monaco within 100ms budget
#[command]
pub async fn get_ai_completions(
    completion_engine: State<'_, Arc<Mutex<AiCompletionEngine>>>,
    request: CompletionRequest,
) -> Result<EnhancedCompletionsResponse, String> {
    let engine = completion_engine.lock().await;

    let trigger_kind = match request.trigger_kind.as_str() {
        "invoked" => CompletionTriggerKind::Invoked,
        "trigger_character" => CompletionTriggerKind::TriggerCharacter,
        "incomplete" => CompletionTriggerKind::TriggerForIncompleteCompletions,
        _ => CompletionTriggerKind::Invoked,
    };

    let context = CompletionContext {
        file_path: request.file_path,
        language: request.language,
        code: request.code,
        line: request.line,
        column: request.column,
        trigger_kind,
        prefix: request.prefix,
        scope: None,
    };

    let response = engine.get_completions(context).await;

    Ok(EnhancedCompletionsResponse {
        items: response
            .items
            .iter()
            .map(|item| ScoredCompletionItem {
                label: item.item.label.clone(),
                kind: format!("{:?}", item.item.kind),
                detail: item.item.detail.clone(),
                documentation: item.item.documentation.clone(),
                insert_text: item.item.insert_text.clone(),
                score: item.score,
                source: item.source.clone(),
            })
            .collect(),
        total_latency_ms: response.total_latency_ms,
        sources_used: response.sources_used,
        performance_warning: response.performance_warning,
    })
}

/// Update symbol table for a file (call on file save/open)
#[command]
pub async fn update_file_symbols(
    completion_engine: State<'_, Arc<Mutex<AiCompletionEngine>>>,
    file_path: String,
    code: String,
    language: String,
) -> Result<(), String> {
    let engine = completion_engine.lock().await;
    engine.update_symbols(&file_path, &code, &language);
    Ok(())
}

/// Get completion statistics
#[command]
pub async fn get_completion_stats(
    completion_engine: State<'_, Arc<Mutex<AiCompletionEngine>>>,
) -> Result<CompletionStats, String> {
    let engine = completion_engine.lock().await;
    Ok(engine.get_stats())
}

/// Get the performance budget in milliseconds
#[command]
pub fn get_completion_budget() -> u64 {
    PERFORMANCE_BUDGET_MS
}
