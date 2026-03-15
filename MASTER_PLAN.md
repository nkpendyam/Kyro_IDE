# MASTER PLAN — Kyro IDE

Date: 2026-03-15

This plan is built from `AGENTS.md`, `LIMITATIONS_ANALYSIS.md`, `COMPETITOR_ANALYSIS_2026.md`, and `OPENCODE_SESSIONS.md`.

Kyro wins by becoming the best privacy-first, local-first, collaborative AI IDE for serious codebases. It should not try to out-VS-Code VS Code or out-cloud Cursor.

## SECTION 1 — WHAT KYRO CAN WIN RIGHT NOW

| Win lane | Why Kyro can win | 60-day move |
|---|---|---|
| 100% local inference with zero code leaving the machine | Kyro can offer real on-device completions, chat, RAG, and automation with no prompts, files, or embeddings sent to a vendor cloud. Cursor, Windsurf, Copilot, and JetBrains cannot offer the same default privacy boundary even with enterprise controls. | Finish embedded inference hardening, real model download, GPU backend parity, and a visible "offline mode" trust surface. |
| Privacy-first AI for regulated teams and OSS maintainers | Kyro can make data boundaries explicit: local by default, cloud optional, no training on user code, auditable tool permissions, and deterministic execution logs. Competitors mostly ask users to trust policy pages and hosted control planes. | Ship audit logs, permission checkpoints, local-only mode, and a simple privacy dashboard. |
| Secure collaborative AI coding | Kyro already has built-in collaboration and E2EE-oriented architecture, which Cursor and Windsurf do not treat as core product identity. If presence, cursor sync, and reliability are hardened, Kyro can own secure pair programming. | Fix collaboration reliability, add shared agent state, and market "secure multiplayer AI coding" as a first-class workflow. |
| Local repo intelligence and RAG | Kyro can provide repo search, symbol context, embeddings, and code graph context without uploading source code. That is a stronger privacy/value story than cloud retrieval layers. | Finish tree-sitter wiring, harden `@codebase` and `@docs`, and add graph-aware local context. |
| Cost-predictable AI development | Kyro can eliminate recurring per-seat/per-token dependence for many workflows by running local models first and cloud only when requested. That is a real economic advantage over Cursor, Windsurf, and Copilot. | Add clear local/cloud routing, token and cost counters, and "burst to cloud" only as an opt-in action. |
| Open-source, auditable AI editor control | Kyro can let teams inspect the core, patch behavior, self-host pieces, and verify how agents operate. Most direct AI IDE rivals cannot match that degree of control. | Keep core flows open, document security boundaries, and expose agent execution traces. |
| Rust-native alternative to Electron-heavy AI IDEs | Kyro cannot beat Zed on pure native-editor latency, but it can beat Electron tools on desktop footprint, memory discipline, and packaging story while still keeping a modern web UI. | Finish Monaco lazy loading, file tree virtualization, and LSP persistence, then publish before/after benchmarks. |
| Hybrid local-plus-cloud agent architecture with explicit permissions | Kyro can turn local-by-default plus cloud-optional into a product strength instead of a fallback. Competitors are mostly cloud-first; Kyro can make cloud burst transparent and deliberate. | Finish autonomous executor, tool permission gates, and cloud burst mode with checkpointing. |
| Open AI IDE for trust-sensitive teams | Kyro can combine privacy, open source, local execution, and collaboration into a bundle that no major competitor fully owns. | Ship production hardening, signing, CI security gates, and regulated-team messaging. |
| OSS alternative in the "private AI IDE" narrative | Void validated the demand but stalled. Kyro can occupy that narrative with a more complete and maintained product. | Package a stable release, clear docs, and a sharper positioning page focused on local-first trust. |

## SECTION 2 — WHAT KYRO CANNOT WIN AND SHOULD IGNORE

