---
name: github-expert
description: Use this skill for anything involving GitHub — creating PRs, managing issues, searching repos, reading other projects' code, setting up GitHub Actions, managing releases, branch operations, and repository automation. Triggers on: "github", "PR", "pull request", "issue", "release", "branch", "workflow", "actions", "open source".
---

# GitHub Expert Skill

## What this skill covers
- Creating and managing GitHub releases
- Reading open source code for reference
- Setting up GitHub Actions workflows
- Managing issues and pull requests
- Searching GitHub for examples and solutions
- Automating repo operations

## Core GitHub operations via shell tool

### Releases
```bash
# Create a new release tag
git tag v1.0.0 -m "Kyro IDE v1.0.0 — Production Release"
git push origin v1.0.0

# List all releases
gh release list

# Create release manually with notes
gh release create v1.0.0 \
  --title "Kyro IDE v1.0.0" \
  --notes "Production release" \
  --latest
```

### Issues
```bash
# List open issues
gh issue list

# Create an issue
gh issue create --title "Bug: git_stage not working" \
  --body "Description of the bug"

# Close an issue
gh issue close 42
```

### Pull Requests
```bash
# Create a PR
gh pr create --title "feat: implement git staging commands" \
  --body "Fixes #12, #13, #14, #15, #16, #17"

# List open PRs
gh pr list

# Merge a PR
gh pr merge 5 --squash
```

### Searching GitHub for examples
```bash
# Search for Tauri v2 updater examples
gh search repos "tauri v2 updater" --language rust --sort stars

# Search code for specific patterns
gh search code "tauri::command git2" --language rust

# Find similar open source IDEs for reference
gh search repos "tauri ide" --sort stars --limit 10
```

### GitHub Actions
```bash
# List workflow runs
gh run list

# Watch a specific run
gh run watch

# Re-run failed jobs
gh run rerun --failed

# Download workflow artifacts
gh run download
```

## Key open source repos to reference for Kyro IDE

When implementing features, always check these for inspiration:
```bash
# Zed editor (Rust + Tauri, similar to Kyro)
gh search repos "zed-industries/zed" 

# lapce editor (Rust IDE)
gh search repos "lapce/lapce"

# Tauri official examples
gh search repos "tauri-apps/tauri" --topic examples

# tree-sitter grammars
gh search repos language:tree-sitter-grammar --sort stars
```

## Kyro-specific GitHub automation

### Auto-generate changelog
```bash
# Install git-cliff
cargo install git-cliff

# Generate changelog since last tag
git cliff --latest -o CHANGELOG.md

# Generate full changelog
git cliff --unreleased -o CHANGELOG.md
```

### Check CI status before releasing
```bash
# Must be green before tagging
gh run list --branch main --limit 5
gh workflow view ci.yml
```

### Release checklist automation
```bash
# Run this before every release
cargo test --workspace &&
pnpm test &&
pnpm typecheck &&
cargo audit &&
pnpm audit --audit-level=high &&
echo "All checks passed — safe to release"
```
