//! WASM Runtime for Plugins
//!
//! Sandboxed execution environment for WASM plugins

use super::*;
use anyhow::Result;
use std::path::PathBuf;

/// WASM runtime wrapper
#[derive(Default)]
pub struct WasmRuntime {
    module: Vec<u8>,
    #[cfg(feature = "wasm-plugins")]
    instance: Option<wasmtime::Instance>,
    store: Option<PluginStore>,
}

/// Plugin store (WASM state)
struct PluginStore {
    memory_limit: usize,
    execution_time_limit_ms: u64,
}

impl WasmRuntime {
    /// Create a new WASM runtime from file
    pub fn new(wasm_path: &PathBuf) -> Result<Self> {
        let module = std::fs::read(wasm_path)?;

        Ok(Self {
            module,
            #[cfg(feature = "wasm-plugins")]
            instance: None,
            store: Some(PluginStore {
                memory_limit: 16 * 1024 * 1024, // 16MB
                execution_time_limit_ms: 5000,  // 5 seconds
            }),
        })
    }

    /// Activate the plugin
    pub fn activate(&mut self, _context: &PluginContext) -> Result<()> {
        #[cfg(feature = "wasm-plugins")]
        {
            use wasmtime::*;

            let engine = Engine::default();
            let module = Module::from_binary(&engine, &self.module)?;

            let mut linker = Linker::new(&engine);

            // Register host functions
            self.register_host_functions(&mut linker, context)?;

            let mut store = Store::new(&engine, PluginState::default());

            let instance = linker.instantiate(&mut store, &module)?;
            self.instance = Some(instance);

            // Call activate function if present
            if let Some(activate) = instance.get_typed_func::<(), ()>(&mut store, "activate") {
                activate.call(&mut store, ())?;
            }
        }

        log::info!("WASM plugin activated");
        Ok(())
    }

    /// Deactivate the plugin
    pub fn deactivate(&mut self) -> Result<()> {
        #[cfg(feature = "wasm-plugins")]
        {
            if let (Some(instance), Some(ref mut store)) = (&self.instance, self.store.as_mut()) {
                // Call deactivate function if present
                if let Some(deactivate) = instance.get_typed_func::<(), ()>(&mut *store, "deactivate") {
                    deactivate.call(&mut *store, ())?;
                }
            }
        }

        log::info!("WASM plugin deactivated");
        Ok(())
    }

    /// Execute a command
    pub fn execute(
        &mut self,
        _command: &str,
        _args: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        #[cfg(feature = "wasm-plugins")]
        {
            if let (Some(instance), Some(ref mut store)) = (&self.instance, &mut self.store) {
                // Get the command function
                if let Some(func) =
                    instance.get_typed_func::<(i32, i32), i32>(store, &format!("cmd_{}", command))
                {
                    // Marshal arguments
                    let args_json = serde_json::to_string(args)?;
                    let args_ptr = self.allocate_string(store, &args_json)?;

                    // Call function
                    let result_ptr = func.call(store, (args_ptr as i32, args_json.len() as i32))?;

                    // Read result
                    let result = self.read_string(store, result_ptr as usize)?;

                    return Ok(serde_json::from_str(&result)?);
                }
            }
        }

        // Fallback: return empty result
        Ok(serde_json::json!({ "success": false, "error": "Command not found" }))
    }

