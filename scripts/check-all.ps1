$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Invoke-Step {
    param(
        [string]$Label,
        [scriptblock]$Command
    )

    Write-Step $Label
    & $Command

    if ($LASTEXITCODE -ne 0) {
        throw "$Label failed with exit code $LASTEXITCODE"
    }
}

Invoke-Step "Install dependencies" { bun install }

Invoke-Step "Frontend lint" { bun run lint }

Invoke-Step "Frontend type-check" { bun run type-check }

Invoke-Step "Frontend tests" { bun run test }

Invoke-Step "Frontend build (ensures out/ exists for Tauri)" { bun run build }

Push-Location src-tauri
Invoke-Step "Rust cargo check" { cargo check --workspace --locked }

Invoke-Step "Rust tests" { cargo test --workspace --lib --tests }

Invoke-Step "Rust clippy" { cargo clippy --workspace --all-targets }
Pop-Location

Write-Host "All checks passed." -ForegroundColor Green
