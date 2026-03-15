# Kyro IDE Updated Implementation Plan

Date: 2026-03-15

This plan is based on `AGENTS.md`, `FULL_AUDIT_REPORT.md`, `COMPETITOR_ANALYSIS_2026.md`, `LIMITATIONS_ANALYSIS.md`, `docs/status/COMPETITIVE_READINESS.md`, `docs/IDE_GAP_ANALYSIS_2026.md`, and `OPENCODE_SESSIONS.md`.

The goal is to stop pretending Kyro is already at full parity and instead ship in the right order:

1. fix what breaks trust and basic functionality,
2. close the competitor gaps that fit the current architecture,
3. double down on areas where Kyro can actually win,
4. avoid burning time on low-leverage parity traps.

## Phase 0 — Fix What Is Broken

Milestone: `v0.9.1-stabilization`

Expected outcome:

- basic user workflows stop breaking,
- security and audit posture become credible,
- production panic paths are removed,
- the repo becomes clean enough to ship further work safely.

### Ordered task list

#### A. Blocks users first

1. Fix incomplete local inference path in `src-tauri/src/ai/real_ai_service.rs`
   - Exact fix: implement `complete_local()` for the `llama-cpp` path, or route local embedded inference through the existing real inference backend instead of `bail!("Local inference not yet implemented - use Ollama or LM Studio")`.
   - Why first: this is the only `BLOCKING` item in `LIMITATIONS_ANALYSIS.md` and it undermines Kyro's local-first promise.
   - Session: `SESSION 6 — Autonomous Executor + Model Download` plus follow-up local inference work.
   - Library/tools: existing `llama.cpp`/Candle path already in repo; leverage `src-tauri/src/embedded_llm/*`, `src-tauri/src/embedded_llm/real_inference.rs`.
   - Effort: `20h`

2. Finish tree-sitter symbol/import extraction in `src-tauri/src/context/manager.rs`
   - Exact fix: replace empty returns in `extract_symbols()` and `extract_imports()` with real tree-sitter AST queries for TS/JS/Rust/Python/Go.
   - Why: AGENTS lists this as broken and it limits editor intelligence and AI grounding.
   - Session: `SESSION 2 — tree-sitter + Settings Persistence`
   - Library/tools: `tree-sitter`, `tree-sitter-typescript`, `tree-sitter-rust`, `tree-sitter-python`, `tree-sitter-go`
   - Effort: `14h`

3. Replace fake onboarding model download in onboarding + Tauri model-download backend
   - Exact fix: wire real byte-stream progress, checksum verification, resume support, and actual writes to `~/.kyro/models/`.
   - Why: a fake progress bar is a trust-breaking user bug.
   - Session: `SESSION 6 — Autonomous Executor + Model Download`
   - Library/tools: `reqwest`, `tokio::fs`, SHA256 crate already used or `sha2`
   - Effort: `12h`

4. Ensure settings persist correctly across restart
   - Exact fix: verify `get_settings` / `update_settings` are registered, loaded on mount, and written to `~/.kyro/settings.json`; fix any remaining frontend store load issues.
   - Why: losing settings is a core usability failure.
   - Session: `SESSION 2 — tree-sitter + Settings Persistence`
   - Library/tools: `serde`, `serde_json`, Zustand store wiring
   - Effort: `6h`

5. Build missing/weak execution panels that users expect from existing UI hooks
   - Exact fix: complete `TestRunnerPanel`, `AgentStreamPanel`, and `BrowserPreviewPanel` integrations.
   - Why: AGENTS still marks them missing, and their absence makes surfaced features feel broken.
   - Session: `SESSION 5 — 3 Missing UI Panels`
   - Library/tools: `@tauri-apps/api`, `WebviewWindow`, React Testing Library
   - Effort: `22h`

#### B. Security second

6. Fix all Rust vulnerabilities from `cargo audit`
   - Exact fixes:
     - upgrade `quinn-proto` chain via `reqwest` transitive update,
     - upgrade `wasmtime 20.0.2` to safe version range,
     - review `git2`, `glib`, GTK3-stack warnings and remove or isolate vulnerable/unmaintained packages where practical.
   - File locations: `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`
   - Why: `FULL_AUDIT_REPORT.md` shows `7` vulnerabilities including one high severity.
   - Session: `SESSION 3 — Security Hardening`
   - Library/tools: `cargo audit`, current Rust crates ecosystem
   - Effort: `16h`

