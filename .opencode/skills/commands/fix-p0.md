# /fix-p0
# Shortcut to fix all P0 broken command wires in one go

Read AGENTS.md. Implement all 7 missing P0 Tauri commands:
git_stage, git_unstage, git_stage_all, git_unstage_all, git_discard, 
git_stage_hunk in src-tauri/src/git/mod.rs and broadcast_cursor in 
src-tauri/src/collab/mod.rs. Use ? operator only, no unwrap(). Register 
all in main.rs. Add TypeScript bindings. Write 2 tests each. Run 
cargo test --workspace and fix all failures. Commit each separately.