| Do not compete on | Why it is unwinnable or wasteful | What Kyro should do instead |
|---|---|---|
| VS Code extension marketplace size | VS Code has the network effects, extension authors, docs, and default mindshare. Kyro will not beat that marketplace breadth in the near term. | Support Open VSX well, publish a compatibility matrix, and focus on the top 100 extensions users actually need. |
| Full VS Code platform breadth | Web IDE, Codespaces, notebooks, browser preview, GitHub workflows, and remote stacks are years of accumulated platform work. | Build only the workflows that reinforce Kyro's privacy-first desktop identity. |
| Cursor's hosted AI loop | Cursor wins with cloud agents, managed infrastructure, and heavy hosted model access. Matching that feature-for-feature would pull Kyro out of its lane. | Win on local-first trust, explicit permissions, and hybrid optional cloud. |
| JetBrains language-depth moat | JetBrains has decades of language-specific IDE investment. Kyro cannot reproduce that depth quickly across many ecosystems. | Aim for strong generalist workflows and excellent AI context, not language-by-language supremacy. |
| Zed's pure native-performance benchmark | Kyro's Tauri + Next.js + React + Monaco stack has a structural ceiling versus a more purely native editor. | Benchmark against Electron-class AI IDEs, not against Zed's lowest-level latency claims. |
| Frontier-cloud model quality on consumer hardware | Local models on 4-8GB VRAM machines will not match frontier hosted reasoning and giant context windows. | Compete on privacy, cost, predictability, and "good enough local by default" rather than absolute model quality. |
| Absolute zero-dependency purity | Optional AirLLM, Python, Ollama, LM Studio, and platform build tools make the "zero dependency" promise too fragile as an absolute claim. | Market "no required vendor cloud" and "local-first" instead of "zero dependency." |
| Perfect VS Code extension compatibility | Many extensions assume the real VS Code runtime and webview model. Full fidelity is a structural trap. | Support a curated compatibility layer and document what works, what partially works, and what never will. |
| Marketplace breadth as a branding promise | Saying "40,000+ extensions" invites the wrong comparison and creates permanent support debt. | Promise "Open VSX access plus curated compatibility" instead. |
| Enterprise admin surface parity with Microsoft and JetBrains | Their support, compliance, procurement, and platform partnerships are not near-term matches. | Target trust-sensitive teams that care more about local control than giant vendor admin tooling. |
| Store-grade distribution breadth | Platform stores and enterprise-managed distribution channels require more partnerships and compliance than Kyro needs to win its lane. | Focus on signed GitHub Releases, auto-update, and self-host-friendly installs. |
| Brand gravity and incumbent habit | Developers already live in VS Code, JetBrains, GitHub, Cursor, and Zed ecosystems. Trying to "replace everything for everyone" is a losing message. | Target privacy-sensitive teams, OSS maintainers, and local-first power users first. |

## SECTION 3 — MISSING FEATURES TO BUILD (ordered by user impact)

