//! Test Runner Commands
//!
//! Execute project test suites and parse results.
//! Detects the project type (Rust/Node/Python/Go) and runs the appropriate
//! test command, streaming results via Tauri events.

use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::{command, AppHandle, Emitter};

/// Test run result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRunResult {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_ms: u64,
    pub output: String,
    pub test_results: Vec<SingleTestResult>,
    pub success: bool,
}

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleTestResult {
    pub name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

/// Detected project type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Unknown,
}

/// Detect project type from root path
fn detect_project_type(project_path: &str) -> ProjectType {
    let root = Path::new(project_path);
    if root.join("Cargo.toml").exists() {
        ProjectType::Rust
    } else if root.join("package.json").exists() {
        ProjectType::Node
    } else if root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join("requirements.txt").exists()
    {
        ProjectType::Python
    } else if root.join("go.mod").exists() {
        ProjectType::Go
    } else {
        ProjectType::Unknown
    }
}

/// Get the default test command for a project type
fn default_test_command(project_type: &ProjectType) -> &'static str {
    match project_type {
        ProjectType::Rust => "cargo test",
        ProjectType::Node => "npx vitest run",
        ProjectType::Python => "python -m pytest -v",
        ProjectType::Go => "go test ./...",
        ProjectType::Unknown => "echo 'No test framework detected'",
    }
}

// ============ Tauri Commands ============

/// Detect the project type
#[command]
pub fn detect_test_framework(project_path: String) -> Result<serde_json::Value, String> {
    let pt = detect_project_type(&project_path);
    Ok(serde_json::json!({
        "type": format!("{:?}", pt),
        "command": default_test_command(&pt),
    }))
}

/// Run the project's test suite
#[command]
pub async fn run_tests(
    app: AppHandle,
    project_path: String,
    custom_command: Option<String>,
    test_filter: Option<String>,
) -> Result<TestRunResult, String> {
    let pt = detect_project_type(&project_path);
    let base_cmd = custom_command.unwrap_or_else(|| default_test_command(&pt).to_string());

    // Append filter if provided
    let cmd = match (&pt, &test_filter) {
        (ProjectType::Rust, Some(filter)) => format!("{} -- {}", base_cmd, filter),
        (ProjectType::Node, Some(filter)) => format!("{} -t \"{}\"", base_cmd, filter),
        (ProjectType::Python, Some(filter)) => format!("{} -k \"{}\"", base_cmd, filter),
        (ProjectType::Go, Some(filter)) => format!("{} -run \"{}\"", base_cmd, filter),
        _ => base_cmd,
    };

    let _ = app.emit("test-run-started", serde_json::json!({ "command": &cmd }));

    let start = std::time::Instant::now();

    // Run the test command
    let output = tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
        .args(if cfg!(windows) {
            vec!["/C", &cmd]
        } else {
            vec!["-c", &cmd]
        })
        .current_dir(&project_path)
        .output()
        .await
        .map_err(|e| format!("Failed to run tests: {}", e))?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let full_output = format!("{}\n{}", stdout, stderr);
    let success = output.status.success();

    // Parse test results from output
    let test_results = parse_test_output(&full_output, &pt);
    let passed = test_results
        .iter()
        .filter(|t| matches!(t.status, TestStatus::Passed))
        .count() as u32;
    let failed = test_results
        .iter()
        .filter(|t| matches!(t.status, TestStatus::Failed))
        .count() as u32;
    let skipped = test_results
        .iter()
        .filter(|t| matches!(t.status, TestStatus::Skipped))
        .count() as u32;
    let total = test_results.len() as u32;

    let result = TestRunResult {
        total,
        passed,
        failed,
        skipped,
        duration_ms,
        output: full_output,
        test_results,
        success,
    };

    let _ = app.emit(
        "test-run-complete",
        serde_json::json!({
            "suite": project_path,
            "total": result.total,
            "passed": result.passed,
            "failed": result.failed,
            "success": result.success,
            "duration_ms": result.duration_ms,
            "results": result.test_results.iter().map(|t| serde_json::json!({
                "name": t.name,
                "passed": matches!(t.status, TestStatus::Passed),
                "duration_ms": t.duration_ms,
                "output": t.message.clone().unwrap_or_default(),
            })).collect::<Vec<_>>(),
        }),
    );

    Ok(result)
}

