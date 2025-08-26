#!/usr/bin/env bash
#
# AGCodex Uninstallation Script for Unix-based systems (Linux/macOS)
# This script removes AGCodex from the system
#

set -euo pipefail

# Configuration
BINARY_NAME="agcodex"
CONFIG_DIR="${HOME}/.agcodex"
DATA_DIRS=(
    "${HOME}/.agcodex"
    "${HOME}/.local/share/agcodex"
    "${HOME}/.cache/agcodex"
)

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
PURGE=false
FORCE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --purge)
            PURGE=true
            shift
            ;;
        --force|-f)
            FORCE=true
            shift
            ;;
        --help|-h)
            cat << EOF
AGCodex Uninstallation Script

Usage: $0 [OPTIONS]

OPTIONS:
    --purge        Remove all user data and configuration files
    --force, -f    Skip confirmation prompts
    --help, -h     Show this help message

Without --purge, only the binary is removed. User data is preserved.

EXAMPLES:
    $0                  # Remove binary only, preserve user data
    $0 --purge          # Remove everything including user data
    $0 --force --purge  # Remove everything without confirmation

EOF
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Find binary locations
find_binary_locations() {
    local locations=()
    local search_paths=(
        "/usr/local/bin"
        "/usr/bin"
        "${HOME}/.local/bin"
        "${HOME}/bin"
        "/opt/agcodex/bin"
    )
    
    for path in "${search_paths[@]}"; do
        if [ -f "${path}/${BINARY_NAME}" ]; then
            locations+=("${path}/${BINARY_NAME}")
        fi
    done
    
    # Also check if it's in PATH
    if command -v "${BINARY_NAME}" &> /dev/null; then
        local cmd_path
        cmd_path=$(command -v "${BINARY_NAME}")
        if [[ ! " ${locations[@]} " =~ " ${cmd_path} " ]]; then
            locations+=("$cmd_path")
        fi
    fi
    
    printf '%s\n' "${locations[@]}"
}

# Remove binary
remove_binary() {
    local binary_path="$1"
    
    log_info "Removing binary: ${binary_path}"
    
    if [ -w "$(dirname "$binary_path")" ]; then
        rm -f "$binary_path"
    else
        log_info "Root permission required to remove ${binary_path}"
        sudo rm -f "$binary_path"
    fi
    
    log_success "Binary removed: ${binary_path}"
}

# Remove symbolic links
remove_symlinks() {
    local binary_locations=("$@")
    
    for location in "${binary_locations[@]}"; do
        if [ -L "$location" ]; then
            log_info "Removing symbolic link: $location"
            
            if [ -w "$(dirname "$location")" ]; then
                rm -f "$location"
            else
                sudo rm -f "$location"
            fi
            
            log_success "Symbolic link removed"
        fi
    done
}

# Remove configuration and data
remove_user_data() {
    log_warning "Removing user data and configuration..."
    
    for dir in "${DATA_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            log_info "Removing: $dir"
            rm -rf "$dir"
            log_success "Removed: $dir"
        fi
    done
    
    # Remove any remaining cache files
    local cache_locations=(
        "${HOME}/.cache/agcodex"
        "${XDG_CACHE_HOME:-$HOME/.cache}/agcodex"
    )
    
    for cache in "${cache_locations[@]}"; do
        if [ -d "$cache" ]; then
            log_info "Removing cache: $cache"
            rm -rf "$cache"
        fi
    done
}

# Check for running processes
check_running_processes() {
    if pgrep -x "$BINARY_NAME" > /dev/null; then
        log_warning "AGCodex is currently running"
        
        if [ "$FORCE" = true ]; then
            log_info "Stopping AGCodex processes..."
            pkill -x "$BINARY_NAME" || true
            sleep 2
        else
            read -p "Stop running AGCodex processes? (y/N): " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                pkill -x "$BINARY_NAME" || true
                sleep 2
            else
                log_error "Cannot uninstall while AGCodex is running"
                exit 1
            fi
        fi
    fi
}

# Remove from shell configuration
remove_from_shell_config() {
    local shell_configs=(
        "${HOME}/.bashrc"
        "${HOME}/.zshrc"
        "${HOME}/.profile"
        "${HOME}/.bash_profile"
    )
    
    for config in "${shell_configs[@]}"; do
        if [ -f "$config" ] && grep -q "agcodex" "$config"; then
            log_info "Found AGCodex references in $config"
            
            if [ "$FORCE" = true ]; then
                sed -i.bak '/agcodex/d' "$config"
                log_success "Removed AGCodex references from $config"
            else
                log_warning "You may want to manually remove AGCodex references from $config"
            fi
        fi
    done
}

# Calculate data size
calculate_data_size() {
    local total_size=0
    
    for dir in "${DATA_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            local size
            size=$(du -sh "$dir" 2>/dev/null | cut -f1)
            log_info "Data in $dir: $size"
        fi
    done
}

# Main uninstallation flow
main() {
    echo "╔═══════════════════════════════════════╗"
    echo "║    AGCodex Uninstallation Script      ║"
    echo "╚═══════════════════════════════════════╝"
    echo
    
    # Check for running processes
    check_running_processes
    
    # Find binary locations
    log_info "Searching for AGCodex installations..."
    mapfile -t binary_locations < <(find_binary_locations)
    
    if [ ${#binary_locations[@]} -eq 0 ]; then
        log_warning "AGCodex binary not found in standard locations"
        
        if [ "$PURGE" = true ]; then
            log_info "Proceeding with user data removal..."
        else
            log_error "Nothing to uninstall"
            exit 0
        fi
    else
        log_info "Found AGCodex in the following locations:"
        printf '%s\n' "${binary_locations[@]}"
        echo
    fi
    
    # Show data that will be affected
    if [ "$PURGE" = true ]; then
        log_warning "The following user data will be removed:"
        calculate_data_size
        echo
    else
        log_info "User data will be preserved in:"
        for dir in "${DATA_DIRS[@]}"; do
            if [ -d "$dir" ]; then
                echo "  - $dir"
            fi
        done
        echo
    fi
    
    # Confirmation
    if [ "$FORCE" = false ]; then
        if [ "$PURGE" = true ]; then
            echo -e "${YELLOW}WARNING: This will remove all AGCodex data and configuration!${NC}"
        fi
        
        read -p "Continue with uninstallation? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Uninstallation cancelled"
            exit 0
        fi
    fi
    
    # Remove binaries
    for location in "${binary_locations[@]}"; do
        remove_binary "$location"
    done
    
    # Remove symbolic links
    remove_symlinks "${binary_locations[@]}"
    
    # Remove from shell configuration
    remove_from_shell_config
    
    # Remove user data if requested
    if [ "$PURGE" = true ]; then
        remove_user_data
        log_success "All user data removed"
    else
        log_info "User data preserved. Use --purge to remove it."
    fi
    
    # Final check
    if command -v "${BINARY_NAME}" &> /dev/null; then
        log_warning "AGCodex is still accessible in PATH"
        log_info "You may need to restart your shell or run: hash -r"
    else
        log_success "AGCodex has been successfully uninstalled"
    fi
    
    echo
    echo "╔═══════════════════════════════════════╗"
    echo "║    Uninstallation Complete!           ║"
    echo "╚═══════════════════════════════════════╝"
    
    if [ "$PURGE" = false ] && [ -d "$CONFIG_DIR" ]; then
        echo
        log_info "To completely remove all data, run:"
        echo "    $0 --purge"
    fi
}

# Run main uninstallation
main