| Rank | Missing feature | Why users would switch for it | Effort | Open source implementation to use | Session |
|---|---|---|---|---|---|
| 1 | Production-grade PR review panel with AI comments, review summaries, and one-click fix apply | This closes one of the clearest gaps versus Cursor, Copilot, and JetBrains, and creates a team workflow users use every day. | Large (3-4 weeks) | `react-diff-view`, `git2` now and `gitoxide` later, GitHub CLI or `octocrab` for PR metadata | 11 |
| 2 | Fully hardened remote SSH and devcontainer workflows | Many serious users will not switch editors without remote coding, containerized workspaces, and reliable reconnect behavior. | Large (4-6 weeks) | `russh`, `bollard`, `portable-pty`, devcontainer spec via `serde_json` | 12 |
| 3 | Reliable `@web`, `@docs`, `@git`, `@terminal`, and `@codebase` context pipeline | This directly improves AI usefulness and trust, and it is a gap the competitor analysis calls out repeatedly. | Medium-large (2-3 weeks) | `tantivy`, `readability-rs`, `html2md`, `tree-sitter`, existing git and terminal bridges | 13 |
| 4 | Next-edit prediction that feels instant and trustworthy | This is a daily-use feature that strongly affects perceived editor quality and makes Kyro feel modern beside Cursor, Zed, and Copilot. | Large (3-5 weeks) | `llama.cpp` fill-in-the-middle models, `candle`, `tokenizers`, speculative decoding already planned in `kyro-ai` | 14 |
| 5 | Strong inline editing with diff preview and multi-file apply | Users expect command-k style editing, patch preview, and scoped apply flows across files. | Large (3-4 weeks) | `react-diff-view`, Monaco inline decorations, structured patch application in Rust | 14 |
| 6 | Notebook and REPL workflow for Python and exploratory work | This unlocks data, scripting, and teaching workflows that keep many users anchored in VS Code and JetBrains. | Large (4-6 weeks) | `@jupyterlab/services`, `@datalayer/jupyter-react`, notebook JSON via `nbformat` compatibility | 15 |
| 7 | Mature background and autonomous agent workflows | Users increasingly expect long-running agents, checkpoints, retry, and visible execution state instead of only chat and inline actions. | Large (4-6 weeks) | Existing `autonomous/developer.rs`, `@modelcontextprotocol/sdk`, structured tool execution and event streaming | 6, 18 |
| 8 | AI memory across project, workspace, and team contexts | Windsurf and Zed-class memory makes AI feel cumulative; Kyro's memory is still too thin. | Medium-large (2-4 weeks) | `rusqlite`, local vector store, `tantivy`, deterministic memory schemas | 16 |
| 9 | Strong terminal AI with agentic fix-and-run loops | This keeps users inside Kyro during debug/build loops instead of bouncing back to shell tools or competitor agents. | Medium (2-3 weeks) | `portable-pty`, structured command capture, patch/apply loop in Rust | 17 |
| 10 | One-click deployment and preview environment flows | This is not Kyro's core moat, but it is a sticky quality-of-life feature competitors use to keep users in-editor. | Medium-large (2-4 weeks) | GitHub CLI, Docker CLI, `bollard`, provider-specific templates | 17 |
| 11 | Git UX polish: split diffs, 3-way merge, smarter staging and review | Git remains a daily workflow and Kyro still trails established editors here. | Medium (2-3 weeks) | `gitoxide`, `react-diff-view`, existing `git2`/Tauri command layer | 11, 17 |
| 12 | Settings sync, profiles, and durable workspace identity | Users expect settings not just to persist, but to travel across machines and roles. | Medium (2-3 weeks) | `tauri-plugin-store`, `serde_json`, optional GitHub gist or filesystem sync backends | 16 |
| 13 | Browser preview and test result surfaces integrated into workflow | VS Code-level workflow completeness depends on fast feedback loops for web apps and tests. | Medium (1-2 weeks) | Tauri `WebviewWindow`, existing React panel system, Playwright and test event bridges | 5 |
| 14 | Real-time agent stream and execution visualization | Agent UX feels more trustworthy when users can see plan, progress, tool use, and checkpoints. | Medium (1-2 weeks) | Existing Tauri event system, timeline UI, structured event payloads | 5, 18 |
| 15 | Voice input and voice commands | Lower-impact than the features above, but increasingly expected by some users and visible in the market. | Small-medium (1-2 weeks) | `whisper.cpp` or local STT via `whisper-rs` | 19 |
| 16 | Theme and icon-theme ecosystem depth | This will not drive a primary switch, but it helps polish and adoption. | Small-medium (1-2 weeks) | Open VSX theme ingestion, VS Code theme JSON compatibility | 20 |

## SECTION 4 — NEW TECHNOLOGIES TO INTEGRATE