/// Parse test output into individual results (best-effort)
fn parse_test_output(output: &str, project_type: &ProjectType) -> Vec<SingleTestResult> {
    let mut results = Vec::new();

    match project_type {
        ProjectType::Rust => {
            // Parse "test module::test_name ... ok" or "... FAILED"
            for line in output.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("test ")
                    && (trimmed.ends_with("... ok")
                        || trimmed.ends_with("... FAILED")
                        || trimmed.contains("... ignored"))
                {
                    let name = trimmed.strip_prefix("test ").unwrap_or(trimmed);
                    let (name, status) = if name.ends_with("... ok") {
                        (
                            name.strip_suffix(" ... ok").unwrap_or(name),
                            TestStatus::Passed,
                        )
                    } else if name.ends_with("... FAILED") {
                        (
                            name.strip_suffix(" ... FAILED").unwrap_or(name),
                            TestStatus::Failed,
                        )
                    } else {
                        (
                            name.strip_suffix(" ... ignored").unwrap_or(name),
                            TestStatus::Skipped,
                        )
                    };
                    results.push(SingleTestResult {
                        name: name.to_string(),
                        status,
                        duration_ms: 0,
                        message: None,
                    });
                }
            }
        }
        ProjectType::Node => {
            // Parse vitest/jest: "✓ test name" or "✗ test name"
            for line in output.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("✓") || trimmed.starts_with("√") {
                    results.push(SingleTestResult {
                        name: trimmed[3..].trim().to_string(),
                        status: TestStatus::Passed,
                        duration_ms: 0,
                        message: None,
                    });
                } else if trimmed.starts_with("✗") || trimmed.starts_with("×") {
                    results.push(SingleTestResult {
                        name: trimmed[3..].trim().to_string(),
                        status: TestStatus::Failed,
                        duration_ms: 0,
                        message: None,
                    });
                }
            }
        }
        ProjectType::Python => {
            // Parse pytest: "test_file.py::test_name PASSED" or "FAILED"
            for line in output.lines() {
                let trimmed = line.trim();
                if trimmed.contains("PASSED")
                    || trimmed.contains("FAILED")
                    || trimmed.contains("SKIPPED")
                {
                    let parts: Vec<&str> = trimmed.rsplitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let status = match parts[0] {
                            "PASSED" => TestStatus::Passed,
                            "FAILED" => TestStatus::Failed,
                            "SKIPPED" => TestStatus::Skipped,
                            _ => continue,
                        };
                        results.push(SingleTestResult {
                            name: parts[1].to_string(),
                            status,
                            duration_ms: 0,
                            message: None,
                        });
                    }
                }
            }
        }
        ProjectType::Go => {
            // Parse "--- PASS: TestName" or "--- FAIL: TestName"
            for line in output.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("--- PASS:") {
                    let name = trimmed.strip_prefix("--- PASS: ").unwrap_or(trimmed);
                    let name = name.split_whitespace().next().unwrap_or(name);
                    results.push(SingleTestResult {
                        name: name.to_string(),
                        status: TestStatus::Passed,
                        duration_ms: 0,
                        message: None,
                    });
                } else if trimmed.starts_with("--- FAIL:") {
                    let name = trimmed.strip_prefix("--- FAIL: ").unwrap_or(trimmed);
                    let name = name.split_whitespace().next().unwrap_or(name);
                    results.push(SingleTestResult {
                        name: name.to_string(),
                        status: TestStatus::Failed,
                        duration_ms: 0,
                        message: None,
                    });
                }
            }
        }
        ProjectType::Unknown => {}
    }

    results
}
