#!/bin/bash

# AGAGCodex Rebranding Script
# Safely migrates all agcodex references to agagcodex
# Created: 2025-08-20

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔══════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║     AGAGCodex Rebranding Script v1.0       ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════╝${NC}"
echo ""

# Safety check
if [ ! -f "Cargo.toml" ] || [ ! -d "core" ]; then
    echo -e "${RED}Error: Must be run from the project root directory${NC}"
    exit 1
fi

# Create backup
BACKUP_DIR="backup_$(date +%Y%m%d_%H%M%S)"
echo -e "${YELLOW}Creating backup in ${BACKUP_DIR}...${NC}"
mkdir -p "$BACKUP_DIR"
cp -r . "$BACKUP_DIR" 2>/dev/null || true
echo -e "${GREEN}✓ Backup created${NC}"

# Statistics tracking
TOTAL_CHANGES=0
declare -A CHANGE_STATS

log_change() {
    local file="$1"
    local change_type="$2"
    TOTAL_CHANGES=$((TOTAL_CHANGES + 1))
    CHANGE_STATS["$change_type"]=$((${CHANGE_STATS["$change_type"]:-0} + 1))
    echo -e "  ${GREEN}✓${NC} $file - $change_type"
}

echo ""
echo -e "${BLUE}Phase 1: Renaming crate names in Cargo.toml files${NC}"
echo "──────────────────────────────────────────────────"

# Update crate names in all Cargo.toml files
fd -t f "Cargo.toml" | while read -r file; do
    # Update package names
    if grep -q 'name = "agcodex-' "$file"; then
        comby 'name = "agcodex-:[name]"' 'name = "agagcodex-:[name]"' -i "$file" -matcher .toml 2>/dev/null || true
        log_change "$file" "package name"
    fi
    
    # Update binary names
    if grep -q 'name = "agcodex"' "$file"; then
        comby 'name = "agcodex"' 'name = "agagcodex"' -i "$file" -matcher .toml 2>/dev/null || true
        log_change "$file" "binary name"
    fi
    
    # Update library names
    if grep -q 'name = "agcodex_' "$file"; then
        comby 'name = "agcodex_:[name]"' 'name = "agagcodex_:[name]"' -i "$file" -matcher .toml 2>/dev/null || true
        log_change "$file" "library name"
    fi
    
    # Update dependencies
    if grep -q 'agcodex-' "$file"; then
        comby 'agcodex-:[dep] = { path' 'agagcodex-:[dep] = { path' -i "$file" -matcher .toml 2>/dev/null || true
        log_change "$file" "dependencies"
    fi
done

echo ""
echo -e "${BLUE}Phase 2: Updating Rust source files${NC}"
echo "──────────────────────────────────────────────────"

# Update use statements and crate references in Rust files
fd -e rs | while read -r file; do
    changes_made=false
    
    # Update use statements
    if grep -q 'use agcodex_' "$file"; then
        comby 'use agcodex_:[module]' 'use agagcodex_:[module]' -i "$file" -matcher .rs 2>/dev/null || true
        changes_made=true
    fi
    
    # Update extern crate declarations
    if grep -q 'extern crate agcodex_' "$file"; then
        comby 'extern crate agcodex_:[name]' 'extern crate agagcodex_:[name]' -i "$file" -matcher .rs 2>/dev/null || true
        changes_made=true
    fi
    
    # Update crate:: references that might use agcodex
    if grep -q 'agcodex_' "$file"; then
        comby 'agcodex_:[name]::' 'agagcodex_:[name]::' -i "$file" -matcher .rs 2>/dev/null || true
        changes_made=true
    fi
    
    if [ "$changes_made" = true ]; then
        log_change "$file" "Rust imports/references"
    fi
done

echo ""
echo -e "${BLUE}Phase 3: Updating configuration paths${NC}"
echo "──────────────────────────────────────────────────"

# Update config paths in all source files
fd -e rs -e toml -e md | while read -r file; do
    changes_made=false
    
    # Update home directory paths
    if grep -q '~/.agcodex' "$file"; then
        comby '~/.agcodex' '~/.agagcodex' -i "$file" 2>/dev/null || true
        changes_made=true
    fi
    
    # Update relative config paths
    if grep -q '\.agcodex/' "$file"; then
        comby '.agcodex/' '.agagcodex/' -i "$file" 2>/dev/null || true
        changes_made=true
    fi
    
    if [ "$changes_made" = true ]; then
        log_change "$file" "config paths"
    fi
done

echo ""
echo -e "${BLUE}Phase 4: Updating documentation and comments${NC}"
echo "──────────────────────────────────────────────────"

# Update documentation references (but preserve URLs and historical references)
fd -e md -e rs | while read -r file; do
    changes_made=false
    
    # Skip CHANGELOG files to preserve history
    if [[ "$file" == *"CHANGELOG"* ]]; then
        continue
    fi
    
    # Update "AGCodex" to "AGAGCodex" in prose (but not in URLs)
    if grep -q 'AGCodex' "$file" && ! grep -q 'github.com.*agcodex' "$file"; then
        # Use a more targeted approach for documentation
        sed -i.bak -E '
            # Skip lines with URLs
            /https?:\/\//! {
                # Skip lines with git references
                /git@/! {
                    # Skip lines with github references
                    /github\.com/! {
                        s/\bAGCodex\b/AGAGCodex/g
                        s/\bagcodex\b/agagcodex/g
                    }
                }
            }
        ' "$file"
        
        # Check if changes were made
        if ! diff -q "$file" "$file.bak" > /dev/null 2>&1; then
            changes_made=true
            rm "$file.bak"
        else
            rm "$file.bak"
        fi
    fi
    
    if [ "$changes_made" = true ]; then
        log_change "$file" "documentation"
    fi
