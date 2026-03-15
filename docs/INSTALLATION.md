# Kyro IDE Installation Guide

This guide is the production baseline for installing and building Kyro IDE on Windows, macOS, and Linux.

## 0) Bootstrap Scripts (recommended)

From the repository root in PowerShell:

```powershell
.\scripts\setup.ps1
```

This installs/verifies Rust, Node.js LTS, Bun, Tauri Windows prerequisites (Build Tools + WebView2), runs `bun install`, and runs `kyro doctor`.

On macOS/Linux:

```bash
./scripts/setup.sh
```

This verifies required tools, checks OS prerequisites, installs dependencies, and runs `kyro doctor`.

## 1) Prerequisites (All Platforms)

- Git
- Bun (recommended package/runtime): https://bun.sh
- Rust stable toolchain (`rustup`, `cargo`): https://rustup.rs
- Tauri CLI (project-local via `@tauri-apps/cli` in `package.json`)

Verify tools:

```bash
bun --version
rustc --version
cargo --version
```

Run environment diagnostics:

```bash
bun run doctor
bun run doctor:full
```

- `doctor` checks required prerequisites (Node, Bun, Rust, Cargo, Tauri project dependency, WebView2 on Windows).
- `doctor:full` additionally checks optional integrations (AirLLM, Ollama, PicoClaw, n8n).

## 2) Clone + Bootstrap

```bash
git clone https://github.com/nkpendyam/Kyro_IDE.git
cd Kyro_IDE
bun install
```

## 3) OS-Specific System Dependencies

## Windows 10/11

- Visual Studio 2022 Build Tools (C++ workload)
- WebView2 Runtime (usually preinstalled on Win11; install manually if missing)

Recommended (PowerShell as Administrator):

```powershell
winget install Microsoft.VisualStudio.2022.BuildTools
winget install Microsoft.EdgeWebView2Runtime
```

## macOS (Intel / Apple Silicon)

- Xcode Command Line Tools

```bash
xcode-select --install
```

## Linux (Ubuntu/Debian)

```bash
sudo apt update
sudo apt install -y \
  build-essential \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  librsvg2-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  patchelf
```

## 4) Development Run

From repo root:

```bash
bun run tauri:dev
```

The enforced order is:
1. `bun install`
2. start Next.js dev server
3. launch Tauri with `devUrl=http://localhost:3000`

Run full local verification before committing:

```powershell
.\scripts\check-all.ps1
```

## 5) Production Build (Cross-Platform)

From repo root:

```bash
bun run tauri:build
```

The enforced order is:
1. `bun install`
2. `bun run build` (Next.js static export to `out/`)
3. `tauri build` (Rust compile + bundling)

Output artifacts are generated under `src-tauri/target/release/bundle/`.

## 5.1) Windows Installer Experience (EXE-first)

Kyro now ships with a standard NSIS Windows setup executable.

Build it:

```powershell
.\scripts\build-windows.ps1
```

Primary installer artifact:

- `src-tauri/target/release/bundle/nsis/*.exe` (recommended installer)
- `src-tauri/target/release/bundle/msi/*.msi` (enterprise/MSI environments)

Installer behavior for Windows users:

- Setup wizard (`.exe`) with normal IDE-style installation flow.
- Start Menu entry under `Kyro IDE`.
- Desktop shortcut creation.
- Explorer context menu entries:
  - `Open with Kyro IDE` for files
  - `Open with Kyro IDE` for folders/background
- Terminal launcher command: `kyro`.

After install, from PowerShell/CMD:

```powershell
kyro .
```

## 6) Platform Build Notes

- Windows bundles: NSIS `.exe` (recommended) and MSI
- macOS bundles: DMG/App bundle
- Linux bundles: AppImage/DEB (depending on target + host tools)

If cross-compiling, ensure Rust target toolchains and platform SDK/toolchains are installed.

## 7) AI Backend Installation

Kyro runs local-first AI. Install at least one backend before enabling AI features.

## Embedded LLM (in-app)

- No separate runtime service required.
- Best for lightweight/local fallback scenarios.

## Ollama (recommended baseline backend)

Install Ollama and pull at least one model:

```bash
# after installing Ollama runtime
ollama pull qwen2.5-coder:7b
```

Then verify:

```bash
ollama list
```

## AirLLM (optional large-model backend)

AirLLM lives in `services/airllm-service/`.

```bash
cd services/airllm-service
python -m venv .venv
# Windows: .venv\Scripts\activate
# macOS/Linux: source .venv/bin/activate
pip install -r requirements.txt
python main.py
```

Use this backend for larger local models when system VRAM/RAM allows.

By default, doctor checks `http://127.0.0.1:8765/health`. Override with:

```bash
export KYRO_AIRLLM_URL=http://127.0.0.1:8765/health
# PowerShell: $env:KYRO_AIRLLM_URL = "http://127.0.0.1:8765/health"
```

## Optional backend feature flags

Kyro can run without optional providers. If you enable a provider in production, set its URL:

- `KYRO_ENABLE_AIRLLM=1` + `KYRO_AIRLLM_URL`
- `KYRO_ENABLE_OLLAMA=1` + `KYRO_OLLAMA_URL`
- `KYRO_ENABLE_PICOCLAW=1` + `KYRO_PICOCLAW_URL`
- `KYRO_ENABLE_N8N=1` + `KYRO_N8N_URL`

Production sanity checks validate these combinations during build.

## 8) Troubleshooting Quick Checks

- `frontendDist` errors (`../out` missing): run `bun run build` from repo root.
- Tauri CLI not found: use project scripts (`bun run tauri:dev` / `bun run tauri:build`) instead of global commands.
- Linux WebKit errors: verify `libwebkit2gtk-4.1-dev` and GTK packages are installed.
- Windows linker errors: ensure VS Build Tools C++ workload is installed.

## 9) CI Parity

Local build pipeline now matches CI expectations:

- Frontend static export is built explicitly (`bun run build`).
- Rust/Tauri jobs consume real `out/` assets (no placeholder `out/index.html` hack).

## 10) Runtime Wiring Baseline

Current production wiring expectations:

- `src/components/editor/CodeEditor.tsx` is the primary editor integration point.
- `src/lib/themeSystem.ts` + `src/components/theme/ThemeProvider.tsx` are the single theme source of truth.
- `src/lib/fileOperations.ts` is the file I/O abstraction for frontend flows.
- `src/components/extensions/UnifiedMarketplace.tsx` is the main extensions marketplace UI.
- `src/components/accessibility/AccessibilityProvider.tsx` is mounted globally from `src/app/layout.tsx`.

## 11) Dead Export Hygiene

- Run `npx ts-prune` before larger refactor merges.
- Treat intentional exceptions as explicit dev-only helpers and keep them documented.
