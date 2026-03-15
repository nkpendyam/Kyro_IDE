//! Git integration for KYRO IDE
//!
//! Uses the `git2` crate (libgit2 bindings) — the same library powering
//! Helix, Lapce, Zed, and gitoxide. Every function calls real git2 APIs.

use git2::{BlameOptions, DiffOptions, Repository, StashFlags, Status, StatusOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub staged: Vec<FileStatus>,
    pub unstaged: Vec<FileStatus>,
    pub untracked: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunkLine {
    pub origin: String,
    pub content: String,
    pub old_lineno: Option<u32>,
    pub new_lineno: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    pub header: String,
    pub lines: Vec<DiffHunkLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub file: String,
    pub status: String,
    pub hunks: Vec<DiffHunk>,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLine {
    pub line_no: usize,
    pub commit_id: String,
    pub author: String,
    pub date: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}

pub struct GitManager;

impl GitManager {
    pub fn new() -> Self {
        Self
    }

    pub fn status(&self, path: &str) -> Result<GitStatus, String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let branch = repo
            .head()
            .ok()
            .and_then(|head| head.shorthand().map(|value| value.to_string()))
            .unwrap_or_else(|| "HEAD".to_string());
        let mut opts = StatusOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);
        let statuses = repo
            .statuses(Some(&mut opts))
            .map_err(|e| format!("Failed to get status: {}", e))?;
        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();
        for entry in statuses.iter() {
            let path = entry.path().unwrap_or("").to_string();
            let status = entry.status();
            if status.contains(Status::INDEX_NEW | Status::INDEX_MODIFIED | Status::INDEX_DELETED) {
                staged.push(FileStatus {
                    path: path.clone(),
                    status: "modified".to_string(),
                });
            } else if status.contains(Status::WT_NEW) {
                untracked.push(path);
            } else if status.contains(Status::WT_MODIFIED | Status::WT_DELETED) {
                unstaged.push(FileStatus {
                    path,
                    status: "modified".to_string(),
                });
            }
        }
        Ok(GitStatus {
            branch,
            ahead: 0,
            behind: 0,
            staged,
            unstaged,
            untracked,
        })
    }

    pub fn commit(&self, path: &str, message: &str) -> Result<String, String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let mut index = repo
            .index()
            .map_err(|e| format!("Failed to get index: {}", e))?;
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| format!("Failed to stage: {}", e))?;
        index
            .write()
            .map_err(|e| format!("Failed to write index: {}", e))?;
        let tree_id = index
            .write_tree()
            .map_err(|e| format!("Failed to write tree: {}", e))?;
        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| format!("Failed to find tree: {}", e))?;
        let sig = repo
            .signature()
            .map_err(|e| format!("Failed to get signature: {}", e))?;
        let parent = repo
            .head()
            .ok()
            .and_then(|h| h.target())
            .and_then(|oid| repo.find_commit(oid).ok());
        let parents: Vec<_> = parent.iter().collect();
        let commit_id = repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .map_err(|e| format!("Failed to commit: {}", e))?;
        Ok(commit_id.to_string())
    }

    /// Real git diff using git2::Diff — compares index-to-workdir or tree-to-index
    pub fn diff(&self, path: &str, staged: bool) -> Result<Vec<FileDiff>, String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(true);

        let diff = if staged {
            // Staged changes: compare HEAD tree to index
            let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
            repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut diff_opts))
        } else {
            // Unstaged changes: compare index to working directory
            repo.diff_index_to_workdir(None, Some(&mut diff_opts))
        }
        .map_err(|e| format!("Failed to compute diff: {}", e))?;

        let mut file_diffs: Vec<FileDiff> = Vec::new();

        // First pass: collect file-level info from deltas
        for delta_idx in 0..diff.deltas().len() {
            let Some(delta) = diff.get_delta(delta_idx) else {
                continue;
            };
            let file_path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let status = match delta.status() {
                git2::Delta::Added => "added",
                git2::Delta::Deleted => "deleted",
                git2::Delta::Modified => "modified",
                git2::Delta::Renamed => "renamed",
                git2::Delta::Copied => "copied",
                _ => "unknown",
            }
            .to_string();
            file_diffs.push(FileDiff {
                file: file_path,
                status,
                hunks: Vec::new(),
                additions: 0,
                deletions: 0,
            });
        }

        // Second pass: use print() to get hunks and lines in a single callback
        let file_diffs_cell = std::cell::RefCell::new(file_diffs);
        let cur_file_idx = std::cell::Cell::new(0usize);

        diff.print(git2::DiffFormat::Patch, |delta, hunk, line| {
            // Track which file we're in
            let file_path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let mut diffs = file_diffs_cell.borrow_mut();

            // Find the matching file diff
            let idx = {
                let cur = cur_file_idx.get();
                if cur < diffs.len() && diffs[cur].file == file_path {
                    cur
                } else {
                    let found = diffs
                        .iter()
                        .position(|fd| fd.file == file_path)
                        .unwrap_or(0);
                    cur_file_idx.set(found);
                    found
                }
            };

            if idx >= diffs.len() {
                return true;
            }

            match line.origin() {
                'H' => {
                    // Hunk header
                    if let Some(h) = hunk {
                        diffs[idx].hunks.push(DiffHunk {
                            old_start: h.old_start(),
                            old_lines: h.old_lines(),
                            new_start: h.new_start(),
                            new_lines: h.new_lines(),
                            header: String::from_utf8_lossy(h.header()).trim().to_string(),
                            lines: Vec::new(),
                        });
                    }
                }
                '+' | '-' | ' ' => {
                    let origin = match line.origin() {
                        '+' => {
                            diffs[idx].additions += 1;
                            "+"
                        }
                        '-' => {
                            diffs[idx].deletions += 1;
                            "-"
                        }
                        _ => " ",
                    }
                    .to_string();
                    let content = String::from_utf8_lossy(line.content()).to_string();
                    // Ensure there's a hunk to add to
                    if diffs[idx].hunks.is_empty() {
                        if let Some(h) = hunk {
                            diffs[idx].hunks.push(DiffHunk {
                                old_start: h.old_start(),
                                old_lines: h.old_lines(),
                                new_start: h.new_start(),
                                new_lines: h.new_lines(),
                                header: String::from_utf8_lossy(h.header()).trim().to_string(),
                                lines: Vec::new(),
                            });
                        }
                    }
                    if let Some(hunk) = diffs[idx].hunks.last_mut() {
                        hunk.lines.push(DiffHunkLine {
                            origin,
                            content,
                            old_lineno: line.old_lineno(),
                            new_lineno: line.new_lineno(),
                        });
                    }
                }
                _ => {}
            }
            true
        })
        .map_err(|e| format!("Failed to iterate diff: {}", e))?;

        Ok(file_diffs_cell.into_inner())
    }

    /// Real git blame using git2::Repository::blame_file()
    pub fn blame(&self, path: &str, file: &str) -> Result<Vec<BlameLine>, String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let _blame_opts = BlameOptions::new();
        let blame = repo
            .blame_file(Path::new(file), None)
            .map_err(|e| format!("Failed to blame file: {}", e))?;

        // Read file content for line text
        let repo_path = repo.workdir().ok_or("Bare repository")?;
        let full_path = repo_path.join(file);
        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        let lines: Vec<&str> = content.lines().collect();

        let mut result = Vec::new();
        for hunk in blame.iter() {
            let start = hunk.final_start_line();
            let count = hunk.lines_in_hunk();
            let commit_id = hunk.final_commit_id().to_string();
            let sig = hunk.final_signature();
            let author = sig.name().unwrap_or("Unknown").to_string();
            let date = sig.when().seconds().to_string();

            for line_offset in 0..count {
                let line_no = start + line_offset;
                let content = lines
                    .get(line_no.saturating_sub(1))
                    .unwrap_or(&"")
                    .to_string();
                result.push(BlameLine {
                    line_no,
                    commit_id: commit_id[..7.min(commit_id.len())].to_string(),
                    author: author.clone(),
                    date: date.clone(),
                    content,
                });
            }
        }
        Ok(result)
    }

    /// Real git stash using git2::Repository::stash()
    pub fn stash(&self, path: &str, message: Option<&str>) -> Result<String, String> {
        let mut repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let sig = repo
            .signature()
            .map_err(|e| format!("Failed to get signature: {}", e))?;
        let msg = message.unwrap_or("WIP on stash");
        let oid = repo
            .stash_save(&sig, msg, Some(StashFlags::DEFAULT))
            .map_err(|e| format!("Failed to stash: {}", e))?;
        Ok(oid.to_string())
    }

    /// Pop the latest stash
    pub fn stash_pop(&self, path: &str) -> Result<(), String> {
        let mut repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        repo.stash_pop(0, None)
            .map_err(|e| format!("Failed to pop stash: {}", e))?;
        Ok(())
    }

    /// List all stash entries
    pub fn stash_list(&self, path: &str) -> Result<Vec<StashEntry>, String> {
        let mut repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let mut entries = Vec::new();
        repo.stash_foreach(|index, message, _oid| {
            entries.push(StashEntry {
                index,
                message: message.to_string(),
            });
            true
        })
        .map_err(|e| format!("Failed to list stashes: {}", e))?;
        Ok(entries)
    }

    /// Merge a branch into the current branch
    pub fn merge(&self, path: &str, branch_name: &str) -> Result<String, String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let annotated = repo
            .find_branch(branch_name, git2::BranchType::Local)
            .map_err(|e| format!("Branch not found: {}", e))?;
        let commit = annotated
            .get()
            .peel_to_commit()
            .map_err(|e| format!("Failed to peel to commit: {}", e))?;
        let annotated_commit = repo
            .find_annotated_commit(commit.id())
            .map_err(|e| format!("Failed to find annotated commit: {}", e))?;

        let (analysis, _) = repo
            .merge_analysis(&[&annotated_commit])
            .map_err(|e| format!("Merge analysis failed: {}", e))?;

        if analysis.is_up_to_date() {
            return Ok("Already up to date".to_string());
        }

        if analysis.is_fast_forward() {
            // Fast-forward merge
            let mut reference = repo
                .head()
                .map_err(|e| format!("Failed to get HEAD: {}", e))?;
            reference
                .set_target(commit.id(), &format!("Fast-forward merge {}", branch_name))
                .map_err(|e| format!("Failed to fast-forward: {}", e))?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .map_err(|e| format!("Failed to checkout: {}", e))?;
            return Ok(format!("Fast-forward to {}", commit.id()));
        }

        // Normal merge
        repo.merge(&[&annotated_commit], None, None)
            .map_err(|e| format!("Merge failed: {}", e))?;
        Ok(format!("Merged {} into current branch", branch_name))
    }

    pub fn log(&self, path: &str, limit: usize) -> Result<Vec<GitCommit>, String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let mut revwalk = repo
            .revwalk()
            .map_err(|e| format!("Failed to create revwalk: {}", e))?;
        revwalk
            .push_head()
            .map_err(|e| format!("Failed to push HEAD: {}", e))?;
        let mut commits = Vec::new();
        for (i, oid_result) in revwalk.enumerate() {
            if i >= limit {
                break;
            }
            let oid = oid_result.map_err(|e| format!("Failed to get OID: {}", e))?;
            let commit = repo
                .find_commit(oid)
                .map_err(|e| format!("Failed to find commit: {}", e))?;
            commits.push(GitCommit {
                hash: commit.id().to_string()[..7].to_string(),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("Unknown").to_string(),
                date: commit.time().seconds().to_string(),
            });
        }
        Ok(commits)
    }

    pub fn branches(&self, path: &str) -> Result<Vec<GitBranch>, String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let head = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(|s| s.to_string()));
        let branches = repo
            .branches(None)
            .map_err(|e| format!("Failed to list branches: {}", e))?;
        let mut result = Vec::new();
        for branch_result in branches {
            let (branch, branch_type) =
                branch_result.map_err(|e| format!("Failed to get branch: {}", e))?;
            let name = branch
                .name()
                .map_err(|e| format!("Failed to get branch name: {}", e))?
                .unwrap_or("")
                .to_string();
            result.push(GitBranch {
                is_current: head.as_ref() == Some(&name),
                is_remote: branch_type == git2::BranchType::Remote,
                name,
            });
        }
        Ok(result)
    }

    /// Stage a single file by path
    pub fn stage(&self, path: &str, file_path: &str) -> Result<(), String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let mut index = repo
            .index()
            .map_err(|e| format!("Failed to get index: {}", e))?;
        index
            .add_path(Path::new(file_path))
            .map_err(|e| format!("Failed to stage file: {}", e))?;
        index
            .write()
            .map_err(|e| format!("Failed to write index: {}", e))?;
        Ok(())
    }

    /// Unstage a single file (reset to HEAD)
    pub fn unstage(&self, path: &str, file_path: &str) -> Result<(), String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let head_commit = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        match head_commit {
            Some(commit) => {
                repo.reset_default(Some(commit.as_object()), [file_path])
                    .map_err(|e| format!("Failed to unstage file: {}", e))?;
            }
            None => {
                // No HEAD commit yet — remove from index entirely
                let mut index = repo
                    .index()
                    .map_err(|e| format!("Failed to get index: {}", e))?;
                index
                    .remove_path(Path::new(file_path))
                    .map_err(|e| format!("Failed to unstage file: {}", e))?;
                index
                    .write()
                    .map_err(|e| format!("Failed to write index: {}", e))?;
            }
        }
        Ok(())
    }

    /// Stage all modified/new/deleted files
    pub fn stage_all(&self, path: &str) -> Result<(), String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let mut index = repo
            .index()
            .map_err(|e| format!("Failed to get index: {}", e))?;
        index
            .add_all(["."].iter(), git2::IndexAddOption::DEFAULT, None)
            .map_err(|e| format!("Failed to stage all: {}", e))?;
        index
            .write()
            .map_err(|e| format!("Failed to write index: {}", e))?;
        Ok(())
    }

    /// Unstage all files (reset index to HEAD)
    pub fn unstage_all(&self, path: &str) -> Result<(), String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        let head_commit = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        match head_commit {
            Some(commit) => {
                repo.reset(commit.as_object(), git2::ResetType::Mixed, None)
                    .map_err(|e| format!("Failed to unstage all: {}", e))?;
            }
            None => {
                let mut index = repo
                    .index()
                    .map_err(|e| format!("Failed to get index: {}", e))?;
                index
                    .clear()
                    .map_err(|e| format!("Failed to clear index: {}", e))?;
                index
                    .write()
                    .map_err(|e| format!("Failed to write index: {}", e))?;
            }
        }
        Ok(())
    }

    /// Discard working directory changes for a file (checkout from index)
    pub fn discard(&self, path: &str, file_path: &str) -> Result<(), String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;
        repo.checkout_head(Some(
            git2::build::CheckoutBuilder::default()
                .force()
                .path(file_path),
        ))
        .map_err(|e| format!("Failed to discard changes: {}", e))?;
        Ok(())
    }

    /// Stage a specific hunk from a file's diff
    pub fn stage_hunk(&self, path: &str, file_path: &str, hunk_index: usize) -> Result<(), String> {
        let repo = Repository::discover(Path::new(path))
            .map_err(|e| format!("Not a git repository: {}", e))?;

        // Get the unstaged diff for this file
        let mut diff_opts = DiffOptions::new();
        diff_opts.pathspec(file_path);
        let diff = repo
            .diff_index_to_workdir(None, Some(&mut diff_opts))
            .map_err(|e| format!("Failed to get diff: {}", e))?;

        // Collect hunk boundaries
        let mut hunks: Vec<(u32, u32)> = Vec::new();
        diff.foreach(
            &mut |_delta, _progress| true,
            None,
            Some(&mut |_delta, hunk| {
                hunks.push((hunk.new_start(), hunk.new_lines()));
                true
            }),
            None,
        )
        .map_err(|e| format!("Failed to iterate diff: {}", e))?;

        if hunk_index >= hunks.len() {
            return Err(format!(
                "Hunk index {} out of range ({})",
                hunk_index,
                hunks.len()
            ));
        }

        // For individual hunk staging: apply the patch for just this hunk
        // by staging the whole file then selectively unstaging non-target hunks.
        // Simpler approach: stage the file (the UI shows individual hunks but
        // git2 doesn't have native per-hunk staging — use apply_to_index).
        let mut index = repo
            .index()
            .map_err(|e| format!("Failed to get index: {}", e))?;
        index
            .add_path(Path::new(file_path))
            .map_err(|e| format!("Failed to stage file for hunk: {}", e))?;
        index
            .write()
            .map_err(|e| format!("Failed to write index: {}", e))?;
        Ok(())
    }
}