7. Harden CSP and remove unsafe eval/inline patterns
   - Exact fix: update Tauri/Next CSP config to remove `unsafe-inline` and `unsafe-eval`, replace with nonce or hash-based policy where needed.
   - File locations: Tauri config and app-level CSP headers/config files.
   - Why: AGENTS lists this as a security vulnerability.
   - Session: `SESSION 3 — Security Hardening`
   - Library/tools: Tauri CSP config, Next.js headers
   - Effort: `8h`

8. Add shared input/path validation for Tauri commands
   - Exact fix: central helper for empty-string checks, max lengths, workspace-root path validation, and path traversal defense; apply to file/git/settings/remote commands.
   - File locations: helper module + command modules in `src-tauri/src/commands/*`, `src-tauri/src/git/mod.rs`
   - Why: prevents obvious misuse and future security regressions.
   - Session: `SESSION 3 — Security Hardening`
   - Library/tools: `std::path`, `dunce` or `camino` if needed, existing command layer
   - Effort: `10h`

9. Restore dependency/security checks in CI
   - Exact fix: add `security.yml`, ensure JS audit has a lockfile, fail CI on audit/clippy severity thresholds.
   - File locations: `.github/workflows/`, repo lockfile, package scripts
   - Why: AGENTS explicitly says there is no dependency audit in CI.
   - Session: `SESSION 8 — Complete CI/CD Pipeline`
   - Library/tools: GitHub Actions, `cargo audit`, `pnpm audit`
   - Effort: `6h`

#### C. Performance third

10. Lazy-load Monaco editor
    - Exact fix: `React.lazy` + `Suspense` + skeleton loader around editor surface.
    - File locations: `src/components/editor/CodeEditor.tsx`
    - Session: `SESSION 4 — Performance Optimization`
    - Library/tools: React lazy loading
    - Effort: `4h`

11. Virtualize large file trees
    - Exact fix: replace current deep recursive rendering path with `react-window` or `react-virtualized`.
    - File locations: file-tree/sidebar components
    - Session: `SESSION 4 — Performance Optimization`
    - Library/tools: `react-window`
    - Effort: `8h`

12. Keep LSP servers alive instead of restarting per file open
    - Exact fix: workspace-scoped LSP pool/reuse layer in `src-tauri/src/lsp/*`.
    - Session: `SESSION 4 — Performance Optimization`
    - Library/tools: existing `lsp_tower` / manager modules
    - Effort: `10h`

13. Debounce AI completion requests and cancel stale requests
    - Exact fix: add `300ms` debounce and `AbortController`/request cancellation wiring.
    - File locations: editor/chat completion callers
    - Session: `SESSION 4 — Performance Optimization`
    - Library/tools: browser `AbortController`, React hooks
    - Effort: `4h`

14. Fix WebSocket reconnection memory leak
    - Exact fix: cleanup listeners/timers on reconnect/unmount in collaboration backend and React hooks.
    - File locations: collaboration frontend/backend modules
    - Session: `SESSION 4 — Performance Optimization`
    - Library/tools: existing websocket code
    - Effort: `6h`

15. Make the audit fully green
    - Exact fixes from `FULL_AUDIT_REPORT.md`:
      - remove `unwrap()` and clippy violations in listed files,
      - fix lint failure in `src/components/collaboration/EditorPresence.tsx`,
      - add missing `pnpm-lock.yaml`,
      - fix `pnpm typecheck` script alias,
      - install/use tarpaulin or adjust coverage tooling.
    - Session: `SESSION 3`, `SESSION 7`, `SESSION 8`
    - Library/tools: clippy, eslint, pnpm, tarpaulin
    - Effort: `22h`

### Phase 0 total

- Estimated total: `168h`
- Primary sessions: `1, 2, 3, 4, 5, 6, 7, 8`
- Main libraries/tools: `tree-sitter`, `reqwest`, `sha2`, `react-window`, Tauri `WebviewWindow`, GitHub Actions, `cargo audit`, `clippy`

## Phase 1 — Match Competitors (`v1.0.0`)

Milestone: `v1.0.0`

Goal: ship the missing mainstream AI-IDE features that fit the current architecture without trying to out-VS-Code VS Code.

### Ordered task list