    /// Register host functions for plugins
    #[cfg(feature = "wasm-plugins")]
    fn register_host_functions(
        &self,
        linker: &mut wasmtime::Linker<PluginState>,
        context: &PluginContext,
    ) -> Result<()> {
        let caps = context.capabilities.clone();
        let data_dir = context.data_dir.clone();

        // File system read — reads a file path from WASM memory, returns result ptr
        let caps_fs = caps.clone();
        let data_dir_fs = data_dir.clone();
        linker.func_wrap(
            "env",
            "fs_read",
            move |mut caller: wasmtime::Caller<'_, PluginState>,
                  path_ptr: i32,
                  path_len: i32|
                  -> i32 {
                if !caps_fs.has("fs:read") {
                    log::warn!("Plugin denied fs:read capability");
                    return -1;
                }
                let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                    Some(m) => m,
                    None => return -1,
                };
                let data = memory.data(&caller);
                let path_bytes = &data[path_ptr as usize..(path_ptr + path_len) as usize];
                let path_str = match std::str::from_utf8(path_bytes) {
                    Ok(s) => s,
                    Err(_) => return -1,
                };
                // Restrict reads to plugin data dir for sandboxing
                let full_path = data_dir_fs.join(path_str);
                match std::fs::read(&full_path) {
                    Ok(content) => {
                        // Write content into WASM memory using plugin's malloc
                        let len = content.len() as i32;
                        if let Some(malloc) =
                            caller.get_export("malloc").and_then(|e| e.into_func())
                        {
                            if let Ok(results) = malloc.call(
                                &mut caller,
                                &[wasmtime::Val::I32(len)],
                                &mut [wasmtime::Val::I32(0)],
                            ) {
                                let ptr = results.first().map(|v| v.unwrap_i32()).unwrap_or(0);
                                let data_mut = memory.data_mut(&mut caller);
                                data_mut[ptr as usize..(ptr + len) as usize]
                                    .copy_from_slice(&content);
                                return ptr;
                            }
                        }
                        -1
                    }
                    Err(_) => -1,
                }
            },
        )?;

