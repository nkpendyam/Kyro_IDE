#!/bin/bash
# Development setup script for Kyro IDE
# This script sets up the development environment with hot reload

set -e

echo "🚀 Setting up Kyro IDE development environment..."

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed. Please install Node.js 18+ first."
    exit 1
fi

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust is not installed. Please install Rust first."
    exit 1
fi

# Check if Bun is installed (optional, fallback to npm)
if command -v bun &> /dev/null; then
    PKG_MANAGER="bun"
    echo "✅ Using Bun as package manager"
else
    PKG_MANAGER="npm"
    echo "✅ Using npm as package manager"
fi

# Install frontend dependencies
echo "📦 Installing frontend dependencies..."
$PKG_MANAGER install

# Build Rust backend in debug mode
echo "🦀 Building Rust backend (debug mode)..."
cd src-tauri
cargo build
cd ..

# Generate Prisma client
echo "🗄️  Generating Prisma client..."
$PKG_MANAGER run db:generate

echo "🩺 Running kyro doctor..."
$PKG_MANAGER run doctor

echo "✅ Development environment setup complete!"
echo ""
echo "To start development:"
echo "  1. Start Tauri + Next: $PKG_MANAGER run tauri:dev"