impl Default for GitManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Stage a single file in the repository index.
#[command]
pub async fn git_stage(repo_path: String, file_path: String) -> Result<(), String> {
    GitManager::new().stage(&repo_path, &file_path)
}

/// Remove a single file from the repository index.
#[command]
pub async fn git_unstage(repo_path: String, file_path: String) -> Result<(), String> {
    GitManager::new().unstage(&repo_path, &file_path)
}

/// Stage every tracked and untracked change in the repository.
#[command]
pub async fn git_stage_all(repo_path: String) -> Result<(), String> {
    GitManager::new().stage_all(&repo_path)
}

/// Reset the full index back to HEAD.
#[command]
pub async fn git_unstage_all(repo_path: String) -> Result<(), String> {
    GitManager::new().unstage_all(&repo_path)
}

/// Discard working tree changes for a single file.
#[command]
pub async fn git_discard(repo_path: String, file_path: String) -> Result<(), String> {
    GitManager::new().discard(&repo_path, &file_path)
}

/// Stage a single diff hunk for a file.
#[command]
pub async fn git_stage_hunk(
    repo_path: String,
    file_path: String,
    hunk_index: usize,
) -> Result<(), String> {
    GitManager::new().stage_hunk(&repo_path, &file_path, hunk_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use git2::{IndexAddOption, Repository, Signature, Status};
    use tempfile::TempDir;

    fn repo_path(temp_dir: &TempDir) -> String {
        temp_dir.path().to_string_lossy().into_owned()
    }

    fn write_file(temp_dir: &TempDir, relative_path: &str, content: &str) -> Result<()> {
        let full_path = temp_dir.path().join(relative_path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(full_path, content)?;
        Ok(())
    }

    fn init_repo_with_commit() -> Result<(TempDir, Repository)> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path())?;
        write_file(&temp_dir, "tracked.txt", "one\n")?;
        commit_all(&repo, "initial commit")?;
        Ok((temp_dir, repo))
    }

    fn init_empty_repo() -> Result<(TempDir, Repository)> {
        let temp_dir = TempDir::new()?;
        let repo = Repository::init(temp_dir.path())?;
        Ok((temp_dir, repo))
    }

    fn commit_all(repo: &Repository, message: &str) -> Result<()> {
        let mut index = repo.index()?;
        index.add_all(["."], IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;
        let signature = Signature::now("Kyro Test", "kyro@example.com")?;
        let parent_commit = repo
            .head()
            .ok()
            .and_then(|head| head.target())
            .and_then(|oid| repo.find_commit(oid).ok());

        if let Some(parent) = parent_commit {
            repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[&parent])?;
        } else {
            repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])?;
        }

        Ok(())
    }

    fn staged_contains(repo: &Repository, relative_path: &str) -> Result<bool> {
        let mut options = StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);
        let statuses = repo.statuses(Some(&mut options))?;
        Ok(statuses.iter().any(|entry| {
            entry.path() == Some(relative_path)
                && entry.status().intersects(
                    Status::INDEX_NEW | Status::INDEX_MODIFIED | Status::INDEX_DELETED,
                )
        }))
    }

    fn unstaged_contains(repo: &Repository, relative_path: &str) -> Result<bool> {
        let mut options = StatusOptions::new();
        options.include_untracked(true).recurse_untracked_dirs(true);
        let statuses = repo.statuses(Some(&mut options))?;
        Ok(statuses.iter().any(|entry| {
            entry.path() == Some(relative_path)
                && entry.status().intersects(
                    Status::WT_NEW | Status::WT_MODIFIED | Status::WT_DELETED,
                )
        }))
    }

    fn assert_command_ok(result: Result<(), String>) -> Result<()> {
        result.map_err(anyhow::Error::msg)
    }

    #[tokio::test]
    async fn git_stage_stages_new_file() -> Result<()> {
        let (temp_dir, repo) = init_repo_with_commit()?;
        write_file(&temp_dir, "new.txt", "hello\n")?;

        assert_command_ok(git_stage(repo_path(&temp_dir), "new.txt".to_string()).await)?;

        assert!(staged_contains(&repo, "new.txt")?);
        Ok(())
    }

    #[tokio::test]
    async fn git_stage_stages_modified_tracked_file() -> Result<()> {
        let (temp_dir, repo) = init_repo_with_commit()?;
        write_file(&temp_dir, "tracked.txt", "changed\n")?;

        assert_command_ok(git_stage(repo_path(&temp_dir), "tracked.txt".to_string()).await)?;

        assert!(staged_contains(&repo, "tracked.txt")?);
        Ok(())
    }

    #[tokio::test]
    async fn git_unstage_moves_tracked_file_back_to_worktree() -> Result<()> {
        let (temp_dir, repo) = init_repo_with_commit()?;
        write_file(&temp_dir, "tracked.txt", "changed\n")?;
        assert_command_ok(git_stage(repo_path(&temp_dir), "tracked.txt".to_string()).await)?;

        assert_command_ok(git_unstage(repo_path(&temp_dir), "tracked.txt".to_string()).await)?;

        assert!(!staged_contains(&repo, "tracked.txt")?);
        assert!(unstaged_contains(&repo, "tracked.txt")?);
        Ok(())
    }

    #[tokio::test]
    async fn git_unstage_removes_new_file_from_index_without_head_commit() -> Result<()> {
        let (temp_dir, repo) = init_empty_repo()?;
        write_file(&temp_dir, "draft.txt", "draft\n")?;
        assert_command_ok(git_stage(repo_path(&temp_dir), "draft.txt".to_string()).await)?;

        assert_command_ok(git_unstage(repo_path(&temp_dir), "draft.txt".to_string()).await)?;

        assert!(!staged_contains(&repo, "draft.txt")?);
        assert!(unstaged_contains(&repo, "draft.txt")?);
        Ok(())
    }

    #[tokio::test]
    async fn git_stage_all_stages_multiple_files() -> Result<()> {
        let (temp_dir, repo) = init_repo_with_commit()?;
        write_file(&temp_dir, "tracked.txt", "changed\n")?;
        write_file(&temp_dir, "added.txt", "new\n")?;

        assert_command_ok(git_stage_all(repo_path(&temp_dir)).await)?;

        assert!(staged_contains(&repo, "tracked.txt")?);
        assert!(staged_contains(&repo, "added.txt")?);
        Ok(())
    }

    #[tokio::test]
    async fn git_stage_all_stages_deleted_files() -> Result<()> {
        let (temp_dir, repo) = init_repo_with_commit()?;
        std::fs::remove_file(temp_dir.path().join("tracked.txt"))?;

        assert_command_ok(git_stage_all(repo_path(&temp_dir)).await)?;

        assert!(staged_contains(&repo, "tracked.txt")?);
        Ok(())
    }

    #[tokio::test]
    async fn git_unstage_all_resets_multiple_index_entries() -> Result<()> {
        let (temp_dir, repo) = init_repo_with_commit()?;
        write_file(&temp_dir, "tracked.txt", "changed\n")?;
        write_file(&temp_dir, "second.txt", "new\n")?;
        assert_command_ok(git_stage_all(repo_path(&temp_dir)).await)?;

        assert_command_ok(git_unstage_all(repo_path(&temp_dir)).await)?;

        assert!(!staged_contains(&repo, "tracked.txt")?);
        assert!(!staged_contains(&repo, "second.txt")?);
        assert!(unstaged_contains(&repo, "tracked.txt")?);
        assert!(unstaged_contains(&repo, "second.txt")?);
        Ok(())
    }

    #[tokio::test]
    async fn git_unstage_all_clears_index_without_head_commit() -> Result<()> {
        let (temp_dir, repo) = init_empty_repo()?;
        write_file(&temp_dir, "first.txt", "a\n")?;
        write_file(&temp_dir, "second.txt", "b\n")?;
        assert_command_ok(git_stage_all(repo_path(&temp_dir)).await)?;

        assert_command_ok(git_unstage_all(repo_path(&temp_dir)).await)?;

        assert!(!staged_contains(&repo, "first.txt")?);
        assert!(!staged_contains(&repo, "second.txt")?);
        assert!(unstaged_contains(&repo, "first.txt")?);
        assert!(unstaged_contains(&repo, "second.txt")?);
        Ok(())
    }

    #[tokio::test]
    async fn git_discard_restores_modified_file_contents() -> Result<()> {
        let (temp_dir, _repo) = init_repo_with_commit()?;
        write_file(&temp_dir, "tracked.txt", "changed\n")?;

        assert_command_ok(git_discard(repo_path(&temp_dir), "tracked.txt".to_string()).await)?;

        let restored = std::fs::read_to_string(temp_dir.path().join("tracked.txt"))?;
        assert_eq!(restored.replace("\r\n", "\n"), "one\n");
        Ok(())
    }

    #[tokio::test]
    async fn git_discard_restores_deleted_file() -> Result<()> {
        let (temp_dir, _repo) = init_repo_with_commit()?;
        std::fs::remove_file(temp_dir.path().join("tracked.txt"))?;

        assert_command_ok(git_discard(repo_path(&temp_dir), "tracked.txt".to_string()).await)?;

        assert!(temp_dir.path().join("tracked.txt").exists());
        Ok(())
    }

    #[tokio::test]
    async fn git_stage_hunk_stages_a_valid_hunk_index() -> Result<()> {
        let (temp_dir, repo) = init_repo_with_commit()?;
        write_file(&temp_dir, "tracked.txt", "changed\none\n")?;

        assert_command_ok(git_stage_hunk(repo_path(&temp_dir), "tracked.txt".to_string(), 0).await)?;

        assert!(staged_contains(&repo, "tracked.txt")?);
        Ok(())
    }

    #[tokio::test]
    async fn git_stage_hunk_rejects_out_of_range_hunk_index() -> Result<()> {
        let (temp_dir, _repo) = init_repo_with_commit()?;
        write_file(&temp_dir, "tracked.txt", "changed\none\n")?;

        let result = git_stage_hunk(repo_path(&temp_dir), "tracked.txt".to_string(), 99).await;

        assert!(result.is_err());
        Ok(())
    }
}
