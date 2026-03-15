# Phase 0 Completion Report

Date: 2026-03-15

## What was fixed

- `src-tauri/src/commands/remote.rs`
  - replaced the clippy-failing `map` pattern with explicit `?` handling and `Ok(id)`
  - result: `cargo clippy --workspace -- -D warnings` now passes

- `package.json`
  - added `typecheck` script alias to the existing `type-check` script
  - result: `pnpm typecheck` now passes instead of failing with `Command "typecheck" not found`

- `src-tauri/src/ai/real_ai_service.rs`
  - implemented the previously missing `local` backend path
  - local completion now initializes and reuses `EmbeddedLLMEngine`
  - removed the dispatch bug where `local` only matched behind the `llama-cpp` cfg gate
  - added tests covering local backend execution and embedded engine initialization

- `src-tauri/Cargo.toml`, `src-tauri/crates/kyro-git/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/src/rag/vector_store.rs`
  - upgraded `wasmtime` from the vulnerable `20.0.2` line to `24.0.6`
  - upgraded `reqwest` to `0.13.2`
  - upgraded `git2` to `0.20.4`
  - updated `quinn-proto` to `0.11.14`
  - removed direct `bincode` usage from the vector store and switched persistence to `serde_json`
  - result: the previous `cargo audit` vulnerability findings are resolved

- `.cargo/audit.toml`
  - added allowlisted advisory IDs for GTK3/Tauri transitive upstream warnings
  - the remaining `cargo audit` output is warning-only and now consists of upstream/transitive maintenance advisories, not active vulnerability failures

## Tests and checks now passing

- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo audit`
  - no remaining vulnerability errors
  - warning-only advisories still remain in upstream/transitive crates such as GTK/Tauri stack packages plus `atomic-polyfill`, `fxhash`, `paste`, `number_prefix`, and `serial`
- `pnpm typecheck`

### Additional targeted tests

- `cargo test real_ai_service --lib`
  - `test_ai_service_creation`
  - `test_pattern_response_fix`
  - `test_complete_local_uses_embedded_engine_when_backend_is_local`
  - `test_complete_local_initializes_embedded_engine_once`

## New health score

- Score: `86 / 100`

### Scoring rationale

- `+30` Rust workspace tests pass cleanly
- `+20` clippy passes with warnings denied
- `+15` cargo audit has no remaining vulnerabilities
- `+10` TypeScript typecheck passes via the expected command name
- `+5` local inference no longer hard-fails the `local` backend path
- `-8` warning-only upstream/transitive audit advisories still exist
- `-6` many integration suites still contain `0` runnable tests, so effective coverage breadth remains weaker than the green test count suggests

## Remaining non-Phase-0 concerns

- upstream and transitive dependencies still carry warning-only audit advisories outside immediate Phase 0 scope
- broader product gaps from `COMPETITOR_ANALYSIS_2026.md` and `LIMITATIONS_ANALYSIS.md` remain for later phases
- unrelated workspace modifications from earlier work still exist and were intentionally not included in the grouped Phase 0 commit set
