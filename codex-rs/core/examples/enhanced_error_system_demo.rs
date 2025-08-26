//! # Enhanced Error System Demo
//!
//! This example demonstrates the comprehensive error handling enhancements in codex-rs:
//!
//! 1. Granular error types with context
//! 2. Structured error codes
//! 3. Recovery strategies with retry logic
//! 4. Error reporting and analytics
//! 5. Graceful degradation patterns
//!
//! ## Features Demonstrated:
//! - AST parsing errors with language context
//! - Tool execution errors with command details  
//! - Context retrieval errors for RAG failures
//! - Sandbox violation errors with security context
//! - Configuration errors with validation details
//! - Semantic indexing errors with fallback strategies
//! - Error chain traversal and reporting
//! - Retry logic with exponential backoff
//! - Error pattern analysis and suggestions

use agcodex_core::error::CodexErr;
use agcodex_core::error::ErrorCode;
use agcodex_core::error::ErrorContext;
use agcodex_core::error::ErrorLocation;
use agcodex_core::error::LanguageInfo;
use agcodex_core::error::RecoveryStrategy;
use agcodex_core::error::RetryConfig;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::time::Duration;
use tokio::time::sleep;

type Result<T> = std::result::Result<T, CodexErr>;

/// Simulate AST parsing with error handling
async fn simulate_ast_parsing(code: &str, language: &str) -> Result<String> {
    // Simulate parsing failure for invalid syntax
    if code.contains("syntax_error") {
        let lang_info = LanguageInfo {
            name: language.to_string(),
            file_extension: Some(
                match language {
                    "rust" => "rs",
                    "python" => "py",
                    "javascript" => "js",
                    _ => "txt",
                }
                .to_string(),
            ),
            mime_type: Some(format!("text/{}", language)),
        };

        let mut error = CodexErr::ast_parse_error(lang_info, "Invalid syntax detected");

        // Add location information
        if let CodexErr::AstParseError { location, .. } = &mut error {
            *location = Some(ErrorLocation {
                file: PathBuf::from("src/main.rs"),
                line: Some(42),
                column: Some(15),
                context: None,
            });
        }

        return Err(error);
    }

    Ok(format!("Parsed {} code successfully", language))
}

/// Simulate tool execution with error handling
async fn simulate_tool_execution(tool: &str, args: Vec<String>) -> Result<String> {
    // Simulate tool not found
    if tool == "nonexistent" {
        let mut error = CodexErr::tool_execution_error(tool, "Tool not found in PATH");

        // Add command details
        if let CodexErr::ToolExecutionError {
            command,
            exit_code,
            stderr,
            ..
        } = &mut error
        {
            *command = Some(format!("{} {}", tool, args.join(" ")));
            *exit_code = Some(127); // Standard exit code for command not found
            *stderr = Some("command not found: nonexistent".to_string());
        }

        return Err(error);
    }

    // Simulate tool failure
    if tool == "failing_tool" {
        return Err(CodexErr::ToolExecutionError {
            tool: tool.to_string(),
            command: Some(format!("{} {}", tool, args.join(" "))),
            exit_code: Some(1),
            stdout: Some("Processing...".to_string()),
            stderr: Some("Error: Operation failed".to_string()),
            message: "Tool exited with non-zero status".to_string(),
            code: ErrorCode::ToolExecutionFailed,
            context: ErrorContext::new("Tool execution"),
            recovery: RecoveryStrategy::Retry(RetryConfig::default()),
        });
    }

    Ok(format!("Executed {} successfully", tool))
}