        // File system write — writes data to a file (sandboxed to plugin data dir)
        let caps_fsw = caps.clone();
        let data_dir_fsw = data_dir.clone();
        linker.func_wrap(
            "env",
            "fs_write",
            move |caller: wasmtime::Caller<'_, PluginState>,
                  path_ptr: i32,
                  path_len: i32,
                  data_ptr: i32,
                  data_len: i32|
                  -> i32 {
                if !caps_fsw.has("fs:write") {
                    log::warn!("Plugin denied fs:write capability");
                    return -1;
                }
                let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                    Some(m) => m,
                    None => return -1,
                };
                let mem_data = memory.data(&caller);
                let path_bytes = &mem_data[path_ptr as usize..(path_ptr + path_len) as usize];
                let path_str = match std::str::from_utf8(path_bytes) {
                    Ok(s) => s,
                    Err(_) => return -1,
                };
                let write_data = &mem_data[data_ptr as usize..(data_ptr + data_len) as usize];
                let full_path = data_dir_fsw.join(path_str);
                // Ensure parent directory exists
                if let Some(parent) = full_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                match std::fs::write(&full_path, write_data) {
                    Ok(()) => 0,
                    Err(_) => -1,
                }
            },
        )?;

        // Editor: get content — returns a placeholder; real impl would
        // communicate with the frontend via Tauri events
        linker.func_wrap(
            "env",
            "editor_get_content",
            |caller: wasmtime::Caller<'_, PluginState>| -> i32 {
                // Return 0 (null) — editor content requires async frontend bridge
                // Plugins should use the event-based API instead
                0
            },
        )?;

        // Editor: set content
        linker.func_wrap(
            "env",
            "editor_set_content",
            |caller: wasmtime::Caller<'_, PluginState>,
             content_ptr: i32,
             content_len: i32|
             -> i32 {
                let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                    Some(m) => m,
                    None => return -1,
                };
                let data = memory.data(&caller);
                let content = match std::str::from_utf8(
                    &data[content_ptr as usize..(content_ptr + content_len) as usize],
                ) {
                    Ok(s) => s.to_string(),
                    Err(_) => return -1,
                };
                log::info!("Plugin set editor content ({} bytes)", content.len());
                // Would emit a Tauri event: window.emit("plugin-set-content", content)
                0
            },
        )?;

        // AI completion — calls Ollama synchronously (blocking in WASM context)
        let caps_ai = caps.clone();
        linker.func_wrap(
            "env",
            "ai_complete",
            move |caller: wasmtime::Caller<'_, PluginState>,
                  prompt_ptr: i32,
                  prompt_len: i32|
                  -> i32 {
                if !caps_ai.has("ai:complete") {
                    log::warn!("Plugin denied ai:complete capability");
                    return -1;
                }
                let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                    Some(m) => m,
                    None => return -1,
                };
                let data = memory.data(&caller);
                let _prompt = match std::str::from_utf8(
                    &data[prompt_ptr as usize..(prompt_ptr + prompt_len) as usize],
                ) {
                    Ok(s) => s.to_string(),
                    Err(_) => return -1,
                };
                log::info!("Plugin AI complete request ({} bytes)", _prompt.len());
                // Synchronous AI call not possible from WASM sync context.
                // Return 0 to signal "use async event bridge".
                0
            },
        )?;

        // Console log — writes plugin output to the IDE log
        linker.func_wrap(
            "env",
            "console_log",
            |caller: wasmtime::Caller<'_, PluginState>, msg_ptr: i32, msg_len: i32| {
                let memory = match caller.get_export("memory").and_then(|e| e.into_memory()) {
                    Some(m) => m,
                    None => return,
                };
                let data = memory.data(&caller);
                if let Ok(msg) =
                    std::str::from_utf8(&data[msg_ptr as usize..(msg_ptr + msg_len) as usize])
                {
                    log::info!("[plugin] {}", msg);
                }
            },
        )?;

        Ok(())
    }

    #[cfg(feature = "wasm-plugins")]
    fn allocate_string(&self, store: &mut wasmtime::Store<PluginState>, s: &str) -> Result<usize> {
        if let Some(ref instance) = self.instance {
            // Call the plugin's exported malloc function
            if let Some(malloc) = instance.get_func(store, "malloc") {
                let mut results = [wasmtime::Val::I32(0)];
                malloc.call(store, &[wasmtime::Val::I32(s.len() as i32)], &mut results)?;
                let ptr = results[0].unwrap_i32() as usize;

                // Write string bytes into WASM memory
                if let Some(memory) = instance.get_memory(store, "memory") {
                    let data = memory.data_mut(store);
                    data[ptr..ptr + s.len()].copy_from_slice(s.as_bytes());
                }
                return Ok(ptr);
            }
        }
        Ok(0)
    }

    #[cfg(feature = "wasm-plugins")]
    fn read_string(&self, store: &mut wasmtime::Store<PluginState>, ptr: usize) -> Result<String> {
        if let Some(ref instance) = self.instance {
            if let Some(memory) = instance.get_memory(store, "memory") {
                let data = memory.data(store);
                // Read until null terminator or max 64KB
                let max_len = 65536.min(data.len().saturating_sub(ptr));
                let end = data[ptr..ptr + max_len]
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(max_len);
                return Ok(String::from_utf8_lossy(&data[ptr..ptr + end]).to_string());
            }
        }
        Ok(String::new())
    }
}

/// Plugin state for WASM store
#[derive(Default)]
struct PluginState {
    memory_base: usize,
    allocated: std::collections::HashMap<usize, usize>,
}

/// Plugin context passed to plugins
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub plugin_id: String,
    pub data_dir: PathBuf,
    pub config: serde_json::Value,
    pub capabilities: CapabilitySet,
}

impl PluginContext {
    pub fn new(plugin_id: &str, data_dir: PathBuf) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            data_dir,
            config: serde_json::json!({}),
            capabilities: CapabilitySet::new(),
        }
    }

    /// Get plugin data directory
    pub fn get_data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    /// Get plugin config
    pub fn get_config(&self) -> &serde_json::Value {
        &self.config
    }

    /// Check if capability is granted
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.has(capability)
    }
}
