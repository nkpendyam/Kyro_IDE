# /release
# Run full pre-release checklist and create a new version

Run these checks in order and stop if any fail:
1. cargo test --workspace
2. pnpm test
3. pnpm typecheck
4. cargo audit
5. pnpm audit --audit-level=high
6. cargo clippy --workspace -- -D warnings

If all pass: ask me what version number to use (e.g. 1.0.0).
Then update version in package.json, src-tauri/Cargo.toml, and 
src-tauri/tauri.conf.json. Run: git cliff --unreleased -o CHANGELOG.md.
Show me the changelog. Ask for confirmation before committing and tagging.