| Technology | Limitation it resolves | Open source implementation to use | Effort |
|---|---|---|---|
| `tree-sitter` query-based symbol and import extraction | Resolves weak code intelligence and broken `extract_symbols()` / `extract_imports()` | Existing `tree-sitter` crates already in repo plus language query files | Medium |
| `@tanstack/react-virtual` or `react-window` | Resolves large file tree freezes and improves large-repo usability | `@tanstack/react-virtual` for modern virtualization | Small-medium |
| Monaco dynamic import plus Suspense skeletons | Resolves cold-start pain from eager editor load | Native `React.lazy` and Monaco code splitting | Small |
| LSP process pool with persistent server registry | Resolves LSP restart-on-open limitation | Existing `tower-lsp` plus a Rust-side pooled hub | Medium |
| Debounced inference queue with cancellation | Resolves wasted AI calls and latency spikes | `tokio_util::sync::CancellationToken` and browser `AbortController` | Small |
| `russh` | Resolves remote SSH immaturity | `russh` for SSH transport and command orchestration | Medium-large |
| `bollard` | Resolves devcontainer and Docker execution immaturity | `bollard` as Rust Docker client | Medium-large |
| Devcontainer spec parser | Resolves current probe-only remote workflow by supporting real workspace config | `serde_json` over `.devcontainer/devcontainer.json` and spec-compatible parsing | Medium |
| `tantivy` | Resolves weak local docs/search/context retrieval and improves AI memory retrieval | `tantivy` local full-text index | Medium |
| `readability-rs` plus `html2md` | Resolves weak `@web` and `@docs` ingestion quality | `readability-rs` for extraction and `html2md` for normalization | Small-medium |
| `petgraph` | Resolves shallow local RAG by adding graph-aware codebase context | `petgraph` for symbol and dependency graph traversal | Medium |
| `llama.cpp` with FIM models | Resolves weak next-edit prediction and strengthens local inference | `llama.cpp` GGUF backends with fill-in-the-middle capable models | Large |
| `candle` plus speculative decoding path | Resolves local inference maturity and latency on supported hardware | Existing `candle-core`, `candle-nn`, `candle-transformers` | Large |
| Vulkan and CUDA-enabled `llama.cpp` builds | Resolves Windows and Linux GPU parity issues for local models | Upstream `llama.cpp` GPU backends | Medium-large |
| `react-diff-view` | Resolves PR review and inline diff preview UX gaps | `react-diff-view` | Medium |
| `gitoxide` | Resolves long-term git performance and safety limitations and reduces future dependence on `git2` | `gitoxide` | Large |
| `@jupyterlab/services` plus notebook UI bindings | Resolves notebook and REPL parity gaps | `@jupyterlab/services` and `@datalayer/jupyter-react` | Large |
| `tauri-plugin-store` | Resolves settings persistence and profiles depth limitations | `tauri-plugin-store` | Small-medium |
| `rusqlite`-backed memory schema | Resolves weak AI memory and project rule durability | Existing `rusqlite` with explicit memory tables | Medium |
| `cargo-nextest` plus Playwright | Resolves shallow effective test breadth and improves production hardening | `cargo-nextest`, Playwright | Medium |
| `cargo-deny` | Resolves warning-only dependency drift and improves CI hardening | `cargo-deny` | Small |
| `sigstore/cosign` and platform signing flows | Resolves release trust and installer verification gaps | `cosign` plus existing GitHub Actions | Medium |
| `OpenTelemetry` | Resolves low-visibility production diagnostics for agent and remote workflows | `opentelemetry-rust` and OTLP exporters, optional in self-host mode | Medium |
| `whisper-rs` or `whisper.cpp` | Resolves voice input absence without requiring cloud speech APIs | `whisper-rs` or `whisper.cpp` | Small-medium |

## SECTION 5 — COMPLETE SESSION ROADMAP

The first 10 sessions come from `OPENCODE_SESSIONS.md`. Sessions 11-20 are added to cover the remaining competitive gaps.

### Session 1 — Fix All 7 Broken P0 Command Wires

- Goal: Finish all missing Tauri command wires and tests for git staging and cursor broadcast.
- Mega-prompt:

```text
Read AGENTS.md fully. Then implement and verify all missing P0 Tauri command wires in the correct Rust modules, register them in main.rs, add TypeScript bindings, and write happy-path plus error-path tests for each. After implementation, run cargo test --workspace, fix every failure, and keep the code free of unwrap().
```

- Estimated messages: 20-30
- Expected output: working git commands, working `broadcast_cursor`, registered bindings, passing Rust tests.

### Session 2 — tree-sitter + Settings Persistence

- Goal: Wire real AST extraction for symbols/imports and make settings survive restart.
- Mega-prompt:

```text
Read AGENTS.md fully. Fix the partial tree-sitter and settings persistence bugs by wiring extract_symbols() and extract_imports() to real language queries, adding fixture-backed tests, creating a real settings persistence module, exposing get_settings/update_settings Tauri commands, and wiring the frontend store. End with cargo test --workspace and pnpm typecheck both passing.
```

- Estimated messages: 25-35
- Expected output: real symbol/import extraction, persistent settings, passing Rust and TypeScript validation.

### Session 3 — Security Hardening

- Goal: Remove obvious security debt and enforce safer defaults.
- Mega-prompt:

```text
Read AGENTS.md fully. Run a security sweep, remove unsafe CSP entries, re-enable key ESLint protections, add input and path validation helpers for Tauri commands, run cargo audit and pnpm audit, and produce SECURITY_AUDIT_REPORT.md. Also review collaboration code for E2EE weaknesses and fix anything that blocks a trustworthy local-first security story.
```

- Estimated messages: 25-40
- Expected output: safer CSP, restored lint checks, validated Tauri inputs, security report, cleaner audit posture.

### Session 4 — Performance Optimization

- Goal: Fix the biggest performance bottlenecks that block large-repo adoption.
- Mega-prompt:

```text
Read AGENTS.md fully. Measure and fix Monaco cold start, file tree virtualization, LSP restart churn, AI debounce, and WebSocket leak behavior. Keep before/after notes for each change, preserve existing UI language, and end with cargo test --workspace plus frontend test coverage staying green.
```