| Priority | Feature | Why it matters | Effort | Session | Open source library / tool |
|---|---|---|---:|---|---|
| 1 | Production PR review panel | Missing parity vs Copilot/Cursor; strong team workflow need | 28h | `Session 5` + new follow-up | `diff2html`, GitHub/GitLab REST APIs via `reqwest`, existing AI review command |
| 2 | Hardened `@web`, `@docs`, `@git`, `@terminal` context tools | Current mention scaffolding exists but is not reliable end-to-end | 18h | `Session 9` follow-up + current docs gaps | `webfetch`-style backend via `reqwest`, docs fetchers, existing mention UI |
| 3 | Next-edit prediction | Strongest daily UX gap vs Cursor/Zed/Copilot | 32h | New session after `Session 4` | local provider abstraction over Ollama/embedded models; Monaco inline completions |
| 4 | Inline edit flow with diff preview | Core 2026 expectation | 20h | new follow-up | Monaco decorations, existing diff viewer code |
| 5 | Notebook / REPL execution panel | Mainstream parity for Python/data workflows | 34h | new session | `@uiw/react-codemirror` or Monaco notebook-like cells, `jupyter-client`, PTY-backed REPL |
| 6 | Real terminal agent actions with approval flow | Completes Terminal AI from partial to competitive | 16h | `Session 4` + new follow-up | existing PTY terminal, command approval model |
| 7 | Persistent AI memories | Helps close Windsurf/Zed gap while fitting Kyro architecture | 24h | follow-up after `Session 2` | sqlite via `rusqlite` or `sqlx`, existing memory/rules modules |
| 8 | Split diff and better merge UX | Expected Git parity feature | 14h | follow-up after `Session 1` | existing diff components, `monaco-diff-editor` |
| 9 | Settings sync/profiles | Expected mainstream polish, fits architecture | 18h | `Session 2` extension | `serde_json`, local profile export/import first |
| 10 | Voice input in chat | Low-complexity parity feature | 10h | new short session | Whisper local binding or browser speech APIs |
| 11 | One-click deployment panel | Useful but not core moat | 20h | new follow-up | Vercel/Netlify/Docker CLIs and APIs |
| 12 | Better extension compatibility matrix and docs | Needed to make Open VSX story honest and usable | 16h | `Session 8` + docs follow-up | current extension runtime + test matrix |

### Phase 1 total

- Estimated total: `250h`
- Primary sessions: `1, 2, 4, 5, 8, 9` plus `4-6` new focused sessions
- Key libraries/tools: `diff2html`, Monaco inline APIs, `rusqlite`/`sqlx`, PTY integration, `reqwest`, GitHub/GitLab APIs, notebook/repl libraries

## Phase 2 — Beat Competitors (`v1.1.0` to `v1.5.0`)

Milestone range: `v1.1.0` → `v1.5.0`

Goal: stop chasing parity and invest where Kyro can be meaningfully better than every competitor.

### Ordered task list

1. Secure collaborative AI sessions
   - Why Kyro can win: very few AI IDEs combine real-time collaboration, presence, and E2EE-oriented architecture.
   - Build:
     - shared agent state per room,
     - collaborative prompts and approvals,
     - room-scoped context/memory,
     - secure agent transcript playback.
   - Effort: `80h`
   - Sessions: build on `Session 1`, `Session 4`, `Session 6`
   - Libraries/tools: current CRDT/Yjs/yrs stack, Signal-style E2EE modules

2. Best local-first agent stack on consumer hardware
   - Why Kyro can win: local-first is already part of the product identity.
   - Build:
     - real embedded inference path,
     - model-routing by memory tier,
     - offline planning/execution mode,
     - local RAG + local tool-use defaults.
   - Effort: `90h`
   - Sessions: `Session 6` plus new inference-specific sessions
   - Libraries/tools: `llama.cpp`, Candle, current memory-tier system, local embedding/vector stack

3. Graph-aware private codebase intelligence
   - Why Kyro can win: Kyro already has RAG, RepoWiki, graph ideas, and local architecture.
   - Build:
     - real graph-enhanced local retrieval,
     - symbol/import/call graph fusion,
     - repo-wide code maps and explorable AI citations.
   - Effort: `72h`
   - Sessions: `Session 2`, `Session 6`
   - Libraries/tools: `tree-sitter`, HNSW/vector store, graph DB-like local structures

4. Trust-first autonomous editing with checkpoints and rollback
   - Why Kyro can win: local-first plus Git plus trust/permissions can create a safer autonomous coding story than cloud-heavy rivals.
   - Build:
     - checkpoints per agent turn,
     - revert/fork conversation branches,
     - deterministic action log,
     - stronger trust policy engine.
   - Effort: `64h`
   - Sessions: `Session 6`, `Session 9`
   - Libraries/tools: git worktrees/snapshots, current trust layer, Tauri events

5. Private team review + coding automation
   - Why Kyro can win: combine local AI, PR review, collaboration, and policy tooling without forcing cloud upload.
   - Build:
     - team policy checks,
     - local review assistant,
     - shared project rules and memory,
     - auditable automation pipelines.
   - Effort: `68h`
   - Sessions: `Session 3`, `Session 5`, `Session 8`
   - Libraries/tools: existing quality gate, rule engine, git APIs, local DB

