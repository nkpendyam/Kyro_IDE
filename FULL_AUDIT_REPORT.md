# FULL AUDIT REPORT

## 1. Test results

- Command run: `cargo test --workspace`
- Result: passed
- Summary:
  - Rust unit/integration/doc tests passed: 39 executable tests passed, 0 failed
  - Ignored doc tests: 3
  - Multiple integration suites exist but currently contain 0 runnable tests: `auth_test`, `collaboration_integration`, `collaboration_test`, `e2ee_test`, `integration_tests`, `lsp_test`, `mod`, `performance_test`, `security_test`, `vscode_compat_test`
- Failures: none in this run
- Notes:
  - The workspace test command succeeded, but coverage breadth is still limited because many test binaries contain zero tests.

## 2. Code quality

- Command run: `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used`
- Result: failed
- Total reported issues: 31 errors

### Clippy errors

1. `src/ai/quality_gate.rs:216` — `context.project_path.unwrap()` on `Option`
2. `src/commands/gitcrdt.rs:173` — `git2::Signature::now(...).unwrap()` inside `unwrap_or_else`
3. `src/commands/remote.rs:53` — `map` used where `inspect` is required (`clippy::manual_inspect`)
4. `src/rag/vector_store.rs:73` — `self.vectors.lock().unwrap()`
5. `src/rag/vector_store.rs:85` — `self.vectors.lock().unwrap()`
6. `src/rag/vector_store.rs:90` — `query.as_slice().unwrap()`
7. `src/swarm_ai/p2p_swarm.rs:524-526` — `duration_since(...).unwrap()`
8. `src/swarm_ai/mod.rs:235-237` — `self.local_engine.as_ref().unwrap()`
9. `src/benchmark/startup.rs:684` — `tokio::runtime::Runtime::new().unwrap()`
10. `src/benchmark/startup.rs:697` — `tokio::runtime::Runtime::new().unwrap()`
11. `src/update/delta.rs:251` — `Self::new().unwrap()`
12. `src/lsp_tower/backend.rs:1003` — `Url::parse("file:///unknown").unwrap()`
13. `src/lsp_tower/backend.rs:1075` — `Url::parse("file:///unknown").unwrap()`
14. `src/inference/sampler.rs:85` — `partial_cmp(...).unwrap()`
15. `src/inference/sampler.rs:106` — `partial_cmp(...).unwrap()`
16. `src/inference/sampler.rs:162` — `partial_cmp(...).unwrap()`
17. `src/e2ee/double_ratchet.rs:97` — `self.receiving_chain.as_mut().unwrap()`
18. `src/agent_editor/tools.rs:257` — `current_hunk.as_mut().unwrap()`
19. `src/agent_editor/tools.rs:263` — `current_hunk.as_mut().unwrap()`
20. `src/agent_editor/tools.rs:277` — `current_hunk.as_mut().unwrap()`
21. `src/agent_editor/tools.rs:283` — `current_hunk.as_mut().unwrap()`
22. `src/agent_editor/tools.rs:324` — `current_hunk.as_mut().unwrap()`
23. `src/agent_editor/tools.rs:330` — `current_hunk.as_mut().unwrap()`
24. `src/agent_editor/tools.rs:336` — `current_hunk.as_mut().unwrap()`
25. `src/agent_editor/tools.rs:342` — `current_hunk.as_mut().unwrap()`
26. `src/agent_editor/tools.rs:346` — `current_hunk.as_mut().unwrap()`
27. `src/agent_editor/tools.rs:352` — `current_hunk.as_mut().unwrap()`
28. `src/agent_editor/tools.rs:354` — `current_hunk.as_mut().unwrap()`
29. `src/agent_editor/tools.rs:360` — `current_hunk.as_mut().unwrap()`
30. `src/picoclaw/mod.rs:288` — `partial_cmp(...).unwrap()`
31. `src/picoclaw/mod.rs:678` — `partial_cmp(...).unwrap()`

### Interpretation

- The Rust codebase does not currently meet the AGENTS.md rule `NEVER use unwrap()`.
- At least one non-unwrap lint is also promoted to error: `clippy::manual_inspect` in `src/commands/remote.rs:53`.

## 3. Security

### Rust audit

- Command run: `cargo audit`
- Result: failed
- Vulnerabilities found: 7
- Additional warnings (unmaintained/unsound crates): 24

### CVEs / RustSec advisories reported as vulnerabilities

1. `RUSTSEC-2026-0037` — `quinn-proto 0.11.13`
   - Title: Denial of service in Quinn endpoints
   - Severity: high (8.7)
   - Via: `reqwest 0.12.28`
   - Fix: upgrade to `>=0.11.14`

2. `RUSTSEC-2024-0438` — `wasmtime 20.0.2`
   - Title: Wasmtime doesn't fully sandbox all the Windows device filenames
   - Fix: upgrade to `>=24.0.2` or newer safe ranges

