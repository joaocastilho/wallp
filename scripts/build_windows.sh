#!/bin/bash
set -e

# Get the project root directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

cd "$PROJECT_ROOT"

echo "🚀 Building Wallp for Windows (x64 GNU)..."
cargo build --release --target x86_64-pc-windows-gnu

echo "📂 Creating release directory..."
mkdir -p release

echo "📦 Copying binary to release/wallp.exe..."
cp target/x86_64-pc-windows-gnu/release/wallp.exe release/wallp.exe

echo "✅ Done! Windows binary is in 'release/wallp.exe'"
