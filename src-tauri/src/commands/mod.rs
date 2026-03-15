//! Tauri Commands Module
//!
//! All command handlers exposed to the frontend via Tauri's invoke system.

// ============ File Operations ============
pub mod fs;

// ============ Terminal Operations ============
pub mod terminal;

// ============ AI Operations ============
pub mod ai;

// ============ Git Operations ============
pub mod git;
pub mod review;

// ============ LSP Operations ============
pub mod lsp;
pub mod lsp_real;

// ============ Embedded LLM Operations ============
pub mod embedded_llm;

// ============ Authentication Operations ============
pub mod auth;

// ============ Collaboration Operations ============
pub mod collaboration;

// ============ E2E Encryption Operations ============
pub mod e2ee;

// ============ VS Code Compatibility Operations ============
pub mod vscode_compat;

// ============ MCP/Agent Operations ============
pub mod mcp;

// ============ Swarm AI Operations ============
pub mod swarm;

// ============ Plugin Operations ============
pub mod plugin;

// ============ Update Operations ============
pub mod update;

// ============ RAG Operations ============
pub mod rag;

// ============ WebSocket Operations ============
pub mod websocket;

// ============ Git CRDT Operations ============
pub mod gitcrdt;

// ============ Chat Agent Operations ============
pub mod chat_agent;

// ============ Extensions & Marketplace ============
pub mod agent_store;
pub mod extensions;
pub mod marketplace;

// ============ AirLLM Operations ============
pub mod airllm;

// ============ PicoClaw Operations ============
pub mod picoclaw;

// ============ AoT Reasoning Operations ============
pub mod aot;

// ============ Orchestrator (Mission Control) ============
pub mod orchestrator;

// ============ Feedback / Learning Flywheel ============
pub mod feedback;

// ============ Search Operations ============
pub mod search;

// ============ Debug Operations ============
pub mod debug;

// ============ Settings Persistence ============
pub mod settings;

// ============ Project Config ============
pub mod project_config;

// ============ Model Download ============
pub mod model_download;

// ============ Test Runner ============
pub mod testing;

// ============ RepoWiki Operations ============
pub mod repowiki;

// ============ Autonomous Execution ============
pub mod autonomous;

// ============ Remote Dev Environments ============
pub mod remote;
