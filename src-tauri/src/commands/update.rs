use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{command, AppHandle, Manager};
use tauri_plugin_updater::{target, Update, UpdaterExt};
use tokio::sync::{Mutex, RwLock};
use url::Url;

const DEFAULT_UPDATE_ENDPOINT: &str =
    "https://github.com/nkpendyam/Kyro_IDE/releases/latest/download/latest.json";

lazy_static::lazy_static! {
    static ref UPDATE_STATE: Arc<UpdateState> = Arc::new(UpdateState::new());
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateCommandError {
    #[error("updater is not configured for production releases")]
    MissingConfiguration,
    #[error("there is no pending update to download")]
    NoPendingUpdate,
    #[error("there is no downloaded update ready to install")]
    NoDownloadedUpdate,
    #[error(transparent)]
    Updater(#[from] tauri_plugin_updater::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error(transparent)]
    Url(#[from] url::ParseError),
}

impl Serialize for UpdateCommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub release_date: Option<String>,
    pub release_notes: String,
    pub channel: String,
    pub size_mb: Option<f32>,
    pub mandatory: bool,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percentage: f32,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecord {
    pub version: String,
    pub installed_at: String,
    pub channel: String,
}

#[derive(Default)]
struct DownloadedUpdate {
    update: Option<Update>,
    bytes: Option<Vec<u8>>,
}

pub struct UpdateState {
    channel: RwLock<String>,
    auto_update: RwLock<bool>,
    last_check: RwLock<Option<String>>,
    skipped_versions: RwLock<Vec<String>>,
    history: RwLock<Vec<UpdateRecord>>,
    pending_update: Mutex<Option<Update>>,
    downloaded_update: Mutex<DownloadedUpdate>,
    progress: RwLock<UpdateProgress>,
}

impl UpdateState {
    pub fn new() -> Self {
        Self {
            channel: RwLock::new("stable".to_string()),
            auto_update: RwLock::new(true),
            last_check: RwLock::new(None),
            skipped_versions: RwLock::new(Vec::new()),
            history: RwLock::new(Vec::new()),
            pending_update: Mutex::new(None),
            downloaded_update: Mutex::new(DownloadedUpdate::default()),
            progress: RwLock::new(UpdateProgress {
                downloaded_bytes: 0,
                total_bytes: None,
                percentage: 0.0,
                completed: false,
            }),
        }
    }
}

fn update_channel_endpoint(channel: &str) -> String {
    match channel {
        "beta" => "https://github.com/nkpendyam/Kyro_IDE/releases/download/beta/latest.json"
            .to_string(),
        "nightly" => {
            "https://github.com/nkpendyam/Kyro_IDE/releases/download/nightly/latest.json".to_string()
        }
        _ => DEFAULT_UPDATE_ENDPOINT.to_string(),
    }
}

fn updater_pubkey() -> Option<String> {
    std::env::var("TAURI_UPDATER_PUBKEY")
        .ok()
        .or_else(|| std::env::var("TAURI_SIGNING_PUBKEY").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn update_info_from_update(update: &Update, channel: String) -> UpdateInfo {
    let release_date = update.date.map(|value| value.to_string());
    let release_notes = update.body.clone().unwrap_or_default();
    let size_mb = update
        .raw_json
        .get("size")
        .and_then(|value| value.as_u64())
        .map(|value| value as f32 / 1_048_576.0);
    let mandatory = release_notes.to_lowercase().contains("critical")
        || release_notes.to_lowercase().contains("security");

    UpdateInfo {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
        release_date,
        release_notes,
        channel,
        size_mb,
        mandatory,
        target: update.target.clone(),
    }
}

async fn build_runtime_updater(
    app: &AppHandle,
    channel: &str,
) -> Result<tauri_plugin_updater::Updater, UpdateCommandError> {
    let endpoint = Url::parse(&update_channel_endpoint(channel))?;
    let builder = app.updater_builder();
    let builder = if let Some(pubkey) = updater_pubkey() {
        builder.pubkey(pubkey)
    } else {
        builder
    };

    Ok(builder.endpoints(vec![endpoint])?.build()?)
}

fn reset_progress(progress: &mut UpdateProgress) {
    progress.downloaded_bytes = 0;
    progress.total_bytes = None;
    progress.percentage = 0.0;
    progress.completed = false;
}

/// Check the configured release channel for a signed update.
#[command]
pub async fn check_for_updates(
    app: AppHandle,
) -> Result<Option<UpdateInfo>, UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    let channel = state.channel.read().await.clone();
    let updater = build_runtime_updater(&app, &channel).await?;

    let update = updater.check().await?;

    *state.last_check.write().await = Some(chrono::Utc::now().to_rfc3339());

    if let Some(update) = update {
        let skipped = state.skipped_versions.read().await;
        if skipped.contains(&update.version) {
            return Ok(None);
        }
        drop(skipped);

        let info = update_info_from_update(&update, channel);
        *state.pending_update.lock().await = Some(update);
        *state.downloaded_update.lock().await = DownloadedUpdate::default();
        let mut progress = state.progress.write().await;
        reset_progress(&mut progress);
        return Ok(Some(info));
    }

    *state.pending_update.lock().await = None;
    *state.downloaded_update.lock().await = DownloadedUpdate::default();
    let mut progress = state.progress.write().await;
    reset_progress(&mut progress);
    Ok(None)
}

/// Download a pending update with byte-level progress tracking.
#[command]
pub async fn download_update(
) -> Result<UpdateProgress, UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    let pending = state.pending_update.lock().await.clone();
    let update = pending.ok_or(UpdateCommandError::NoPendingUpdate)?;

    {
        let mut progress = state.progress.write().await;
        reset_progress(&mut progress);
    }

    let progress_state = Arc::clone(&state);
    let bytes = update
        .download(
            move |chunk_length, content_length| {
                let progress_state = Arc::clone(&progress_state);
                tauri::async_runtime::spawn(async move {
                    let mut progress = progress_state.progress.write().await;
                    progress.downloaded_bytes = progress.downloaded_bytes.saturating_add(chunk_length as u64);
                    progress.total_bytes = content_length;
                    progress.percentage = match content_length {
                        Some(total) if total > 0 => {
                            ((progress.downloaded_bytes as f64 / total as f64) * 100.0) as f32
                        }
                        _ => 0.0,
                    };
                    progress.completed = false;
                });
            },
            {
                let progress_state = Arc::clone(&state);
                move || {
                    tauri::async_runtime::spawn(async move {
                        let mut progress = progress_state.progress.write().await;
                        progress.completed = true;
                        if progress.total_bytes.is_some() {
                            progress.percentage = 100.0;
                        }
                    });
                }
            },
        )
        .await?;

    *state.downloaded_update.lock().await = DownloadedUpdate {
        update: Some(update),
        bytes: Some(bytes),
    };

    let progress = state.progress.read().await.clone();
    Ok(progress)
}

/// Read current update download progress.
#[command]
pub async fn get_download_progress(
) -> Result<UpdateProgress, UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    let progress = state.progress.read().await.clone();
    Ok(progress)
}

/// Install the downloaded update package.
#[command]
pub async fn install_update(
    app: AppHandle,
) -> Result<(), UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    let mut downloaded = state.downloaded_update.lock().await;
    let update = downloaded
        .update
        .clone()
        .ok_or(UpdateCommandError::NoDownloadedUpdate)?;
    let bytes = downloaded
        .bytes
        .clone()
        .ok_or(UpdateCommandError::NoDownloadedUpdate)?;

    update.install(bytes)?;

    let channel = state.channel.read().await.clone();
    state.history.write().await.push(UpdateRecord {
        version: update.version.clone(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        channel,
    });
    *downloaded = DownloadedUpdate::default();
    *state.pending_update.lock().await = None;

    app.restart();
}

/// Cancel the in-memory pending or downloaded update state.
#[command]
pub async fn cancel_update(
) -> Result<(), UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    *state.pending_update.lock().await = None;
    *state.downloaded_update.lock().await = DownloadedUpdate::default();
    let mut progress = state.progress.write().await;
    reset_progress(&mut progress);
    Ok(())
}

/// Return the active updater release channel.
#[command]
pub async fn get_update_channel(
) -> Result<String, UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    let channel = state.channel.read().await.clone();
    Ok(channel)
}

