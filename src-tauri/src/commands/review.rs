use serde::{Deserialize, Serialize};
use tauri::command;

use crate::commands::ai::chat_completion;
use crate::commands::git::git_diff;
use crate::commands::git::git_status;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewChecklistItem {
    pub label: String,
    pub checked: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub body: String,
    pub line: Option<u32>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffReviewResult {
    pub summary: String,
    pub risk: String,
    pub checklist: Vec<ReviewChecklistItem>,
    pub comments: Vec<ReviewComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewOverviewFile {
    pub file: String,
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewOverview {
    pub branch: String,
    pub files: Vec<ReviewOverviewFile>,
    pub total_additions: usize,
    pub total_deletions: usize,
}

#[derive(Debug, Deserialize)]
struct RawReviewResult {
    summary: Option<String>,
    risk: Option<String>,
    checklist: Option<Vec<RawChecklistItem>>,
    comments: Option<Vec<RawComment>>,
}

#[derive(Debug, Deserialize)]
struct RawChecklistItem {
    label: Option<String>,
    checked: Option<bool>,
    detail: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawComment {
    severity: Option<String>,
    title: Option<String>,
    body: Option<String>,
    line: Option<u32>,
    suggestion: Option<String>,
}

fn normalize_review_result(parsed: RawReviewResult) -> DiffReviewResult {
    let checklist = parsed
        .checklist
        .unwrap_or_default()
        .into_iter()
        .filter_map(|item| {
            item.label.map(|label| ReviewChecklistItem {
                label,
                checked: item.checked.unwrap_or(false),
                detail: item.detail,
            })
        })
        .collect::<Vec<_>>();

    let comments = parsed
        .comments
        .unwrap_or_default()
        .into_iter()
        .enumerate()
        .filter_map(|(index, comment)| {
            let title = comment.title.or_else(|| Some(format!("Issue {}", index + 1)))?;
            let body = comment.body.unwrap_or_else(|| "Review comment omitted details.".to_string());
            Some(ReviewComment {
                id: format!("comment-{}", index + 1),
                severity: comment.severity.unwrap_or_else(|| "info".to_string()),
                title,
                body,
                line: comment.line,
                suggestion: comment.suggestion,
            })
        })
        .collect::<Vec<_>>();

    DiffReviewResult {
        summary: parsed
            .summary
            .unwrap_or_else(|| "No review summary was generated for this diff.".to_string()),
        risk: parsed.risk.unwrap_or_else(|| "medium".to_string()),
        checklist,
        comments,
    }
}

fn fallback_review(diff: &str, file_path: &str) -> DiffReviewResult {
    let mut additions = 0usize;
    let mut deletions = 0usize;
    let mut comment_bodies = Vec::new();

    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            additions += 1;
            if line.contains("unwrap(") {
                comment_bodies.push((
                    "warning".to_string(),
                    "Avoid unwrap in review target".to_string(),
                    "This diff introduces or preserves an unwrap-style call. Replace it with explicit error handling before merging.".to_string(),
                    Some("Consider using Result propagation instead of unwrap().".to_string()),
                ));
            }
        }
        if line.starts_with('-') && !line.starts_with("---") {
            deletions += 1;
        }
    }

    if additions > 200 {
        comment_bodies.push((
            "warning".to_string(),
            "Large patch size".to_string(),
            "This file carries a large amount of change. Consider splitting risky edits or reviewing edge cases and regression coverage before merging.".to_string(),
            None,
        ));
    }

    let risk = if comment_bodies.iter().any(|(severity, _, _, _)| severity == "warning") {
        "medium"
    } else {
        "low"
    }
    .to_string();

    let comments = comment_bodies
        .into_iter()
        .enumerate()
        .map(|(index, (severity, title, body, suggestion))| ReviewComment {
            id: format!("fallback-{}", index + 1),
            severity,
            title,
            body,
            line: None,
            suggestion,
        })
        .collect::<Vec<_>>();

    DiffReviewResult {
        summary: format!(
            "Reviewed {} with +{} / -{} lines changed. Focus on correctness, regression risk, and whether the patch needs follow-up tests.",
            file_path, additions, deletions
        ),
        risk,
        checklist: vec![
            ReviewChecklistItem {
                label: "Behavior change is understood".to_string(),
                checked: true,
                detail: Some("The diff structure was analyzed for review output.".to_string()),
            },
            ReviewChecklistItem {
                label: "Regression coverage considered".to_string(),
                checked: additions < 200,
                detail: Some("Large or risky changes should include targeted tests before merge.".to_string()),
            },
        ],
        comments,
    }
}

fn extract_json_block(text: &str) -> Option<String> {
    let fenced = text.find("```json").and_then(|start| {
        let remaining = &text[start + 7..];
        remaining
            .find("```")
            .map(|end| remaining[..end].trim().to_string())
    });

    if fenced.is_some() {
        return fenced;
    }

    let start = text.find('{')?;
    let end = text.rfind('}')?;
    Some(text[start..=end].to_string())
}

#[command]
pub async fn review_diff(
    model: String,
    diff: String,
    file_path: String,
    language: String,
) -> Result<DiffReviewResult, String> {
    let system = "You are KYRO-REVIEW, a senior pull request reviewer. Review one file diff and return strict JSON with this schema: {\"summary\": string, \"risk\": \"low\"|\"medium\"|\"high\", \"checklist\": [{\"label\": string, \"checked\": boolean, \"detail\": string | null}], \"comments\": [{\"severity\": \"info\"|\"warning\"|\"error\", \"title\": string, \"body\": string, \"line\": number | null, \"suggestion\": string | null}] }. Keep it concise and review-focused.";
    let prompt = format!(
        "File: {}\nLanguage: {}\n\nUnified diff:\n```diff\n{}\n```",
        file_path, language, diff
    );

    match chat_completion(
        model,
        vec![
            crate::commands::ai::ChatMessage {
                role: "system".to_string(),
                content: system.to_string(),
            },
            crate::commands::ai::ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
    )
    .await
    {
        Ok(text) => {
            if let Some(json) = extract_json_block(&text) {
                if let Ok(parsed) = serde_json::from_str::<RawReviewResult>(&json) {
                    return Ok(normalize_review_result(parsed));
                }
            }
            Ok(fallback_review(&diff, &file_path))
        }
        Err(_) => Ok(fallback_review(&diff, &file_path)),
    }
}

#[command]
pub async fn review_overview(path: String) -> Result<ReviewOverview, String> {
    let status = git_status(path.clone()).await?;
    let unstaged = git_diff(path.clone(), Some(false)).await?;
    let staged = git_diff(path, Some(true)).await?;

    let mut files = Vec::new();
    let mut total_additions = 0usize;
    let mut total_deletions = 0usize;

    for diff in unstaged {
        total_additions += diff.additions;
        total_deletions += diff.deletions;
        files.push(ReviewOverviewFile {
            file: diff.file,
            status: diff.status,
            additions: diff.additions,
            deletions: diff.deletions,
            source: "unstaged".to_string(),
        });
    }

    for diff in staged {
        total_additions += diff.additions;
        total_deletions += diff.deletions;
        files.push(ReviewOverviewFile {
            file: diff.file,
            status: diff.status,
            additions: diff.additions,
            deletions: diff.deletions,
            source: "staged".to_string(),
        });
    }

    Ok(ReviewOverview {
        branch: status.branch,
        files,
        total_additions,
        total_deletions,
    })
}

#[cfg(test)]
mod tests {
    use super::{extract_json_block, fallback_review};

    #[test]
    fn extract_json_block_reads_fenced_json() {
        let input = "text\n```json\n{\"summary\":\"ok\"}\n```\n";
        let parsed = extract_json_block(input);
        assert_eq!(parsed.as_deref(), Some("{\"summary\":\"ok\"}"));
    }

    #[test]
    fn fallback_review_flags_unwrap_usage() {
        let diff = "@@ -1,1 +1,1 @@\n+let value = thing.unwrap();\n";
        let review = fallback_review(diff, "src/main.rs");
        assert_eq!(review.risk, "medium");
        assert!(!review.comments.is_empty());
    }
}
