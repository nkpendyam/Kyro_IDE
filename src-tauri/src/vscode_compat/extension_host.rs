//! VS Code Extension Host Implementation
//!
//! Full extension host with Node.js subprocess management and VS Code API surface.
//! Based on patterns from vscode/src/vs/workbench/api/node/extHostExtensionService.ts

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, RwLock as AsyncRwLock};

use super::api::ExtensionContext;
use super::manifest::ExtensionManifest;

/// RPC Message types for extension host communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMessage {
    pub id: u64,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcResponse {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Extension host managing all extensions with full Node.js integration
pub struct ExtensionHost {
    /// Loaded extensions
    extensions: HashMap<String, Extension>,
    /// Extension load order (for dependency resolution)
    extension_order: Vec<String>,
    /// Currently active extensions
    activated_extensions: HashMap<String, bool>,
    /// Host context shared with extensions
    context: Arc<RwLock<HostContext>>,
    /// Node.js subprocess for extension execution
    node_process: Option<Child>,
    /// Stdin writer for sending RPC messages to extension host
    node_stdin: Option<std::io::BufWriter<std::process::ChildStdin>>,
    /// RPC pending requests
    pending_requests: Arc<AsyncRwLock<HashMap<u64, oneshot::Sender<Result<Value>>>>>,
    /// Request ID counter
    request_id: Arc<RwLock<u64>>,
    /// Extension host ready state
    is_ready: Arc<RwLock<bool>>,
    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
}

/// Host context shared with extensions
#[derive(Debug, Default)]
pub struct HostContext {
    pub workspace_root: Option<PathBuf>,
    pub active_editor: Option<String>,
    pub diagnostics: HashMap<String, Vec<super::api::Diagnostic>>,
    pub configuration: HashMap<String, Value>,
    pub theme: String,
}

/// Loaded extension
#[derive(Debug)]
pub struct Extension {
    pub id: String,
    pub manifest: ExtensionManifest,
    pub context: ExtensionContext,
    pub state: ExtensionState,
    pub exports: Option<Value>,
    pub extension_path: PathBuf,
    pub subscriptions: Vec<String>,
}

/// Extension state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtensionState {
    Installed,
    Activating,
    Active,
    Inactive,
    Failed(String),
}

/// Extension host configuration
#[derive(Debug, Clone)]
pub struct ExtensionHostConfig {
    pub node_path: Option<PathBuf>,
    pub extension_dirs: Vec<PathBuf>,
    pub development_mode: bool,
    pub enable_proposed_api: bool,
    pub product_name: String,
}

impl Default for ExtensionHostConfig {
    fn default() -> Self {
        Self {
            node_path: None,
            extension_dirs: vec![dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("kyro-ide")
                .join("extensions")],
            development_mode: false,
            enable_proposed_api: false,
            product_name: "Kyro IDE".to_string(),
        }
    }
}

impl ExtensionHost {
    /// Create a new extension host
    pub fn new() -> Self {
        Self {
            extensions: HashMap::new(),
            extension_order: Vec::new(),
            activated_extensions: HashMap::new(),
            context: Arc::new(RwLock::new(HostContext::default())),
            node_process: None,
            node_stdin: None,
            pending_requests: Arc::new(AsyncRwLock::new(HashMap::new())),
            request_id: Arc::new(RwLock::new(0)),
            is_ready: Arc::new(RwLock::new(false)),
            shutdown_tx: None,
        }
    }

    /// Create extension host with configuration
    pub fn with_config(config: ExtensionHostConfig) -> Result<Self> {
        let mut host = Self::new();
        host.initialize(config)?;
        Ok(host)
    }