- Estimated messages: 25-35
- Expected output: lazy Monaco, virtualized tree, persistent LSP behavior, debounced AI completion, fixed collaboration leak.

### Session 5 — 3 Missing UI Panels

- Goal: Add the missing test, agent, and browser workflow panels.
- Mega-prompt:

```text
Read AGENTS.md fully. Build TestRunnerPanel, AgentStreamPanel, and BrowserPreviewPanel in the existing panel system, wire them to real Tauri events and commands, add loading and error states, preserve the current visual language, and write React tests for each panel.
```

- Estimated messages: 20-30
- Expected output: three production-usable panels, UI tests, real event wiring.

### Session 6 — Autonomous Executor + Real Model Download

- Goal: Replace partial AI execution with a real loop and a real model installer.
- Mega-prompt:

```text
Read AGENTS.md fully. Implement the full autonomous executor loop in Rust with plan, tool execution, verification, retry, and streamed updates, then replace the fake onboarding model progress bar with a real resumable Hugging Face download flow including checksum verification and progress telemetry.
```

- Estimated messages: 30-45
- Expected output: usable autonomous executor, real model download, passing Rust tests.

### Session 7 — Test Coverage 60% → 80%

- Goal: Raise effective coverage and add the missing E2E paths.
- Mega-prompt:

```text
Read AGENTS.md fully. Use coverage tools to find the biggest gaps, add Rust tests for kyro-git and autonomous execution, then add E2E coverage for editor, git, AI completion, settings persistence, collaboration, onboarding, and model download flows until overall coverage reaches at least 80%.
```

- Estimated messages: 30-45
- Expected output: broader Rust and Playwright coverage, coverage artifacts, higher confidence on critical workflows.

### Session 8 — Complete CI/CD Pipeline

- Goal: Make production quality enforceable in CI instead of manual only.
- Mega-prompt:

```text
Read AGENTS.md fully. Add missing E2E and security workflows, strengthen CI with coverage and bundle guards, make release builds depend on passing checks, and ensure audit and lint failures block merge so Kyro's production bar is enforced continuously.
```

- Estimated messages: 20-30
- Expected output: new GitHub workflows, stronger merge gates, safer release pipeline.

### Session 9 — Final Polish + v1.0.0 Prep

- Goal: Make the product honest, documented, and releasable.
- Mega-prompt:

```text
Read AGENTS.md fully. Do a final code and docs sweep, fix remaining blocking issues, make README claims accurate, update install and model setup docs, bump versions to 1.0.0 only when validation is clean, and run the full release checklist before stopping.
```

- Estimated messages: 20-30
- Expected output: accurate docs, version bump, fully validated production-ready branch.

### Session 10 — Ship v1.0.0

- Goal: Tag, push, and publish the first stable release.
- Mega-prompt:

```text
Read AGENTS.md fully. Verify a clean release state, confirm the last validation results, tag v1.0.0, push main and the release tag, monitor the GitHub Actions artifacts, and verify installers plus auto-update metadata are correct before calling the release done.
```

- Estimated messages: 10-15
- Expected output: production tag, GitHub Release artifacts, validated first stable release.

### Session 11 — PR Review and Fix Application

- Goal: Build the strongest missing daily team workflow.
- Mega-prompt:

```text
Read AGENTS.md fully. Add a real PR Review surface with split diff view, threaded AI comments, per-file review summaries, checklists, and one-click "apply fix" actions that generate and apply patches locally. Use existing git and GitHub integrations where possible, keep all risky actions checkpointed, and end with tests for review rendering and fix application.
```

- Estimated messages: 35-50
- Expected output: PR review panel, AI review summaries, patch apply workflow, tests, and demo docs.

### Session 12 — Remote SSH and Devcontainers Productionization

- Goal: Turn remote from probe logic into a real workflow.
- Mega-prompt:

```text
Read AGENTS.md fully. Replace the current probe-only remote path with a production-capable remote stack: SSH session management, workspace discovery, terminal and port forwarding, devcontainer parsing and startup, reconnect behavior, remote LSP reuse, and end-to-end tests against a containerized fixture workspace.
```

- Estimated messages: 40-60
- Expected output: real remote connections, devcontainer bring-up, reconnect logic, and test coverage.

### Session 13 — Complete AI Context Mentions and Local RAG

- Goal: Make AI context injection reliable enough to trust on real projects.
- Mega-prompt:

