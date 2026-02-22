#!/bin/bash
set -e

# Get the project root directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

HOOK_PATH_PUSH="$PROJECT_ROOT/.git/hooks/pre-push"
HOOK_PATH_COMMIT="$PROJECT_ROOT/.git/hooks/pre-commit"

echo "⚓ Installing git hooks..."

# Pre-push hook
cat > "$HOOK_PATH_PUSH" <<EOF
#!/bin/bash
echo "🛡️  Running verification checks before push..."
./scripts/verify.sh
EOF

# Pre-commit hook
cat > "$HOOK_PATH_COMMIT" <<EOF
#!/bin/bash
echo "🛡️  Checking formatting before commit..."
cargo fmt -- --check || { echo "❌ Formatting check failed. Run 'cargo fmt' to fix."; exit 1; }
EOF

chmod +x "$HOOK_PATH_PUSH" "$HOOK_PATH_COMMIT"

echo "✅ Git hooks installed successfully (pre-push and pre-commit)!"
