---
name: web-researcher
description: Use this skill whenever you need to search the web, find latest docs, check GitHub repos, find open source libraries, research new technology, or verify anything that may have changed recently. Triggers on: "find", "search", "latest", "what is", "how to", "docs for", "best library for", "open source alternative".
---

# Web Research Skill

## When to use this skill
- Finding latest Tauri v2 documentation
- Searching for open source Rust crates on crates.io
- Finding the best npm package for a specific need
- Checking GitHub repos for examples and issues
- Researching new technology before implementing
- Verifying API signatures that may have changed

## How to use built-in tools

### Web Search (find information)
Use the websearch tool to discover:
```
websearch: "tauri v2 updater plugin latest docs"
websearch: "rust crate for SHA256 file verification"
websearch: "react virtualization large lists 2026"
websearch: "HuggingFace Hub API download model rust"
```

### Web Fetch (retrieve specific URL)
Use the webfetch tool to get full content:
```
webfetch: https://docs.rs/tauri/latest/tauri/
webfetch: https://crates.io/crates/git2
webfetch: https://github.com/tauri-apps/tauri/blob/dev/CHANGELOG.md
```

### GitHub Search via gh_grep MCP
Use gh_grep to find real code examples:
```
use the gh_grep tool to find examples of tauri v2 updater implementation in rust
use the gh_grep tool to find open source implementations of speculative decoding in rust
```

### Context7 MCP (always up-to-date docs)
Use context7 to get current library docs:
```
use context7 to get the latest tauri v2 plugin-updater docs
use context7 to get react-window virtualization API docs
use context7 to get tree-sitter rust bindings documentation
```

## Research workflow for Kyro IDE

Before implementing ANY new feature:
1. Search for latest docs for the library being used
2. Search GitHub for real-world examples
3. Check crates.io/npmjs for the most maintained package
4. Verify the API hasn't changed since your training data

## Key sources for Kyro IDE tech stack
- Tauri docs: https://tauri.app/
- Tauri v2 plugin docs: https://docs.rs/tauri-plugin-updater
- tree-sitter: https://tree-sitter.github.io/tree-sitter/
- llama.cpp: https://github.com/ggerganov/llama.cpp
- HuggingFace Hub API: https://huggingface.co/docs/hub/api
- crates.io: https://crates.io/
- Monaco Editor: https://microsoft.github.io/monaco-editor/
- shadcn/ui: https://ui.shadcn.com/docs
