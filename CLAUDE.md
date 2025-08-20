# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the Rust implementation of Codex CLI - a coding agent from OpenAI that runs locally. The project is structured as a Cargo workspace containing multiple crates that work together to provide CLI functionality, TUI interface, and code assistance capabilities.

## Build and Development Commands

### Building the Project
```bash
# Build all crates in the workspace
cargo build

# Build with release optimizations
cargo build --release

# Build specific binary
cargo build --bin codex
```

### Running Tests
```bash
# Run all tests in the workspace
cargo test

# Run tests for a specific crate
cargo test -p codex-core

# Run a specific test
cargo test test_name

# Run tests with output displayed
cargo test -- --nocapture
```

### Code Quality Checks
```bash
# Format code (REQUIRED before committing)
cargo fmt -- --config imports_granularity=Item

# Run clippy linter (REQUIRED before committing)
cargo clippy --tests

# Check for compilation errors without building
cargo check
```

### Running the Application
```bash
# Run the TUI with a prompt
cargo run --bin codex -- "explain this codebase to me"

# Run in exec mode (non-interactive)
cargo run --bin codex exec -- "your task here"

# Run with specific model
cargo run --bin codex -- --model o3
```

## Architecture and Code Organization

### Workspace Structure
The codebase is organized as a Cargo workspace with the following key crates:

- **`core/`**: Business logic and main functionality. The heart of Codex operations.
- **`tui/`**: Terminal UI implementation using Ratatui
- **`cli/`**: Command-line interface entry point and argument parsing
- **`exec/`**: Headless/non-interactive execution mode
- **`mcp-client/`**, **`mcp-server/`**, **`mcp-types/`**: Model Context Protocol implementation
- **`file-search/`**: File discovery and fuzzy search functionality
- **`apply-patch/`**: Code modification and patching functionality
- **`protocol/`**: Communication protocol definitions
- **`execpolicy/`**: Sandboxing and execution policy enforcement
- **`linux-sandbox/`**: Linux-specific sandboxing using Landlock/seccomp

### Key Architectural Components

#### 1. Client Architecture (`core/src/client*.rs`)
- Handles API communication with OpenAI/compatible providers
- Supports both Chat Completions and Responses APIs
- Manages authentication (ChatGPT login and API keys)

#### 2. Conversation Management (`core/src/conversation_*.rs`)
- Manages conversation state and history
- Handles turn-based interactions
- Tracks diffs and changes per turn

#### 3. Execution Environment (`core/src/exec*.rs`)
- Sandboxed command execution
- Platform-specific sandbox implementations (Seatbelt on macOS, Landlock on Linux)
- Safety checks and approval workflows

#### 4. Configuration System (`core/src/config*.rs`)
- TOML-based configuration
- Profile support for different configurations
- Model provider definitions

## Areas for Enhancement

### 1. Code Intelligence and AST Operations
**Current State**: Basic tree-sitter integration exists but is underutilized.

**Recommended Improvements**:
- Enhance `file-search/` crate to use tree-sitter for semantic code search
- Integrate ast-grep for structural pattern matching
- Add srgn for advanced code transformations
- Implement smart context retrieval using AST-compacted representations

**Implementation Strategy**:
```rust
// Add to file-search/Cargo.toml:
tree-sitter = "0.25"
tree-sitter-rust = "0.25"
tree-sitter-python = "0.25"
tree-sitter-javascript = "0.25"
tree-sitter-typescript = "0.25"
ast-grep-core = "latest"
```

### 2. Error Handling Enhancement
**Current State**: Already uses `thiserror` in `core/src/error.rs`

**Recommended Improvements**:
- Expand error types to be more granular
- Add context to errors using `.context()` from anyhow where appropriate
- Implement error recovery strategies
- Add structured error codes for better debugging

### 3. Type Safety Improvements
**Current State**: Good use of Rust's type system but could be stronger

**Recommended Improvements**:
- Use newtype pattern for domain-specific types
- Add phantom types for compile-time guarantees
- Implement builder patterns for complex structs
- Use typestate pattern for state machines

### 4. Smart Context Retrieval System
**Proposed Architecture**:
```rust
// New module: core/src/context_engine/
mod ast_compactor;  // Compress code to essential structure
mod semantic_index; // Index code semantically
mod retrieval;      // Smart retrieval based on query
```

### 5. Tool Integration Module
**Proposed Structure**:
```rust
// New module: core/src/code_tools/
mod ripgrep;     // rg integration
mod fd_find;     // fd integration  
mod ast_grep;    // ast-grep integration
mod tree_sitter; // Enhanced tree-sitter usage
```

## Development Guidelines

### Error Handling Pattern
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DomainError {
    #[error("failed to parse {path}: {reason}")]
    ParseError { path: String, reason: String },
    
    #[error("AST operation failed")]
    AstError(#[from] tree_sitter::Error),
}
```

### Type Safety Pattern
```rust
// Use newtype pattern
pub struct FilePath(PathBuf);
pub struct AstNodeId(usize);

// Use builder pattern
pub struct QueryBuilder {
    // fields...
}

impl QueryBuilder {
    pub fn with_language(mut self, lang: Language) -> Self { /*...*/ }
    pub fn build(self) -> Result<Query, Error> { /*...*/ }
}
```

### Testing Approach
- Unit tests go next to the code being tested
- Integration tests go in `tests/` directory
- Use `core_test_support` crate for test utilities
- Mock external services using `wiremock`

## Platform-Specific Considerations

### macOS
- Sandbox uses Apple Seatbelt (`sandbox-exec`)
- Test sandbox policies with `codex debug seatbelt [COMMAND]`

### Linux
- Sandbox uses Landlock and seccomp
- Test sandbox policies with `codex debug landlock [COMMAND]`
- May not work in all containerized environments

## Performance Optimization Notes

- Release builds use fat LTO and strip symbols for smaller binaries
- Cargo.toml configured for single codegen unit in release for better optimization
- Consider using `cargo flamegraph` for performance profiling

## MCP (Model Context Protocol) Integration

The codebase supports MCP both as client and server:
- Client: Configure MCP servers in `~/.codex/config.toml` under `[mcp_servers]`
- Server: Run `codex mcp` to expose Codex functionality via MCP

## Security Considerations

- All file operations should respect sandbox boundaries
- Network access is controlled by sandbox policy
- User approval required for operations outside workspace
- Never commit sensitive data (API keys, tokens)

## Common Workflows

### Adding a New Tool Integration
1. Create module in `core/src/code_tools/`
2. Define tool interface trait
3. Implement tool wrapper
4. Add to tool registry
5. Write integration tests

### Implementing a New Error Type
1. Add variant to `CodexErr` in `core/src/error.rs`
2. Use `thiserror` derive macro
3. Add recovery logic where the error is handled
4. Document error conditions

### Adding AST-based Features
1. Add language grammar dependency to Cargo.toml
2. Create parser in `core/src/ast/`
3. Implement visitor pattern for traversal
4. Add caching for parsed ASTs
5. Write comprehensive tests