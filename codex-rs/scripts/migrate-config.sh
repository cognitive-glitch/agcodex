#!/bin/bash

# AGAGCodex User Configuration Migration Script
# Migrates user configuration from ~/.agcodex to ~/.agagcodex

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo -e "${BLUE}      AGAGCodex Configuration Migration          ${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo ""

if [ -d "$HOME/.agcodex" ]; then
    if [ ! -d "$HOME/.agagcodex" ]; then
        echo -e "${YELLOW}Found existing AGCodex configuration at ~/.agcodex${NC}"
        echo -e "Migrating to ~/.agagcodex..."
        
        # Copy configuration
        cp -r "$HOME/.agcodex" "$HOME/.agagcodex"
        
        # Update any internal references
        if [ -f "$HOME/.agagcodex/config.toml" ]; then
            sed -i 's/\.agcodex/\.agagcodex/g' "$HOME/.agagcodex/config.toml" 2>/dev/null || true
            sed -i 's/agcodex/agagcodex/g' "$HOME/.agagcodex/config.toml" 2>/dev/null || true
        fi
        
        echo -e "${GREEN}✓ Configuration migrated successfully!${NC}"
        echo ""
        echo "Your old configuration is preserved at ~/.agcodex"
        echo "You can remove it with: rm -rf ~/.agcodex"
    else
        echo -e "${YELLOW}~/.agagcodex already exists${NC}"
        echo "No migration performed to avoid overwriting existing configuration."
    fi
else
    echo -e "${BLUE}No existing AGCodex configuration found at ~/.agcodex${NC}"
    
    if [ -d "$HOME/.agagcodex" ]; then
        echo -e "${GREEN}✓ AGAGCodex configuration already exists at ~/.agagcodex${NC}"
    else
        echo "AGAGCodex will create a fresh configuration on first run."
    fi
fi

echo ""