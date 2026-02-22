#!/bin/bash
set -e

# Get the project root directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

cd "$PROJECT_ROOT"

echo "🚀 Building Wallp for Linux (x64)..."
cargo build --release

echo "📂 Creating release directory..."
mkdir -p release

echo "📦 Copying binary to release/wallp-linux..."
cp target/release/wallp release/wallp-linux

echo "✅ Done! Linux binary is in 'release/wallp-linux'"
