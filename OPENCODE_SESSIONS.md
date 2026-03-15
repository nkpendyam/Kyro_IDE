# KYRO IDE — OpenCode + GitHub Copilot Pro Session Guide
# Tool: OpenCode CLI with GitHub Copilot Pro (GPT-4.1)
# 10 sessions from 88% → 100% production

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## ONE-TIME SETUP (do this once, never again)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

# Step 1 — Install OpenCode
curl -fsSL https://opencode.ai/install | bash
# OR with npm:
npm i -g opencode-ai@latest

# Step 2 — Connect GitHub Copilot Pro (no API key needed)
opencode
# Inside OpenCode TUI, type:
/connect
# Select: GitHub Copilot
# Complete the GitHub device login in your browser
# Done — your Copilot Pro subscription is now active

# Step 3 — Verify GPT-4.1 is available
# Inside OpenCode TUI, type:
/models
# Look for: copilot.gpt-4.1 or similar
# Note: model names may vary — use whatever GPT-5.x is shown

# Step 4 — Copy AGENTS.md and opencode.json to your repo root
# Copy .opencode/agents/*.md to your repo

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## HOW TO START EVERY SESSION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

# Navigate to your project
cd ~/path/to/Kyro_IDE

# Start OpenCode (standard — it will ask before each file change)
opencode

# Start OpenCode (full auto — no confirmations, like yolo mode)
opencode -p "your prompt here" 
# OR start interactive with auto-approve:
opencode --dangerously-skip-permissions 2>/dev/null || opencode

# Switch between build/plan agents inside TUI
# Press TAB to toggle between build agent and plan agent

# First message of EVERY session:
"Read AGENTS.md fully before starting. Then: [your task]"

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## SESSION 1 — Fix All 7 Broken P0 Command Wires
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Paste this prompt into OpenCode:

```
Read AGENTS.md fully. Then implement all 7 missing Tauri commands from P0 bugs.

For each command:
1. Implement in correct Rust module:
   - git commands → src-tauri/src/git/mod.rs (use git2 crate)
   - broadcast_cursor → src-tauri/src/collab/mod.rs
2. NO unwrap() — use ? operator for all errors
3. Register in src-tauri/src/main.rs invoke_handler
4. Add TypeScript binding in src/lib/tauri-commands.ts
5. Write 2 unit tests per command: happy path + error path

Commands:
- git_stage(repo_path: String, file_path: String)
- git_unstage(repo_path: String, file_path: String)
- git_stage_all(repo_path: String)
- git_unstage_all(repo_path: String)
- git_discard(repo_path: String, file_path: String) → checkout from HEAD
- git_stage_hunk(repo_path: String, file_path: String, hunk_header: String)
- broadcast_cursor(user_id: String, file: String, line: u32, col: u32) → emit Tauri event

After all 7: run `cargo test --workspace` and fix ALL failures.
Commit each: "fix: implement git_stage Tauri command"

Then: @code-reviewer review the 7 new commands in src-tauri/src/git/mod.rs
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## SESSION 2 — tree-sitter + Settings Persistence
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

```
Read AGENTS.md. Fix P1 bugs: tree-sitter AST wiring and settings persistence.

TASK 1 — tree-sitter:
In src-tauri/src/context/manager.rs:
- extract_symbols() returns empty vec → wire to tree-sitter real AST
  Support: TypeScript, JavaScript, Rust, Python, Go
- extract_imports() returns empty vec → wire to tree-sitter import queries
- Add to Cargo.toml: tree-sitter="0.22", tree-sitter-typescript="0.21",
  tree-sitter-rust="0.21", tree-sitter-python="0.21"
- Write tests with fixture files in tests/fixtures/

TASK 2 — Settings persistence:
- Create src-tauri/src/settings/mod.rs
- Persist to ~/.kyro/settings.json via serde_json
- load_settings(), save_settings(settings: Settings)
- Tauri commands: get_settings, update_settings
- Register in main.rs
- Wire to src/store/settingsStore.ts

Run: cargo test --workspace && pnpm typecheck
Commit: "feat: wire tree-sitter AST to symbol extraction"
Commit: "feat: implement settings persistence to ~/.kyro/settings.json"
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## SESSION 3 — Security Hardening
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

