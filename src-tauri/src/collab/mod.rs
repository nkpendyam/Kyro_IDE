//! Real WebSocket Collaboration with CRDT
//!
//! Implements real-time collaborative editing using:
//! - Yrs (Yjs Rust port) for CRDT-based conflict resolution
//! - WebSocket for real-time communication
//! - Awareness protocol for presence

pub mod awareness;
pub mod document;
pub mod sync;

use anyhow::Result;
use lazy_static::lazy_static;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::command;
use tokio::sync::{broadcast, mpsc, RwLock};

pub use awareness::*;
pub use sync::*;

/// Collaboration room
#[derive(Clone)]
pub struct CollabRoom {
    pub id: String,
    pub document: Arc<RwLock<CollabDocument>>,
    pub awareness: Arc<RwLock<AwarenessState>>,
    pub users: HashMap<String, CollabUser>,
    pub created_at: u64,
}

impl std::fmt::Debug for CollabRoom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollabRoom")
            .field("id", &self.id)
            .field("document", &"<CollabDocument>")
            .field("awareness", &self.awareness)
            .field("users", &self.users)
            .field("created_at", &self.created_at)
            .finish()
    }
}

impl CollabRoom {
    pub fn new(id: String) -> Self {
        Self {
            id: id.clone(),
            document: Arc::new(RwLock::new(CollabDocument::new(&id))),
            awareness: Arc::new(RwLock::new(AwarenessState::new())),
            users: HashMap::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

/// Collaborative user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollabUser {
    pub id: String,
    pub name: String,
    pub color: String,
    pub cursor: Option<CursorPosition>,
    pub selection: Option<SelectionRange>,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
    pub file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRange {
    pub start: CursorPosition,
    pub end: CursorPosition,
}

/// Collaboration message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CollabMessage {
    #[serde(rename = "sync")]
    Sync { room_id: String, data: SyncMessage },
    #[serde(rename = "awareness")]
    Awareness {
        room_id: String,
        data: AwarenessMessage,
    },
    #[serde(rename = "join")]
    Join { room_id: String, user: CollabUser },
    #[serde(rename = "leave")]
    Leave { room_id: String, user_id: String },
    #[serde(rename = "cursor")]
    Cursor {
        room_id: String,
        user_id: String,
        cursor: CursorPosition,
    },
    #[serde(rename = "selection")]
    Selection {
        room_id: String,
        user_id: String,
        selection: SelectionRange,
    },
    #[serde(rename = "chat")]
    Chat {
        room_id: String,
        user_id: String,
        message: String,
        timestamp: u64,
    },
}

/// Sync message from Yjs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncMessage {
    pub update: Vec<u8>,
    pub vector_clock: HashMap<String, u64>,
}

/// Awareness message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwarenessMessage {
    pub user_id: String,
    pub state: HashMap<String, serde_json::Value>,
}

/// Collaboration manager
pub struct CollabManager {
    rooms: HashMap<String, CollabRoom>,
    local_user: CollabUser,
    message_tx: mpsc::Sender<CollabMessage>,
    message_rx: Option<mpsc::Receiver<CollabMessage>>,
    broadcast_tx: broadcast::Sender<CollabMessage>,
}

impl CollabManager {
    pub fn new(user_id: String, user_name: String) -> Self {
        let (message_tx, message_rx) = mpsc::channel(256);
        let (broadcast_tx, _) = broadcast::channel(256);

        let local_user = CollabUser {
            id: user_id,
            name: user_name,
            color: Self::generate_user_color(),
            cursor: None,
            selection: None,
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };

        Self {
            rooms: HashMap::new(),
            local_user,
            message_tx,
            message_rx: Some(message_rx),
            broadcast_tx,
        }
    }

    fn generate_user_color() -> String {
        let colors = [
            "#FF6B6B", "#4ECDC4", "#45B7D1", "#96CEB4", "#FFEAA7", "#DDA0DD", "#98D8C8", "#F7DC6F",
            "#BB8FCE", "#85C1E9", "#F8B500", "#00CED1",
        ];
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now().hash(&mut hasher);
        let idx = hasher.finish() as usize % colors.len();
        colors[idx].to_string()
    }

