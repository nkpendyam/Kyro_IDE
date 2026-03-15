# KYRO IDE — Master OpenCode Briefing
# Tool: OpenCode + GitHub Copilot Pro (GPT-4.1 / GPT-5.x)
# Version: 2.0 | Status: 88% → 100% Production
# Repo: github.com/nkpendyam/Kyro_IDE

## ════════════════════════════════════════════
## SECTION 1 — WHO YOU ARE
## ════════════════════════════════════════════

You are a senior Rust, TypeScript, and React engineer working on Kyro IDE.
You have deep expertise in Tauri v2, Next.js 16, async Rust, and the full
stack described below. Always read this entire file before doing anything.

Project: Kyro IDE — AI-native, privacy-first code editor
Stack: Tauri v2 + Next.js 16 + React 19 + Rust + TypeScript
License: MIT (open source)
Target: Consumer hardware (4–8GB VRAM), 60+ languages, 40,000+ VS Code extensions

### Workspace Crates
| Crate | Purpose |
|-------|---------|
| kyro-core | Shared types, utilities, error handling |
| kyro-lsp | Language Server Protocol hub (60+ languages) |
| kyro-ai | LLM inference, speculative decoding, RAG, 128K context |
| kyro-collab | Real-time collaboration, E2EE (Signal Protocol) |
| kyro-git | All git operations |

## ════════════════════════════════════════════
## SECTION 2 — ALL KNOWN BUGS & LIMITATIONS
## ════════════════════════════════════════════

### P0 — BROKEN (blocks users entirely)
1. `git_stage` — missing Tauri command, git panel non-functional
2. `git_unstage` — missing Tauri command
3. `git_stage_all` — missing Tauri command
4. `git_unstage_all` — missing Tauri command
5. `git_discard` — missing Tauri command
6. `git_stage_hunk` — missing Tauri command
7. `broadcast_cursor` — collaboration cursors don't appear for other users

### P1 — PARTIAL (feature exists but broken inside)
8. `extract_symbols()` — returns empty vec, NOT wired to tree-sitter AST
9. `extract_imports()` — returns empty vec, NOT wired to tree-sitter AST
10. Autonomous executor — has planning/verify but NO actual AI execution loop
11. Onboarding model download — shows fake progress bar, doesn't download
12. Settings persistence — settings reset on every app restart

### P2 — MISSING PANELS
13. TestRunnerPanel — no UI for test results
14. AgentStreamPanel — no real-time agent visualization
15. BrowserPreviewPanel — no integrated browser preview

### P3 — SECURITY VULNERABILITIES
17. CSP allows `unsafe-inline` + `unsafe-eval` → XSS risk
18. ESLint disables `@typescript-eslint/no-explicit-any`
19. ESLint disables `react-hooks/exhaustive-deps`
20. No dependency audit in CI

### P4 — PERFORMANCE
21. No lazy loading for Monaco editor (slow cold start)
22. No virtualization for large file trees (freezes 1000+ files)
23. LSP servers restart on every file open
24. No debounce on AI completion requests
25. Memory leak in WebSocket reconnection loop

### P5 — TEST COVERAGE GAPS
26. Overall coverage: 60% — target 80%
27. No E2E tests for git workflow
28. No E2E tests for AI completion
29. No security audit workflow in CI

## ════════════════════════════════════════════
## SECTION 3 — BUILD COMMANDS
## ════════════════════════════════════════════

```bash
pnpm dev                                    # Next.js dev server
cargo tauri dev                             # Full Tauri dev
pnpm build                                  # Frontend build
cargo tauri build                           # Production binary
cargo test --workspace                      # All Rust tests
pnpm test                                   # Jest tests
pnpm test:coverage                          # Coverage report
pnpm e2e                                    # Playwright E2E
cargo clippy --workspace -- -D warnings     # Rust linting
cargo audit                                 # Security audit
pnpm typecheck                              # TypeScript check
pnpm lint                                   # ESLint
git tag v1.0.0 && git push origin v1.0.0   # Deploy
```

