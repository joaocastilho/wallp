#!/bin/bash
set -e

# Get the project root directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

cd "$PROJECT_ROOT"

echo "🎨 Checking formatting..."
cargo fmt -- --check

echo "🔍 Running clippy..."
cargo clippy --all-targets -- -D warnings -W clippy::pedantic

echo "🧪 Running tests..."
cargo test

echo "✅ All checks passed!"
