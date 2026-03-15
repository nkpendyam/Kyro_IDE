//! KRO_IDE - GPU-Accelerated AI-Native Code Editor
//!
//! A zero-dependency, privacy-first IDE with embedded LLM, MCP agent swarm,
//! and real-time collaboration capabilities.
//!
//! ## Architecture
//! - **UI Layer**: Tauri v2 (WebView Shell) + GPUI Canvas (Rust)
//! - **AI Engine**: Embedded llama.cpp with Metal/CUDA/Vulkan backends
//! - **Agent Framework**: MCP (Model Context Protocol) with FastMCP patterns
//! - **Collaboration**: Yjs CRDT with Git persistence
//! - **Plugin System**: WASM sandbox with capability-based security
//!
//! ## Memory Model (8GB VRAM Target)
//! - Model weights (Q4_K_M): ~4.5GB
//! - KV cache (8K context): ~2GB
//! - System overhead: ~1GB
//! - Total: ~7.5GB (safe headroom)
//!
//! ## Zero-Dependency Promise
//! - No Ollama installation required
//! - No VC++ redistributables (Windows)
//! - No Python/Node.js runtime
//! - Single static binary per platform

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]

// ============ Core Modules ============
mod ai;
mod commands;
mod files;
mod git;
mod lsp;
mod terminal;

// ============ AI Modules ============
mod embedded_llm;
mod mcp;
mod rag;
mod swarm_ai;

// ============ Collaboration Modules ============
mod git_crdt;

// ============ Platform Modules ============
mod telegram;

// ============ Verification Modules ============
// symbolic_verify - removed (incomplete)

// ============ Agent System ============
mod agents;

// ============ Infrastructure Modules ============
mod accessibility;
mod benchmark;
mod plugin_sandbox;
mod telemetry;
mod update;

// ============ VS Code Compatibility ============
mod vscode_compat;

// ============ Tower-LSP Integration ============
mod lsp_tower;

// ============ LSP Transport (Real Implementation) ============
mod lsp_transport;

// ============ Collaboration (CRDT-based) ============
mod collab;

// ============ Debug Adapter Protocol ============
mod debug;

// ============ AI Inference (based on Candle) ============
mod inference;

// ============ Text Buffer (based on Ropey) ============
mod buffer;

// ============ Authentication (JWT + OAuth) ============
mod auth;

// ============ End-to-End Encryption (Signal Protocol) ============
mod e2ee;

// ============ P2P Collaboration (Phase 5) ============
mod p2p;

// ============ AirLLM Integration (Layer-wise Inference) ============
mod airllm;

// ============ PicoClaw Integration (Ultra-lightweight AI) ============
mod picoclaw;

// ============ Atoms of Thought (AoT Reasoning) ============
mod aot;

// ============ Orchestrator (Mission Control) ============
mod orchestrator;

// ============ Learning Flywheel (Feedback DB) ============
mod feedback;

// ============ RepoWiki (Auto-Documentation) ============
mod repowiki;

// ============ Extension System (Open VSX) ============
mod extensions;

// ============ Trust Layer (Critical) ============
mod trust;

// ============ Hierarchical Memory ============
mod memory;

// ============ Quality Control ============
mod quality;

// ============ Business Model ============
mod business;

// ============ Autonomous Agent ============
mod autonomous;

// ============ Chat Sidebar with RAG ============
mod chat_sidebar;

// ============ MCP Agent Editor ============
mod agent_editor;

// ============ Agent Store ============
mod agent_store;