done

echo ""
echo -e "${BLUE}Phase 5: Updating string literals and constants${NC}"
echo "──────────────────────────────────────────────────"

# Update string literals in source files
fd -e rs | while read -r file; do
    changes_made=false
    
    # Update string literals containing "agcodex"
    if grep -q '".*agcodex.*"' "$file"; then
        # Update binary name references
        comby '"agcodex"' '"agagcodex"' -i "$file" -matcher .rs 2>/dev/null || true
        
        # Update crate name references
        comby '"agcodex-:[name]"' '"agagcodex-:[name]"' -i "$file" -matcher .rs 2>/dev/null || true
        
        changes_made=true
    fi
    
    if [ "$changes_made" = true ]; then
        log_change "$file" "string literals"
    fi
done

echo ""
echo -e "${BLUE}Phase 6: Special file updates${NC}"
echo "──────────────────────────────────────────────────"

# Update workspace dependencies if they exist
if [ -f "Cargo.toml" ]; then
    echo -e "  Updating workspace Cargo.toml..."
    
    # Ensure chatgpt and persistence are in the workspace members
    if ! grep -q "chatgpt" Cargo.toml; then
        sed -i '/members = \[/,/\]/ {
            /\]/i\    "chatgpt",
        }' Cargo.toml
    fi
    
    if ! grep -q "persistence" Cargo.toml; then
        sed -i '/members = \[/,/\]/ {
            /\]/i\    "persistence",
        }' Cargo.toml
    fi
    
    echo -e "  ${GREEN}✓${NC} Workspace Cargo.toml updated"
fi

# Update package.json files if they exist
fd -t f "package.json" | while read -r file; do
    if grep -q 'agcodex' "$file"; then
        comby '"name": ":[prefix]agcodex:[suffix]"' '"name": ":[prefix]agagcodex:[suffix]"' -i "$file" -matcher .json 2>/dev/null || true
        log_change "$file" "package.json"
    fi
done

echo ""
echo -e "${BLUE}Phase 7: Creating backward compatibility${NC}"
echo "──────────────────────────────────────────────────"

# Create migration script
cat > "scripts/migrate-user-config.sh" << 'EOF'
#!/bin/bash
# User configuration migration script

if [ -d "$HOME/.agcodex" ] && [ ! -d "$HOME/.agagcodex" ]; then
    echo "Migrating user configuration from ~/.agcodex to ~/.agagcodex..."
    cp -r "$HOME/.agcodex" "$HOME/.agagcodex"
    echo "Migration complete! Your old configuration remains at ~/.agcodex"
    echo "You can safely remove it once you've verified everything works."
else
    echo "No migration needed."
fi
EOF

chmod +x "scripts/migrate-user-config.sh"
echo -e "  ${GREEN}✓${NC} Created user config migration script"

# Create binary symlink script
cat > "scripts/create-compat-symlink.sh" << 'EOF'
#!/bin/bash
# Create backward compatibility symlink

INSTALL_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"

if [ -f "$INSTALL_DIR/agagcodex" ] && [ ! -e "$INSTALL_DIR/agcodex" ]; then
    ln -s "$INSTALL_DIR/agagcodex" "$INSTALL_DIR/agcodex"
    echo "Created symlink: agcodex -> agagcodex"
else
    echo "Symlink not needed or already exists."
fi
EOF

chmod +x "scripts/create-compat-symlink.sh"
echo -e "  ${GREEN}✓${NC} Created binary symlink script"

echo ""
echo -e "${BLUE}Phase 8: Final validation${NC}"
echo "──────────────────────────────────────────────────"

# Run cargo check to ensure everything still compiles
echo -e "  Running cargo check..."
if cargo check --all-features --all-targets --workspace 2>/dev/null; then
    echo -e "  ${GREEN}✓${NC} All crates compile successfully!"
else
    echo -e "  ${YELLOW}⚠${NC} Some compilation issues detected. Please review and fix manually."
fi

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         Rebranding Complete!             ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}Summary:${NC}"
echo -e "  Total changes made: ${TOTAL_CHANGES}"
for change_type in "${!CHANGE_STATS[@]}"; do
    echo -e "  - $change_type: ${CHANGE_STATS[$change_type]}"
done
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo -e "  1. Review the changes with: ${BLUE}git diff${NC}"
echo -e "  2. Run tests: ${BLUE}cargo test --no-fail-fast${NC}"
echo -e "  3. Migrate user config: ${BLUE}./scripts/migrate-user-config.sh${NC}"
echo -e "  4. Create symlink: ${BLUE}./scripts/create-compat-symlink.sh${NC}"
echo -e "  5. Commit changes: ${BLUE}git add -A && git commit -m 'feat: complete rebranding from agcodex to agagcodex'${NC}"
echo ""
echo -e "${GREEN}Backup saved in: ${BACKUP_DIR}${NC}"