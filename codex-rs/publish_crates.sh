#!/bin/bash

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[!]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

# Function to publish a crate
publish_crate() {
    local crate_name="$1"
    local crate_dir="$2"
    
    echo ""
    echo "============================================"
    echo "Publishing $crate_name..."
    echo "============================================"
    
    # Check if already published (optional - you might want to skip this check for updates)
    if cargo search "$crate_name" | grep -q "^$crate_name "; then
        print_warning "$crate_name already exists on crates.io, skipping..."
        return 0
    fi
    
    # Run cargo publish with --dry-run first
    # if cargo publish --dry-run -p "$crate_name" 2>&1 | grep -q "error"; then
    #     print_error "Dry run failed for $crate_name"
    #     return 1
    # fi
    
    # Actually publish
    if cargo publish -p "$crate_name" --no-verify; then
        print_status "$crate_name published successfully!"
        # Wait a bit for crates.io to index the crate
        echo "Waiting for crates.io to index..."
        sleep 30
    else
        print_error "Failed to publish $crate_name"
        return 1
    fi
}

# Main publishing sequence
echo "==========================================="
echo "AGCodex Crate Publishing Script"
echo "==========================================="
echo ""
echo "This script will publish all AGCodex crates to crates.io"
echo "in the correct dependency order."
echo ""
echo "Prerequisites:"
echo "1. You must be logged in to crates.io (cargo login)"
echo "2. All crates must have proper metadata"
echo "3. All tests should pass"
echo ""
read -p "Continue? (y/n) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# Build everything first to ensure it compiles
echo ""
echo "Building all crates to verify compilation..."
if cargo build --all --release; then
    print_status "Build successful!"
else
    print_error "Build failed! Fix errors before publishing."
    exit 1
fi

# Publish in dependency order
# Level 0: No internal dependencies
print_status "Publishing Level 0 crates (no internal dependencies)..."
publish_crate "agcodex-ansi-escape" "ansi-escape"
publish_crate "agcodex-apply-patch" "apply-patch"
publish_crate "agcodex-ast" "ast"
publish_crate "agcodex-execpolicy" "execpolicy"
publish_crate "agcodex-file-search" "file-search"
publish_crate "agcodex-login" "login"
publish_crate "agcodex-mcp-types" "mcp-types"

# Level 1: Dependencies only on Level 0
print_status "Publishing Level 1 crates..."
publish_crate "agcodex-mcp-client" "mcp-client"  # depends on mcp-types
publish_crate "agcodex-protocol" "protocol"      # depends on mcp-types

# Level 2: Dependencies on Level 0 and 1
print_status "Publishing Level 3 crates..."
publish_crate "agcodex-core" "core"              # depends on apply-patch, ast, login, mcp-client, protocol, mcp-types

# Level 3: Core (depends on many)
print_status "Publishing Level 2 crates..."
publish_crate "agcodex-common" "common"          # depends on protocol
publish_crate "agcodex-protocol-ts" "protocol-ts" # depends on protocol

# Level 4: Dependencies on core
print_status "Publishing Level 4 crates..."
publish_crate "agcodex-chatgpt" "chatgpt"        # depends on common, core, login
publish_crate "agcodex-ollama" "ollama"          # depends on core
publish_crate "agcodex-persistence" "persistence" # depends on core
publish_crate "agcodex-linux-sandbox" "linux-sandbox" # depends on common, core

# Level 5: Higher-level components
print_status "Publishing Level 5 crates..."
publish_crate "agcodex-exec" "exec"              # depends on common, core, ollama, protocol
publish_crate "agcodex-mcp-server" "mcp-server"  # depends on common, core, login, protocol, mcp-types

# Level 6: TUI (depends on many)
print_status "Publishing Level 6 crates..."
publish_crate "agcodex-tui" "tui"                # depends on ansi-escape, common, core, file-search, login, ollama, persistence, protocol, mcp-types

# Level 7: CLI (depends on many including tui)
print_status "Publishing Level 7 crates..."
publish_crate "agcodex-cli" "cli"                # depends on chatgpt, common, core, exec, login, mcp-server, protocol, tui, protocol-ts

# Level 8: arg0 (main binary not ready yet)
print_status "Publishing Level 8 crates..."
publish_crate "agcodex-arg0" "arg0"              # depends on apply-patch, core, linux-sandbox
# publish_crate "agcodex" "agcodex"              # Main binary - NOT READY (dependencies commented out)

echo ""
echo "==========================================="
print_status "All crates published successfully!"
echo "==========================================="
echo ""
echo "You can verify the published crates at:"
echo "https://crates.io/search?q=agcodex"
echo ""
echo "Next steps:"
echo "1. Tag the release in git: git tag v0.1.0"
echo "2. Push the tag: git push origin v0.1.0"
echo "3. Create a GitHub release"