# Development setup script for Kyro IDE (Windows)
# This script sets up the development environment with hot reload

$ErrorActionPreference = "Stop"

Write-Host "🚀 Setting up Kyro IDE development environment..." -ForegroundColor Cyan

# Check if Node.js is installed
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Node.js is not installed. Please install Node.js 18+ first." -ForegroundColor Red
    exit 1
}

# Check if Rust is installed
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Rust is not installed. Please install Rust first." -ForegroundColor Red
    exit 1
}

# Check if Bun is installed (optional, fallback to npm)
if (Get-Command bun -ErrorAction SilentlyContinue) {
    $PKG_MANAGER = "bun"
    Write-Host "✅ Using Bun as package manager" -ForegroundColor Green
} else {
    $PKG_MANAGER = "npm"
    Write-Host "✅ Using npm as package manager" -ForegroundColor Green
}

# Install frontend dependencies
Write-Host "📦 Installing frontend dependencies..." -ForegroundColor Cyan
& $PKG_MANAGER install

# Build Rust backend in debug mode
Write-Host "🦀 Building Rust backend (debug mode)..." -ForegroundColor Cyan
Push-Location src-tauri
cargo build
Pop-Location

# Generate Prisma client
Write-Host "🗄️  Generating Prisma client..." -ForegroundColor Cyan
& $PKG_MANAGER run db:generate

Write-Host "🩺 Running kyro doctor..." -ForegroundColor Cyan
& $PKG_MANAGER run doctor

Write-Host "✅ Development environment setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "To start development:"
Write-Host "  1. Start Tauri + Next: $PKG_MANAGER run tauri:dev"
