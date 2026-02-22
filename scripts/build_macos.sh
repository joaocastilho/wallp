#!/bin/bash
set -e

# Get the project root directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

cd "$PROJECT_ROOT"

TARGET="aarch64-apple-darwin"

echo "🚀 Building Wallp for macOS (ARM64)..."

# Note: Cross-compiling for macOS from Linux requires osxcross or a similar toolchain.
# On macOS, this will work natively if the target is added.
if ! rustup target list | grep -q "$TARGET (installed)"; then
    echo "ℹ️ Adding target $TARGET..."
    rustup target add $TARGET || echo "⚠️ Could not add target automatically."
fi

cargo build --release --target $TARGET

echo "📂 Creating release directory..."
mkdir -p release

echo "📦 Copying binary to release/wallp-macos..."
cp target/$TARGET/release/wallp release/wallp-macos

echo "✅ Done! macOS binary is in 'release/wallp-macos'"