```
Read AGENTS.md. Full security audit and fix.

STEP 1 — Run and document:
  cargo audit
  pnpm audit
  cargo clippy --workspace -- -D warnings -D clippy::unwrap_used

STEP 2 — Fix P3 vulnerabilities:
CSP (tauri.conf.json):
- Remove unsafe-inline from script-src
- Remove unsafe-eval from script-src
- Replace with nonce-based CSP

ESLint:
- Re-enable @typescript-eslint/no-explicit-any as "warn"
- Re-enable react-hooks/exhaustive-deps as "warn"
- Fix all new warnings

Input validation on every Tauri command:
- Validate all String inputs (not empty, max length)
- Validate file paths within workspace root (no path traversal)
- Add validate_workspace_path() helper, use everywhere

Also invoke: @security-auditor audit E2EE in src-tauri/src/collab/

Produce: SECURITY_AUDIT_REPORT.md
Run: cargo audit && pnpm audit (must show 0 vulnerabilities)
Commit: "security: harden CSP, fix input validation, strict linting"
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## SESSION 4 — Performance Optimization
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

```
Read AGENTS.md. Fix all 5 P4 performance issues. Measure before and after each.

FIX 1 — Monaco lazy load: React.lazy + Suspense + skeleton. Target <3s cold start.
FIX 2 — File tree: replace with react-window FixedSizeList. Test 1000+ files.
FIX 3 — LSP persistence: server pool in src-tauri/src/lsp/hub.rs, reuse per workspace.
FIX 4 — AI debounce: 300ms in editor, AbortController for in-flight cancellation.
FIX 5 — WS leak: cleanup on disconnect in collab/mod.rs + useEffect cleanup in React.

Then: @performance-optimizer find any additional bottlenecks beyond these 5

Run: cargo test --workspace && pnpm test
Commit: "perf: lazy Monaco, virtualize file tree, fix LSP persistence"
Commit: "perf: debounce AI completion, fix WebSocket memory leak"
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## SESSION 5 — 3 Missing UI Panels
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

```
Read AGENTS.md. Build all 3 missing panels in src/components/panels/.
Match existing dark theme and shadcn/ui components.

PANEL 1 — TestRunnerPanel.tsx:
- Tabs: All | Failed | Passed
- Tree: test files → suites → tests
- Icons: ✓ pass, ✗ fail, ○ skipped, ⟳ running
- Live streaming via Tauri event listener
- Run All button → run_tests Tauri command (implement in src-tauri/src/)
- Click failed test → open in editor
- Duration per test ("23ms")

PANEL 2 — AgentStreamPanel.tsx:
- Agent role badge (Architect/Coder/Tester/Reviewer/Deployer)
- Progress bar 0-100%
- Live streaming text of agent actions
- Completed steps timeline
- Pause + Cancel buttons
- Token counter
- Listens for "agent_update" Tauri events

PANEL 3 — BrowserPreviewPanel.tsx:
- Editable URL bar (default localhost:3000)
- Refresh + Open External buttons
- Tauri v2 WebviewWindow for real browser
- Auto-detect dev server port from package.json
- Empty state: "No server running" + Start Dev Server button

Write React Testing Library tests for each panel.
Commit: "feat: add TestRunnerPanel, AgentStreamPanel, BrowserPreviewPanel"
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## SESSION 6 — Autonomous Executor + Model Download
## Use plan agent first (TAB to switch), then build agent
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

```
Read AGENTS.md. Two big tasks.

TASK 1 — Autonomous Executor (src-tauri/src/autonomous/developer.rs):
Implement the execute() full loop:
1. Build prompt from task + project context
2. Call LlmClient::generate()
3. Parse response using XML tags:
   <create_file path="...">content</create_file>
   <edit_file path="..."><search>old</search><replace>new</replace></edit_file>
   <run_command>cargo test</run_command>
4. Apply FileOperations via std::fs
5. Emit "agent_update" Tauri events with progress
6. Run SelfVerifier on changed files
7. Retry up to max_retries if verification fails
8. Return TaskResult::Success or TaskResult::Failed

TASK 2 — Real Model Downloader (onboarding wizard):
Replace fake progress bar with:
- Download Mistral 7B Q4_K_M from HuggingFace Hub
- Real byte-level progress → Tauri events → UI progress bar
- SHA256 verification after download
- Save to ~/.kyro/models/
- Show: filename, size, speed, ETA
- Resume support (HTTP Range headers)

Run: cargo test --workspace
Commit: "feat: implement autonomous executor AI-driven loop"
Commit: "feat: implement real model download with verification"
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## SESSION 7 — Test Coverage 60% → 80%
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

