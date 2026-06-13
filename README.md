# Kyro IDE — The lightest AI-native IDE

**Competes with VS Code, Antigravity, and Cursor** using **only local LLM models and agents** — with **Atoms-of-Thought reasoning** (less GPU load, get things done), **AirLLM + browser + Ollama** integrated, and **n8n automation development** (GLM5, Kimi K2.5, and other large local models).

## Features

- **Local-only AI:** No cloud by default. [Ollama](https://ollama.ai), embedded LLM, [AirLLM](https://github.com/lyogavin/airllm) (70B on 4–8GB VRAM), [PicoClaw](https://github.com/sipeed/picoclaw) — optional premium API.
- **Atoms of Thought:** Agents reason in atomic subquestions to cut GPU use and deliver results efficiently.
- **Lightest IDE:** Target <100MB RAM idle; heavy work in optional model processes.
- **Integrated browser:** In-app browser for preview, testing, and n8n-style flows.
- **n8n automations:** Build and edit [n8n](https://n8n.io) workflows with local LLMs; large models (GLM5, Kimi K2.5, Qwen2.5) via AirLLM.
- **Cross-OS:** Windows, macOS, Linux via [Tauri](https://tauri.app/) (Rust backend + web frontend).
- **Agents:** Up to 10 parallel AI agents; orchestrator for plan → edit → test → review → deploy; chat + PicoClaw control.

## Architecture

- **Single editor abstraction:** `CodeEditor` is the canonical editor surface (Monaco, LSP bridge, ghost text, inline chat widget, minimap controls).
- **Single theme system:** `ThemeProvider` + `lib/themeSystem` own app theme and Monaco theme registration/application.
- **Single file operations layer:** UI components route file read/write/tree/create/rename/delete via `lib/fileOperations`.
- **Single extensions UI:** Sidebar extensions view is powered by `UnifiedMarketplace`.
- **Accessibility baseline:** Global `AccessibilityProvider` + skip link are mounted in `layout.tsx`.

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop | Tauri v2 |
| Frontend | Next.js 16, React 19, Monaco Editor, Tailwind, shadcn/ui |
| Backend | Rust (LSP, CRDT, MCP, agents, embedded LLM, AirLLM bridge, PicoClaw) |
| AI | Ollama, Candle/llama.cpp, AirLLM (Python), PicoClaw; GLM5/Kimi K2.5 via AirLLM |
| Reasoning | Atoms-of-Thought (AoT) in agents |
| Browser / n8n | Integrated browser; n8n workflow editing with local LLMs |
| Extensions | Open VSX registry API |
| Collab | Yjs (yrs), WebSocket, E2EE (Signal-style) |

## Prerequisites

- Git
- [Bun](https://bun.sh)
- Rust stable toolchain (`rustup`, `cargo`)
- Tauri CLI (project-local via `@tauri-apps/cli`)

Verify your setup:

```bash
bun --version
rustc --version
cargo --version
```

## Installation

```bash
git clone https://github.com/nkpendyam/Kyro_IDE.git
cd Kyro_IDE
bun install
```

Or use the bootstrap scripts, which install/verify prerequisites and run diagnostics:

```powershell
# Windows (PowerShell)
./scripts/setup.ps1
./scripts/check-all.ps1
```

```bash
# macOS / Linux
./scripts/setup.sh
```

Run diagnostics anytime:

```bash
bun run doctor
bun run doctor:full
```

## Usage

```bash
bun run tauri:dev    # start the dev app
bun run tauri:build  # production build
```

Windows installer build:

```powershell
.\scripts\build-windows.ps1
```

Generates a setup `.exe` at `src-tauri/target/release/bundle/nsis/` with desktop/start menu shortcuts and a `kyro` terminal launcher.

## Configuration

Optional AI backends are off by default. Enable via environment variables before a production build/run:

| Variable | Description |
|---|---|
| `KYRO_ENABLE_OLLAMA=1` / `KYRO_OLLAMA_URL` | Enable Ollama backend and its server URL |
| `KYRO_ENABLE_AIRLLM=1` / `KYRO_AIRLLM_URL` | Enable AirLLM backend and its server URL |
| `KYRO_ENABLE_PICOCLAW=1` / `KYRO_PICOCLAW_URL` | Enable PicoClaw backend and its server URL |
| `KYRO_ENABLE_N8N=1` / `KYRO_N8N_URL` | Enable n8n integration and its instance URL |

See [docs/INSTALLATION.md](docs/INSTALLATION.md) for platform-specific dependencies and full setup details.

## Documentation

- [docs/status/ROADMAP.md](docs/status/ROADMAP.md) — version goals
- [docs/KYRO_IDE_2026_ENGINEERING_PLAN.md](docs/KYRO_IDE_2026_ENGINEERING_PLAN.md) — full 2026 engineering plan
- [docs/INSTALLATION.md](docs/INSTALLATION.md) — production setup and platform dependencies

## License

MIT