use parking_lot::RwLock;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::{Mutex, RwLock as AsyncRwLock};

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write;
            writeln!(
                buf,
                "[{} {}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();

    log::info!("=================================");
    log::info!("  KRO_IDE v{} Starting", env!("CARGO_PKG_VERSION"));
    log::info!("=================================");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .expect("Failed to get main webview window");

            // ============ Hardware Detection ============
            log::info!("Detecting hardware capabilities...");
            let hardware_caps = detect_hardware_capabilities();
            log::info!("GPU: {:?}", hardware_caps.gpu_name);
            log::info!(
                "VRAM: {} GB",
                hardware_caps.vram_bytes / (1024 * 1024 * 1024)
            );
            log::info!("Memory Tier: {:?}", hardware_caps.recommended_tier);
            log::info!("Backend: {}", hardware_caps.recommended_backend);

            // ============ Initialize Core Services ============

            // Terminal manager
            let terminal_manager = terminal::TerminalManager::new();
            app.manage(Arc::new(Mutex::new(terminal_manager)));

            // AI client (Ollama fallback)
            let ai_client = ai::AiClient::new();
            let ai_client_arc = Arc::new(Mutex::new(ai_client));
            app.manage(ai_client_arc.clone());

            // File watcher
            let file_watcher = files::FileWatcher::new(window.clone()).unwrap_or_else(|e| {
                eprintln!("Failed to create file watcher: {}", e);
                panic!("File watcher initialization failed");
            });
            app.manage(Arc::new(Mutex::new(file_watcher)));

            // Git manager
            let git_manager = git::GitManager::new();
            app.manage(Arc::new(Mutex::new(git_manager)));

            // ============ Initialize LSP System ============

            let molecular_lsp = lsp::MolecularLsp::new();
            let molecular_lsp_arc = Arc::new(RwLock::new(molecular_lsp));
            app.manage(Arc::new(Mutex::new(lsp::MolecularLsp::new())));

            // AI completion engine
            let completion_engine =
                lsp::completion_engine::AiCompletionEngine::new(molecular_lsp_arc);
            app.manage(Arc::new(Mutex::new(completion_engine)));

            // ============ Initialize Swarm AI Engine ============

            let app_handle = app.handle().clone();
            let hardware_caps_clone = hardware_caps.clone();
            tauri::async_runtime::spawn(async move {
                let config = swarm_ai::SwarmConfig {
                    max_memory_gb: hardware_caps_clone.vram_bytes as f32
                        / (1024.0 * 1024.0 * 1024.0),
                    ..Default::default()
                };

                if let Ok(engine) = swarm_ai::SwarmAIEngine::new(config).await {
                    app_handle.manage(Arc::new(AsyncRwLock::new(engine)));
                    log::info!("✓ Swarm AI engine initialized");
                } else {
                    log::warn!("⚠ Swarm AI engine initialization failed, using fallback");
                }
            });

            // ============ Initialize Embedded LLM ============

            // Initialize embedded LLM state (for commands to access)
            let embedded_llm_state = commands::embedded_llm::EmbeddedLLMState {
                engine: None,
                hardware: hardware_caps.clone(),
            };
            app.manage(Arc::new(AsyncRwLock::new(embedded_llm_state)));
            log::info!("✓ Embedded LLM state initialized");

            #[cfg(feature = "embedded-llm")]
            {
                let app_handle = app.handle().clone();
                let hardware_caps = hardware_caps.clone();
                tauri::async_runtime::spawn(async move {
                    let config = embedded_llm::EmbeddedLLMConfig {
                        max_vram_mb: (hardware_caps.vram_bytes / (1024 * 1024)) as u64 * 80 / 100, // 80% of VRAM
                        context_size: hardware_caps.recommended_tier.recommended_context_size(),
                        n_gpu_layers: hardware_caps.recommended_tier.gpu_layers(),
                        default_model: hardware_caps
                            .recommended_tier
                            .recommended_models()
                            .first()
                            .unwrap_or(&"phi-2b-q4_k_m")
                            .to_string(),
                        ..Default::default()
                    };

                    match embedded_llm::EmbeddedLLMEngine::new(config).await {
                        Ok(engine) => {
                            app_handle.manage(Arc::new(AsyncRwLock::new(engine)));
                            log::info!("✓ Embedded LLM engine initialized");
                        }
                        Err(e) => {
                            log::warn!("⚠ Embedded LLM initialization failed: {}", e);
                        }
                    }
                });
            }

            // ============ Initialize MCP Server ============

            let mcp_server = mcp::MCPServer::new(mcp::MCPConfig::default());
            app.manage(Arc::new(AsyncRwLock::new(mcp_server)));
            log::info!("✓ MCP server initialized");

            // ============ Virtual PICO Bridge ============
            // virtual_pico module removed - incomplete feature

            // ============ Initialize Collaboration ============

            let collab_config = git_crdt::CollaborationConfig::default();
            let collab_manager = git_crdt::CollaborationManager::new(collab_config);
            app.manage(Arc::new(Mutex::new(collab_manager)));
            log::info!("✓ Collaboration manager initialized");

            // ============ Verification ============
            // symbolic_verify module removed - incomplete feature

            // ============ Initialize Plugin Manager ============

            let plugins_dir = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("kro_ide")
                .join("plugins");

            let plugin_manager = plugin_sandbox::PluginManager::new(plugins_dir);
            app.manage(Arc::new(Mutex::new(plugin_manager)));
            log::info!("✓ Plugin manager initialized");

            // ============ Initialize Update Manager ============

            let update_config = update::UpdateConfig::default();
            if let Ok(update_manager) = update::UpdateManager::new(update_config) {
                app.manage(Arc::new(AsyncRwLock::new(update_manager)));
                log::info!("✓ Update manager initialized");
            }

            // ============ Initialize Telemetry ============

            let telemetry = telemetry::TelemetryManager::new(telemetry::TelemetryConfig::default());
            app.manage(Arc::new(Mutex::new(telemetry)));
            log::info!("✓ Telemetry initialized");

            // ============ Initialize RAG State ============

            let rag_state = commands::rag::RagState::default();
            app.manage(Arc::new(AsyncRwLock::new(rag_state)));
            log::info!("✓ RAG state initialized");

            // ============ Initialize WebSocket State ============

            let ws_state = commands::websocket::WsState::default();
            app.manage(Arc::new(AsyncRwLock::new(ws_state)));
            log::info!("✓ WebSocket state initialized");

            // ============ Initialize Git CRDT State ============

            let git_crdt_state = commands::gitcrdt::GitCrdtState::default();
            app.manage(Arc::new(AsyncRwLock::new(git_crdt_state)));
            log::info!("✓ Git CRDT state initialized");

            // ============ Initialize Enhanced LSP State ============

            let lsp_state = commands::lsp_real::LspState::default();
            app.manage(Arc::new(AsyncRwLock::new(lsp_state)));
            log::info!("✓ Enhanced LSP state initialized");

            // ============ Initialize AirLLM State ============
            let airllm_state = commands::airllm::AirLLMState(tokio::sync::Mutex::new(None));
            app.manage(airllm_state);
            log::info!("✓ AirLLM state initialized");

            // ============ Initialize PicoClaw State ============
            let picoclaw_engine =
                picoclaw::PicoClawEngine::new(picoclaw::PicoClawConfig::default());
            let picoclaw_state =
                commands::picoclaw::PicoClawState(std::sync::Mutex::new(picoclaw_engine));
            app.manage(picoclaw_state);
            log::info!("✓ PicoClaw state initialized");

            // ============ Initialize Orchestrator (Mission Control) ============
            let orchestrator =
                orchestrator::KyroOrchestrator::new(orchestrator::OrchestratorConfig::default());
            app.manage(commands::orchestrator::OrchestratorState(Arc::new(
                tokio::sync::RwLock::new(orchestrator),
            )));
            log::info!("✓ Orchestrator initialized");

            // ============ Initialize AoT Reasoning Engine ============
            let aot_reasoner = aot::AotReasoner::new(aot::AotConfig::default());
            let aot_state = commands::aot::AotState(std::sync::Mutex::new(aot_reasoner));
            app.manage(aot_state);
            log::info!("✓ Atoms-of-Thought engine initialized");

            // ============ Initialize Extension Store ============
            let ext_store_path = dirs::data_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("kro_ide")
                .join("extensions");
            let ext_manager = extensions::ExtensionManager::new(ext_store_path);
            app.manage(commands::extensions::ExtensionState(
                tokio::sync::Mutex::new(ext_manager),
            ));
            log::info!("✓ Extension store initialized");

            // ============ Initialize Agent Store ============
            let agent_store = agent_store::AgentStore::new();
            app.manage(commands::agent_store::AgentStoreState(
                tokio::sync::Mutex::new(agent_store),
            ));
            log::info!("✓ Agent store initialized");

            // ============ Initialize GitHub Marketplace ============
            let marketplace = extensions::GitHubMarketplace::new();
            app.manage(commands::marketplace::MarketplaceState(
                tokio::sync::Mutex::new(marketplace),
            ));
            log::info!("✓ GitHub marketplace initialized");

            // ============ Initialize Debug State ============
            let debug_state = commands::debug::DebugState::default();
            app.manage(Arc::new(Mutex::new(debug_state)));
            log::info!("✓ Debug state initialized");

            // ============ Initialize Remote Dev Environment State ============
            app.manage(commands::remote::RemoteState::default());
            log::info!("✓ Remote dev environment state initialized");

            // ============ Startup Complete ============

            log::info!("=================================");
            log::info!("  KRO_IDE Ready");
            log::info!("  - Languages: 25+ (Tree-sitter)");
            log::info!("  - AI Agents: 8 specialized");
            log::info!("  - Memory Tier: {:?}", hardware_caps.recommended_tier);
            log::info!("=================================");

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ============ File Operations ============
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::list_directory,
            commands::fs::create_file,
            commands::fs::create_directory,
            commands::fs::delete_file,
            commands::fs::delete_directory,
            commands::fs::rename_file,
            commands::fs::get_file_tree,
            commands::fs::is_first_run_complete,
            commands::fs::save_first_run_complete,
            commands::fs::fs_list_supported_languages,
            commands::fs::get_file_metadata,
            commands::fs::path_exists,
            commands::fs::is_directory,
            commands::fs::is_file,
            commands::fs::watch_directory,
            commands::fs::unwatch_directory,
            // ============ Terminal Operations ============
            commands::terminal::create_terminal,
            commands::terminal::write_to_terminal,
            commands::terminal::resize_terminal,
            commands::terminal::kill_terminal,
            // ============ AI Operations ============
            commands::ai::chat_completion,
            commands::ai::code_completion,
            commands::ai::code_review,
            commands::ai::generate_tests,
            commands::ai::explain_code,
            commands::ai::refactor_code,
            commands::ai::fix_code,
            commands::ai::check_ollama_status,
            commands::ai::list_models,
            commands::ai::ai_code_completion,
            commands::ai::ai_stream_completion,
            commands::ai::ai_inline_chat,
            // ============ Git Operations ============
            commands::git::git_status,
            commands::git::git_commit,
            commands::git::git_diff,
            commands::git::git_diff_file,
            commands::git::git_log,
            commands::git::git_branch,
            commands::git::git_blame,
            commands::git::git_stash,
            commands::git::git_stash_pop,
            commands::git::git_stash_list,
            commands::git::git_merge,
            commands::review::review_diff,
            commands::review::review_overview,
            git::git_stage,
            git::git_unstage,
            git::git_stage_all,
            git::git_unstage_all,
            git::git_discard,
            git::git_stage_hunk,
            // ============ LSP Operations ============
            commands::lsp::detect_language,
            commands::lsp::extract_symbols,
            commands::lsp::extract_imports,
            commands::lsp::get_completions,
            commands::lsp::get_diagnostics,
            commands::lsp::lsp_list_supported_languages,
            commands::lsp::get_ai_completions,
            commands::lsp::update_file_symbols,
            commands::lsp::get_completion_stats,
            commands::lsp::get_completion_budget,
            // ============ Embedded LLM Operations ============
            commands::embedded_llm::get_hardware_info,
            commands::embedded_llm::init_embedded_llm,
            commands::embedded_llm::load_model,
            commands::embedded_llm::unload_model,
            commands::embedded_llm::list_local_models,
            commands::embedded_llm::embedded_complete,
            commands::embedded_llm::embedded_chat,
            commands::embedded_llm::embedded_code_complete,
            commands::embedded_llm::is_embedded_llm_ready,
            commands::embedded_llm::get_loaded_models,
            // ============ Authentication Operations ============
            commands::auth::login_user,
            commands::auth::logout_user,
            commands::auth::register_user,
            commands::auth::get_current_user,
            commands::auth::is_authenticated,
            commands::auth::update_user_role,
            commands::auth::validate_session,
            commands::auth::get_oauth_url,
            commands::auth::handle_oauth_callback,
            // ============ Collaboration Operations ============
            commands::collaboration::create_room,
            commands::collaboration::join_room,
            commands::collaboration::leave_room,
            commands::collaboration::get_room_users,
            commands::collaboration::update_presence,
            commands::collaboration::get_room_presence,
            commands::collaboration::get_room_cursors,
            commands::collaboration::send_operation,
            commands::collaboration::send_chat_message,
            commands::collaboration::get_collab_stats,
            commands::collaboration::is_connected_to_room,
            commands::collaboration::get_current_room,
            commands::collaboration::list_rooms,
            collab::broadcast_cursor,
            // ============ E2E Encryption Operations ============
            commands::e2ee::generate_key_pair,
            commands::e2ee::get_public_key,
            commands::e2ee::create_key_bundle,
            commands::e2ee::init_encrypted_channel,
            commands::e2ee::encrypt_message,
            commands::e2ee::decrypt_message,
            commands::e2ee::has_e2ee_session,
            commands::e2ee::has_encrypted_channel,
            commands::e2ee::rotate_keys,
            commands::e2ee::get_prekey_count,
            commands::e2ee::delete_e2ee_session,
            // ============ VS Code Compatibility Operations ============
            commands::vscode_compat::search_extensions,
            commands::vscode_compat::get_extension_details,
            commands::vscode_compat::install_extension,
            commands::vscode_compat::uninstall_extension,
            commands::vscode_compat::enable_extension,
            commands::vscode_compat::disable_extension,
            commands::vscode_compat::list_installed_extensions,
            commands::vscode_compat::get_extension_status,
            commands::vscode_compat::reload_extensions,
            commands::vscode_compat::get_extension_recommendations,
            commands::vscode_compat::get_popular_extensions,
            commands::vscode_compat::search_extensions_unified,
            commands::vscode_compat::install_extension_unified,
            commands::vscode_compat::get_openvsx_popular,
            commands::vscode_compat::get_extension_readme,
            // ============ MCP/Agent Operations ============
            commands::mcp::list_agents,
            commands::mcp::create_agent,
            commands::mcp::run_agent,
            commands::mcp::get_agent_status,
            commands::mcp::delete_agent,
            commands::mcp::list_mcp_tools,
            commands::mcp::execute_tool,
            commands::mcp::list_mcp_resources,
            commands::mcp::read_mcp_resource,
            commands::mcp::register_tool,
            commands::mcp::unregister_tool,
            // ============ Swarm AI Operations ============
            commands::swarm::list_swarm_agents,
            commands::swarm::create_swarm_agent,
            commands::swarm::submit_swarm_task,
            commands::swarm::execute_swarm_task,
            commands::swarm::get_swarm_task_status,
            commands::swarm::list_swarm_tasks,
            commands::swarm::cancel_swarm_task,
            commands::swarm::get_swarm_stats,
            commands::swarm::delete_swarm_agent,
            commands::swarm::send_agent_message,
            // ============ Multi-Model Router ============
            commands::swarm::router_route,
            commands::swarm::router_register_endpoint,
            commands::swarm::router_unregister_endpoint,
            commands::swarm::router_list_endpoints,
            commands::swarm::router_refresh_health,
            commands::swarm::router_get_config,
            commands::swarm::router_set_config,
            // ============ Plugin Operations ============
            commands::plugin::list_plugins,
            commands::plugin::install_plugin,
            commands::plugin::uninstall_plugin,
            commands::plugin::enable_plugin,
            commands::plugin::disable_plugin,
            commands::plugin::execute_plugin_function,
            commands::plugin::get_plugin_capabilities,
            commands::plugin::plugin_has_capability,
            commands::plugin::get_plugin_status,
            commands::plugin::reload_plugins,
            commands::plugin::get_plugin_memory_usage,
            // ============ Update Operations ============
            commands::update::check_for_updates,
            commands::update::download_update,
            commands::update::get_download_progress,
            commands::update::install_update,
            commands::update::cancel_update,
            commands::update::get_update_channel,
            commands::update::set_update_channel,
            commands::update::get_update_history,
            commands::update::set_auto_update,
            commands::update::is_auto_update_enabled,
            commands::update::skip_update,
            commands::update::get_last_update_check,
            // ============ RAG Operations ============
            commands::rag::get_rag_status,
            commands::rag::index_project,
            commands::rag::semantic_search,
            commands::rag::clear_rag_index,
            commands::rag::get_rag_config,
            commands::rag::set_rag_config,
            commands::rag::get_indexed_paths,
            commands::rag::remove_indexed_path,
            commands::rag::graph_enhanced_semantic_search,
            // ============ WebSocket Operations ============
            commands::websocket::ws_connect,
            commands::websocket::ws_disconnect,
            commands::websocket::ws_get_status,
            commands::websocket::ws_join_room,
            commands::websocket::ws_leave_room,
            commands::websocket::ws_send_message,
            commands::websocket::ws_send_presence,
            commands::websocket::ws_send_operation,
            commands::websocket::ws_get_server_url,
            commands::websocket::ws_set_reconnect_handler,
            // ============ Git CRDT Operations ============
            commands::gitcrdt::git_crdt_status,
            commands::gitcrdt::git_crdt_sync,
            commands::gitcrdt::git_crdt_commit,
            commands::gitcrdt::git_crdt_auto_commit,
            commands::gitcrdt::git_crdt_auto_push,
            commands::gitcrdt::git_crdt_resolve_conflict,
            commands::gitcrdt::git_crdt_get_history,
            commands::gitcrdt::git_crdt_create_branch,
            commands::gitcrdt::git_crdt_switch_branch,
            // ============ Enhanced LSP Operations ============
            commands::lsp_real::lsp_start_server,
            commands::lsp_real::lsp_stop_server,
            commands::lsp_real::lsp_get_servers,
            commands::lsp_real::lsp_get_completions,
            commands::lsp_real::lsp_hover,
            commands::lsp_real::lsp_goto_definition,
            commands::lsp_real::lsp_find_references,
            commands::lsp_real::lsp_get_diagnostics,
            commands::lsp_real::lsp_rename,
            commands::lsp_real::lsp_format_document,
            commands::lsp_real::lsp_code_actions,
            // ============ AirLLM Operations ============
            commands::airllm::airllm_check_availability,
            commands::airllm::airllm_get_config,
            commands::airllm::airllm_load_model,
            commands::airllm::airllm_unload_model,
            commands::airllm::airllm_generate,
            commands::airllm::airllm_get_status,
            // ============ PicoClaw Operations ============
            commands::picoclaw::picoclaw_complete,
            commands::picoclaw::picoclaw_analyze,
            commands::picoclaw::picoclaw_memory_usage,
            commands::picoclaw::picoclaw_is_available,
            // ============ Orchestrator (Mission Control) Operations ============
            commands::orchestrator::orchestrator_start_mission,
            commands::orchestrator::orchestrator_get_mission,
            commands::orchestrator::orchestrator_list_missions,
            commands::orchestrator::orchestrator_update_mission_phase,
            commands::orchestrator::orchestrator_get_config,
            commands::orchestrator::quest_start,
            commands::orchestrator::quest_execute,
            commands::orchestrator::quest_get_status,
            // ============ Feedback / Learning Flywheel ============
            commands::feedback::feedback_log_suggestion,
            commands::feedback::feedback_accept,
            commands::feedback::feedback_reject,
            commands::feedback::feedback_correct,
            commands::feedback::feedback_stats,
            commands::feedback::feedback_recent,
            // ============ AoT Reasoning Operations ============
            commands::aot::aot_decompose,
            commands::aot::aot_optimize_context,
            commands::aot::aot_get_stats,
            commands::aot::aot_is_available,
            // ============ Extension Store Operations ============
            commands::extensions::search_extensions_registry,
            commands::extensions::install_extension_registry,
            commands::extensions::uninstall_extension_registry,
            commands::extensions::list_extensions,
            commands::extensions::toggle_extension,
            // ============ Agent Store Operations ============
            commands::agent_store::search_agents,
            commands::agent_store::install_agent,
            commands::agent_store::list_installed_agents,
            commands::agent_store::uninstall_agent,
            commands::agent_store::toggle_agent,
            commands::agent_store::execute_agent,
            commands::agent_store::featured_agents,
            // ============ GitHub Marketplace Operations ============
            commands::marketplace::search_marketplace,
            commands::marketplace::get_github_extension_details,
            commands::marketplace::get_extension_versions,
            commands::marketplace::install_from_github,
            commands::marketplace::get_featured_extensions,
            commands::marketplace::get_trending_extensions,
            // ============ Chat Agent Operations ============
            commands::ai::detect_ai_backends,
            commands::ai::smart_ai_completion,
            commands::ai::ai_inline_edit,
            commands::ai::create_chat_session,
            commands::ai::rag_chat,
            commands::ai::agent_command,
            commands::ai::agent_approve,
            commands::ai::agent_reject,
            commands::chat_agent::chat_agent_detect_backends,
            commands::chat_agent::chat_agent_smart_completion,
            commands::chat_agent::chat_agent_inline_edit,
            commands::chat_agent::chat_agent_create_session,
            commands::chat_agent::chat_agent_rag_chat,
            commands::chat_agent::chat_agent_run_command,
            commands::chat_agent::chat_agent_approve,
            commands::chat_agent::chat_agent_reject,
            // ============ Project Search Operations ============
            commands::search::search_in_project,
            commands::search::replace_in_project,
            // ============ Debug Operations ============
            commands::debug::debug_start,
            commands::debug::debug_stop,
            commands::debug::debug_continue,
            commands::debug::debug_pause,
            commands::debug::debug_step_over,
            commands::debug::debug_step_into,
            commands::debug::debug_step_out,
            commands::debug::debug_add_breakpoint,
            commands::debug::debug_remove_breakpoint,
            commands::debug::debug_set_breakpoint_condition,
            commands::debug::debug_evaluate,
            // ============ Settings Persistence ============
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::save_settings,
            commands::settings::reset_settings,
            commands::settings::export_settings,
            commands::settings::import_settings,
            commands::settings::is_first_run,
            // ============ Project Config ============
            commands::project_config::init_project_config,
            commands::project_config::get_project_config,
            commands::project_config::set_project_config,
            // ============ Model Download ============
            commands::model_download::list_available_models,
            commands::model_download::download_model,
            commands::model_download::delete_model,
            commands::model_download::get_download_status,
            // ============ Test Runner ============
            commands::testing::detect_test_framework,
            commands::testing::run_tests,
            // ============ RepoWiki Operations ============
            commands::repowiki::repowiki_init,
            commands::repowiki::repowiki_generate,
            commands::repowiki::repowiki_status,
            commands::repowiki::repowiki_get_config,
            commands::repowiki::repowiki_set_config,
            commands::repowiki::repowiki_start_sync,
            commands::repowiki::repowiki_stop_sync,
            commands::repowiki::repowiki_clean,
            commands::repowiki::repowiki_get_graph,
            commands::repowiki::repowiki_get_mermaid,
            commands::repowiki::repowiki_graph_stats,
            commands::repowiki::repowiki_list_pages,
            // ============ Autonomous Execution ============
            commands::autonomous::execute_step,
            commands::autonomous::execute_plan,
            commands::autonomous::plan_task,
            commands::autonomous::autonomous_status,
            // ============ Remote Dev Environments ============
            commands::remote::remote_connect,
            commands::remote::remote_disconnect,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Detect hardware capabilities at startup
fn detect_hardware_capabilities() -> embedded_llm::HardwareCapabilities {
    let cpu_cores = num_cpus::get();

    // Detect CPU features
    let mut cpu_features = vec![];
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") {
            cpu_features.push("avx2".to_string());
        }
        if is_x86_feature_detected!("avx512f") {
            cpu_features.push("avx512".to_string());
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        cpu_features.push("neon".to_string());
    }

    // Get system memory
    let mut sys = sysinfo::System::new_all();
    sys.refresh_memory();
    let ram_bytes = sys.total_memory();

    // Detect GPU and VRAM
    let (gpu_name, vram_bytes, recommended_backend, recommended_tier) = detect_gpu();

    embedded_llm::HardwareCapabilities {
        vram_bytes,
        ram_bytes,
        gpu_name,
        gpu_compute_capability: None,
        recommended_backend,
        recommended_tier,
        cpu_cores,
        cpu_features,
    }
}

