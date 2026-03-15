$ErrorActionPreference = "Stop"

function Write-Step {
    param([string]$Message)
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Test-Command {
    param([string]$Name)
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Ensure-WingetPackage {
    param(
        [string]$CommandName,
        [string]$WingetId,
        [string]$DisplayName
    )

    if (Test-Command $CommandName) {
        Write-Host "[OK] $DisplayName already installed." -ForegroundColor Green
        return
    }

    if (-not (Test-Command "winget")) {
        throw "winget is required to install $DisplayName automatically. Install App Installer from Microsoft Store."
    }

    Write-Host "[INFO] Installing $DisplayName via winget ($WingetId)..." -ForegroundColor Yellow
    winget install --id $WingetId --silent --accept-source-agreements --accept-package-agreements
}

Write-Step "Checking Windows prerequisites"
if ($PSVersionTable.PSVersion.Major -lt 5) {
    throw "PowerShell 5+ is required."
}

Ensure-WingetPackage -CommandName "node" -WingetId "OpenJS.NodeJS.LTS" -DisplayName "Node.js LTS (20+)"
Ensure-WingetPackage -CommandName "rustup" -WingetId "Rustlang.Rustup" -DisplayName "Rustup"
Ensure-WingetPackage -CommandName "bun" -WingetId "Oven-sh.Bun" -DisplayName "Bun"

if (-not (Test-Command "cargo")) {
    Write-Step "Initializing Rust toolchain"
    rustup default stable
}

Write-Step "Checking Tauri Windows dependencies"

$vsWherePath = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vsWherePath)) {
    Write-Host "[INFO] Installing Visual Studio Build Tools (C++ workload)..." -ForegroundColor Yellow
    winget install --id Microsoft.VisualStudio.2022.BuildTools --silent --accept-source-agreements --accept-package-agreements --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
} else {
    Write-Host "[OK] Visual Studio Build Tools detected." -ForegroundColor Green
}

$webView2RegPath = "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
if (-not (Test-Path $webView2RegPath)) {
    Write-Host "[INFO] Installing WebView2 Runtime..." -ForegroundColor Yellow
    winget install --id Microsoft.EdgeWebView2Runtime --silent --accept-source-agreements --accept-package-agreements
} else {
    Write-Host "[OK] WebView2 Runtime detected." -ForegroundColor Green
}

Write-Step "Installing project dependencies"
bun install

Write-Step "Running kyro doctor"
bun run doctor

Write-Step "Environment verification"
node --version
bun --version
rustc --version
cargo --version

Write-Host "" 
Write-Host "Setup complete. Next steps:" -ForegroundColor Green
Write-Host "  1) Run checks:  .\scripts\check-all.ps1"
Write-Host "  2) Start dev:    bun run tauri:dev"
Write-Host "  3) Build app:    bun run tauri:build"