3. `RUSTSEC-2024-0439` — `wasmtime 20.0.2`
   - Title: Race condition could lead to WebAssembly control-flow integrity and type safety violations
   - Severity: low (2.9)

4. `RUSTSEC-2025-0046` — `wasmtime 20.0.2`
   - Title: Host panic with `fd_renumber` WASIp1 function
   - Severity: low (3.3)

5. `RUSTSEC-2025-0118` — `wasmtime 20.0.2`
   - Title: Unsound API access to a WebAssembly shared linear memory
   - Severity: low (1.8)

6. `RUSTSEC-2026-0020` — `wasmtime 20.0.2`
   - Title: Guest-controlled resource exhaustion in WASI implementations
   - Severity: medium (6.9)

7. `RUSTSEC-2026-0021` — `wasmtime 20.0.2`
   - Title: Panic adding excessive fields to a `wasi:http/types.fields` instance
   - Severity: medium (6.9)

### Additional RustSec warnings (not counted in the 7 vulnerability total by cargo-audit output)

- `RUSTSEC-2026-0008` — `git2 0.18.3` unsoundness
- `RUSTSEC-2024-0429` — `glib 0.18.5` unsound iterator implementation
- GTK3 stack unmaintained warnings including:
  - `RUSTSEC-2024-0411`, `0412`, `0413`, `0414`, `0415`, `0416`, `0417`, `0418`, `0419`, `0420`
- Other unmaintained packages flagged:
  - `RUSTSEC-2023-0089` `atomic-polyfill`
  - `RUSTSEC-2025-0141` `bincode`
  - `RUSTSEC-2025-0057` `fxhash`
  - `RUSTSEC-2025-0119` `number_prefix`
  - `RUSTSEC-2024-0436` `paste`
  - `RUSTSEC-2024-0370` `proc-macro-error`
  - `RUSTSEC-2017-0008` `serial`
  - `RUSTSEC-2025-0075`, `0080`, `0081`, `0098`, `0100` in the `unic-*` stack

### pnpm audit

- Command run: `pnpm audit --audit-level=moderate`
- Result: could not run
- Reason: `ERR_PNPM_AUDIT_NO_LOCKFILE` — no `pnpm-lock.yaml` exists in the repository root, so pnpm could not audit JS dependencies.

## 4. TypeScript errors

### TypeScript typecheck

- Requested command: `pnpm typecheck`
- Actual result: command failed because the script does not exist in `package.json`
- Error: `Command "typecheck" not found`
- Available script in repo: `pnpm type-check`

### Actual TypeScript compiler run

- Command run: `pnpm type-check`
- Result: passed
- TypeScript errors found: 0

### Lint-related frontend error

- Command run: `pnpm lint`
- Result: failed with 1 error
- Finding:
  - `src/components/collaboration/EditorPresence.tsx:177`
  - Rule: `react-hooks/preserve-manual-memoization`
  - Reason: `useCallback` dependencies omit `currentUserName` even though it is referenced in the callback body

## 5. Test coverage

### Requested coverage command

- Command run: `cargo tarpaulin --workspace`
- Result: could not run
- Reason: `cargo-tarpaulin` is not installed in the environment (`cargo: no such command: tarpaulin`)

### Coverage percentage per module

- Not available from this audit run because tarpaulin did not execute.
- The only project-stated baseline remains the AGENTS.md note:
  - overall coverage: `60%`
  - target coverage: `80%`

### Coverage-relevant observations from test run

- `kyro-ide` has 19 library tests and 19 binary tests executing successfully
- Many integration test targets currently run `0 tests`, which means real coverage breadth is lower than the passing test summary suggests
- No per-module percentages could be measured in this environment without installing `cargo-tarpaulin`

## 6. Overall health score out of 100

- Score: `58 / 100`

### Scoring rationale

- `+25` Rust tests are currently green across the workspace
- `+10` TypeScript compiler passes when using the actual script name
- `-15` Clippy fails with 31 promoted errors, many from `unwrap()` in production code
- `-12` Rust dependency audit reports 7 vulnerabilities, including 1 high severity and multiple medium issues in `wasmtime`
- `-5` Frontend lint currently fails
- `-5` JS dependency audit cannot run because there is no `pnpm-lock.yaml`
- `-5` Coverage audit cannot run because `cargo-tarpaulin` is missing and no measured module percentages are available
- `-5` Effective test breadth is weaker than the pass count suggests because many integration suites contain zero tests

## Raw command outcomes

- `cargo test --workspace` -> passed
- `cargo clippy --workspace -- -D warnings -D clippy::unwrap_used` -> failed
- `cargo audit` -> failed
- `pnpm typecheck` -> failed because script is missing
- `pnpm type-check` -> passed
- `pnpm lint` -> failed
- `pnpm audit --audit-level=moderate` -> failed because no lockfile exists
- `cargo tarpaulin --workspace` -> failed because tarpaulin is not installed
