---
name: new-tech-integrator
description: Use this skill when integrating cutting-edge technology into Kyro IDE — MCP servers, new AI models, GraphRAG, speculative decoding, GGUF models, HuggingFace Hub, n8n workflows, WebRTC, or any technology released in the last 2 years. Triggers on: "MCP", "model", "GGUF", "GraphRAG", "speculative", "n8n", "WebRTC", "new feature".
---

# New Technology Integrator Skill

## Rule: Always search before implementing

New technology changes fast. Before writing a single line of code:
```
websearch: "[technology] latest docs 2026"
use context7 to get [library] latest documentation
```

## MCP Servers for Kyro IDE development

### Add these to opencode.json to supercharge OpenCode:

```json
{
  "mcp": {
    "context7": {
      "type": "remote",
      "url": "https://mcp.context7.com/mcp"
    },
    "gh_grep": {
      "type": "remote",
      "url": "https://grep.mcp.vercel.app/mcp"
    },
    "github": {
      "type": "local",
      "command": ["npx", "-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_TOKEN": "${GITHUB_TOKEN}"
      }
    },
    "filesystem": {
      "type": "local",
      "command": ["npx", "-y", "@modelcontextprotocol/server-filesystem",
        "/home/user/Kyro_IDE"
      ]
    }
  }
}
```

### What each MCP gives you
- **context7** — always up-to-date docs for any library (Tauri, React, Rust, etc.)
- **gh_grep** — search GitHub code without leaving OpenCode
- **github** — create PRs, manage issues, releases from inside OpenCode
- **filesystem** — explicit file access beyond current directory

### Using MCP in prompts
```
use context7 to find the latest tauri-plugin-updater API
use the gh_grep tool to find examples of broadcast_cursor in rust tauri projects
use github tools to create a PR for the current branch
```

## AI Models available via GitHub Copilot Pro

### Check what's available on your plan
```bash
opencode models
# or inside TUI: /models
```

### Model selection strategy for Kyro
```
Complex architecture/reasoning → copilot.o3 or copilot.o1
Routine implementation → copilot.gpt-4.1
Quick fixes, small edits → copilot.gpt-4.1-mini (faster, cheaper)
```

### Switch model mid-session
Inside OpenCode TUI, type:
```
/model copilot.o3
```

## Kyro IDE new features using latest tech

### GraphRAG (enhanced context retrieval)
```
websearch: "GraphRAG rust implementation open source 2026"
use context7 to get GraphRAG documentation
```
Then implement in kyro-ai crate as enhancement to existing RAG.

### GGUF Model Management
```
websearch: "GGUF model download HuggingFace Hub rust 2026"
webfetch: https://huggingface.co/docs/hub/api
```
Use reqwest with stream feature for resumable downloads.

### Speculative Decoding improvements
```
websearch: "speculative decoding improvements 2026 llama.cpp"
gh search repos "speculative decoding" --language rust --sort stars
```

### WebRTC for collaboration (replacing WebSocket)
```
websearch: "webrtc rust tauri real-time collaboration 2026"
websearch: "best rust webrtc crate str0m vs webrtc-rs 2026"
```

### n8n workflow editor integration
```
websearch: "n8n embedded editor javascript 2026"
webfetch: https://docs.n8n.io/embed/
```
Embed via Tauri WebviewWindow (same approach as BrowserPreviewPanel).

## Tauri v2 latest features to use

Always check latest Tauri docs before implementing:
```
use context7 to get tauri v2 capabilities documentation
use context7 to get tauri-plugin-updater v2 documentation
websearch: "tauri v2 new features 2026"
```

Key Tauri v2 features for Kyro:
- `tauri-plugin-updater` — auto-updates (use for Session implementation)
- `WebviewWindow` — browser preview panel
- Capabilities system — fine-grained permission control
- `tauri-plugin-shell` — run terminal commands from frontend
- `tauri-plugin-fs` — secure file system access