/// Simulate context retrieval with error handling
async fn simulate_context_retrieval(query: &str, chunks_needed: usize) -> Result<Vec<String>> {
    // Simulate empty query error
    if query.is_empty() {
        return Err(CodexErr::ContextRetrievalError {
            operation: "search".to_string(),
            query: Some(query.to_string()),
            chunks_retrieved: 0,
            relevance_score: Some(0.0),
            message: "Empty query provided".to_string(),
            code: ErrorCode::ContextRetrievalFailed,
            context: ErrorContext::new("Context retrieval"),
            recovery: RecoveryStrategy::Degrade("Use basic keyword search".to_string()),
        });
    }

    // Simulate partial retrieval
    if chunks_needed > 5 {
        return Err(CodexErr::ContextRetrievalError {
            operation: "retrieve".to_string(),
            query: Some(query.to_string()),
            chunks_retrieved: 5,
            relevance_score: Some(0.65),
            message: format!("Only retrieved 5 chunks out of {} requested", chunks_needed),
            code: ErrorCode::ContextRetrievalFailed,
            context: ErrorContext::new("Context retrieval"),
            recovery: RecoveryStrategy::Degrade("Use available chunks".to_string()),
        });
    }

    Ok((0..chunks_needed)
        .map(|i| format!("Chunk {}: {}", i + 1, query))
        .collect())
}

/// Simulate sandbox violation
async fn simulate_sandbox_operation(path: &str) -> Result<String> {
    // Simulate access to restricted path
    if path.starts_with("/etc") || path.starts_with("/sys") {
        return Err(CodexErr::SandboxViolationError {
            violation_type: "path_access".to_string(),
            attempted_operation: format!("read {}", path),
            blocked_path: Some(path.to_string()),
            message: "Access to system directory denied".to_string(),
            code: ErrorCode::SandboxViolation,
            context: ErrorContext::new("Sandbox enforcement"),
            recovery: RecoveryStrategy::FailFast,
        });
    }

    Ok(format!("Accessed {} successfully", path))
}

/// Simulate configuration validation
async fn simulate_config_validation(config: HashMap<String, String>) -> Result<()> {
    // Check required fields
    if !config.contains_key("api_key") {
        return Err(CodexErr::ConfigurationError {
            section: "authentication".to_string(),
            key: Some("api_key".to_string()),
            expected_type: Some("string".to_string()),
            actual_value: None,
            message: "Missing required API key".to_string(),
            code: ErrorCode::ConfigurationMissing,
            context: ErrorContext::new("Configuration validation"),
            recovery: RecoveryStrategy::Fallback("Use environment variable".to_string()),
        });
    }

    // Validate format
    if let Some(timeout) = config.get("timeout")
        && timeout.parse::<u64>().is_err() {
            return Err(CodexErr::ConfigurationError {
                section: "connection".to_string(),
                key: Some("timeout".to_string()),
                expected_type: Some("integer".to_string()),
                actual_value: Some(timeout.clone()),
                message: "Timeout must be a valid integer".to_string(),
                code: ErrorCode::ConfigurationInvalid,
                context: ErrorContext::new("Configuration validation"),
                recovery: RecoveryStrategy::Fallback("Use default timeout of 30s".to_string()),
            });
        }

    Ok(())
}

/// Simulate semantic indexing operation
async fn simulate_semantic_indexing(operation: &str, documents: usize) -> Result<String> {
    // Simulate index corruption
    if operation == "load" && documents > 1000 {
        return Err(CodexErr::SemanticIndexError {
            operation: operation.to_string(),
            index_name: Some("main_index".to_string()),
            document_count: Some(documents),
            message: "Index corrupted - checksum mismatch".to_string(),
            code: ErrorCode::SemanticIndexCorrupted,
            context: ErrorContext::new("Semantic indexing"),
            recovery: RecoveryStrategy::Fallback("Rebuild index from scratch".to_string()),
        });
    }

    // Simulate indexing failure
    if operation == "update" && documents == 0 {
        return Err(CodexErr::SemanticIndexError {
            operation: operation.to_string(),
            index_name: Some("main_index".to_string()),
            document_count: Some(documents),
            message: "No documents to index".to_string(),
            code: ErrorCode::SemanticIndexCreationFailed,
            context: ErrorContext::new("Semantic indexing"),
            recovery: RecoveryStrategy::Degrade("Skip indexing step".to_string()),
        });
    }

    Ok(format!("Indexed {} documents successfully", documents))
}

