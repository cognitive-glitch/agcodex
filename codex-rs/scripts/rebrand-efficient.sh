#!/bin/bash

# AGCodex Efficient Rebranding Script
# Performs targeted rebranding without timeouts

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo -e "${BLUE}     AGCodex Efficient Rebranding v2.0         ${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo ""

# Safety check
if [ ! -f "Cargo.toml" ] || [ ! -d "core" ]; then
    echo -e "${RED}Error: Must be run from the project root directory${NC}"
    exit 1
fi

echo -e "${BLUE}Step 1: Updating Cargo.toml files${NC}"
echo "─────────────────────────────────────"

# Update package names in Cargo.toml files (exclude backups and target)
fd -t f "Cargo.toml" -E "backup_*" -E "target" -E ".git" | while read -r file; do
    echo -e "  Processing: $file"
    
    # Replace package names
    sed -i.bak 's/name = "codex-/name = "agcodex-/g' "$file"
    sed -i 's/name = "codex"/name = "agcodex"/g' "$file"
    sed -i 's/name = "codex_/name = "agcodex_/g' "$file"
    
    # Replace dependencies
    sed -i 's/codex-\([a-z-]*\) = {/agcodex-\1 = {/g' "$file"
    
    # Clean up backup
    rm -f "${file}.bak"
done

echo ""
echo -e "${BLUE}Step 2: Adding missing crates to workspace${NC}"
echo "─────────────────────────────────────"

# Add chatgpt and persistence to workspace if not present
if ! grep -q '"chatgpt"' Cargo.toml; then
    sed -i '/members = \[/,/\]/ {
        /members = \[/a\    "chatgpt",
    }' Cargo.toml
    echo -e "  ${GREEN}✓${NC} Added chatgpt to workspace"
fi

if ! grep -q '"persistence"' Cargo.toml; then
    sed -i '/members = \[/,/\]/ {
        /members = \[/a\    "persistence",
    }' Cargo.toml
    echo -e "  ${GREEN}✓${NC} Added persistence to workspace"
fi

echo ""
echo -e "${BLUE}Step 3: Updating Rust imports${NC}"
echo "─────────────────────────────────────"

# Use comby for Rust imports (much faster)
echo -e "  Updating use statements..."
comby 'use codex_:[module]' 'use agcodex_:[module]' .rs -exclude-dir 'target,backup_*,.git' -i

echo -e "  Updating extern crate declarations..."
comby 'extern crate codex_:[name]' 'extern crate agcodex_:[name]' .rs -exclude-dir 'target,backup_*,.git' -i

echo -e "  Updating module paths..."
comby 'codex_:[name]::' 'agcodex_:[name]::' .rs -exclude-dir 'target,backup_*,.git' -i

echo ""
echo -e "${BLUE}Step 4: Updating configuration paths${NC}"
echo "─────────────────────────────────────"

# Update config paths
echo -e "  Updating home directory paths..."
fd -e rs -e toml -e md -E "backup_*" -E "target" | xargs -I {} sed -i 's|~/.codex|~/.agcodex|g' {}

echo -e "  Updating relative config paths..."
fd -e rs -e toml -e md -E "backup_*" -E "target" | xargs -I {} sed -i 's|\.codex/|.agcodex/|g' {}

echo ""
echo -e "${BLUE}Step 5: Updating string literals${NC}"
echo "─────────────────────────────────────"

# Update string literals in Rust files
echo -e "  Updating binary name references..."
comby '"codex"' '"agcodex"' .rs -exclude-dir 'target,backup_*,.git' -match-only '"codex"' -i

echo -e "  Updating crate name references in strings..."
comby '"codex-:[name]"' '"agcodex-:[name]"' .rs -exclude-dir 'target,backup_*,.git' -i

echo ""
echo -e "${BLUE}Step 6: Updating documentation${NC}"
echo "─────────────────────────────────────"

# Update documentation (preserve URLs and CHANGELOG)
fd -e md -E "CHANGELOG*" -E "backup_*" -E "target" | while read -r file; do
    echo -e "  Processing: $file"
    # Only update Codex references that aren't URLs
    sed -i -E '/https?:\/\//! s/\bCodex\b/AGCodex/g' "$file"
    sed -i -E '/https?:\/\//! s/\bcodex\b/agcodex/g' "$file"