    /// Create or join a room
    pub async fn join_room(&mut self, room_id: &str) -> Result<&CollabRoom> {
        if !self.rooms.contains_key(room_id) {
            let room = CollabRoom::new(room_id.to_string());
            self.rooms.insert(room_id.to_string(), room);
            info!("Created room: {}", room_id);
        }

        let room = self
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| anyhow::anyhow!("Failed to get room after creation"))?;
        room.users
            .insert(self.local_user.id.clone(), self.local_user.clone());

        // Broadcast join
        let _ = self.broadcast_tx.send(CollabMessage::Join {
            room_id: room_id.to_string(),
            user: self.local_user.clone(),
        });

        self.rooms
            .get(room_id)
            .ok_or_else(|| anyhow::anyhow!("Room not found after join"))
    }

    /// Leave a room
    pub async fn leave_room(&mut self, room_id: &str) -> Result<()> {
        if let Some(room) = self.rooms.get_mut(room_id) {
            room.users.remove(&self.local_user.id);

            // Broadcast leave
            let _ = self.broadcast_tx.send(CollabMessage::Leave {
                room_id: room_id.to_string(),
                user_id: self.local_user.id.clone(),
            });
        }

        if let Some(room) = self.rooms.get(room_id) {
            if room.users.is_empty() {
                self.rooms.remove(room_id);
                info!("Removed empty room: {}", room_id);
            }
        }

        Ok(())
    }

    /// Update cursor position
    pub async fn update_cursor(&mut self, room_id: &str, cursor: CursorPosition) -> Result<()> {
        let room = self
            .rooms
            .get_mut(room_id)
            .ok_or_else(|| anyhow::anyhow!("Room not found: {}", room_id))?;

        let user = room
            .users
            .get_mut(&self.local_user.id)
            .ok_or_else(|| anyhow::anyhow!("Local user not found in room: {}", room_id))?;

        user.cursor = Some(cursor.clone());
        user.last_seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Update awareness
        let mut awareness = room.awareness.write().await;
        awareness.set_local_state_field("cursor", serde_json::to_value(&cursor)?);

        // Broadcast cursor update
        let _ = self.broadcast_tx.send(CollabMessage::Cursor {
            room_id: room_id.to_string(),
            user_id: self.local_user.id.clone(),
            cursor,
        });

        Ok(())
    }

    /// Update selection
    pub async fn update_selection(
        &mut self,
        room_id: &str,
        selection: SelectionRange,
    ) -> Result<()> {
        if let Some(room) = self.rooms.get_mut(room_id) {
            if let Some(user) = room.users.get_mut(&self.local_user.id) {
                user.selection = Some(selection.clone());
            }

            // Update awareness
            let mut awareness = room.awareness.write().await;
            awareness.set_local_state_field("selection", serde_json::to_value(&selection)?);
        }

        // Broadcast selection update
        let _ = self.broadcast_tx.send(CollabMessage::Selection {
            room_id: room_id.to_string(),
            user_id: self.local_user.id.clone(),
            selection,
        });

        Ok(())
    }

    /// Apply text update to document
    pub async fn apply_update(&mut self, room_id: &str, update: &[u8]) -> Result<()> {
        if let Some(room) = self.rooms.get(room_id) {
            let mut doc = room.document.write().await;
            doc.apply_update(update)?;

            // Broadcast sync
            let _ = self.message_tx.send(CollabMessage::Sync {
                room_id: room_id.to_string(),
                data: SyncMessage {
                    update: update.to_vec(),
                    vector_clock: doc.get_vector_clock(),
                },
            });
        }

        Ok(())
    }

    /// Get document content
    pub async fn get_document_content(&self, room_id: &str) -> Option<String> {
        if let Some(room) = self.rooms.get(room_id) {
            let doc = room.document.read().await;
            Some(doc.get_content())
        } else {
            None
        }
    }

    /// Get room users
    pub fn get_room_users(&self, room_id: &str) -> Option<Vec<CollabUser>> {
        self.rooms
            .get(room_id)
            .map(|r| r.users.values().cloned().collect())
    }

    /// Get message receiver
    pub fn take_message_receiver(&mut self) -> Option<mpsc::Receiver<CollabMessage>> {
        self.message_rx.take()
    }

    /// Subscribe to broadcast messages
    pub fn subscribe(&self) -> broadcast::Receiver<CollabMessage> {
        self.broadcast_tx.subscribe()
    }

    /// Handle incoming message
    pub async fn handle_message(&mut self, msg: CollabMessage) -> Result<()> {
        match msg {
            CollabMessage::Sync { room_id, data } => {
                if let Some(room) = self.rooms.get(&room_id) {
                    let mut doc = room.document.write().await;
                    doc.apply_update(&data.update)?;
                }
            }
            CollabMessage::Awareness { room_id, data } => {
                if let Some(room) = self.rooms.get(&room_id) {
                    let mut awareness = room.awareness.write().await;
                    awareness.set_state(data.user_id, data.state);
                }
            }
            CollabMessage::Join { room_id, user } => {
                if let Some(room) = self.rooms.get_mut(&room_id) {
                    room.users.insert(user.id.clone(), user);
                }
            }
            CollabMessage::Leave { room_id, user_id } => {
                if let Some(room) = self.rooms.get_mut(&room_id) {
                    room.users.remove(&user_id);

                    let mut awareness = room.awareness.write().await;
                    awareness.remove_state(&user_id);
                }
            }
            CollabMessage::Cursor {
                room_id,
                user_id,
                cursor,
            } => {
                if let Some(room) = self.rooms.get_mut(&room_id) {
                    if let Some(user) = room.users.get_mut(&user_id) {
                        user.cursor = Some(cursor);
                    }
                }
            }
            CollabMessage::Selection {
                room_id,
                user_id,
                selection,
            } => {
                if let Some(room) = self.rooms.get_mut(&room_id) {
                    if let Some(user) = room.users.get_mut(&user_id) {
                        user.selection = Some(selection);
                    }
                }
            }
            CollabMessage::Chat { .. } => {
                // Handle chat messages
            }
        }

        Ok(())
    }
}

