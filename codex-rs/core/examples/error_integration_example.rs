//! Example showing how to integrate the enhanced error system into existing codex-rs modules
//!
//! This demonstrates practical usage patterns for:
//! - AST processing with rich error context
//! - Tool execution with recovery strategies  
//! - Context retrieval with graceful degradation
//! - Configuration validation with detailed feedback
//! - Error reporting integration

use agcodex_core::error::CodexErr;
use agcodex_core::error::ErrorCode;
use agcodex_core::error::ErrorContext;
use agcodex_core::error::ErrorLocation;
use agcodex_core::error::LanguageInfo;
use agcodex_core::error::RecoveryStrategy;
use agcodex_core::error::RetryConfig;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use tokio::fs;
use tokio::process::Command as AsyncCommand;

type Result<T> = std::result::Result<T, CodexErr>;

// Define example types
#[derive(Debug, Clone)]
pub struct ParsedAst {
    language: LanguageInfo,
    node_count: usize,
    file_path: PathBuf,
}

#[derive(Debug)]
pub struct ProcessedFile {
    file_path: PathBuf,
    language: LanguageInfo,
    symbols: Vec<String>,
}

#[derive(Debug)]
pub struct RetrievedContext {
    query: String,
    chunks: Vec<String>,
    relevance_score: f32,
}

/// Enhanced AST processing with comprehensive error handling
pub struct EnhancedAstProcessor {
    error_count: Arc<AtomicUsize>,
}