/// Detect GPU capabilities
fn detect_gpu() -> (Option<String>, u64, String, embedded_llm::MemoryTier) {
    // Try CUDA first
    #[cfg(feature = "cuda")]
    {
        use std::process::Command;

        if let Ok(output) = Command::new("nvidia-smi")
            .args([
                "--query-gpu=name,memory.total",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().next() {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    if parts.len() >= 2 {
                        if let Ok(mem_mb) = parts[1].parse::<u64>() {
                            let vram_bytes = mem_mb * 1024 * 1024;
                            let tier = embedded_llm::MemoryTier::from_vram(vram_bytes);
                            return (
                                Some(parts[0].to_string()),
                                vram_bytes,
                                "cuda".to_string(),
                                tier,
                            );
                        }
                    }
                }
            }
        }
    }

    // Try Metal on macOS
    #[cfg(target_os = "macos")]
    {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        let ram = sys.total_memory();
        let usable_vram = (ram as f64 * 0.75) as u64; // Metal gives ~75% of unified memory
        let tier = embedded_llm::MemoryTier::from_vram(usable_vram);
        return (
            Some("Apple Silicon".to_string()),
            usable_vram,
            "metal".to_string(),
            tier,
        );
    }

    #[cfg(target_os = "windows")]
    {
        if let Some((gpu_name, vram_bytes)) = detect_windows_gpu_info() {
            return (
                Some(gpu_name),
                vram_bytes,
                "cpu".to_string(),
                embedded_llm::MemoryTier::Cpu,
            );
        }
    }

    // Fallback to CPU
    let usable = 0;
    (
        None,
        usable,
        "cpu".to_string(),
        embedded_llm::MemoryTier::Cpu,
    )
}

#[cfg(target_os = "windows")]
fn detect_windows_gpu_info() -> Option<(String, u64)> {
    use std::process::Command;

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Get-CimInstance Win32_VideoController | ForEach-Object { \"$($_.Name)|$($_.AdapterRAM)\" }",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut best: Option<(String, u64)> = None;

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.splitn(2, '|');
        let name = parts.next()?.trim();
        let ram_str = parts.next().unwrap_or("0").trim();
        let adapter_ram = ram_str.parse::<u64>().unwrap_or(0);

        if adapter_ram > 0 {
            match &best {
                Some((_, current_best)) if *current_best >= adapter_ram => {}
                _ => best = Some((name.to_string(), adapter_ram)),
            }
        }
    }

    best
}
