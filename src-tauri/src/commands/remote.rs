//! Remote Dev Environment Commands
//!
//! Provides `remote_connect` and `remote_disconnect` to establish and tear down
//! SSH, Dev Container (Docker), and WSL connections from the IDE.
//!
//! - SSH: probes reachability via the system `ssh` binary
//! - Dev Container: confirms Docker daemon is running
//! - WSL: validates the distribution is running on Windows

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{command, State};

/// Connection identifier stored in app state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: String,
    pub connection_type: String,
    pub host: String,
}

/// In-memory registry of active connections (pid or logical handle)
pub struct RemoteState(pub Mutex<HashMap<String, ConnectionInfo>>);

impl Default for RemoteState {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

/// Establish a remote connection.
///
/// # Arguments
/// * `connection_type` — one of `"ssh"`, `"devcontainer"`, `"wsl"`
/// * `host` — hostname or WSL distro name
/// * `config` — extra key/value pairs (e.g. `username`, `port`, `distro`)
///
/// Returns a stable `connection_id` for subsequent `remote_disconnect` calls.
#[command]
pub async fn remote_connect(
    state: State<'_, RemoteState>,
    connection_type: String,
    host: String,
    config: HashMap<String, String>,
) -> Result<String, String> {
    let id = match connection_type.as_str() {
        "ssh" => connect_ssh(&host, &config),
        "devcontainer" => connect_devcontainer(&host, &config),
        "wsl" => connect_wsl(&host, &config),
        other => Err(format!("Unknown connection type: {other}")),
    }?;

    if let Ok(mut map) = state.0.lock() {
        map.insert(
            id.clone(),
            ConnectionInfo {
                id: id.clone(),
                connection_type,
                host,
            },
        );
    }

    Ok(id)
}

/// Tear down a previously established connection.
///
/// # Arguments
/// * `connection_id` — the ID returned by `remote_connect`
#[command]
pub async fn remote_disconnect(
    state: State<'_, RemoteState>,
    connection_id: String,
) -> Result<(), String> {
    if let Ok(mut map) = state.0.lock() {
        map.remove(&connection_id);
    }
    Ok(())
}

// ────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ────────────────────────────────────────────────────────────────────────────

/// Test SSH reachability by running:
/// `ssh -o BatchMode=yes -o ConnectTimeout=5 [user@]host echo kyro_ok`
///
/// On success the host is reachable. We don't keep a persistent shell open
/// here — the IDE terminal panel handles interactive sessions via PTY.
fn connect_ssh(host: &str, config: &HashMap<String, String>) -> Result<String, String> {
    let port = config.get("port").map(|p| p.as_str()).unwrap_or("22");
    let user_at_host = if let Some(user) = config.get("username") {
        format!("{user}@{host}")
    } else {
        host.to_string()
    };

    let output = std::process::Command::new("ssh")
        .args([
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "-o",
            "StrictHostKeyChecking=accept-new",
            "-p",
            port,
            &user_at_host,
            "echo",
            "kyro_ok",
        ])
        .output()
        .map_err(|e| format!("ssh binary not found or failed to spawn: {e}"))?;

    if output.status.success() {
        let id = format!("ssh-{host}-{}", chrono::Utc::now().timestamp_millis());
        Ok(id)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("SSH connection to {host} failed: {stderr}"))
    }
}

/// Verify Docker daemon is running and the devcontainer image/container can be
/// started. Runs `docker info` as a lightweight reachability probe.
fn connect_devcontainer(host: &str, config: &HashMap<String, String>) -> Result<String, String> {
    let output = std::process::Command::new("docker")
        .arg("info")
        .output()
        .map_err(|e| format!("docker binary not found: {e}"))?;

    if output.status.success() {
        let image = config
            .get("image")
            .map(|s| s.as_str())
            .unwrap_or("default");
        let id = format!(
            "devcontainer-{image}-{}",
            chrono::Utc::now().timestamp_millis()
        );
        Ok(id)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Docker daemon not reachable: {stderr}"))
    }
}

/// On Windows, run `wsl --list --running` and verify the requested distribution
/// is active. On non-Windows platforms, returns an error.
fn connect_wsl(host: &str, config: &HashMap<String, String>) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let distro = config
            .get("distro")
            .map(|s| s.as_str())
            .unwrap_or(host);

        let output = std::process::Command::new("wsl")
            .args(["--list", "--running"])
            .output()
            .map_err(|e| format!("wsl.exe not found: {e}"))?;

        // wsl --list uses UTF-16 on some Windows versions; decode both
        let stdout_raw = output.stdout;
        let stdout = if stdout_raw.len() >= 2
            && stdout_raw[0] == 0xFF
            && stdout_raw[1] == 0xFE
        {
            // UTF-16 LE BOM
            let words: Vec<u16> = stdout_raw
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect();
            String::from_utf16_lossy(&words)
        } else {
            String::from_utf8_lossy(&stdout_raw).to_string()
        };

        if stdout.contains(distro) {
            let id = format!("wsl-{distro}-{}", chrono::Utc::now().timestamp_millis());
            Ok(id)
        } else {
            Err(format!(
                "WSL distribution '{distro}' is not running. \
                 Start it with `wsl -d {distro}` first."
            ))
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (host, config);
        Err("WSL connections are only supported on Windows".to_string())
    }
}
