#!/bin/bash

# AGCodex Binary Symlink Script
# Creates backward compatibility symlink: codex -> agcodex

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo -e "${BLUE}    AGCodex Backward Compatibility Setup       ${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo ""

# Determine cargo bin directory
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"

if [ ! -d "$CARGO_BIN" ]; then
    echo -e "${RED}Error: Cargo bin directory not found at $CARGO_BIN${NC}"
    echo "Please ensure Rust/Cargo is installed correctly."
    exit 1
fi

echo -e "${BLUE}Checking for AGCodex binary...${NC}"

if [ -f "$CARGO_BIN/agcodex" ]; then
    echo -e "${GREEN}✓ Found agcodex at $CARGO_BIN/agcodex${NC}"
    
    if [ -e "$CARGO_BIN/codex" ]; then
        if [ -L "$CARGO_BIN/codex" ]; then
            # It's a symlink, update it
            echo -e "${YELLOW}Updating existing symlink...${NC}"
            rm "$CARGO_BIN/codex"
            ln -s "$CARGO_BIN/agcodex" "$CARGO_BIN/codex"
            echo -e "${GREEN}✓ Updated symlink: codex -> agcodex${NC}"
        else
            # It's a real file
            echo -e "${YELLOW}Warning: $CARGO_BIN/codex exists and is not a symlink${NC}"
            echo "This might be the old Codex binary."
            echo ""
            read -p "Would you like to backup the old binary and create the symlink? (y/N) " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                mv "$CARGO_BIN/codex" "$CARGO_BIN/codex.old"
                ln -s "$CARGO_BIN/agcodex" "$CARGO_BIN/codex"
                echo -e "${GREEN}✓ Backed up old binary to codex.old${NC}"
                echo -e "${GREEN}✓ Created symlink: codex -> agcodex${NC}"
            else
                echo "Symlink not created. The old 'codex' command remains."
            fi
        fi
    else
        # Create new symlink
        ln -s "$CARGO_BIN/agcodex" "$CARGO_BIN/codex"
        echo -e "${GREEN}✓ Created symlink: codex -> agcodex${NC}"
    fi
    
    echo ""
    echo -e "${BLUE}You can now use both commands:${NC}"
    echo "  • agcodex (recommended)"
    echo "  • codex (backward compatibility)"
else
    echo -e "${YELLOW}AGCodex binary not found at $CARGO_BIN/agcodex${NC}"
    echo ""
    echo "Please build and install AGCodex first:"
    echo -e "${BLUE}  cargo install --path cli${NC}"
    echo ""
    echo "Then run this script again."
    exit 1
fi

echo ""