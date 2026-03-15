---
name: open-source-finder
description: Use this skill when you need to find the best open source library, crate, npm package, or tool for any task. Also use when looking for open source alternatives to paid services, or when referencing how other open source projects solved the same problem. Triggers on: "find a library", "best package for", "open source alternative", "how does X project do this", "what crate should I use".
---

# Open Source Finder Skill

## Finding the right crate/package for Kyro IDE

### Step 1 — Search pattern
Always search in this order:
1. websearch for current recommendations
2. webfetch crates.io or npmjs for stats
3. gh_grep for real-world usage examples

### For Rust crates
```
websearch: "best rust crate for [task] site:crates.io OR site:lib.rs"
webfetch: https://crates.io/search?q=[keyword]&sort=downloads
webfetch: https://lib.rs/search?q=[keyword]
```

Minimum acceptance criteria:
- Downloads: >100k/month (crates.io)
- Last updated: within 6 months
- License: MIT or Apache-2.0
- Open issues: not hundreds of unresolved bugs

### For npm packages
```
websearch: "best npm package for [task] 2026"
webfetch: https://www.npmjs.com/search?q=[keyword]
webfetch: https://bundlephobia.com/package/[package-name]
```

Minimum acceptance criteria:
- Weekly downloads: >50k
- Bundle size: reasonable for use case
- TypeScript types: included (not @types/separate)

## Kyro IDE specific recommendations

### Already decided (DO NOT change without research)
| Task | Package | Why |
|------|---------|-----|
| Git ops | git2 (Rust) | Most complete git bindings for Rust |
| AST parsing | tree-sitter | Industry standard, used by VS Code |
| AI inference | llama.cpp via bindings | Best local LLM performance |
| UI components | shadcn/ui | Already integrated |
| State | Zustand | Already integrated |
| Editor | Monaco | Already integrated |
| E2EE | ring + custom Signal Protocol | Already built |

### Still to decide (research before implementing)
```
# Model download with resume support
websearch: "rust crate http download resume range headers 2026"

# GraphRAG for enhanced context retrieval
websearch: "graph rag rust implementation open source 2026"

# Real-time test output streaming
websearch: "rust stream subprocess output to websocket tauri"
```

## Finding how other IDEs solved the same problem

### Reference these open source projects
```bash
# How Zed handles LSP
gh search code "language_server" --repo zed-industries/zed --language rust

# How Lapce handles file tree virtualization
gh search code "virtual_list file_tree" --repo lapce/lapce

# How Helix handles tree-sitter
gh search code "tree_sitter highlight" --repo helix-editor/helix --language rust

# How VS Code handles Monaco lazy loading
gh search code "monaco lazy" --repo microsoft/vscode --language typescript
```

## New technology research pattern

When implementing something new to Kyro (GraphRAG, browser preview, etc.):

1. Search for the concept:
```
websearch: "[technology name] rust implementation 2026"
websearch: "[technology name] tauri integration example"
```

2. Find the best open source example:
```
gh search repos "[technology]" --language rust --sort stars --limit 5
```

3. Read their implementation:
```
webfetch: https://github.com/[best-result]/blob/main/src/[relevant-file]
```

4. Adapt for Kyro, citing the reference in code comments:
```rust
// Adapted from: github.com/example/repo/blob/main/src/module.rs
```