impl Default for EnhancedAstProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl EnhancedAstProcessor {
    pub fn new() -> Self {
        Self {
            error_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Process a source file with enhanced error handling
    pub async fn process_file(&self, file_path: &Path) -> Result<ProcessedFile> {
        // Detect language from file extension
        let language = self.detect_language(file_path)?;

        // Read file with error context
        let content =
            fs::read_to_string(file_path)
                .await
                .map_err(|e| CodexErr::ConfigurationError {
                    section: "file_access".to_string(),
                    key: Some(file_path.display().to_string()),
                    expected_type: Some("readable file".to_string()),
                    actual_value: None,
                    message: format!("Cannot read file: {}", e),
                    code: ErrorCode::ConfigurationMissing,
                    context: ErrorContext::new("File reading"),
                    recovery: RecoveryStrategy::FailFast,
                })?;

        // Parse AST with retry logic for transient failures
        let ast = self
            .parse_ast_internal(&content, &language, file_path)
            .await?;

        // Process AST with context
        let processed = self.process_ast(ast, &language)?;

        Ok(processed)
    }

    /// Detect language from file extension
    fn detect_language(&self, file_path: &Path) -> Result<LanguageInfo> {
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| CodexErr::General("Cannot determine file extension".to_string()))?;

        let (name, mime_type) = match extension {
            "rs" => ("Rust", "text/x-rust"),
            "js" => ("JavaScript", "application/javascript"),
            "ts" => ("TypeScript", "application/typescript"),
            "py" => ("Python", "text/x-python"),
            _ => {
                return Err(CodexErr::General(format!(
                    "Unsupported language: {}",
                    extension
                )));
            }
        };

        Ok(LanguageInfo {
            name: name.to_string(),
            file_extension: Some(extension.to_string()),
            mime_type: Some(mime_type.to_string()),
        })
    }

    /// Internal AST parsing with detailed error context
    async fn parse_ast_internal(
        &self,
        content: &str,
        language: &LanguageInfo,
        file_path: &Path,
    ) -> Result<ParsedAst> {
        // Simulate AST parsing - in real implementation, this would use tree-sitter
        if content.contains("syntax_error") {
            let mut error =
                CodexErr::ast_parse_error(language.clone(), "Syntax error detected in source code");

            // Add detailed location information
            if let CodexErr::AstParseError {
                location,
                source_code,
                ..
            } = &mut error
            {
                // Find the line with syntax error
                for (line_num, line) in content.lines().enumerate() {
                    if line.contains("syntax_error") {
                        *location = Some(ErrorLocation {
                            file: file_path.to_path_buf(),
                            line: Some(line_num + 1),
                            column: Some(line.find("syntax_error").unwrap() + 1),
                            context: None,
                        });
                        *source_code = Some(line.to_string());
                        break;
                    }
                }
            }

            self.error_count.fetch_add(1, Ordering::SeqCst);
            return Err(error);
        }

        // Simulate successful parsing
        Ok(ParsedAst {
            language: language.clone(),
            node_count: content.lines().count(),
            file_path: file_path.to_path_buf(),
        })
    }

    /// Process parsed AST
    fn process_ast(&self, ast: ParsedAst, language: &LanguageInfo) -> Result<ProcessedFile> {
        // Simulate processing
        let symbols = vec![
            format!("function_1_{}", language.name),
            format!("class_1_{}", language.name),
            format!("variable_1_{}", language.name),
        ];

        Ok(ProcessedFile {
            file_path: ast.file_path,
            language: ast.language,
            symbols,
        })
    }
}

/// Context retrieval with graceful degradation
pub struct ContextRetriever;

impl ContextRetriever {
    pub async fn retrieve_context(&self, query: &str) -> Result<RetrievedContext> {
        // Simulate context retrieval
        if query.is_empty() {
            return Err(CodexErr::ContextRetrievalError {
                operation: "query".to_string(),
                query: Some(query.to_string()),
                chunks_retrieved: 0,
                relevance_score: Some(0.0),
                message: "Empty query provided".to_string(),
                code: ErrorCode::ContextRetrievalFailed,
                context: ErrorContext::new("Context retrieval"),
                recovery: RecoveryStrategy::Degrade("Use fallback search".to_string()),
            });
        }

        Ok(RetrievedContext {
            query: query.to_string(),
            chunks: vec![
                "Chunk 1: relevant code".to_string(),
                "Chunk 2: documentation".to_string(),
            ],
            relevance_score: 0.85,
        })
    }
}

/// Tool execution with detailed error reporting
pub struct ToolExecutor;

impl ToolExecutor {
    pub async fn execute_tool(&self, tool_name: &str, args: Vec<String>) -> Result<String> {
        let output = AsyncCommand::new(tool_name)
            .args(&args)
            .output()
            .await
            .map_err(|e| {
                CodexErr::tool_execution_error(tool_name, format!("Failed to execute tool: {}", e))
            })?;

        if !output.status.success() {
            let mut error = CodexErr::tool_execution_error(tool_name, "Tool execution failed");

            if let CodexErr::ToolExecutionError {
                stdout,
                stderr,
                exit_code,
                ..
            } = &mut error
            {
                *stdout = Some(String::from_utf8_lossy(&output.stdout).to_string());
                *stderr = Some(String::from_utf8_lossy(&output.stderr).to_string());
                *exit_code = output.status.code();
            }

            return Err(error);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// Example of retry logic with error handling
async fn execute_with_retry<F, T>(mut operation: F, config: RetryConfig) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let mut attempts = 0;
    let mut last_error = None;

    while attempts < config.max_attempts {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempts += 1;
                last_error = Some(e);

                if attempts < config.max_attempts {
                    tokio::time::sleep(config.base_delay.mul_f32(2_f32.powi(attempts as i32)))
                        .await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize processor
    let processor = EnhancedAstProcessor::new();

    // Process a test file
    let test_file = PathBuf::from("test.rs");

    // Create a test file for demonstration
    fs::write(&test_file, "fn main() { println!(\"Hello, world!\"); }")
        .await
        .map_err(|e| CodexErr::General(format!("Failed to create test file: {}", e)))?;

    match processor.process_file(&test_file).await {
        Ok(processed) => {
            println!("Successfully processed file: {:?}", processed);
        }
        Err(e) => {
            eprintln!("Error processing file: {}", e);

            // Check recovery strategy
            match e.recovery_strategy() {
                RecoveryStrategy::Retry(config) => {
                    println!("Retrying with config: max_attempts={}", config.max_attempts);
                }
                RecoveryStrategy::Fallback(fallback) => {
                    println!("Using fallback: {}", fallback);
                }
                RecoveryStrategy::Degrade(degraded) => {
                    println!("Degrading functionality: {}", degraded);
                }
                RecoveryStrategy::FailFast => {
                    println!("Failing fast - no recovery possible");
                }
            }
        }
    }

    // Test context retrieval
    let retriever = ContextRetriever;
    match retriever.retrieve_context("find main function").await {
        Ok(context) => {
            println!("Retrieved context: {:?}", context);
        }
        Err(e) => {
            eprintln!("Context retrieval failed: {}", e);
        }
    }

    // Test tool execution
    let executor = ToolExecutor;
    match executor
        .execute_tool("echo", vec!["Hello from tool".to_string()])
        .await
    {
        Ok(output) => {
            println!("Tool output: {}", output);
        }
        Err(e) => {
            eprintln!("Tool execution failed: {}", e);
        }
    }

    // Clean up test file
    let _ = fs::remove_file(&test_file).await;

    Ok(())
}
