// Collaboration Tauri Commands — Real operation tracking implementation
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{command, AppHandle, Emitter};
use tokio::sync::RwLock;

lazy_static::lazy_static! {
    static ref COLLAB_STATE: Arc<RwLock<CollaborationState>> = Arc::new(RwLock::new(CollaborationState::new()));
}

#[derive(Debug)]
pub struct CollaborationState {
    rooms: HashMap<String, RoomInfo>,
    current_room: Option<String>,
    connected: bool,
    operation_counts: HashMap<String, u64>,
    chat_history: HashMap<String, Vec<ChatEntry>>,
    room_created_at: HashMap<String, std::time::Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEntry {
    pub user_id: String,
    pub message: String,
    pub timestamp: String,
}

impl Default for CollaborationState {
    fn default() -> Self {
        Self::new()
    }
}

impl CollaborationState {
    pub fn new() -> Self {
        Self {
            rooms: HashMap::new(),
            current_room: None,
            connected: false,
            operation_counts: HashMap::new(),
            chat_history: HashMap::new(),
            room_created_at: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    pub users: Vec<CollaboratorInfo>,
    pub created_at: String,
    pub max_users: usize,
    pub is_encrypted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    pub name: String,
    pub max_users: Option<usize>,
    pub enable_e2ee: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRoomRequest {
    pub room_id: String,
    pub user_id: Option<String>,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaboratorInfo {
    pub id: String,
    pub name: String,
    pub color: String,
    pub cursor_line: Option<u32>,
    pub cursor_col: Option<u32>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceUpdate {
    pub user_id: String,
    pub cursor_line: Option<u32>,
    pub cursor_col: Option<u32>,
    pub selection_start: Option<u32>,
    pub selection_end: Option<u32>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextOperation {
    pub op_type: String,
    pub position: u64,
    pub text: Option<String>,
    pub length: Option<u64>,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomStats {
    pub user_count: usize,
    pub operation_count: u64,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteCursor {
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "userName")]
    pub user_name: String,
    pub color: String,
    pub line: u32,
    pub column: u32,
    pub selection: Option<CursorSelection>,
}

fn build_remote_cursors(room: &RoomInfo) -> Vec<RemoteCursor> {
    room.users
        .iter()
        .filter_map(|user| match (user.cursor_line, user.cursor_col) {
            (Some(line), Some(column)) => Some(RemoteCursor {
                user_id: user.id.clone(),
                user_name: user.name.clone(),
                color: user.color.clone(),
                line,
                column,
                selection: None,
            }),
            _ => None,
        })
        .collect()
}

#[command]
pub async fn create_room(request: CreateRoomRequest) -> Result<RoomInfo, String> {
    let mut state = COLLAB_STATE.write().await;
    let id = uuid::Uuid::new_v4().to_string();
    let room = RoomInfo {
        id: id.clone(),
        name: request.name,
        users: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
        max_users: request.max_users.unwrap_or(50),
        is_encrypted: request.enable_e2ee.unwrap_or(false),
    };
    state.rooms.insert(id.clone(), room.clone());
    state.operation_counts.insert(id.clone(), 0);
    state.chat_history.insert(id.clone(), Vec::new());
    state.room_created_at.insert(id, std::time::Instant::now());
    Ok(room)
}

#[command]
pub async fn join_room(request: JoinRoomRequest) -> Result<RoomInfo, String> {
    let mut state = COLLAB_STATE.write().await;
    let room = state
        .rooms
        .get_mut(&request.room_id)
        .ok_or("Room not found")?;
    room.users.push(CollaboratorInfo {
        id: request
            .user_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        name: request.username,
        color: format!("#{:06x}", rand::random::<u32>() & 0xFFFFFF),
        cursor_line: None,
        cursor_col: None,
        status: "active".to_string(),
    });

    // Clone the room info before mutating other fields on `state`
    let room_clone = room.clone();
    state.current_room = Some(request.room_id.clone());
    state.connected = true;
    Ok(room_clone)
}

#[command]
pub async fn leave_room(room_id: String) -> Result<(), String> {
    let mut state = COLLAB_STATE.write().await;
    if state.current_room.as_deref() == Some(&room_id) {
        state.current_room = None;
        state.connected = false;
    }
    Ok(())
}

#[command]
pub async fn get_room_users(room_id: String) -> Result<Vec<CollaboratorInfo>, String> {
    let state = COLLAB_STATE.read().await;
    let room = state.rooms.get(&room_id).ok_or("Room not found")?;
    Ok(room.users.clone())
}

#[command]
pub async fn update_presence(room_id: String, presence: PresenceUpdate) -> Result<(), String> {
    let mut state = COLLAB_STATE.write().await;
    let room = state.rooms.get_mut(&room_id).ok_or("Room not found")?;
    let user_idx = room.users.iter().position(|u| u.id == presence.user_id);
    if let Some(idx) = user_idx {
        room.users[idx].cursor_line = presence.cursor_line;
        room.users[idx].cursor_col = presence.cursor_col;
        room.users[idx].status = presence.status;
    }
    Ok(())
}

#[command]
pub async fn get_room_presence(room_id: String) -> Result<Vec<PresenceUpdate>, String> {
    let state = COLLAB_STATE.read().await;
    let room = state.rooms.get(&room_id).ok_or("Room not found")?;
    Ok(room
        .users
        .iter()
        .map(|u| PresenceUpdate {
            user_id: u.id.clone(),
            cursor_line: u.cursor_line,
            cursor_col: u.cursor_col,
            selection_start: None,
            selection_end: None,
            status: u.status.clone(),
        })
        .collect())
}

#[command]
pub async fn send_operation(room_id: String, operation: TextOperation) -> Result<(), String> {
    let mut state = COLLAB_STATE.write().await;
    let _room = state.rooms.get(&room_id).ok_or("Room not found")?;
    // Track the operation
    *state.operation_counts.entry(room_id.clone()).or_insert(0) += 1;
    log::info!(
        "Operation #{} in {}: {:?}",
        state.operation_counts[&room_id],
        room_id,
        operation.op_type
    );
    Ok(())
}

#[command]
pub async fn send_chat_message(room_id: String, message: String) -> Result<(), String> {
    let mut state = COLLAB_STATE.write().await;
    let _room = state.rooms.get(&room_id).ok_or("Room not found")?;
    // Store chat message
    let entry = ChatEntry {
        user_id: "local".to_string(),
        message: message.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    state
        .chat_history
        .entry(room_id.clone())
        .or_default()
        .push(entry);
    log::info!("Chat in {}: {}", room_id, message);
    Ok(())
}

#[command]
pub async fn get_collab_stats(room_id: String) -> Result<RoomStats, String> {
    let state = COLLAB_STATE.read().await;
    let room = state.rooms.get(&room_id).ok_or("Room not found")?;
    let op_count = state.operation_counts.get(&room_id).copied().unwrap_or(0);
    let uptime = state
        .room_created_at
        .get(&room_id)
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0);
    Ok(RoomStats {
        user_count: room.users.len(),
        operation_count: op_count,
        uptime_secs: uptime,
    })
}

#[command]
pub async fn is_connected_to_room() -> Result<bool, String> {
    let state = COLLAB_STATE.read().await;
    Ok(state.connected)
}

#[command]
pub async fn get_current_room() -> Result<Option<RoomInfo>, String> {
    let state = COLLAB_STATE.read().await;
    if let Some(ref room_id) = state.current_room {
        Ok(state.rooms.get(room_id).cloned())
    } else {
        Ok(None)
    }
}

#[command]
pub async fn list_rooms() -> Result<Vec<RoomInfo>, String> {
    let state = COLLAB_STATE.read().await;
    Ok(state.rooms.values().cloned().collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorBroadcast {
    pub line: u32,
    pub column: u32,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    pub selection: Option<CursorSelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorSelection {
    #[serde(rename = "startLine")]
    pub start_line: u32,
    #[serde(rename = "startColumn")]
    pub start_column: u32,
    #[serde(rename = "endLine")]
    pub end_line: u32,
    #[serde(rename = "endColumn")]
    pub end_column: u32,
}

pub async fn broadcast_cursor_legacy(
    app: AppHandle,
    room_id: String,
    cursor: CursorBroadcast,
) -> Result<(), String> {
    let mut state = COLLAB_STATE.write().await;
    let room = state.rooms.get_mut(&room_id).ok_or("Room not found")?;
    // Update the user's cursor in presence
    let user_id = cursor.user_id.as_deref().unwrap_or("local");
    if let Some(user) = room
        .users
        .iter_mut()
        .find(|u| u.id == user_id || u.name == user_id)
    {
        user.cursor_line = Some(cursor.line);
        user.cursor_col = Some(cursor.column);
    }

    let payload = serde_json::json!({
        "type": "cursors",
        "data": build_remote_cursors(room),
    });
    app.emit("collab:presence", payload)
        .map_err(|e| format!("Failed to emit cursor update: {}", e))?;

    Ok(())
}

#[command]
pub async fn get_room_cursors(room_id: String) -> Result<Vec<RemoteCursor>, String> {
    let state = COLLAB_STATE.read().await;
    let room = state.rooms.get(&room_id).ok_or("Room not found")?;
    Ok(build_remote_cursors(room))
}
