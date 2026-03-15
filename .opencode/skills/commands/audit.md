# /audit
# Run full security and quality audit

Run all of these and produce a summary report:
1. cargo audit (Rust CVEs)
2. pnpm audit --audit-level=moderate (JS CVEs)
3. cargo clippy --workspace -- -D warnings -D clippy::unwrap_used
4. pnpm typecheck
5. pnpm lint
6. cargo tarpaulin --workspace (test coverage)

Produce AUDIT_REPORT.md with: findings table, severity levels, 
recommended fixes, current test coverage percentage.
Fix any CRITICAL or HIGH severity findings immediately.