done

echo ""
echo -e "${BLUE}Step 7: Special fixes${NC}"
echo "─────────────────────────────────────"

# Fix specific known issues
echo -e "  Fixing double-renamed imports..."
fd -e rs -E "backup_*" -E "target" | xargs -I {} sed -i 's/agagcodex/agcodex/g' {}

echo -e "  Fixing environment variables..."
fd -e rs -E "backup_*" -E "target" | xargs -I {} sed -i 's/CODEX_/AGCODEX_/g' {}

echo ""
echo -e "${BLUE}Step 8: Creating compatibility scripts${NC}"
echo "─────────────────────────────────────"

# Create migration script
cat > "scripts/migrate-config.sh" << 'EOF'
#!/bin/bash
# Migrate user configuration from ~/.codex to ~/.agcodex

if [ -d "$HOME/.codex" ] && [ ! -d "$HOME/.agcodex" ]; then
    echo "Migrating configuration from ~/.codex to ~/.agcodex..."
    cp -r "$HOME/.codex" "$HOME/.agcodex"
    echo "✓ Migration complete!"
    echo ""
    echo "Your old configuration is preserved at ~/.codex"
    echo "You can remove it with: rm -rf ~/.codex"
else
    if [ -d "$HOME/.agcodex" ]; then
        echo "~/.agcodex already exists, no migration needed."
    else
        echo "No existing configuration found."
    fi
fi
EOF
chmod +x "scripts/migrate-config.sh"
echo -e "  ${GREEN}✓${NC} Created migrate-config.sh"

# Create symlink script
cat > "scripts/create-symlink.sh" << 'EOF'
#!/bin/bash
# Create backward compatibility symlink

CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"

if [ -f "$CARGO_BIN/agcodex" ]; then
    if [ ! -e "$CARGO_BIN/codex" ]; then
        ln -sf "$CARGO_BIN/agcodex" "$CARGO_BIN/codex"
        echo "✓ Created symlink: codex -> agcodex"
    else
        echo "codex already exists in $CARGO_BIN"
    fi
else
    echo "agcodex not found in $CARGO_BIN"
    echo "Please build and install first: cargo install --path cli"
fi
EOF
chmod +x "scripts/create-symlink.sh"
echo -e "  ${GREEN}✓${NC} Created create-symlink.sh"

echo ""
echo -e "${BLUE}Step 9: Final validation${NC}"
echo "─────────────────────────────────────"

# Quick validation
echo -e "  Checking Cargo.toml files..."
if fd -t f "Cargo.toml" -E "backup_*" -E "target" -x grep -l 'name = "codex' {} \; 2>/dev/null | grep -q .; then
    echo -e "  ${YELLOW}⚠${NC} Some Cargo.toml files may still contain 'codex' references"
else
    echo -e "  ${GREEN}✓${NC} All Cargo.toml files updated"
fi

echo -e "  Running cargo check..."
if cargo check --all-features --all-targets --workspace 2>/dev/null; then
    echo -e "  ${GREEN}✓${NC} Code compiles successfully!"
else
    echo -e "  ${YELLOW}⚠${NC} Compilation issues detected - fixing..."
    
    # Auto-fix common issues
    echo -e "  Fixing any double-renamed references..."
    fd -e rs -E "backup_*" -E "target" | xargs -I {} sed -i 's/agagcodex/agcodex/g' {}
    
    # Try again
    if cargo check --all-features --all-targets --workspace 2>/dev/null; then
        echo -e "  ${GREEN}✓${NC} Fixed! Code now compiles."
    else
        echo -e "  ${RED}✗${NC} Manual intervention needed. Run: cargo check"
    fi
fi

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo -e "${BLUE}         Rebranding Complete!                  ${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════${NC}"
echo ""
echo -e "${GREEN}Next Steps:${NC}"
echo "  1. Review changes: ${BLUE}git diff${NC}"
echo "  2. Test the build: ${BLUE}cargo test --no-fail-fast${NC}"
echo "  3. Migrate config: ${BLUE}./scripts/migrate-config.sh${NC}"
echo "  4. Create symlink: ${BLUE}./scripts/create-symlink.sh${NC}"
echo "  5. Commit changes: ${BLUE}git add -A && git commit -m 'feat: complete rebranding to agcodex'${NC}"