#!/usr/bin/env bash
#
# AGCodex Installation Script for Unix-based systems (Linux/macOS)
# This script downloads and installs AGCodex from GitHub releases
#

set -euo pipefail

# Configuration
REPO_OWNER="agcodex"
REPO_NAME="agcodex"
BINARY_NAME="agcodex"
CONFIG_DIR="${HOME}/.agcodex"
VERSION="${1:-latest}"

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

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

# Detect OS and architecture
detect_platform() {
    local os arch

    # Detect OS
    case "$(uname -s)" in
        Linux*)
            os="linux"
            ;;
        Darwin*)
            os="macos"
            ;;
        *)
            log_error "Unsupported operating system: $(uname -s)"
            exit 1
            ;;
    esac

    # Detect architecture
    case "$(uname -m)" in
        x86_64)
            arch="x86_64"
            ;;
        aarch64|arm64)
            arch="aarch64"
            ;;
        armv7l)
            arch="armv7"
            ;;
        *)
            log_error "Unsupported architecture: $(uname -m)"
            exit 1
            ;;
    esac

    echo "${os}-${arch}"
}

# Check for required dependencies
check_dependencies() {
    local deps=("curl" "tar")
    local missing=()

    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            missing+=("$dep")
        fi
    done

    if [ ${#missing[@]} -gt 0 ]; then
        log_error "Missing required dependencies: ${missing[*]}"
        log_info "Please install them and try again."
        exit 1
    fi
}

# Get the latest release version from GitHub
get_latest_version() {
    local api_url="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest"
    local version

    version=$(curl -s "$api_url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    
    if [ -z "$version" ]; then
        log_error "Failed to fetch latest version from GitHub"
        exit 1
    fi

    echo "$version"
}

# Download binary from GitHub releases
download_binary() {
    local version="$1"
    local platform="$2"
    local download_url temp_dir

    # Remove 'v' prefix if present
    version="${version#v}"

    download_url="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/v${version}/${BINARY_NAME}-${version}-${platform}.tar.gz"
    temp_dir=$(mktemp -d)

    log_info "Downloading AGCodex v${version} for ${platform}..."
    
    if ! curl -L -o "${temp_dir}/${BINARY_NAME}.tar.gz" "$download_url"; then
        log_error "Failed to download binary from: $download_url"
        rm -rf "$temp_dir"
        exit 1
    fi

    # Download checksum file
    local checksum_url="${download_url}.sha256"
    if curl -L -o "${temp_dir}/${BINARY_NAME}.tar.gz.sha256" "$checksum_url" 2>/dev/null; then
        log_info "Verifying checksum..."
        
        cd "$temp_dir"
        if command -v sha256sum &> /dev/null; then
            if ! sha256sum -c "${BINARY_NAME}.tar.gz.sha256" &> /dev/null; then
                log_error "Checksum verification failed!"
                rm -rf "$temp_dir"
                exit 1
            fi
        elif command -v shasum &> /dev/null; then
            if ! shasum -a 256 -c "${BINARY_NAME}.tar.gz.sha256" &> /dev/null; then
                log_error "Checksum verification failed!"
                rm -rf "$temp_dir"
                exit 1
            fi
        else
            log_warning "Cannot verify checksum (no sha256sum or shasum available)"
        fi
        cd - > /dev/null
        log_success "Checksum verified"
    else
        log_warning "Checksum file not found, skipping verification"
    fi

    # Extract binary
    log_info "Extracting binary..."
    tar -xzf "${temp_dir}/${BINARY_NAME}.tar.gz" -C "$temp_dir"
    
    if [ ! -f "${temp_dir}/${BINARY_NAME}" ]; then
        log_error "Binary not found in archive"
        rm -rf "$temp_dir"
        exit 1
    fi

    echo "$temp_dir"
}

# Determine installation directory
get_install_dir() {
    local install_dir

    # Check if we have write permission to /usr/local/bin
    if [ -w "/usr/local/bin" ] || [ -w "/usr/local" ]; then
        install_dir="/usr/local/bin"
    else
        install_dir="${HOME}/.local/bin"
        
        # Create ~/.local/bin if it doesn't exist
        if [ ! -d "$install_dir" ]; then
            log_info "Creating ${install_dir}..."
            mkdir -p "$install_dir"
        fi
        
        # Check if ~/.local/bin is in PATH
        if [[ ":$PATH:" != *":$install_dir:"* ]]; then
            log_warning "${install_dir} is not in your PATH"
            log_info "Add the following line to your shell configuration file (.bashrc, .zshrc, etc.):"
            echo "    export PATH=\"\$PATH:${install_dir}\""
        fi
    fi

    echo "$install_dir"
}

# Install the binary
install_binary() {
    local temp_dir="$1"
    local install_dir="$2"
    local binary_path="${temp_dir}/${BINARY_NAME}"
    local target_path="${install_dir}/${BINARY_NAME}"

    # Check if binary already exists
    if [ -f "$target_path" ]; then
        log_warning "AGCodex is already installed at ${target_path}"
        read -p "Do you want to replace it? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Installation cancelled"
            rm -rf "$temp_dir"
            exit 0
        fi
    fi

    # Install binary
    log_info "Installing to ${target_path}..."
    
    if [ -w "$install_dir" ]; then
        cp "$binary_path" "$target_path"
        chmod +x "$target_path"
    else
        log_info "Root permission required to install to ${install_dir}"
        sudo cp "$binary_path" "$target_path"
        sudo chmod +x "$target_path"
    fi

    rm -rf "$temp_dir"
    log_success "Binary installed successfully"
}

# Setup configuration directory
setup_config() {
    log_info "Setting up configuration directory..."

    # Create config directory
    mkdir -p "${CONFIG_DIR}"
    mkdir -p "${CONFIG_DIR}/agents"
    mkdir -p "${CONFIG_DIR}/history"
    mkdir -p "${CONFIG_DIR}/cache"

    # Create default config if it doesn't exist
    if [ ! -f "${CONFIG_DIR}/config.toml" ]; then
        log_info "Creating default configuration..."
        cat > "${CONFIG_DIR}/config.toml" << 'EOF'
# AGCodex Configuration File
# This file is automatically created during installation
# Edit this file to customize your AGCodex settings

[model]
provider = "openai"
name = "gpt-4"
temperature = 0.7
max_tokens = 4096

[tui]
theme = "dark"
history_limit = 100

[modes]
default = "build"

[search]
intelligence = "medium"  # light, medium, hard
chunk_size = 512

[cache]
enabled = true
max_size_mb = 500

[security]
sandbox_enabled = true
approval_mode = "auto"
EOF
        log_success "Default configuration created"
    else
        log_info "Configuration file already exists, skipping..."
    fi

    # Set proper permissions
    chmod 700 "${CONFIG_DIR}"
    chmod 600 "${CONFIG_DIR}/config.toml" 2>/dev/null || true
}

# Verify installation
verify_installation() {
    if command -v "${BINARY_NAME}" &> /dev/null; then
        local version
        version=$("${BINARY_NAME}" --version 2>/dev/null || echo "unknown")
        log_success "AGCodex ${version} installed successfully!"
        log_info "Configuration directory: ${CONFIG_DIR}"
        log_info "Run '${BINARY_NAME} --help' to get started"
    else
        log_error "Installation verification failed"
        log_info "Please ensure ${install_dir} is in your PATH"
        exit 1
    fi
}

# Main installation flow
main() {
    echo "╔═══════════════════════════════════════╗"
    echo "║     AGCodex Installation Script       ║"
    echo "╚═══════════════════════════════════════╝"
    echo

    # Check dependencies
    check_dependencies

    # Detect platform
    platform=$(detect_platform)
    log_info "Detected platform: ${platform}"

    # Get version to install
    if [ "$VERSION" = "latest" ]; then
        VERSION=$(get_latest_version)
        log_info "Latest version: ${VERSION}"
    else
        log_info "Installing specified version: ${VERSION}"
    fi

    # Download binary
    temp_dir=$(download_binary "$VERSION" "$platform")

    # Get installation directory
    install_dir=$(get_install_dir)

    # Install binary
    install_binary "$temp_dir" "$install_dir"

    # Setup configuration
    setup_config

    # Verify installation
    verify_installation

    echo
    echo "╔═══════════════════════════════════════╗"
    echo "║    Installation Complete! 🎉          ║"
    echo "╚═══════════════════════════════════════╝"
}

# Handle script arguments
show_help() {
    cat << EOF
AGCodex Installation Script

Usage: $0 [OPTIONS] [VERSION]

OPTIONS:
    -h, --help     Show this help message
    -v, --version  Install specific version (default: latest)

EXAMPLES:
    $0                    # Install latest version
    $0 v1.0.0            # Install specific version
    $0 --help            # Show help

ENVIRONMENT VARIABLES:
    AGCODEX_INSTALL_DIR  Override installation directory
    AGCODEX_CONFIG_DIR   Override configuration directory

EOF
}

# Parse arguments
case "${1:-}" in
    -h|--help)
        show_help
        exit 0
        ;;
    -v|--version)
        VERSION="${2:-latest}"
        ;;
esac

# Override directories with environment variables if set
if [ -n "${AGCODEX_CONFIG_DIR:-}" ]; then
    CONFIG_DIR="$AGCODEX_CONFIG_DIR"
fi

# Run main installation
main