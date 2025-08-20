#!/bin/bash

# AGCodex User Configuration Migration Script
# Migrates user configuration from ~/.codex to ~/.agcodex

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo -e "${BLUE}      AGCodex Configuration Migration          ${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo ""

if [ -d "$HOME/.codex" ]; then
    if [ ! -d "$HOME/.agcodex" ]; then
        echo -e "${YELLOW}Found existing Codex configuration at ~/.codex${NC}"
        echo -e "Migrating to ~/.agcodex..."
        
        # Copy configuration
        cp -r "$HOME/.codex" "$HOME/.agcodex"
        
        # Update any internal references
        if [ -f "$HOME/.agcodex/config.toml" ]; then
            sed -i 's/\.codex/\.agcodex/g' "$HOME/.agcodex/config.toml" 2>/dev/null || true
            sed -i 's/codex/agcodex/g' "$HOME/.agcodex/config.toml" 2>/dev/null || true
        fi
        
        echo -e "${GREEN}✓ Configuration migrated successfully!${NC}"
        echo ""
        echo "Your old configuration is preserved at ~/.codex"
        echo "You can remove it with: rm -rf ~/.codex"
    else
        echo -e "${YELLOW}~/.agcodex already exists${NC}"
        echo "No migration performed to avoid overwriting existing configuration."
    fi
else
    echo -e "${BLUE}No existing Codex configuration found at ~/.codex${NC}"
    
    if [ -d "$HOME/.agcodex" ]; then
        echo -e "${GREEN}✓ AGCodex configuration already exists at ~/.agcodex${NC}"
    else
        echo "AGCodex will create a fresh configuration on first run."
    fi
fi

echo ""