```
Read AGENTS.md. Push test coverage from 60% to 80%+.

STEP 1: cargo tarpaulin --workspace --out Html --output-dir coverage/
        pnpm test:coverage

STEP 2: @test-writer write tests for all git commands in kyro-git crate
        @test-writer write tests for autonomous executor with mocked LLM

STEP 3: Write E2E tests in tests/e2e/:
1. editor.spec.ts — open, edit, save, verify
2. git.spec.ts — stage, commit, verify in log
3. completion.spec.ts — type, verify AI suggestion
4. settings.spec.ts — change setting, restart, verify persisted
5. collab.spec.ts — two users, verify cursor broadcasts
6. model_download.spec.ts — full download flow
7. onboarding.spec.ts — new user flow end-to-end

STEP 4: Re-run coverage — must show >80%
Commit: "test: push coverage to 80%+ across all modules"
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## SESSION 8 — Complete CI/CD Pipeline
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

```
Read AGENTS.md. Add 2 missing workflows and update existing ones.

NEW: .github/workflows/e2e.yml
- Trigger: push to main + every PR
- Matrix: ubuntu-latest, windows-latest, macos-latest
- Install deps, build app, run Playwright headless
- Upload test report on failure

NEW: .github/workflows/security.yml
- Trigger: every PR + weekly Sunday midnight
- cargo audit (fail on CRITICAL/HIGH)
- pnpm audit --audit-level=high
- cargo clippy -- -D warnings
- Comment results on PR

UPDATE: release.yml
- Run all tests before building installers
- Auto-generate CHANGELOG via git-cliff
- Full changelog in GitHub Release body

UPDATE: ci.yml
- cargo tarpaulin coverage check (fail if <75%)
- Bundle size guard (fail if >5MB increase)

Commit: "ci: add E2E and security workflows"
Commit: "ci: add coverage and bundle size guards"
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## SESSION 9 — Final Polish + v1.0.0 Prep
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

```
Read AGENTS.md. Final production sweep.

STEP 1: @code-reviewer do a final review of all files changed in sessions 1-8
STEP 2: Fix every blocking issue found.

STEP 3: Update README.md:
- Accurate features only (remove n8n editor — doesn't exist)
- Install instructions for Windows/macOS/Linux
- Quick start guide (3 steps)
- Model setup guide (download Mistral 7B)
- Contributing guide

STEP 4: Bump version to 1.0.0 in:
- package.json → "version": "1.0.0"
- src-tauri/Cargo.toml → version = "1.0.0"
- src-tauri/tauri.conf.json → "version": "1.0.0"

STEP 5: Final validation — ALL must pass:
  cargo test --workspace
  pnpm test
  pnpm typecheck
  cargo audit
  pnpm audit --audit-level=high

Commit: "chore: bump version to 1.0.0"
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## SESSION 10 — Ship v1.0.0
## Run these commands yourself in your terminal
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

```bash
# Verify clean state
git status
git log --oneline -15

# Tag and push
git tag v1.0.0 -m "Kyro IDE v1.0.0 — Production Release"
git push origin main
git push origin v1.0.0

# GitHub Actions now automatically:
# → Tests on Windows + macOS + Linux
# → Builds MSI, DMG x2, AppImage, .deb
# → Creates GitHub Release with all installers
# → Updates latest.json → existing users get update popup

# Watch the build:
# https://github.com/KyroIDE/Kyro_IDE/actions
```

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
## OPENCODE TIPS FOR MAXIMUM EFFICIENCY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

1. TAB KEY — switches between build agent (full access) and plan agent
   (read-only). Always start in plan mode for complex sessions, switch
   to build when you're ready to execute.

2. @general — built-in sub-agent for complex searches and multi-step tasks
   Example: @general find all files that call extract_symbols() across the repo

3. NON-INTERACTIVE MODE — for automation:
   opencode -p "Read AGENTS.md then fix git_stage command" -f json

4. RESUME AFTER LIMIT:
   "Read AGENTS.md. Last session stopped after: [what finished].
   Continue with: [remaining tasks]."

5. SPECIALIST AGENTS — invoke with @agentname:
   @security-auditor [task]
   @test-writer [task]
   @performance-optimizer [task]
   @code-reviewer [task]

6. CHECK AVAILABLE MODELS — type inside TUI:
   /models
   Use whichever GPT-5.x or GPT-4.1 variant is available on your plan.
