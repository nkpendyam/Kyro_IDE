// Chat Agent Types — shared types for chat agent features.
// Command implementations live in commands/ai.rs (detect_ai_backends, smart_ai_completion).
use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatAgentConfig {
    pub model: String,
    pub system_prompt: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatAgentMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatAgentResponse {
    pub message: String,
    pub tokens_used: u32,
    pub model: String,
}

#[command]
pub async fn chat_agent_detect_backends() -> Result<Vec<crate::commands::ai::BackendStatus>, String>
{
    crate::commands::ai::detect_ai_backends().await
}

#[command]
pub async fn chat_agent_smart_completion(
    prompt: String,
    system_prompt: Option<String>,
    context: Option<String>,
    history: Vec<crate::commands::ai::ChatMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
) -> Result<crate::commands::ai::SmartCompletionResult, String> {
    crate::commands::ai::smart_ai_completion(
        prompt,
        system_prompt,
        context,
        history,
        temperature,
        max_tokens,
    )
    .await
}

#[command]
pub async fn chat_agent_inline_edit(
    prompt: String,
    selected_code: String,
    context: String,
) -> Result<crate::commands::ai::InlineEditResult, String> {
    crate::commands::ai::ai_inline_edit(prompt, selected_code, context).await
}

#[command]
pub async fn chat_agent_create_session(project_path: String) -> Result<String, String> {
    crate::commands::ai::create_chat_session(project_path).await
}

#[command]
pub async fn chat_agent_rag_chat(
    session_id: String,
    message: String,
    context: serde_json::Value,
) -> Result<crate::commands::ai::RagChatResponse, String> {
    crate::commands::ai::rag_chat(session_id, message, context).await
}

#[command]
pub async fn chat_agent_run_command(
    command_text: String,
    context: crate::commands::ai::AgentContext,
) -> Result<crate::commands::ai::AgentResult, String> {
    crate::commands::ai::agent_command(command_text, context).await
}

#[command]
pub async fn chat_agent_approve(approval_id: String) -> Result<(), String> {
    crate::commands::ai::agent_approve(approval_id).await
}

#[command]
pub async fn chat_agent_reject(approval_id: String) -> Result<(), String> {
    crate::commands::ai::agent_reject(approval_id).await
}
