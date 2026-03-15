// Authentication Tauri Commands — Real session tracking implementation
use crate::auth::{AuthConfig, AuthManager, UserRole};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::command;
use tokio::sync::RwLock;

struct AuthSession {
    manager: AuthManager,
    current_user: Option<UserInfo>,
}

lazy_static::lazy_static! {
    static ref AUTH_STATE: Arc<RwLock<AuthSession>> = Arc::new(RwLock::new(AuthSession {
        manager: AuthManager::new(AuthConfig::default()),
        current_user: None,
    }));
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: String,
    pub avatar_url: Option<String>,
}

fn role_from_str(role: &str) -> Result<UserRole, String> {
    match role.to_lowercase().as_str() {
        "admin" => Ok(UserRole::Admin),
        "editor" => Ok(UserRole::Editor),
        "viewer" => Ok(UserRole::Viewer),
        "owner" => Ok(UserRole::Owner),
        "guest" => Ok(UserRole::Guest),
        _ => Err(format!("Unknown role: {}", role)),
    }
}

#[command]
pub async fn login_user(username: String, password: String) -> Result<UserInfo, String> {
    let mut state = AUTH_STATE.write().await;
    let tokens = state
        .manager
        .login(&username, &password, None)
        .map_err(|e| format!("Login failed: {}", e))?;
    let user = UserInfo {
        id: tokens.user.id.to_string(),
        username: tokens.user.username.clone(),
        email: tokens.user.email.clone(),
        role: format!("{:?}", tokens.user.role),
        avatar_url: None,
    };
    state.current_user = Some(user.clone());
    Ok(user)
}

#[command]
pub async fn logout_user() -> Result<(), String> {
    let mut state = AUTH_STATE.write().await;
    state.current_user = None;
    Ok(())
}

#[command]
pub async fn register_user(
    username: String,
    email: String,
    password: String,
) -> Result<UserInfo, String> {
    let mut state = AUTH_STATE.write().await;
    let user = state
        .manager
        .register(username, email, &password)
        .map_err(|e| format!("Registration failed: {}", e))?;
    let info = UserInfo {
        id: user.id.to_string(),
        username: user.username.clone(),
        email: user.email.clone(),
        role: format!("{:?}", user.role),
        avatar_url: None,
    };
    state.current_user = Some(info.clone());
    Ok(info)
}

#[command]
pub async fn get_current_user() -> Result<Option<UserInfo>, String> {
    let state = AUTH_STATE.read().await;
    Ok(state.current_user.clone())
}

#[command]
pub async fn is_authenticated() -> Result<bool, String> {
    let state = AUTH_STATE.read().await;
    Ok(state.current_user.is_some())
}

#[command]
pub async fn update_user_role(user_id: String, role: String) -> Result<(), String> {
    let mut state = AUTH_STATE.write().await;
    let current_user = state
        .current_user
        .clone()
        .ok_or_else(|| "Authentication required".to_string())?;
    let admin_id = uuid::Uuid::parse_str(&current_user.id)
        .map_err(|e| format!("Invalid current user UUID: {}", e))?;
    let uid = uuid::Uuid::parse_str(&user_id).map_err(|e| format!("Invalid UUID: {}", e))?;
    let new_role = role_from_str(&role)?;
    state
        .manager
        .change_role(admin_id, uid, new_role)
        .map_err(|e| format!("Failed: {}", e))
}

#[command]
pub async fn validate_session(token: String) -> Result<bool, String> {
    let state = AUTH_STATE.read().await;
    Ok(state.manager.validate_token(&token).is_ok())
}

#[command]
pub async fn get_oauth_url(provider: String) -> Result<String, String> {
    let state = AUTH_STATE.read().await;
    state
        .manager
        .get_oauth_url(&provider)
        .map_err(|e| format!("Failed: {}", e))
}

#[command]
pub async fn handle_oauth_callback(provider: String, code: String) -> Result<UserInfo, String> {
    let mut state = AUTH_STATE.write().await;
    let tokens = state
        .manager
        .handle_oauth_callback(&provider, &code)
        .map_err(|e| format!("OAuth failed: {}", e))?;
    let user = UserInfo {
        id: tokens.user.id.to_string(),
        username: tokens.user.username.clone(),
        email: tokens.user.email.clone(),
        role: format!("{:?}", tokens.user.role),
        avatar_url: None,
    };
    state.current_user = Some(user.clone());
    Ok(user)
}
