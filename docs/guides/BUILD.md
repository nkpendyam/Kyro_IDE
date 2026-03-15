# Kyro IDE Build Guide

This guide defines the canonical Kyro build path from source to installers.

## Canonical Build Pipeline

Kyro uses:
- Next.js static export (`output: "export"`) to `out/`
- Tauri `build.frontendDist = "../out"`
- Rust/Tauri bundling from `src-tauri/target/release/bundle/`

The required order is:
1. `bun install`
2. `bun run verify:prod-config`
3. `bun run build` (generates `out/`)
4. `bun run tauri:build`

## Development Run (Tauri + Next)

```bash
bun run tauri:dev
```

Tauri runs `beforeDevCommand: "bun install && bun run dev"` and uses `devUrl: http://localhost:3000`.

## Production Build

### Quick path

```bash
bun run tauri:build
```

### Explicit path

```bash
bun install
bun run verify:prod-config
bun run build
bun run tauri:build
```

## TypeScript Correctness Gate

Kyro production builds do **not** ignore TypeScript errors by default.

- `next.config.ts` only ignores TS build errors when `KYRO_ALLOW_TS_BUILD_ERRORS=1`.
- This flag is emergency-only and should not be used in CI/release pipelines.
- CI runs `bun run type-check` and fails on errors.

## Rust Correctness Gate

Recommended local backend checks:

```bash
cd src-tauri
cargo fmt -- --check
cargo clippy --workspace --all-targets
cargo check --workspace --locked
cargo test --workspace --lib --tests
```

`./scripts/check-all.ps1` also includes cargo checks and frontend type-check.

## Production Sanity Check

`bun run verify:prod-config` validates:
- `next.config.ts` uses static export (`output: "export"`)
- `src-tauri/tauri.conf.json` uses `frontendDist: "../out"`
- `VERSION` has semantic-version format
- Optional AI backend feature flags have required URL env vars

Optional feature env combinations:
- `KYRO_ENABLE_AIRLLM=1` requires `KYRO_AIRLLM_URL`
- `KYRO_ENABLE_OLLAMA=1` requires `KYRO_OLLAMA_URL`
- `KYRO_ENABLE_PICOCLAW=1` requires `KYRO_PICOCLAW_URL`
- `KYRO_ENABLE_N8N=1` requires `KYRO_N8N_URL`

## Installer Outputs

After `bun run tauri:build`, artifacts are in:

- Windows: `src-tauri/target/release/bundle/nsis/*.exe`, `src-tauri/target/release/bundle/msi/*.msi`
- macOS: `src-tauri/target/release/bundle/dmg/*.dmg`
- Linux: `src-tauri/target/release/bundle/appimage/*.AppImage` and/or `*.deb`

Windows installer hooks are in `src-tauri/windows/installer-hooks.nsh`.

## Release + Updates

The release workflow is `.github/workflows/release.yml`:
- Triggered on semantic tags (`v*`)
- Builds Windows/macOS/Linux artifacts
- Runs artifact smoke checks and publishes checksums
- Supports optional Windows code signing and macOS notarization via secrets

Use semantic tags and keep `CHANGELOG.md` aligned with each release.

## Local Verification Checklist

Before merge or release:

```bash
bun run doctor
bun run lint
bun run type-check
bun run test
bun run build
cd src-tauri && cargo check --workspace --locked && cargo test --workspace --lib --tests
```
