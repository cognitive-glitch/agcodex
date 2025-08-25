# AGCodex Deployment Guide

> **Version**: 1.0.0  
> **Last Updated**: 2025-08-21  
> **Status**: Production Ready

## Table of Contents

1. [System Requirements](#system-requirements)
2. [Installation Guide](#installation-guide)
3. [Configuration](#configuration)
4. [First Run Setup](#first-run-setup)
5. [Production Deployment](#production-deployment)
6. [Performance Tuning](#performance-tuning)
7. [Troubleshooting](#troubleshooting)
8. [Security Considerations](#security-considerations)
9. [Monitoring & Observability](#monitoring--observability)
10. [Backup & Recovery](#backup--recovery)
11. [Migration Guide](#migration-guide)
12. [API Reference](#api-reference)

---

## System Requirements

### Minimum Requirements

| Component | Requirement | Notes |
|-----------|------------|-------|
| **CPU** | 4 cores @ 2.0GHz | 8+ cores recommended for multi-agent |
| **RAM** | 8GB | 16GB+ for Hard intelligence mode |
| **Storage** | 10GB available | 50GB+ for large codebases |
| **OS** | Linux (kernel 5.13+), macOS 12+, Windows 10+ | Linux recommended for production |

### Software Dependencies

```bash
# Core Requirements
Rust:         1.75.0+    # Required for building
Git:          2.30.0+    # Worktree support needed
CMake:        3.20+      # Tree-sitter compilation
Python:       3.8+       # Build scripts
Node.js:      18.0+      # MCP inspector (optional)

# Platform-Specific
Linux:        gcc/clang 11+, pkg-config, libssl-dev
macOS:        Xcode Command Line Tools
Windows:      MSVC 2019+, Windows SDK
```

### Tree-sitter Language Dependencies

```bash
# Verify tree-sitter installation
tree-sitter --version  # Should be 0.22.0+

# Required parsers (auto-installed during build)
- tree-sitter-rust      - tree-sitter-python
- tree-sitter-javascript - tree-sitter-typescript
- tree-sitter-go        - tree-sitter-java
- tree-sitter-c         - tree-sitter-cpp
# ... and 40+ more languages
```

### Network Requirements

- **Outbound HTTPS**: API providers (OpenAI, Gemini, Voyage)
- **Localhost**: MCP server (default: 5173)
- **Optional**: Git SSH/HTTPS for repository access

---

## Installation Guide

### 1. Quick Install (Recommended)

```bash
# One-line installer for Unix systems
curl -fsSL https://agcodex.ai/install.sh | sh

# Or using Cargo (Rust package manager)
cargo install agcodex --locked

# Verify installation
agcodex --version
```

### 2. Build from Source

```bash
# Clone repository
git clone https://github.com/agcodex/agcodex.git
cd agcodex/agcodex-rs

# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Build with all features
cargo build --release --all-features

# Run tests to verify build
cargo test --no-fail-fast

# Install globally
cargo install --path cli --locked

# Or copy binary directly
sudo cp target/release/agcodex /usr/local/bin/
sudo chmod +x /usr/local/bin/agcodex
```

### 3. Docker Installation

```bash
# Pull official image
docker pull agcodex/agcodex:latest

# Run with volume mounts
docker run -it \
  -v ~/.agcodex:/root/.agcodex \
  -v $(pwd):/workspace \
  -e OPENAI_API_KEY=$OPENAI_API_KEY \
  agcodex/agcodex:latest

# Or use docker-compose
cat > docker-compose.yml << 'EOF'
version: '3.8'
services:
  agcodex:
    image: agcodex/agcodex:latest
    volumes:
      - ~/.agcodex:/root/.agcodex
      - ./:/workspace
    environment:
      - OPENAI_API_KEY
      - GEMINI_API_KEY
      - VOYAGE_API_KEY
    stdin_open: true
    tty: true
EOF

docker-compose up
```

### 4. Platform-Specific Installation

#### Linux (Debian/Ubuntu)

```bash
# Install dependencies
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev cmake git

# Install from .deb package
wget https://github.com/agcodex/agcodex/releases/latest/download/agcodex_linux_amd64.deb
sudo dpkg -i agcodex_linux_amd64.deb
```

#### Linux (RHEL/Fedora)

```bash
# Install dependencies
sudo dnf install -y gcc gcc-c++ openssl-devel cmake git

# Install from RPM
wget https://github.com/agcodex/agcodex/releases/latest/download/agcodex.rpm
sudo rpm -i agcodex.rpm
```

#### macOS

```bash
# Using Homebrew
brew tap agcodex/tap
brew install agcodex

# Or using MacPorts
sudo port install agcodex
```

#### Windows

```powershell
# Using Scoop
scoop bucket add agcodex https://github.com/agcodex/scoop-bucket
scoop install agcodex

# Or using Chocolatey
choco install agcodex

# Or download MSI installer
# https://github.com/agcodex/agcodex/releases/latest/download/agcodex_windows_x64.msi
```

---

## Configuration

### 1. Initial Configuration Setup

```bash
# Create configuration directory
mkdir -p ~/.agcodex/{agents,history,cache,logs}

# Generate default configuration
agcodex config init

# Or manually create config
cat > ~/.agcodex/config.toml << 'EOF'
# AGCodex Configuration
# Location: ~/.agcodex/config.toml

[general]
default_mode = "build"           # plan, build, or review
reasoning_effort = "high"        # ALWAYS high for GPT-5
verbosity = "high"               # ALWAYS high for detailed responses
auto_save = true                 # Auto-save sessions
save_interval_minutes = 5        # Checkpoint frequency
theme = "dark"                   # dark, light, or auto

[intelligence]
default_level = "medium"         # light, medium, or hard
ast_cache_size_mb = 500         # AST cache size
index_on_startup = true         # Index workspace on launch
compression_level = "standard"   # basic (70%), standard (85%), maximum (95%)

[models]
default_provider = "openai"      # openai, anthropic, gemini, ollama
default_model = "gpt-4"         # Model to use

[[models.providers]]
name = "openai"
type = "openai"
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.openai.com/v1"
models = [
    { name = "gpt-4", max_tokens = 128000 },
    { name = "gpt-4-turbo", max_tokens = 128000 },
    { name = "o3", max_tokens = 200000 },
]

[[models.providers]]
name = "anthropic"
type = "anthropic"
api_key_env = "ANTHROPIC_API_KEY"
base_url = "https://api.anthropic.com"
models = [
    { name = "claude-3-opus", max_tokens = 200000 },
    { name = "claude-3-sonnet", max_tokens = 200000 },
]

[[models.providers]]
name = "gemini"
type = "gemini"
api_key_env = "GEMINI_API_KEY"
models = [
    { name = "gemini-1.5-pro", max_tokens = 1000000 },
    { name = "gemini-1.5-flash", max_tokens = 1000000 },
]

[[models.providers]]
name = "ollama"
type = "ollama"
base_url = "http://localhost:11434"
models = [
    { name = "llama3", max_tokens = 8192 },
    { name = "codellama", max_tokens = 16384 },
]

[embeddings]
enabled = false                  # Disabled by default (zero overhead)
provider = "auto"                # auto, openai, gemini, voyage
cache_embeddings = true          # Cache computed embeddings
batch_size = 100                 # Embedding batch size

[embeddings.openai]
model = "text-embedding-3-small"
dimensions = 1536
api_key_env = "OPENAI_EMBEDDING_KEY"

[embeddings.gemini]
model = "gemini-embedding-001"
dimensions = 768
api_key_env = "GEMINI_API_KEY"

[embeddings.voyage]
model = "voyage-3.5"
input_type = "document"
api_key_env = "VOYAGE_API_KEY"

[security]
sandbox_enabled = true           # Enable sandboxing
require_approval = true          # Require approval for destructive ops
allowed_commands = [             # Whitelist for Plan/Review modes
    "git", "npm", "cargo", "python", "make"
]
max_file_size_mb = 10           # Max file size for operations
audit_logging = true            # Log all operations

[mcp_servers]
# Model Context Protocol servers
[[mcp_servers.servers]]
name = "filesystem"
command = "npx"
args = ["@modelcontextprotocol/server-filesystem", "/workspace"]

[[mcp_servers.servers]]
name = "github"
command = "npx"
args = ["@modelcontextprotocol/server-github"]
env = { GITHUB_TOKEN = "${GITHUB_TOKEN}" }

[tui]
mouse_enabled = true            # Enable mouse support
show_line_numbers = true        # Show line numbers in code
highlight_syntax = true         # Syntax highlighting
auto_wrap = true               # Auto-wrap long lines
notification_sound = true       # Terminal bell on completion

[keybindings]
# Custom keybindings (defaults shown)
mode_switch = "Shift+Tab"      # Cycle modes
new_conversation = "Ctrl+N"
save_session = "Ctrl+S"
load_session = "Ctrl+O"
agent_panel = "Ctrl+A"
history_browser = "Ctrl+H"
jump_to_message = "Ctrl+J"
undo = "Ctrl+Z"
redo = "Ctrl+Y"
branch = "Ctrl+B"
help = "Ctrl+?"

[logging]
level = "info"                  # debug, info, warn, error
file = "~/.agcodex/logs/agcodex.log"
max_size_mb = 100
max_backups = 10
format = "json"                 # json or text
EOF'
```

### 2. Agent Configuration

```bash
# Create global agent
cat > ~/.agcodex/agents/global/code-reviewer.yaml << 'EOF'
name: code-reviewer
description: Comprehensive code quality analysis
mode_override: review
intelligence: hard
tools:
  - Read
  - AST-Search
  - Tree-sitter-analyze
  - Security-scan
prompt: |
  You are an expert code reviewer with deep AST analysis capabilities.
  Focus on:
  - Security vulnerabilities (OWASP Top 10)
  - Performance bottlenecks
  - Code quality and maintainability
  - Best practices and design patterns
EOF

# Create project-specific agent
cat > .agcodex/agents/test-writer.yaml << 'EOF'
name: test-writer
description: Automated test generation
mode_override: build
intelligence: medium
tools:
  - Read
  - Write
  - AST-analyze
  - Test-runner
prompt: |
  Generate comprehensive test suites with:
  - Unit tests for all public functions
  - Integration tests for workflows
  - Edge cases and error conditions
  - Performance benchmarks
EOF
```

### 3. Environment Variables

```bash
# Add to ~/.bashrc or ~/.zshrc
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GEMINI_API_KEY="..."
export VOYAGE_API_KEY="..."
export GITHUB_TOKEN="ghp_..."

# Optional settings
export AGAGCODEX_CONFIG_PATH="~/.agcodex/config.toml"
export AGAGCODEX_LOG_LEVEL="info"
export AGAGCODEX_CACHE_DIR="~/.agcodex/cache"
export RUST_LOG="agcodex=debug"
```

---

## First Run Setup

### 1. Initial Setup Wizard

```bash
# Run setup wizard
agcodex setup

# This will:
# 1. Check system requirements
# 2. Verify API keys
# 3. Test model connections
# 4. Index current directory
# 5. Create initial session
```

### 2. Manual Verification

```bash
# Test core functionality
agcodex doctor

# Output should show:
# ✓ Rust version: 1.75.0
# ✓ Git version: 2.42.0
# ✓ Tree-sitter: 0.22.0
# ✓ Config file: Found at ~/.agcodex/config.toml
# ✓ API Keys: OpenAI ✓, Gemini ✓, Voyage ✗
# ✓ AST Parsers: 52 languages available
# ✓ Sandbox: Landlock available
# ✓ Cache directory: 245MB used
```

### 3. Test Agent Invocations

```bash
# Launch TUI
agcodex

# In TUI, test agents:
# Type: @agent-code-reviewer analyze src/
# Type: @agent-test-writer generate tests for main.rs
# Type: @agent-security scan for vulnerabilities

# Test mode switching
# Press Shift+Tab to cycle through Plan → Build → Review
```

### 4. Verify AST Tools

```bash
# Test AST indexing
agcodex index .

# Test AST search
agcodex search "function that handles authentication"

# Test tree-sitter parsing
agcodex parse src/main.rs --show-ast
```

---

## Production Deployment

### 1. Systemd Service (Linux)

```bash
# Create service file
sudo cat > /etc/systemd/system/agcodex.service << 'EOF'
[Unit]
Description=AGCodex AI Coding Assistant
After=network.target

[Service]
Type=simple
User=agcodex
Group=agcodex
WorkingDirectory=/opt/agcodex
ExecStart=/usr/local/bin/agcodex server --port 8080
Restart=always
RestartSec=10
StandardOutput=append:/var/log/agcodex/stdout.log
StandardError=append:/var/log/agcodex/stderr.log

# Security
PrivateTmp=true
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/agcodex/data /var/log/agcodex

# Resource Limits
LimitNOFILE=65536
LimitNPROC=4096
MemoryLimit=8G
CPUQuota=200%

[Install]
WantedBy=multi-user.target
EOF

# Create user and directories
sudo useradd -r -s /bin/false agcodex
sudo mkdir -p /opt/agcodex/data /var/log/agcodex
sudo chown -R agcodex:agcodex /opt/agcodex /var/log/agcodex

# Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable agcodex
sudo systemctl start agcodex

# Check status
sudo systemctl status agcodex
sudo journalctl -u agcodex -f
```

### 2. Docker Production Setup

```dockerfile
# Dockerfile.production
FROM rust:1.75-slim as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*

# Copy source
WORKDIR /build
COPY . .

# Build release binary
RUN cargo build --release --all-features

# Production image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    git \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -s /bin/false -m -d /home/agcodex agcodex

# Copy binary
COPY --from=builder /build/target/release/agcodex /usr/local/bin/

# Setup directories
RUN mkdir -p /home/agcodex/.agcodex/{agents,history,cache,logs} \
    && chown -R agcodex:agcodex /home/agcodex

# Switch to non-root user
USER agcodex
WORKDIR /workspace

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD agcodex health || exit 1

# Default command
ENTRYPOINT ["agcodex"]
CMD ["--mode", "build"]
```

```yaml
# docker-compose.production.yml
version: '3.8'

services:
  agcodex:
    build:
      context: .
      dockerfile: Dockerfile.production
    image: agcodex:production
    container_name: agcodex-prod
    restart: unless-stopped
    volumes:
      - agcodex-data:/home/agcodex/.agcodex
      - ./workspace:/workspace:rw
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
      - GEMINI_API_KEY=${GEMINI_API_KEY}
      - VOYAGE_API_KEY=${VOYAGE_API_KEY}
      - RUST_LOG=info
    networks:
      - agcodex-net
    deploy:
      resources:
        limits:
          cpus: '4'
          memory: 8G
        reservations:
          cpus: '2'
          memory: 4G
    logging:
      driver: "json-file"
      options:
        max-size: "100m"
        max-file: "10"

  redis:
    image: redis:7-alpine
    container_name: agcodex-redis
    restart: unless-stopped
    volumes:
      - redis-data:/data
    networks:
      - agcodex-net
    command: redis-server --appendonly yes

volumes:
  agcodex-data:
  redis-data:

networks:
  agcodex-net:
    driver: bridge
```

### 3. Kubernetes Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: agcodex
  namespace: agcodex
spec:
  replicas: 3
  selector:
    matchLabels:
      app: agcodex
  template:
    metadata:
      labels:
        app: agcodex
    spec:
      containers:
      - name: agcodex
        image: agcodex/agcodex:latest
        ports:
        - containerPort: 8080
        env:
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: agcodex-secrets
              key: openai-api-key
        - name: GEMINI_API_KEY
          valueFrom:
            secretKeyRef:
              name: agcodex-secrets
              key: gemini-api-key
        resources:
          requests:
            memory: "4Gi"
            cpu: "2"
          limits:
            memory: "8Gi"
            cpu: "4"
        volumeMounts:
        - name: agcodex-data
          mountPath: /home/agcodex/.agcodex
        - name: workspace
          mountPath: /workspace
        livenessProbe:
          exec:
            command:
            - agcodex
            - health
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          exec:
            command:
            - agcodex
            - ready
          initialDelaySeconds: 5
          periodSeconds: 5
      volumes:
      - name: agcodex-data
        persistentVolumeClaim:
          claimName: agcodex-pvc
      - name: workspace
        emptyDir: {}
---
apiVersion: v1
kind: Service
metadata:
  name: agcodex-service
  namespace: agcodex
spec:
  selector:
    app: agcodex
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8080
  type: LoadBalancer
```

### 4. CI/CD Integration

```yaml
# .github/workflows/deploy.yml
name: Deploy AGCodex

on:
  push:
    branches: [main]
    tags: ['v*']

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    - run: cargo test --no-fail-fast
    - run: cargo clippy -- -D warnings

  build:
    needs: test
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Build Docker image
      run: |
        docker build -t agcodex:${{ github.sha }} .
        docker tag agcodex:${{ github.sha }} agcodex:latest
    - name: Push to registry
      run: |
        echo ${{ secrets.DOCKER_PASSWORD }} | docker login -u ${{ secrets.DOCKER_USERNAME }} --password-stdin
        docker push agcodex:${{ github.sha }}
        docker push agcodex:latest

  deploy:
    needs: build
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/')
    steps:
    - name: Deploy to Kubernetes
      run: |
        kubectl set image deployment/agcodex agcodex=agcodex:${{ github.sha }} -n agcodex
        kubectl rollout status deployment/agcodex -n agcodex
```

---

## Performance Tuning

### 1. Memory Configuration

```toml
# ~/.agcodex/config.toml

[performance]
# Memory limits
max_memory_gb = 8               # Total memory limit
ast_cache_mb = 2000             # AST cache size
embedding_cache_mb = 1000       # Embedding cache
session_cache_mb = 500          # Session history cache

# Garbage collection
gc_interval_seconds = 300       # GC frequency
gc_threshold_mb = 1000          # Trigger GC above this

# Buffer sizes
read_buffer_kb = 64            # File read buffer
write_buffer_kb = 64           # File write buffer
network_buffer_kb = 128        # Network I/O buffer
```

### 2. Concurrency Settings

```toml
[concurrency]
# Thread pools
worker_threads = 8              # CPU cores * 2
io_threads = 4                  # I/O thread pool
ast_parser_threads = 4          # AST parsing threads

# Parallelism
max_parallel_agents = 3         # Concurrent agents
max_parallel_searches = 10      # Parallel searches
max_parallel_embeddings = 50    # Embedding batch size

# Rate limiting
api_rate_limit = 100            # Requests per minute
file_operation_limit = 1000     # File ops per minute
```

### 3. Cache Optimization

```bash
# Pre-warm caches on startup
agcodex cache warm --ast --embeddings

# Clear old cache entries
agcodex cache clean --older-than 7d

# Optimize cache database
agcodex cache optimize

# Monitor cache hit rates
agcodex cache stats
```

### 4. AST Index Optimization

```toml
[ast_indexing]
# Indexing strategy
incremental = true              # Incremental updates
watch_files = true              # File system watcher
debounce_ms = 500              # Debounce file changes

# Index optimization
max_file_size_mb = 10          # Skip huge files
exclude_patterns = [
    "*/node_modules/*",
    "*/target/*",
    "*.min.js",
    "*/dist/*"
]

# Compression
compression = "zstd"            # zstd, lz4, snappy
compression_level = 3           # 1-22 for zstd
```

### 5. Network Optimization

```toml
[network]
# Connection pooling
connection_pool_size = 10       # HTTP connection pool
keepalive_seconds = 60          # Keep connections alive
timeout_seconds = 30            # Request timeout

# Retry strategy
max_retries = 3
retry_delay_ms = 1000
exponential_backoff = true

# Caching
cache_responses = true          # Cache API responses
cache_ttl_seconds = 3600       # Cache TTL
```

---

## Troubleshooting

### Common Issues and Solutions

#### 1. Installation Issues

```bash
# Error: "cargo: command not found"
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Error: "failed to compile tree-sitter"
# Install build dependencies
sudo apt-get install build-essential cmake  # Debian/Ubuntu
sudo dnf install gcc gcc-c++ cmake          # Fedora
brew install cmake                          # macOS

# Error: "OPENAI_API_KEY not set"
export OPENAI_API_KEY="sk-..."
echo 'export OPENAI_API_KEY="sk-..."' >> ~/.bashrc
```

#### 2. Runtime Issues

```bash
# High memory usage
# Reduce cache sizes in config.toml
agcodex config set performance.max_memory_gb 4
agcodex cache clean

# Slow performance
# Check and optimize indexes
agcodex perf analyze
agcodex index optimize

# API errors
# Verify API keys and rate limits
agcodex test api --provider openai
agcodex config set network.max_retries 5
```

#### 3. TUI Issues

```bash
# Terminal rendering issues
export TERM=xterm-256color
agcodex --no-color  # Disable colors

# Mouse not working
agcodex config set tui.mouse_enabled false

# Keybindings not working
# Check terminal emulator settings
# Reset to defaults
agcodex config reset keybindings
```

### Debug Commands

```bash
# Enable debug logging
export RUST_LOG=agcodex=debug
agcodex --log-level debug

# Trace specific module
export RUST_LOG=agcodex::ast=trace

# Performance profiling
agcodex perf record --duration 60
agcodex perf report

# Memory profiling
agcodex mem stats
agcodex mem dump

# AST debugging
agcodex ast parse file.rs --debug
agcodex ast stats

# Network debugging
agcodex net trace --provider openai
agcodex net stats
```

### Log Locations

```bash
# Default log locations
~/.agcodex/logs/agcodex.log      # Main application log
~/.agcodex/logs/error.log        # Error log
~/.agcodex/logs/audit.log        # Security audit log
~/.agcodex/logs/performance.log  # Performance metrics

# View logs
tail -f ~/.agcodex/logs/agcodex.log
agcodex logs show --lines 100
agcodex logs search "error"
```

### Recovery Procedures

```bash
# Corrupt configuration
mv ~/.agcodex/config.toml ~/.agcodex/config.toml.backup
agcodex config init

# Corrupt session
agcodex session repair
agcodex session recover --from-backup

# Corrupt AST index
agcodex index rebuild --force

# Corrupt cache
agcodex cache clear --all
agcodex cache rebuild

# Full reset (preserves history)
agcodex reset --keep-history

# Complete reset (WARNING: deletes everything)
rm -rf ~/.agcodex
agcodex setup
```

---

## Security Considerations

### 1. API Key Management

```bash
# Use system keyring (recommended)
agcodex auth add --provider openai --use-keyring

# Encrypted file storage
agcodex auth encrypt --output ~/.agcodex/keys.enc

# Environment variable with restricted permissions
chmod 600 ~/.env
echo 'OPENAI_API_KEY=sk-...' >> ~/.env

# HashiCorp Vault integration
export VAULT_ADDR="https://vault.company.com"
agcodex auth add --provider openai --vault-path secret/agcodex/openai
```

### 2. Sandboxing Configuration

```toml
# ~/.agcodex/config.toml

[security.sandbox]
enabled = true                   # Enable sandboxing
profile = "strict"              # strict, moderate, permissive

[security.sandbox.linux]
backend = "landlock"            # landlock, seccomp, none
allowed_paths = [
    "/workspace",
    "~/.agcodex"
]
denied_syscalls = [
    "mount", "unmount", "chroot"
]

[security.sandbox.macos]
backend = "seatbelt"            # seatbelt, none
profile_path = "/etc/agcodex/sandbox.sb"

[security.sandbox.windows]
backend = "restricted_token"    # restricted_token, none
integrity_level = "medium"
```

### 3. Mode Restrictions

```toml
[security.modes.plan]
# Read-only mode restrictions
allow_read = true
allow_write = false
allow_execute = false
max_file_size_mb = 10
allowed_commands = []

[security.modes.build]
# Full access mode
allow_read = true
allow_write = true
allow_execute = true
require_approval = ["rm", "mv", "git push"]
audit_all_operations = true

[security.modes.review]
# Quality review mode
allow_read = true
allow_write = true  # Limited to <10KB
allow_execute = true
allowed_commands = ["git", "npm test", "cargo test", "pytest"]
max_edit_size_kb = 10
```

### 4. Audit Logging

```toml
[security.audit]
enabled = true
log_file = "~/.agcodex/logs/audit.log"
log_level = "info"              # debug, info, warn, error

# What to audit
log_api_calls = true
log_file_operations = true
log_command_execution = true
log_agent_invocations = true
log_mode_switches = true

# Log retention
max_size_mb = 1000
max_age_days = 90
compress_old_logs = true

# SIEM integration
syslog_enabled = false
syslog_server = "syslog.company.com:514"
syslog_format = "rfc5424"       # rfc3164, rfc5424
```

### 5. Network Security

```bash
# TLS configuration
agcodex config set network.tls.min_version "1.3"
agcodex config set network.tls.verify_certificates true

# Proxy configuration
export HTTPS_PROXY="http://proxy.company.com:8080"
export NO_PROXY="localhost,127.0.0.1,.company.com"

# IP allowlisting
agcodex config set network.allowed_ips ["api.openai.com", "api.anthropic.com"]

# Certificate pinning
agcodex cert pin --provider openai --cert /path/to/cert.pem
```

---

## Monitoring & Observability

### 1. Metrics Collection

```toml
[monitoring]
enabled = true
backend = "prometheus"          # prometheus, statsd, datadog

[monitoring.prometheus]
endpoint = "0.0.0.0:9090"
namespace = "agcodex"
update_interval_seconds = 10

[monitoring.metrics]
# What to track
track_api_latency = true
track_cache_hits = true
track_memory_usage = true
track_ast_operations = true
track_agent_performance = true
```

### 2. Health Checks

```bash
# HTTP health endpoint
curl http://localhost:8080/health

# CLI health check
agcodex health --verbose

# Component checks
agcodex health ast
agcodex health cache
agcodex health api
```

### 3. Grafana Dashboard

```json
{
  "dashboard": {
    "title": "AGCodex Monitoring",
    "panels": [
      {
        "title": "API Latency",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, agcodex_api_latency_seconds)"
          }
        ]
      },
      {
        "title": "Cache Hit Rate",
        "targets": [
          {
            "expr": "rate(agcodex_cache_hits_total[5m]) / rate(agcodex_cache_requests_total[5m])"
          }
        ]
      },
      {
        "title": "Memory Usage",
        "targets": [
          {
            "expr": "agcodex_memory_usage_bytes / 1024 / 1024 / 1024"
          }
        ]
      }
    ]
  }
}
```

### 4. Alerting Rules

```yaml
# prometheus/alerts.yml
groups:
- name: agcodex
  rules:
  - alert: HighMemoryUsage
    expr: agcodex_memory_usage_bytes > 7000000000
    for: 5m
    annotations:
      summary: "AGCodex memory usage above 7GB"
      
  - alert: HighAPILatency
    expr: histogram_quantile(0.95, agcodex_api_latency_seconds) > 5
    for: 5m
    annotations:
      summary: "API latency P95 above 5 seconds"
      
  - alert: LowCacheHitRate
    expr: rate(agcodex_cache_hits_total[5m]) / rate(agcodex_cache_requests_total[5m]) < 0.5
    for: 10m
    annotations:
      summary: "Cache hit rate below 50%"
```

---

## Backup & Recovery

### 1. Backup Strategy

```bash
#!/bin/bash
# backup-agcodex.sh

BACKUP_DIR="/backup/agcodex/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"

# Backup configuration
cp -r ~/.agcodex/config.toml "$BACKUP_DIR/"
cp -r ~/.agcodex/agents "$BACKUP_DIR/"

# Backup history (compressed)
tar czf "$BACKUP_DIR/history.tar.gz" ~/.agcodex/history

# Backup cache (optional, can be rebuilt)
tar czf "$BACKUP_DIR/cache.tar.gz" ~/.agcodex/cache

# Backup logs
tar czf "$BACKUP_DIR/logs.tar.gz" ~/.agcodex/logs

# Create manifest
cat > "$BACKUP_DIR/manifest.json" << EOF
{
  "timestamp": "$(date -Iseconds)",
  "version": "$(agcodex --version)",
  "size": "$(du -sh $BACKUP_DIR | cut -f1)"
}
EOF

echo "Backup completed: $BACKUP_DIR"
```

### 2. Automated Backups

```bash
# Add to crontab
0 2 * * * /usr/local/bin/backup-agcodex.sh

# Or use systemd timer
cat > /etc/systemd/system/agcodex-backup.timer << 'EOF'
[Unit]
Description=Daily AGCodex backup

[Timer]
OnCalendar=daily
Persistent=true

[Install]
WantedBy=timers.target
EOF

systemctl enable agcodex-backup.timer
```

### 3. Recovery Procedures

```bash
#!/bin/bash
# restore-agcodex.sh

BACKUP_DIR="$1"
if [ -z "$BACKUP_DIR" ]; then
    echo "Usage: $0 <backup_directory>"
    exit 1
fi

# Stop AGCodex
systemctl stop agcodex

# Backup current state
mv ~/.agcodex ~/.agcodex.old

# Restore configuration
mkdir -p ~/.agcodex
cp "$BACKUP_DIR/config.toml" ~/.agcodex/
cp -r "$BACKUP_DIR/agents" ~/.agcodex/

# Restore history
tar xzf "$BACKUP_DIR/history.tar.gz" -C /

# Restore cache (optional)
tar xzf "$BACKUP_DIR/cache.tar.gz" -C /

# Verify restoration
agcodex doctor

# Restart service
systemctl start agcodex

echo "Restoration completed from: $BACKUP_DIR"
```

---

## Migration Guide

### From AGCodex to AGCodex

```bash
#!/bin/bash
# migrate-from-agcodex.sh

# 1. Backup old AGCodex data
tar czf ~/agcodex-backup.tar.gz ~/.agcodex

# 2. Install AGCodex
cargo install agcodex --locked

# 3. Migrate configuration
agcodex migrate from-agcodex ~/.agcodex/config.toml

# 4. Migrate history
agcodex migrate history ~/.agcodex/history

# 5. Rebuild indexes
agcodex index rebuild

# 6. Verify migration
agcodex doctor
agcodex test all

echo "Migration completed successfully"
```

### Version Upgrades

```bash
# Backup before upgrade
agcodex backup create

# Upgrade AGCodex
cargo install agcodex --locked --force

# Run migrations
agcodex migrate auto

# Verify upgrade
agcodex --version
agcodex doctor
```

---

## API Reference

### REST API Endpoints

```bash
# Health check
GET /health
Response: {"status": "healthy", "version": "1.0.0"}

# Create conversation
POST /api/conversations
Body: {"mode": "build", "model": "gpt-4"}
Response: {"id": "uuid", "created_at": "2024-01-01T00:00:00Z"}

# Send message
POST /api/conversations/{id}/messages
Body: {"content": "Explain this code", "files": ["src/main.rs"]}
Response: {"id": "uuid", "content": "...", "role": "assistant"}

# Get session
GET /api/sessions/{id}
Response: {"id": "uuid", "messages": [...], "metadata": {...}}

# Invoke agent
POST /api/agents/invoke
Body: {"agent": "code-reviewer", "target": "src/", "options": {...}}
Response: {"id": "uuid", "status": "running", "progress": 0.5}
```

### WebSocket API

```javascript
// Connect to WebSocket
const ws = new WebSocket('ws://localhost:8080/ws');

// Subscribe to events
ws.send(JSON.stringify({
  type: 'subscribe',
  events: ['message', 'agent.progress', 'notification']
}));

// Receive events
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Event:', data.type, data.payload);
};
```

### MCP Server Protocol

```bash
# Start MCP server
agcodex mcp --port 5173

# Test with MCP inspector
npx @modelcontextprotocol/inspector http://localhost:5173

# Example tool invocation
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "ast_search",
    "arguments": {
      "pattern": "function.*auth",
      "language": "rust"
    }
  },
  "id": 1
}
```

---

## Appendix

### A. Supported Languages (Tree-sitter)

| Language | Parser | Extensions |
|----------|--------|------------|
| Rust | tree-sitter-rust | .rs |
| Python | tree-sitter-python | .py, .pyi |
| JavaScript | tree-sitter-javascript | .js, .jsx |
| TypeScript | tree-sitter-typescript | .ts, .tsx |
| Go | tree-sitter-go | .go |
| Java | tree-sitter-java | .java |
| C | tree-sitter-c | .c, .h |
| C++ | tree-sitter-cpp | .cpp, .hpp, .cc |
| C# | tree-sitter-c-sharp | .cs |
| Ruby | tree-sitter-ruby | .rb |
| PHP | tree-sitter-php | .php |
| Swift | tree-sitter-swift | .swift |
| Kotlin | tree-sitter-kotlin | .kt, .kts |
| Scala | tree-sitter-scala | .scala |
| Haskell | tree-sitter-haskell | .hs |
| Elixir | tree-sitter-elixir | .ex, .exs |
| Zig | tree-sitter-zig | .zig |
| SQL | tree-sitter-sql | .sql |
| HTML | tree-sitter-html | .html |
| CSS | tree-sitter-css | .css |
| JSON | tree-sitter-json | .json |
| YAML | tree-sitter-yaml | .yaml, .yml |
| TOML | tree-sitter-toml | .toml |
| Markdown | tree-sitter-markdown | .md |
| Bash | tree-sitter-bash | .sh, .bash |
| Dockerfile | tree-sitter-dockerfile | Dockerfile |
| HCL | tree-sitter-hcl | .hcl, .tf |
| Lua | tree-sitter-lua | .lua |
| Vim | tree-sitter-vim | .vim |
| ... | 50+ total parsers | ... |

### B. Environment Variables Reference

| Variable | Description | Default |
|----------|-------------|---------|
| AGAGCODEX_CONFIG_PATH | Config file location | ~/.agcodex/config.toml |
| AGAGCODEX_LOG_LEVEL | Logging level | info |
| AGAGCODEX_CACHE_DIR | Cache directory | ~/.agcodex/cache |
| OPENAI_API_KEY | OpenAI API key | - |
| ANTHROPIC_API_KEY | Anthropic API key | - |
| GEMINI_API_KEY | Google Gemini API key | - |
| VOYAGE_API_KEY | Voyage AI API key | - |
| OPENAI_EMBEDDING_KEY | Separate OpenAI embedding key | - |
| GITHUB_TOKEN | GitHub API token | - |
| RUST_LOG | Rust logging configuration | - |
| RUST_BACKTRACE | Show backtraces on panic | 0 |
| NO_COLOR | Disable colored output | - |
| TERM | Terminal type | xterm-256color |

### C. Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | API error |
| 4 | File I/O error |
| 5 | Network error |
| 6 | Authentication error |
| 7 | Sandbox violation |
| 8 | AST parsing error |
| 9 | Agent error |
| 10 | Session error |
| 127 | Command not found |
| 130 | Interrupted (Ctrl+C) |

### D. Performance Benchmarks

| Operation | Target | Typical | Maximum |
|-----------|--------|---------|---------|
| TUI startup | <500ms | 300ms | 1s |
| Mode switch | <50ms | 20ms | 100ms |
| File search (10k files) | <100ms | 50ms | 200ms |
| AST parse (1k LOC) | <10ms | 5ms | 20ms |
| Code search (1GB) | <200ms | 100ms | 500ms |
| Session save | <500ms | 200ms | 1s |
| Session load | <500ms | 300ms | 1s |
| Agent spawn | <100ms | 50ms | 200ms |
| API call (GPT-4) | <2s | 1s | 5s |
| Embedding (batch 100) | <1s | 500ms | 2s |

---

## Support & Resources

### Documentation
- Official Docs: https://agcodex.ai/docs
- API Reference: https://agcodex.ai/api
- Video Tutorials: https://youtube.com/@agcodex

### Community
- Discord: https://discord.gg/agcodex
- GitHub: https://github.com/agcodex/agcodex
- Forum: https://forum.agcodex.ai

### Enterprise Support
- Email: enterprise@agcodex.ai
- SLA: 24/7 support with 4-hour response time
- Training: On-site and remote training available
- Consulting: Architecture review and optimization

### License
AGCodex is available under:
- MIT License (open source)
- Commercial License (enterprise features)

---

*This deployment guide is maintained by the AGCodex team. For corrections or suggestions, please open an issue on GitHub.*