/// Example of retry logic with exponential backoff
async fn retry_operation<F, Fut, T>(mut operation: F, max_attempts: usize) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut delay_ms = 100;

    loop {
        attempt += 1;
        println!("Attempt {} of {}", attempt, max_attempts);

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt < max_attempts => {
                println!("Error on attempt {}: {}", attempt, e);

                // Check if error is retryable
                match e.recovery_strategy() {
                    RecoveryStrategy::Retry(_) => {
                        println!("Retrying after {}ms...", delay_ms);
                        sleep(Duration::from_millis(delay_ms)).await;
                        delay_ms *= 2; // Exponential backoff
                    }
                    _ => return Err(e), // Not retryable
                }
            }
            Err(e) => return Err(e), // Max attempts reached
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Enhanced Error System Demo ===\n");

    // Demo 1: AST Parsing Error
    println!("1. AST Parsing Error:");
    match simulate_ast_parsing("let x = syntax_error;", "rust").await {
        Ok(result) => println!("  Success: {}", result),
        Err(e) => {
            println!("  Error: {}", e);
            println!("  Code: {:?}", e.error_code());
            println!("  Recovery: {:?}", e.recovery_strategy());
            println!("  User Message: {}", e.user_message());
        }
    }
    println!();

    // Demo 2: Tool Execution Error
    println!("2. Tool Execution Error:");
    match simulate_tool_execution("nonexistent", vec!["arg1".to_string()]).await {
        Ok(result) => println!("  Success: {}", result),
        Err(e) => {
            println!("  Error: {}", e);
            if let CodexErr::ToolExecutionError {
                command,
                exit_code,
                stderr,
                ..
            } = &e
            {
                println!("  Command: {:?}", command);
                println!("  Exit Code: {:?}", exit_code);
                println!("  Stderr: {:?}", stderr);
            }
        }
    }
    println!();

    // Demo 3: Context Retrieval Error with Degradation
    println!("3. Context Retrieval with Degradation:");
    match simulate_context_retrieval("search query", 10).await {
        Ok(chunks) => println!("  Retrieved {} chunks", chunks.len()),
        Err(e) => {
            println!("  Error: {}", e);
            if let RecoveryStrategy::Degrade(fallback) = e.recovery_strategy() {
                println!("  Degrading: {}", fallback);
                // Attempt degraded operation
                match simulate_context_retrieval("search query", 3).await {
                    Ok(chunks) => println!("  Degraded Success: Retrieved {} chunks", chunks.len()),
                    Err(e) => println!("  Degraded Failed: {}", e),
                }
            }
        }
    }
    println!();

    // Demo 4: Sandbox Violation
    println!("4. Sandbox Violation:");
    match simulate_sandbox_operation("/etc/passwd").await {
        Ok(result) => println!("  Success: {}", result),
        Err(e) => {
            println!("  Security Error: {}", e);
            println!("  Recovery: {:?}", e.recovery_strategy());
        }
    }
    println!();

    // Demo 5: Configuration Error with Fallback
    println!("5. Configuration Validation:");
    let mut config = HashMap::new();
    config.insert("timeout".to_string(), "invalid".to_string());

    match simulate_config_validation(config).await {
        Ok(_) => println!("  Config valid"),
        Err(e) => {
            println!("  Config Error: {}", e);
            if let RecoveryStrategy::Fallback(fallback) = e.recovery_strategy() {
                println!("  Fallback: {}", fallback);
            }
        }
    }
    println!();

    // Demo 6: Semantic Indexing Error
    println!("6. Semantic Indexing Error:");
    match simulate_semantic_indexing("load", 2000).await {
        Ok(result) => println!("  Success: {}", result),
        Err(e) => {
            println!("  Index Error: {}", e);
            if let RecoveryStrategy::Fallback(action) = e.recovery_strategy() {
                println!("  Recovery Action: {}", action);
            }
        }
    }
    println!();

    // Demo 7: Retry Logic with Exponential Backoff
    println!("7. Retry Logic Demo:");
    let result = {
        let mut retry_count = 0;
        retry_operation(
            move || async move {
                retry_count += 1;
                if retry_count < 3 {
                    // Fail first 2 attempts
                    Err(CodexErr::General("Transient failure".to_string()))
                } else {
                    Ok("Success after retries".to_string())
                }
            },
            5,
        )
        .await
    };

    match result {
        Ok(msg) => println!("  {}", msg),
        Err(e) => println!("  Failed after retries: {}", e),
    }
    println!();

    println!("=== Demo Complete ===");
    Ok(())
}