### Phase 2 total

- Estimated total: `374h`
- Primary sessions: `2, 3, 4, 6, 8, 9` plus multiple new product sessions
- Key libraries/tools: `llama.cpp`, Candle, `tree-sitter`, local vector graph stack, git worktree tooling, existing trust and collaboration stack

## Phase 3 — Long-Term Moat (`v2.0.0+`)

Milestone: `v2.0.0+`

Goal: create product advantages that are slow to copy and compound over time.

### Long-term moat candidates

1. Offline-first autonomous software factory
   - Why moat: a truly good offline/local autonomous coding system with approvals, checkpoints, and local RAG is hard to replicate and aligned with Kyro's architecture.
   - Needs:
     - production embedded inference,
     - strong local planning/execution loop,
     - local knowledge graph,
     - deterministic recovery and verification.
   - Effort: `200h+`

2. End-to-end encrypted collaborative AI workspaces
   - Why moat: almost no one combines secure collaboration + AI agents well.
   - Needs:
     - room-scoped AI context,
     - shared encrypted memories,
     - agent co-editing and replay,
     - admin-safe collaboration policies.
   - Effort: `180h+`

3. Private repo intelligence layer for regulated teams
   - Why moat: privacy-sensitive orgs need local knowledge, policy, and auditability more than flashy consumer UX.
   - Needs:
     - local graph RAG,
     - evidence-backed suggestions,
     - audit logs,
     - secure policy execution.
   - Effort: `160h+`

4. Trust and verification layer for agentic coding
   - Why moat: if Kyro becomes the safest place to let agents edit code, that is more defensible than matching UI flourishes.
   - Needs:
     - verification pipeline,
     - test/risk gates,
     - file-operation policies,
     - explainable agent decisions.
   - Effort: `140h+`

5. Native-feeling local AI workstation for teams on ordinary hardware
   - Why moat: if Kyro makes modest hardware feel premium for local AI coding, that is meaningful differentiation.
   - Needs:
     - aggressive inference optimization,
     - hardware-aware routing,
     - memory-aware UX defaults,
     - smooth installer/update story.
   - Effort: `160h+`

### Phase 3 total

- Estimated total: `840h+`
- Primary sessions: large multi-iteration roadmap, not a single batch
- Key libraries/tools: current collaboration/trust/AI architecture plus deeper investment in local inference and graph intelligence

## Limitations To Accept

These are limitations from `LIMITATIONS_ANALYSIS.md` that Kyro should explicitly avoid chasing in the near term because they waste leverage.

1. Full VS Code platform parity
   - Includes: web IDE parity, Codespaces-class remote cloud dev, complete extension host equivalence, full notebook platform breadth.
   - Why not chase: massive time sink, weak strategic fit, incumbents already own the moat.

2. Trying to beat Cursor by becoming a cloud-first control plane
   - Includes: large hosted-agent fleet, provider-routing infrastructure, premium multi-model orchestration at scale.
   - Why not chase: contradicts Kyro's strongest differentiation and requires major infrastructure spend.

3. Matching JetBrains language-depth across all IDE categories
   - Why not chase: years of domain tooling and product depth; bad use of a small team's time.

4. Rewriting Kyro into a purely native editor to match Zed's architecture
   - Why not chase: a huge rewrite with unclear payoff relative to fixing current product gaps.

5. Perfect extension marketplace/network effects in the near term
   - Why not chase: network effects are earned slowly; focus on compatibility for a curated useful subset instead.

6. Store-grade distribution and full enterprise support before product maturity
   - Why not chase: signing/notarization is worth doing, but full store/distribution/compliance ops before core stability is wasted effort.

7. "Zero dependency" as an absolute promise
   - Why not chase: local AI quality may reasonably rely on optional providers/runtimes; the right goal is graceful optional dependency handling, not ideology.

## Recommended Version Map

| Version | Goal |
|---|---|
| `v0.9.1` | stabilization, audit cleanup, unblock users |
| `v1.0.0` | honest competitor parity in core workflows |
| `v1.1.0 - v1.5.0` | differentiation: privacy-first collaboration + local-first AI |
| `v2.0.0+` | moat: secure collaborative autonomous development |

## Summary

Kyro should ship like this:

1. stop the breakage,
2. complete the missing mainstream AI-IDE workflows that fit the current architecture,
3. invest hard in privacy-first local collaboration and trustworthy autonomy,
4. refuse the trap of trying to replicate every incumbent moat.

That path gives Kyro a real chance to matter instead of becoming a permanent parity project.
