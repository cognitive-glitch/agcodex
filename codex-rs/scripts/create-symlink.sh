#!/bin/bash

# AGAGCodex Binary Symlink Script
# Creates backward compatibility symlink: agcodex -> agagcodex

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo -e "${BLUE}    AGAGCodex Backward Compatibility Setup       ${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo ""

# Determine cargo bin directory
CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"

if [ ! -d "$CARGO_BIN" ]; then
    echo -e "${RED}Error: Cargo bin directory not found at $CARGO_BIN${NC}"
    echo "Please ensure Rust/Cargo is installed correctly."
    exit 1
fi

echo -e "${BLUE}Checking for AGAGCodex binary...${NC}"

if [ -f "$CARGO_BIN/agagcodex" ]; then
    echo -e "${GREEN}✓ Found agagcodex at $CARGO_BIN/agagcodex${NC}"
    
    if [ -e "$CARGO_BIN/agcodex" ]; then
        if [ -L "$CARGO_BIN/agcodex" ]; then
            # It's a symlink, update it
            echo -e "${YELLOW}Updating existing symlink...${NC}"
            rm "$CARGO_BIN/agcodex"
            ln -s "$CARGO_BIN/agagcodex" "$CARGO_BIN/agcodex"
            echo -e "${GREEN}✓ Updated symlink: agcodex -> agagcodex${NC}"
        else
            # It's a real file
            echo -e "${YELLOW}Warning: $CARGO_BIN/agcodex exists and is not a symlink${NC}"
            echo "This might be the old AGCodex binary."
            echo ""
            read -p "Would you like to backup the old binary and create the symlink? (y/N) " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                mv "$CARGO_BIN/agcodex" "$CARGO_BIN/agcodex.old"
                ln -s "$CARGO_BIN/agagcodex" "$CARGO_BIN/agcodex"
                echo -e "${GREEN}✓ Backed up old binary to agcodex.old${NC}"
                echo -e "${GREEN}✓ Created symlink: agcodex -> agagcodex${NC}"
            else
                echo "Symlink not created. The old 'agcodex' command remains."
            fi
        fi
    else
        # Create new symlink
        ln -s "$CARGO_BIN/agagcodex" "$CARGO_BIN/agcodex"
        echo -e "${GREEN}✓ Created symlink: agcodex -> agagcodex${NC}"
    fi
    
    echo ""
    echo -e "${BLUE}You can now use both commands:${NC}"
    echo "  • agagcodex (recommended)"
    echo "  • agcodex (backward compatibility)"
else
    echo -e "${YELLOW}AGAGCodex binary not found at $CARGO_BIN/agagcodex${NC}"
    echo ""
    echo "Please build and install AGAGCodex first:"
    echo -e "${BLUE}  cargo install --path cli${NC}"
    echo ""
    echo "Then run this script again."
    exit 1
fi

echo ""