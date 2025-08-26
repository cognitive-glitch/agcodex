#!/bin/bash
# Setup pre-commit hooks for AGAGCodex

set -e

echo "Setting up pre-commit hooks for AGAGCodex..."

# Check if pre-commit is installed
if ! command -v pre-commit &> /dev/null; then
    echo "pre-commit is not installed. Installing..."
    pip install pre-commit || pip3 install pre-commit
fi

# Install the pre-commit hooks
pre-commit install

# Run pre-commit on all files to verify setup
echo "Running pre-commit checks on all files..."
pre-commit run --all-files || true

echo "✅ Pre-commit hooks installed successfully!"
echo ""
echo "The following hooks will run on every commit:"
echo "  • cargo fmt --all -- --check"
echo "  • cargo check --all-features --all-targets --workspace --tests"
echo "  • cargo clippy --all-features --all-targets --workspace --tests -- -D warnings"
echo ""
echo "To run manually: pre-commit run --all-files"
echo "To skip hooks: git commit --no-verify"