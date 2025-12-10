#!/bin/sh
# Setup git hooks for the repository

set -e

REPO_ROOT="$(git rev-parse --show-toplevel)"

echo "Setting up git hooks..."

# Configure git to use .githooks directory
git config core.hooksPath .githooks

# Make hooks executable
chmod +x "$REPO_ROOT/.githooks/"*

echo "✓ Git hooks configured successfully"
echo ""
echo "Hooks will now run automatically on commit."
echo "To disable, run: git config --unset core.hooksPath"