```text
Read AGENTS.md fully. Harden every mention and retrieval surface end-to-end: @file, @folder, @codebase, @git, @terminal, @web, and @docs. Add local indexing, page extraction, doc normalization, token budgeting, relevance ranking, and clear UI evidence for what context was injected into each request.
```

- Estimated messages: 35-50
- Expected output: robust mentions, local docs/web ingestion, visible context chips, tests, and benchmark examples.

### Session 14 — Next-Edit Prediction and Inline Editing Parity

- Goal: Make Kyro feel competitive in moment-to-moment editing.
- Mega-prompt:

```text
Read AGENTS.md fully. Add low-latency next-edit prediction using fill-in-the-middle local models, improve inline edit quality with diff preview and scoped apply, support multi-file edits with review checkpoints, and measure acceptance latency and prediction usefulness against a set of real coding tasks.
```

- Estimated messages: 40-60
- Expected output: tab-style next edit, improved inline edit UX, latency metrics, and validation demos.

### Session 15 — Notebook and REPL Workflows

- Goal: Close the biggest non-traditional coding workflow gap.
- Mega-prompt:

```text
Read AGENTS.md fully. Add a practical notebook and REPL experience for Python-first exploratory workflows, including notebook file open/edit/run, cell output rendering, kernel status, variable inspection, and a lightweight terminal-linked REPL flow for non-notebook languages. Keep the scope narrow but production-usable.
```

- Estimated messages: 40-55
- Expected output: notebook panel/editor, kernel integration, REPL workflow, tests, and docs.

### Session 16 — AI Memory, Profiles, and Settings Sync

- Goal: Make Kyro feel cumulative and personalized across sessions and machines.
- Mega-prompt:

```text
Read AGENTS.md fully. Add durable AI memory for project rules, workspace history, user preferences, and profile-specific settings. Implement local-first storage, explicit memory controls, settings profiles, optional sync backends, and UI surfaces that let users inspect, edit, and delete remembered state.
```

- Estimated messages: 30-45
- Expected output: memory schema, profile system, sync support, control UI, and tests.

### Session 17 — Terminal AI, Git Polish, and Deploy Flows

- Goal: Make the core build-debug-ship loop competitive.
- Mega-prompt:

```text
Read AGENTS.md fully. Turn terminal AI into a full diagnose-fix-run loop, improve Git UX with better diff and merge handling, add one-click deploy and preview flows for common stacks, and keep every potentially destructive action explicit and reviewable.
```

- Estimated messages: 35-50
- Expected output: stronger terminal agent workflow, better git ergonomics, deploy shortcuts, and validation coverage.

### Session 18 — Collaboration Hardening and Shared Agents

- Goal: Turn Kyro's collaboration advantage into a real moat.
- Mega-prompt:

```text
Read AGENTS.md fully. Harden collaboration for long-running sessions, fix reliability edge cases, add shared agent state and room-aware AI actions, expose per-user permissions and action logs, and make secure multiplayer AI coding one of the most polished parts of the product.
```

- Estimated messages: 35-50
- Expected output: reliable collaboration sessions, shared agent workflow, audit logs, and stress-test coverage.

### Session 19 — Windows GPU Parity and Offline Power Features

- Goal: Make local-first credible across mainstream consumer hardware.
- Mega-prompt:

```text
Read AGENTS.md fully. Improve hardware detection, GPU backend selection, model recommendations, and offline power-user features on Windows, macOS, and Linux. Add voice input if time permits, but prioritize stable GPU execution, backend fallbacks, and transparent model capability reporting.
```

- Estimated messages: 30-45
- Expected output: better cross-platform local model behavior, clearer hardware UX, optional voice input, and updated setup docs.

### Session 20 — Extension Strategy, Trust Packaging, and Competitive Launch Story

- Goal: Finish the strategic layer that helps Kyro win the right users.
- Mega-prompt:

```text
Read AGENTS.md fully. Do not chase marketplace parity. Instead, build a curated extension compatibility matrix, better Open VSX onboarding, signed release and updater trust improvements, privacy and audit docs, and a comparison-based launch package that clearly explains why Kyro exists and who it is for.
```

- Estimated messages: 25-40
- Expected output: compatibility docs, improved extension onboarding, stronger release trust assets, and clear competitive positioning for launch.

## Outcome target

If Sessions 1-10 make Kyro truly production-ready and Sessions 11-20 land cleanly, Kyro does not beat competitors by copying all of them. It beats them by owning a lane they do not fully cover: private, local-first, collaborative, auditable AI coding on the desktop.
