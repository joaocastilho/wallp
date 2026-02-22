#!/bin/bash
set -e

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

usage() {
    echo "Usage: $0 [windows|linux|macos|all]"
    echo ""
    echo "Commands:"
    echo "  windows   Build for Windows (x64 GNU)"
    echo "  linux     Build for Linux (x64)"
    echo "  macos     Build for macOS (ARM64)"
    echo "  all       Build for all platforms (default)"
    exit 1
}

TARGET=${1:-all}

case $TARGET in
    windows)
        "$SCRIPT_DIR/scripts/build_windows.sh"
        ;;
    linux)
        "$SCRIPT_DIR/scripts/build_linux.sh"
        ;;
    macos)
        "$SCRIPT_DIR/scripts/build_macos.sh"
        ;;
    all)
        "$SCRIPT_DIR/scripts/build_linux.sh"
        "$SCRIPT_DIR/scripts/build_windows.sh"
        echo "🍎 Attempting macOS build..."
        "$SCRIPT_DIR/scripts/build_macos.sh" || echo "❌ macOS build failed. (Expected if cross-tools are missing)"
        ;;
    *)
        usage
        ;;
esac

echo "🌟 Done! Check the 'release/' directory."
ls -l release/