/// Shared collaboration manager
pub type SharedCollabManager = Arc<RwLock<CollabManager>>;

lazy_static! {
    static ref COMMAND_COLLAB_MANAGER: SharedCollabManager = Arc::new(RwLock::new(
        CollabManager::new("local-user".to_string(), "Local User".to_string())
    ));
}

async fn apply_broadcast_cursor(
    manager: &mut CollabManager,
    room_id: &str,
    cursor: CursorPosition,
) -> Result<()> {
    if manager.get_room_users(room_id).is_none() {
        manager.join_room(room_id).await?;
    }

    manager.update_cursor(room_id, cursor).await
}

/// Broadcast the local user's cursor position within a collaboration room.
#[command]
pub async fn broadcast_cursor(room_id: String, cursor: CursorPosition) -> Result<(), String> {
    let mut collab_manager = COMMAND_COLLAB_MANAGER.write().await;
    apply_broadcast_cursor(&mut collab_manager, &room_id, cursor)
        .await
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[tokio::test]
    async fn broadcast_cursor_updates_cursor_for_existing_room() -> Result<()> {
        let mut manager = CollabManager::new("user-1".to_string(), "Alice".to_string());
        manager.join_room("room-1").await?;

        apply_broadcast_cursor(
            &mut manager,
            "room-1",
            super::CursorPosition {
                line: 7,
                column: 3,
                file: Some("src/main.rs".to_string()),
            },
        )
        .await?;

        let users = manager
            .get_room_users("room-1")
            .ok_or_else(|| anyhow::anyhow!("room should exist"))?;
        let current_user = users.iter().find(|user| user.id == "user-1");

        assert!(current_user.is_some());
        assert_eq!(
            current_user
                .and_then(|user| user.cursor.clone())
                .map(|cursor| cursor.line),
            Some(7)
        );
        Ok(())
    }

    #[tokio::test]
    async fn broadcast_cursor_creates_room_when_missing() -> Result<()> {
        let mut manager = CollabManager::new("user-1".to_string(), "Alice".to_string());

        apply_broadcast_cursor(
            &mut manager,
            "missing-room",
            super::CursorPosition {
                line: 1,
                column: 1,
                file: None,
            },
        )
        .await?;

        let users = manager
            .get_room_users("missing-room")
            .ok_or_else(|| anyhow::anyhow!("room should be created"))?;
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].cursor.as_ref().map(|cursor| cursor.column), Some(1));
        Ok(())
    }
}
