//! Autonomous Execution Commands
//!
//! Tauri commands that expose the autonomous executor to the frontend.
//! Allows the IDE to run multi-step LLM-driven tasks (read, write, edit,
//! run terminal, generate code) based on a structured plan.

use crate::autonomous::executor::{ExecutionResult, Executor};
use crate::autonomous::planner::{PlanStep, PruningStrategy, StepStatus};
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Emitter};

/// Execute a single autonomous plan step.
///
/// The executor dispatches to the appropriate tool:
/// `read_file`, `write_file`, `list_dir`, `ast_prune`,
/// `run_terminal`, `llm_generate`, `apply_edit`.
#[command]
pub async fn execute_step(step: PlanStep) -> Result<ExecutionResult, String> {
    let executor = Executor::default();
    Ok(executor.execute(&step).await)
}

/// Execute a series of plan steps sequentially, respecting dependency order.
///
/// Steps whose dependencies have not yet succeeded are skipped.
/// Emits `quest-progress` events on `app` so `AgentStreamPanel` can stream live updates.
#[command]
pub async fn execute_plan(
    app: AppHandle,
    steps: Vec<PlanStep>,
) -> Result<Vec<ExecutionResult>, String> {
    let executor = Executor::default();
    let mut results: Vec<ExecutionResult> = Vec::new();
    let mut completed_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let total = steps.len();

    for (i, step) in steps.iter().enumerate() {
        // Skip if any dependency failed
        let deps_met = step
            .dependencies
            .iter()
            .all(|dep_id| completed_ids.contains(dep_id));

        if !deps_met {
            let _ = app.emit(
                "quest-progress",
                serde_json::json!({
                    "mission_id": &step.id,
                    "phase": "skipped",
                    "step_index": i,
                    "step_total": total,
                    "step_description": &step.description,
                    "status": "failed",
                    "message": format!("Skipped: dependencies not met: {:?}", step.dependencies),
                }),
            );
            results.push(ExecutionResult {
                step_id: step.id.clone(),
                success: false,
                output: None,
                error: Some(format!(
                    "Skipped: dependencies not met: {:?}",
                    step.dependencies
                )),
                duration_ms: 0,
            });
            continue;
        }

        // Emit "running" event before execution
        let _ = app.emit(
            "quest-progress",
            serde_json::json!({
                "mission_id": &step.id,
                "phase": "executing",
                "step_index": i,
                "step_total": total,
                "step_description": &step.description,
                "status": "running",
                "message": format!("Running step {}/{}: {}", i + 1, total, step.description),
            }),
        );

        let result = executor.execute(step).await;
        let success = result.success;
        let msg = result
            .output
            .clone()
            .unwrap_or_else(|| result.error.clone().unwrap_or_default());

        // Emit "done" or "failed" event after execution
        let _ = app.emit(
            "quest-progress",
            serde_json::json!({
                "mission_id": &step.id,
                "phase": "executing",
                "step_index": i + 1,
                "step_total": total,
                "step_description": &step.description,
                "status": if success { "done" } else { "failed" },
                "message": msg,
            }),
        );

        if success {
            completed_ids.insert(step.id.clone());
        }
        results.push(result);
    }

    // Final completion event
    let all_ok = results.iter().all(|r| r.success);
    let _ = app.emit(
        "quest-progress",
        serde_json::json!({
            "mission_id": "plan",
            "phase": "complete",
            "step_index": total,
            "step_total": total,
            "step_description": "Plan complete",
            "status": if all_ok { "done" } else { "failed" },
            "message": format!("{}/{} steps succeeded", results.iter().filter(|r| r.success).count(), total),
        }),
    );

    Ok(results)
}

/// Build a default set of plan steps for a natural-language goal.
///
/// Returns a ready-to-execute plan that the frontend can display
/// for user review before calling `execute_plan`.
#[command]
pub fn plan_task(goal: String) -> Result<Vec<PlanStep>, String> {
    use uuid::Uuid;

    let steps = vec![
        PlanStep {
            id: Uuid::new_v4().to_string(),
            description: format!("Analyze codebase for: {}", goal),
            tool_name: Some("read_file".to_string()),
            tool_args: serde_json::json!({ "path": "." }),
            dependencies: vec![],
            status: StepStatus::Pending,
            pruning_strategy: PruningStrategy::ImportsOnly,
        },
        PlanStep {
            id: Uuid::new_v4().to_string(),
            description: format!("Generate implementation plan for: {}", goal),
            tool_name: Some("llm_generate".to_string()),
            tool_args: serde_json::json!({
                "prompt": format!(
                    "You are a code assistant. Generate a step-by-step implementation plan for the following task:\n\n{}\n\nRespond with a concise numbered list of concrete code changes needed.",
                    goal
                ),
                "max_tokens": 1024
            }),
            dependencies: vec![],
            status: StepStatus::Pending,
            pruning_strategy: PruningStrategy::None,
        },
    ];

    Ok(steps)
}

/// Lightweight status type returned by `autonomous_status`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutorStatus {
    pub available_tools: Vec<String>,
    pub llm_available: bool,
}

/// Return the executor's available tools and LLM reachability.
#[command]
pub async fn autonomous_status() -> Result<ExecutorStatus, String> {
    let executor = Executor::default();
    let llm_available = crate::ai::AiClient::new().is_available().await;
    Ok(ExecutorStatus {
        available_tools: executor.allowed_tools.clone(),
        llm_available,
    })
}