/// Change the updater release channel.
#[command]
pub async fn set_update_channel(
    channel: String,
) -> Result<(), UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    *state.channel.write().await = channel;
    *state.pending_update.lock().await = None;
    *state.downloaded_update.lock().await = DownloadedUpdate::default();
    let mut progress = state.progress.write().await;
    reset_progress(&mut progress);
    Ok(())
}

/// List installed update records for audit/debug UI.
#[command]
pub async fn get_update_history(
) -> Result<Vec<UpdateRecord>, UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    let history = state.history.read().await.clone();
    Ok(history)
}

/// Enable or disable automatic update checks.
#[command]
pub async fn set_auto_update(
    enabled: bool,
) -> Result<(), UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    *state.auto_update.write().await = enabled;
    Ok(())
}

/// Check whether automatic update checks are enabled.
#[command]
pub async fn is_auto_update_enabled(
) -> Result<bool, UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    let enabled = *state.auto_update.read().await;
    Ok(enabled)
}

/// Skip a specific update version for the current install.
#[command]
pub async fn skip_update(
    version: String,
) -> Result<(), UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    state.skipped_versions.write().await.push(version);
    *state.pending_update.lock().await = None;
    *state.downloaded_update.lock().await = DownloadedUpdate::default();
    Ok(())
}

/// Return the timestamp of the last successful check attempt.
#[command]
pub async fn get_last_update_check(
) -> Result<Option<String>, UpdateCommandError> {
    let state = Arc::clone(&UPDATE_STATE);
    let last_check = state.last_check.read().await.clone();
    Ok(last_check)
}

#[cfg(test)]
mod tests {
    use super::{reset_progress, update_channel_endpoint, UpdateProgress};

    #[test]
    fn update_channel_endpoint_uses_expected_stable_url() {
        assert_eq!(
            update_channel_endpoint("stable"),
            "https://github.com/nkpendyam/Kyro_IDE/releases/latest/download/latest.json"
        );
    }

    #[test]
    fn reset_progress_clears_download_state() {
        let mut progress = UpdateProgress {
            downloaded_bytes: 15,
            total_bytes: Some(20),
            percentage: 75.0,
            completed: true,
        };

        reset_progress(&mut progress);

        assert_eq!(
            progress,
            UpdateProgress {
                downloaded_bytes: 0,
                total_bytes: None,
                percentage: 0.0,
                completed: false,
            }
        );
    }

}