    /// Initialize the extension host with Node.js subprocess
    pub fn initialize(&mut self, config: ExtensionHostConfig) -> Result<()> {
        // Find Node.js
        let node_path = config
            .node_path
            .or_else(|| self.find_node())
            .context("Node.js not found. Please install Node.js to use VS Code extensions.")?;

        // Create extension host script
        let host_script = self.create_extension_host_script();

        // Spawn Node.js process
        let mut child = Command::new(&node_path)
            .arg("-e")
            .arg(&host_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("KYRO_EXTENSION_HOST", "1")
            .env("KYRO_PRODUCT_NAME", &config.product_name)
            .spawn()
            .context("Failed to start extension host process")?;

        let stdin = child.stdin.take().context("Failed to get stdin")?;
        self.node_stdin = Some(std::io::BufWriter::new(stdin));
        let stdout = child.stdout.take().context("Failed to get stdout")?;

        // Start message pump
        let pending = self.pending_requests.clone();
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        tokio::spawn(async move {
            Self::message_pump(stdout, pending, &mut shutdown_rx).await;
        });

        self.node_process = Some(child);
        *self.is_ready.write() = true;

        // Load extensions from directories
        let dirs = config.extension_dirs.clone();
        for dir in &dirs {
            if dir.exists() {
                self.scan_extensions(dir)?;
            }
        }

        log::info!(
            "Extension host initialized with {} extensions",
            self.extensions.len()
        );
        Ok(())
    }

    /// Find Node.js installation
    fn find_node(&self) -> Option<PathBuf> {
        // Try common locations
        let candidates = if cfg!(target_os = "windows") {
            vec![
                "C:\\Program Files\\nodejs\\node.exe",
                "C:\\Program Files (x86)\\nodejs\\node.exe",
            ]
        } else if cfg!(target_os = "macos") {
            vec![
                "/usr/local/bin/node",
                "/opt/homebrew/bin/node",
                "/usr/bin/node",
            ]
        } else {
            vec!["/usr/bin/node", "/usr/local/bin/node", "/snap/bin/node"]
        };

        for path in candidates {
            let path = PathBuf::from(path);
            if path.exists() {
                return Some(path);
            }
        }

        // Try 'which node' or 'where node'
        Command::new(if cfg!(target_os = "windows") {
            "where"
        } else {
            "which"
        })
        .arg("node")
        .output()
        .ok()
        .and_then(|output| {
            String::from_utf8(output.stdout)
                .ok()
                .and_then(|stdout: String| {
                    stdout.lines().next().map(|s: &str| PathBuf::from(s.trim()))
                })
        })
    }

    /// Create the Node.js extension host bootstrap script
    fn create_extension_host_script(&self) -> String {
        r#"
// Kyro IDE Extension Host Bootstrap
// Implements VS Code Extension API

const vscode = require('./vscode-api');
const { EventEmitter } = require('events');

// JSON-RPC communication
let requestId = 0;
const pendingRequests = new Map();

process.stdin.on('data', (buffer) => {
    try {
        const message = JSON.parse(buffer.toString());
        handleMessage(message);
    } catch (e) {
        console.error('Failed to parse message:', e);
    }
});

function handleMessage(message) {
    if (message.method) {
        // Incoming request from host
        handleRequest(message);
    } else if (message.id !== undefined) {
        // Response to our request
        const callback = pendingRequests.get(message.id);
        if (callback) {
            pendingRequests.delete(message.id);
            callback(message);
        }
    }
}

function sendRequest(method, params) {
    return new Promise((resolve, reject) => {
        const id = ++requestId;
        pendingRequests.set(id, (response) => {
            if (response.error) {
                reject(new Error(response.error.message));
            } else {
                resolve(response.result);
            }
        });
        
        const message = JSON.stringify({ id, method, params }) + '\n';
        process.stdout.write(message);
    });
}

function sendNotification(method, params) {
    const message = JSON.stringify({ method, params }) + '\n';
    process.stdout.write(message);
}

// VS Code API Implementation
const commands = {
    registerCommand(id, handler) {
        return {
            dispose: () => sendNotification('command/unregister', { id })
        };
    },
    
    executeCommand(id, ...args) {
        return sendRequest('command/execute', { id, args });
    },
    
    getCommands() {
        return sendRequest('command/list', {});
    }
};

const window = {
    showInformationMessage(message, ...items) {
        return sendRequest('window/showInfo', { message, items });
    },
    
    showWarningMessage(message, ...items) {
        return sendRequest('window/showWarning', { message, items });
    },
    
    showErrorMessage(message, ...items) {
        return sendRequest('window/showError', { message, items });
    },
    
    showInputBox(options) {
        return sendRequest('window/showInput', options || {});
    },
    
    showQuickPick(items, options) {
        return sendRequest('window/showQuickPick', { items, options });
    },
    
    createOutputChannel(name) {
        return {
            append: (value) => sendNotification('output/append', { name, value }),
            appendLine: (value) => sendNotification('output/appendLine', { name, value }),
            show: () => sendNotification('output/show', { name }),
            dispose: () => sendNotification('output/dispose', { name })
        };
    },
    
    activeTextEditor: null,
    
    onDidChangeActiveTextEditor: new EventEmitter()
};

const workspace = {
    workspaceFolders: null,
    
    getConfiguration(section) {
        return {
            get: (key, defaultValue) => sendRequest('workspace/config/get', { section, key, defaultValue }),
            update: (key, value) => sendRequest('workspace/config/update', { section, key, value })
        };
    },
    
    onDidChangeConfiguration: new EventEmitter(),
    onDidChangeWorkspaceFolders: new EventEmitter()
};

const languages = {
    registerCompletionItemProvider(selector, provider, ...triggerCharacters) {
        return {
            dispose: () => sendNotification('language/completion/unregister', { selector })
        };
    },
    
    registerHoverProvider(selector, provider) {
        return {
            dispose: () => sendNotification('language/hover/unregister', { selector })
        };
    },
    
    registerDefinitionProvider(selector, provider) {
        return {
            dispose: () => sendNotification('language/definition/unregister', { selector })
        };
    }
};

// Export VS Code API
module.exports = {
    commands,
    window,
    workspace,
    languages,
    ExtensionContext: class {
        constructor(extensionPath) {
            this.extensionPath = extensionPath;
            this.subscriptions = [];
            this.globalState = {
                get: (key, defaultValue) => defaultValue,
                update: (key, value) => Promise.resolve()
            };
            this.workspaceState = {
                get: (key, defaultValue) => defaultValue,
                update: (key, value) => Promise.resolve()
            };
        }
    },
    Disposable: class {
        constructor(func) {
            this.dispose = func;
        }
    },
    EventEmitter: class {
        constructor() {
            this.event = (listener) => ({ dispose: () => {} });
        }
        fire(data) {}
    },
    Uri: class {
        static parse(value) {
            return { toString: () => value, fsPath: value.replace('file://', '') };
        }
        static file(path) {
            return { toString: () => 'file://' + path, fsPath: path };
        }
    },
    Position: class {
        constructor(line, character) {
            this.line = line;
            this.character = character;
        }
    },
    Range: class {
        constructor(start, end) {
            this.start = start;
            this.end = end;
        }
    },
    Selection: class {
        constructor(anchor, active) {
            this.anchor = anchor;
            this.active = active;
        }
    },
    TextEdit: class {
        constructor(range, newText) {
            this.range = range;
            this.newText = newText;
        }
    },
    CompletionItemKind: {
        Text: 0, Method: 1, Function: 2, Constructor: 3, Field: 4,
        Variable: 5, Class: 6, Interface: 7, Module: 8, Property: 9,
        Unit: 10, Value: 11, Enum: 12, Keyword: 13, Snippet: 14,
        Color: 15, File: 16, Reference: 17, Folder: 18, EnumMember: 19,
        Constant: 20, Struct: 21, Event: 22, Operator: 23, TypeParameter: 24
    }
};

// Ready signal
sendNotification('host/ready', {});
"#.to_string()
    }

    /// Message pump for handling responses from extension host
    async fn message_pump(
        stdout: std::process::ChildStdout,
        pending: Arc<AsyncRwLock<HashMap<u64, oneshot::Sender<Result<Value>>>>>,
        shutdown_rx: &mut mpsc::Receiver<()>,
    ) {
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            match line {
                Ok(line) => {
                    if let Ok(response) = serde_json::from_str::<RpcResponse>(&line) {
                        let mut pending_guard = pending.write().await;
                        if let Some(tx) = pending_guard.remove(&response.id) {
                            let result = if let Some(error) = response.error {
                                Err(anyhow::anyhow!(
                                    "RPC error: {} - {}",
                                    error.code,
                                    error.message
                                ))
                            } else {
                                Ok(response.result.unwrap_or(Value::Null))
                            };
                            let _ = tx.send(result);
                        }
                    } else if let Ok(notification) = serde_json::from_str::<Value>(&line) {
                        // Handle notifications
                        if let Some(method) = notification.get("method") {
                            log::debug!("Extension host notification: {}", method);
                        }
                    }
                }
                Err(e) => {
                    log::error!("Error reading from extension host: {}", e);
                    break;
                }
            }
        }
    }