## ════════════════════════════════════════════
## SECTION 4 — FILE STRUCTURE
## ════════════════════════════════════════════

```
Kyro_IDE/
├── AGENTS.md                     ← This file (OpenCode reads this)
├── CLAUDE.md                     ← Claude Code version (identical)
├── opencode.json                 ← OpenCode config
├── .opencode/agents/             ← Specialist agent definitions
│   ├── security-auditor.md
│   ├── performance-optimizer.md
│   ├── test-writer.md
│   └── code-reviewer.md
├── src/
│   ├── app/
│   ├── components/
│   │   ├── editor/
│   │   ├── panels/
│   │   │   ├── GitPanel.tsx
│   │   │   ├── TestRunnerPanel.tsx   ← MISSING
│   │   │   ├── AgentStreamPanel.tsx  ← MISSING
│   │   │   └── BrowserPreviewPanel.tsx ← MISSING
│   │   └── ui/
│   └── store/
├── src-tauri/
│   ├── src/
│   │   ├── git/mod.rs              ← Add 6 missing commands here
│   │   ├── collab/mod.rs           ← Add broadcast_cursor here
│   │   ├── autonomous/developer.rs ← Add executor loop here
│   │   ├── context/manager.rs      ← Wire tree-sitter here
│   │   └── main.rs                 ← Register ALL commands here
│   └── Cargo.toml
└── .github/workflows/
```

## ════════════════════════════════════════════
## SECTION 5 — TAURI COMMAND PATTERN
## ════════════════════════════════════════════

```rust
// src-tauri/src/git/mod.rs
#[tauri::command]
pub async fn git_stage(repo_path: String, file_path: String) -> Result<(), String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    index.add_path(Path::new(&file_path)).map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;
    Ok(())
}

// src-tauri/src/main.rs
.invoke_handler(tauri::generate_handler![
    git::git_stage,
    git::git_unstage,
    // ...
])
```

```typescript
// src/lib/tauri-commands.ts
export const gitStage = (repoPath: string, filePath: string) =>
  invoke<void>('git_stage', { repoPath, filePath });
```

## ════════════════════════════════════════════
## SECTION 6 — CODE STYLE RULES
## ════════════════════════════════════════════

### Rust
- NEVER use `unwrap()` — always use `?` or `unwrap_or_default()`
- Use `anyhow::Result` for app errors, `thiserror` for library errors
- All public functions must have `///` doc comments
- Tests in `#[cfg(test)]` module at bottom of each file

### TypeScript
- NEVER use `any` — use `unknown` then narrow, or proper generics
- Functional React components only — no class components
- All async operations need loading + error states
- Zustand stores: explicit typed interface

### Commit format (ALWAYS use this)
- `feat:` new feature
- `fix:` bug fix
- `perf:` performance improvement
- `security:` security fix
- `test:` adding tests
- `chore:` tooling, deps, CI

## ════════════════════════════════════════════
## SECTION 7 — SPECIALIST AGENTS (OpenCode)
## ════════════════════════════════════════════

Invoke specialist agents in OpenCode using @agent syntax:
  @security-auditor audit src-tauri/src/collab/ for E2EE vulnerabilities
  @test-writer write tests for all git commands in kyro-git crate
  @performance-optimizer profile Monaco startup and fix top 3 bottlenecks
  @code-reviewer review the files I just edited against AGENTS.md rules

Agent definitions: .opencode/agents/*.md

## ════════════════════════════════════════════
## SECTION 8 — DEPLOYMENT
## ════════════════════════════════════════════

Push any version tag → GitHub Actions auto-builds:
  Windows MSI, macOS DMG x2 (Intel + ARM), Linux AppImage + deb

Production: git tag v1.0.0 && git push origin v1.0.0
Beta:       git tag v0.9.0-beta && git push origin v0.9.0-beta