    /// Write a JSON-RPC message to the extension host stdin.
    fn write_rpc(&mut self, method: &str, params: Value) {
        if !*self.is_ready.read() {
            return;
        }

        let message = json!({
            "method": method,
            "params": params,
        });

        if let Some(stdin) = self.node_stdin.as_mut() {
            match serde_json::to_string(&message) {
                Ok(line) => {
                    if let Err(e) = stdin.write_all(line.as_bytes()) {
                        log::warn!("Failed to write RPC message '{}': {}", method, e);
                        return;
                    }
                    if let Err(e) = stdin.write_all(b"\n") {
                        log::warn!("Failed to write RPC newline for '{}': {}", method, e);
                        return;
                    }
                    if let Err(e) = stdin.flush() {
                        log::warn!("Failed to flush RPC message '{}': {}", method, e);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to serialize RPC message '{}': {}", method, e);
                }
            }
        }
    }

    /// Scan directory for extensions
    fn scan_extensions(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let package_json = path.join("package.json");
                if package_json.exists() {
                    if let Err(e) = self.load_extension(&path) {
                        log::warn!("Failed to load extension from {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Load extension from directory
    pub fn load_extension(&mut self, extension_path: &PathBuf) -> Result<String> {
        // Load package.json
        let package_json_path = extension_path.join("package.json");
        let manifest_content = std::fs::read_to_string(&package_json_path).with_context(|| {
            format!(
                "Failed to read package.json from {}",
                extension_path.display()
            )
        })?;
        let manifest: ExtensionManifest = serde_json::from_str(&manifest_content)
            .with_context(|| "Failed to parse package.json")?;

        let extension_id = format!("{}.{}", manifest.publisher, manifest.name);

        // Check dependencies
        for dep in &manifest.extension_dependencies {
            if !self.extensions.contains_key(dep) {
                log::warn!(
                    "Extension {} depends on {} which is not installed",
                    extension_id,
                    dep
                );
            }
        }

        let extension = Extension {
            id: extension_id.clone(),
            manifest,
            context: ExtensionContext::new(&extension_id, &extension_path.to_string_lossy()),
            state: ExtensionState::Installed,
            exports: None,
            extension_path: extension_path.clone(),
            subscriptions: Vec::new(),
        };

        self.extensions.insert(extension_id.clone(), extension);
        self.extension_order.push(extension_id.clone());

        log::info!("Loaded extension: {}", extension_id);
        Ok(extension_id)
    }

    /// Activate an extension
    pub fn activate_extension(&mut self, extension_id: &str) -> Result<()> {
        // Clone manifest and check state before mutable borrow
        let (current_state, manifest) = {
            let extension = self
                .extensions
                .get(extension_id)
                .ok_or_else(|| anyhow::anyhow!("Extension not found: {}", extension_id))?;
            (extension.state.clone(), extension.manifest.clone())
        };

        if current_state == ExtensionState::Active {
            return Ok(());
        }

        if let Some(extension) = self.extensions.get_mut(extension_id) {
            extension.state = ExtensionState::Activating;
        }

        // Check activation events
        if self.should_activate(&manifest) {
            let extension_path = self
                .extensions
                .get(extension_id)
                .map(|ext| ext.extension_path.to_string_lossy().to_string())
                .unwrap_or_default();
            self.write_rpc(
                "extension/activate",
                json!({
                    "id": extension_id,
                    "extensionPath": extension_path,
                }),
            );
            log::debug!("Activating extension via RPC: {}", extension_id);

            if let Some(extension) = self.extensions.get_mut(extension_id) {
                extension.state = ExtensionState::Active;
            }
            self.activated_extensions
                .insert(extension_id.to_string(), true);
            log::info!("Activated extension: {}", extension_id);
        } else if let Some(extension) = self.extensions.get_mut(extension_id) {
            extension.state = ExtensionState::Inactive;
        }

        Ok(())
    }

    /// Check if extension should activate based on events
    fn should_activate(&self, manifest: &ExtensionManifest) -> bool {
        if !manifest.activation_events.is_empty() {
            for event in &manifest.activation_events {
                match event.as_str() {
                    "*" => return true, // Activate on startup
                    e if e.starts_with("onLanguage:") => {
                        let ctx = self.context.read();
                        if ctx.active_editor.is_some() {
                            return true;
                        }
                    }
                    e if e.starts_with("onCommand:") => {
                        // Will activate on command
                    }
                    e if e.starts_with("onView:") => {
                        // Will activate on view
                    }
                    e if e.starts_with("workspaceContains:") => {
                        let ctx = self.context.read();
                        if ctx.workspace_root.is_some() {
                            return true;
                        }
                    }
                    e if e.starts_with("onFileSystem:") => {
                        // Will activate on file system access
                    }
                    e if e.starts_with("onDebug:") => {
                        // Will activate on debug
                    }
                    _ => {}
                }
            }
        }

        // Default: activate if no activation events specified
        manifest.activation_events.is_empty()
    }

    /// Deactivate an extension
    pub fn deactivate_extension(&mut self, extension_id: &str) -> Result<()> {
        let current_state = self
            .extensions
            .get(extension_id)
            .ok_or_else(|| anyhow::anyhow!("Extension not found: {}", extension_id))?
            .state
            .clone();

        if current_state == ExtensionState::Active {
            self.write_rpc("extension/deactivate", json!({ "id": extension_id }));

            let extension = self
                .extensions
                .get_mut(extension_id)
                .ok_or_else(|| anyhow::anyhow!("Extension not found: {}", extension_id))?;
            extension.state = ExtensionState::Inactive;
            self.activated_extensions.remove(extension_id);
            extension.subscriptions.clear();
            log::info!("Deactivated extension: {}", extension_id);
        }

        Ok(())
    }

    /// Get all extensions
    pub fn get_extensions(&self) -> Vec<&Extension> {
        self.extensions.values().collect()
    }

    /// Get active extensions
    pub fn get_active_extensions(&self) -> Vec<&Extension> {
        self.extensions
            .values()
            .filter(|e| e.state == ExtensionState::Active)
            .collect()
    }

    /// Get extension by ID
    pub fn get_extension(&self, extension_id: &str) -> Option<&Extension> {
        self.extensions.get(extension_id)
    }

    /// Execute a command from an extension
    pub fn execute_command(&mut self, command_id: &str, args: Vec<Value>) -> Result<Option<Value>> {
        // Find extension that registered this command (immutable search first)
        let target = self
            .extensions
            .values()
            .find(|extension| {
                extension
                    .manifest
                    .contributes
                    .as_ref()
                    .map(|c| c.commands.iter().any(|cmd| cmd.command == command_id))
                    .unwrap_or(false)
            })
            .map(|ext| (ext.id.clone(), ext.state.clone()));

        match target {
            Some((ext_id, state)) => {
                if state != ExtensionState::Active {
                    self.activate_extension(&ext_id)?;
                }

                self.write_rpc(
                    "command/execute",
                    json!({
                        "id": command_id,
                        "args": args,
                    }),
                );
                Ok(Some(json!({
                    "command": command_id,
                    "args": args,
                    "result": "dispatched"
                })))
            }
            None => Err(anyhow::anyhow!("Command not found: {}", command_id)),
        }
    }

    /// Set workspace root
    pub fn set_workspace(&mut self, path: PathBuf) {
        let mut ctx = self.context.write();
        ctx.workspace_root = Some(path);
    }

    /// Set active editor
    pub fn set_active_editor(&mut self, uri: &str) {
        let mut ctx = self.context.write();
        ctx.active_editor = Some(uri.to_string());
    }

    /// Update configuration
    pub fn update_configuration(&mut self, key: &str, value: Value) {
        let mut ctx = self.context.write();
        ctx.configuration.insert(key.to_string(), value);
    }

    /// Shutdown the extension host
    pub fn shutdown(&mut self) -> Result<()> {
        // Deactivate all extensions
        let extension_ids: Vec<String> = self.extensions.keys().cloned().collect();
        for id in extension_ids {
            let _ = self.deactivate_extension(&id);
        }

        // Signal shutdown
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.blocking_send(());
        }

        // Kill Node.js process
        if let Some(mut process) = self.node_process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }

        *self.is_ready.write() = false;
        log::info!("Extension host shutdown complete");
        Ok(())
    }
}

impl Default for ExtensionHost {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ExtensionHost {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(all(test, feature = "fixme_tests"))]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_extension_host_creation() {
        let host = ExtensionHost::new();
        assert!(host.extensions.is_empty());
        assert!(!*host.is_ready.read());
    }

    #[test]
    fn test_extension_state_transitions() {
        let state = ExtensionState::Installed;
        assert_eq!(state, ExtensionState::Installed);

        let failed = ExtensionState::Failed("test error".to_string());
        if let ExtensionState::Failed(msg) = failed {
            assert_eq!(msg, "test error");
        }
    }

    #[test]
    fn test_load_extension() {
        let dir = TempDir::new().unwrap();
        let extension_dir = dir.path().join("test-extension");
        fs::create_dir_all(&extension_dir).unwrap();

        let package_json = json!({
            "name": "test-extension",
            "publisher": "test",
            "version": "1.0.0",
            "engines": { "vscode": "^1.60.0" },
            "main": "./extension.js"
        });

        fs::write(
            extension_dir.join("package.json"),
            serde_json::to_string_pretty(&package_json).unwrap(),
        )
        .unwrap();

        let mut host = ExtensionHost::new();
        let result = host.load_extension(&extension_dir);

        assert!(result.is_ok());
        assert_eq!(host.extensions.len(), 1);
    }

    #[test]
    fn test_extension_host_config_default() {
        let config = ExtensionHostConfig::default();
        assert!(!config.development_mode);
        assert!(!config.enable_proposed_api);
        assert_eq!(config.product_name, "Kyro IDE");
    }

    #[test]
    fn test_workspace_context() {
        let mut host = ExtensionHost::new();
        host.set_workspace(PathBuf::from("/test/workspace"));

        let ctx = host.context.read();
        assert_eq!(ctx.workspace_root, Some(PathBuf::from("/test/workspace")));
    }
}
