use reqwest::StatusCode;
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;
use thiserror::Error;
use tokio::task::JoinError;
use uuid::Uuid;

// Additional imports for error reporting and recovery
use rand;
use tracing;

pub type Result<T> = std::result::Result<T, CodexErr>;

/// Atomic counter for unique error IDs
static ERROR_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Structured error codes for better debugging and categorization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // AST and parsing errors (A000-A999)
    AstParseFailure,
    AstLanguageUnsupported,
    AstNodeTraversalError,
    AstCompactorError,

    // Tool execution errors (T000-T999)
    ToolNotFound,
    ToolExecutionTimeout,
    ToolExecutionFailed,
    ToolConfigurationInvalid,

    // Context and RAG errors (C000-C999)
    ContextRetrievalFailed,
    ContextIndexingError,
    ContextEmbeddingError,
    ContextCacheCorrupted,

    // Sandbox and security errors (S000-S999)
    SandboxViolation,
    SandboxSetupFailed,
    SandboxPermissionDenied,
    SandboxResourceExhausted,

    // Configuration errors (F000-F999)
    ConfigurationMissing,
    ConfigurationInvalid,
    ConfigurationPermissionDenied,
    ConfigurationCorrupted,

    // Semantic indexing errors (I000-I999)
    SemanticIndexCreationFailed,
    SemanticIndexQueryFailed,
    SemanticIndexCorrupted,
    SemanticIndexOutOfSync,

    // Network and communication errors (N000-N999)
    NetworkTimeout,
    NetworkConnectionFailed,
    NetworkAuthenticationFailed,
    NetworkRateLimited,

    // System and resource errors (R000-R999)
    ResourceExhausted,
    ResourcePermissionDenied,
    ResourceNotFound,
    ResourceCorrupted,

    // Generic and legacy errors (G000-G999)
    InternalError,
    UnknownError,
    OperationAborted,
    FeatureNotImplemented,
}

impl ErrorCode {
    /// Get the error code as a string for logging and debugging
    pub const fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::AstParseFailure => "A001",
            ErrorCode::AstLanguageUnsupported => "A002",
            ErrorCode::AstNodeTraversalError => "A003",
            ErrorCode::AstCompactorError => "A004",
            ErrorCode::ToolNotFound => "T001",
            ErrorCode::ToolExecutionTimeout => "T002",
            ErrorCode::ToolExecutionFailed => "T003",
            ErrorCode::ToolConfigurationInvalid => "T004",
            ErrorCode::ContextRetrievalFailed => "C001",
            ErrorCode::ContextIndexingError => "C002",
            ErrorCode::ContextEmbeddingError => "C003",
            ErrorCode::ContextCacheCorrupted => "C004",
            ErrorCode::SandboxViolation => "S001",
            ErrorCode::SandboxSetupFailed => "S002",
            ErrorCode::SandboxPermissionDenied => "S003",
            ErrorCode::SandboxResourceExhausted => "S004",
            ErrorCode::ConfigurationMissing => "F001",
            ErrorCode::ConfigurationInvalid => "F002",
            ErrorCode::ConfigurationPermissionDenied => "F003",
            ErrorCode::ConfigurationCorrupted => "F004",
            ErrorCode::SemanticIndexCreationFailed => "I001",
            ErrorCode::SemanticIndexQueryFailed => "I002",
            ErrorCode::SemanticIndexCorrupted => "I003",
            ErrorCode::SemanticIndexOutOfSync => "I004",
            ErrorCode::NetworkTimeout => "N001",
            ErrorCode::NetworkConnectionFailed => "N002",
            ErrorCode::NetworkAuthenticationFailed => "N003",
            ErrorCode::NetworkRateLimited => "N004",
            ErrorCode::ResourceExhausted => "R001",
            ErrorCode::ResourcePermissionDenied => "R002",
            ErrorCode::ResourceNotFound => "R003",
            ErrorCode::ResourceCorrupted => "R004",
            ErrorCode::InternalError => "G001",
            ErrorCode::UnknownError => "G002",
            ErrorCode::OperationAborted => "G003",
            ErrorCode::FeatureNotImplemented => "G004",
        }
    }

    /// Get a category description for this error code
    pub fn category(&self) -> &'static str {
        match self.as_str().chars().next().unwrap() {
            'A' => "AST/Parsing",
            'T' => "Tool Execution",
            'C' => "Context/RAG",
            'S' => "Sandbox/Security",
            'F' => "Configuration",
            'I' => "Semantic Indexing",
            'N' => "Network",
            'R' => "Resource",
            'G' => "Generic",
            _ => "Unknown",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}]", self.as_str(), self.category())
    }
}

/// Enhanced error context with recovery strategies
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error_id: u64,
    pub timestamp: SystemTime,
    pub operation: String,
    pub location: Option<String>,
    pub additional_info: HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            error_id: ERROR_COUNTER.fetch_add(1, Ordering::SeqCst),
            timestamp: SystemTime::now(),
            operation: operation.into(),
            location: None,
            additional_info: HashMap::new(),
        }
    }

    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    pub fn with_info(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.additional_info.insert(key.into(), value.into());
        self
    }
}

/// Retry configuration for error recovery
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: usize,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub exponential_base: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            exponential_base: 2.0,
            jitter: true,
        }
    }
}

/// Recovery strategy for different error types
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    /// Retry the operation with exponential backoff
    Retry(RetryConfig),
    /// Fall back to an alternative implementation
    Fallback(String),
    /// Continue with degraded functionality
    Degrade(String),
    /// Fail fast - no recovery possible
    FailFast,
}

/// Language information for AST parsing errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageInfo {
    pub name: String,
    pub file_extension: Option<String>,
    pub mime_type: Option<String>,
}

/// Location information for errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorLocation {
    pub file: PathBuf,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub context: Option<String>,
}

#[derive(Error, Debug, Clone, PartialEq)]
pub enum SandboxErr {
    /// Error from sandbox execution
    #[error("sandbox denied exec error, exit code: {0}, stdout: {1}, stderr: {2}")]
    Denied(i32, String, String),

    /// Error from linux seccomp filter setup
    #[cfg(target_os = "linux")]
    #[error("seccomp setup error: {0}")]
    SeccompInstall(String),

    /// Error from linux seccomp backend
    #[cfg(target_os = "linux")]
    #[error("seccomp backend error: {0}")]
    SeccompBackend(String),

    /// Command timed out
    #[error("command timed out")]
    Timeout,

    /// Command was killed by a signal
    #[error("command was killed by a signal")]
    Signal(i32),

    /// Error from linux landlock
    #[error("Landlock was not able to fully enforce all sandbox rules")]
    LandlockRestrict,
}

// Manual From implementations for seccompiler errors since they don't implement Clone
#[cfg(target_os = "linux")]
impl From<seccompiler::Error> for SandboxErr {
    fn from(err: seccompiler::Error) -> Self {
        SandboxErr::SeccompInstall(err.to_string())
    }
}

#[cfg(target_os = "linux")]
impl From<seccompiler::BackendError> for SandboxErr {
    fn from(err: seccompiler::BackendError) -> Self {
        SandboxErr::SeccompBackend(err.to_string())
    }
}

#[derive(Error, Debug)]
pub enum CodexErr {
    /// Returned by ResponsesClient when the SSE stream disconnects or errors out **after** the HTTP
    /// handshake has succeeded but **before** it finished emitting `response.completed`.
    ///
    /// The Session loop treats this as a transient error and will automatically retry the turn.
    ///
    /// Optionally includes the requested delay before retrying the turn.
    #[error("stream disconnected before completion: {0}")]
    Stream(String, Option<Duration>),

    #[error("no conversation with id: {0}")]
    ConversationNotFound(Uuid),

    #[error("session configured event was not the first event in the stream")]
    SessionConfiguredNotFirstEvent,

    /// Returned by run_command_stream when the spawned child process timed out (10s).
    #[error("timeout waiting for child process to exit")]
    Timeout,

    /// Returned by run_command_stream when the child could not be spawned (its stdout/stderr pipes
    /// could not be captured). Analogous to the previous `CodexError::Spawn` variant.
    #[error("spawn failed: child stdout/stderr not captured")]
    Spawn,

    /// Returned by run_command_stream when the user pressed Ctrl‑C (SIGINT). Session uses this to
    /// surface a polite FunctionCallOutput back to the model instead of crashing the CLI.
    #[error("interrupted (Ctrl-C)")]
    Interrupted,

    /// Unexpected HTTP status code.
    #[error("unexpected status {0}: {1}")]
    UnexpectedStatus(StatusCode, String),

    #[error("{0}")]
    UsageLimitReached(UsageLimitReachedError),

    #[error(
        "To use Codex with your ChatGPT plan, upgrade to Plus: https://openai.com/chatgpt/pricing."
    )]
    UsageNotIncluded,

    #[error("We're currently experiencing high demand, which may cause temporary errors.")]
    InternalServerError,

    /// Retry limit exceeded.
    #[error("exceeded retry limit, last status: {0}")]
    RetryLimit(StatusCode),

    /// Agent loop died unexpectedly
    #[error("internal error; agent loop died unexpectedly")]
    InternalAgentDied,

    /// Sandbox error
    #[error("sandbox error: {0}")]
    Sandbox(#[from] SandboxErr),

    #[error("agcodex-linux-sandbox was required but not provided")]
    LandlockSandboxExecutableNotProvided,

    // -----------------------------------------------------------------
    // Automatic conversions for common external error types
    // -----------------------------------------------------------------
    #[error(transparent)]
    Io(#[from] io::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[cfg(target_os = "linux")]
    #[error(transparent)]
    LandlockRuleset(#[from] landlock::RulesetError),

    #[cfg(target_os = "linux")]
    #[error(transparent)]
    LandlockPathFd(#[from] landlock::PathFdError),

    #[error(transparent)]
    TokioJoin(#[from] JoinError),

    #[error("{0}")]
    EnvVar(EnvVarError),

    // MCP-related errors
    #[error("MCP server error: {0}")]
    McpServer(String),

    #[error("MCP client start failed for server {server}: {error}")]
    McpClientStart { server: String, error: String },

    #[error("MCP tool not found: {0}")]
    McpToolNotFound(String),

    // Configuration errors
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("invalid working directory: {0}")]
    InvalidWorkingDirectory(String),

    // Mode restriction errors
    #[error("operation not allowed in current mode: {0}")]
    ModeRestriction(String),

    // Undo/Redo system errors
    #[error("no branch point available for creating branch")]
    NoBranchPointAvailable,

    #[error("no current state available for creating checkpoint")]
    NoCurrentStateForCheckpoint,

    #[error("snapshot {0} not found")]
    SnapshotNotFound(Uuid),

    #[error("branch {0} not found")]
    BranchNotFound(Uuid),

    #[error("undo stack is empty")]
    UndoStackEmpty,

    #[error("redo stack is empty")]
    RedoStackEmpty,

    #[error("memory limit exceeded: {current} > {limit} bytes")]
    MemoryLimitExceeded { current: usize, limit: usize },

    // General errors for migration from anyhow
    #[error("{0}")]
    General(String),

    // Enhanced error variants with rich context
    #[error("AST parse error in {language:?}: {message}")]
    AstParseError {
        language: LanguageInfo,
        location: Option<ErrorLocation>,
        message: String,
        source_code: Option<String>,
        code: ErrorCode,
        context: ErrorContext,
        recovery: RecoveryStrategy,
    },

    #[error("Tool execution error for '{tool}': {message}")]
    ToolExecutionError {
        tool: String,
        command: Option<String>,
        exit_code: Option<i32>,
        stdout: Option<String>,
        stderr: Option<String>,
        message: String,
        code: ErrorCode,
        context: ErrorContext,
        recovery: RecoveryStrategy,
    },

    #[error("Context retrieval error during {operation}: {message}")]
    ContextRetrievalError {
        operation: String,
        query: Option<String>,
        chunks_retrieved: usize,
        relevance_score: Option<f32>,
        message: String,
        code: ErrorCode,
        context: ErrorContext,
        recovery: RecoveryStrategy,
    },

    #[error("Sandbox violation ({violation_type}): {message}")]
    SandboxViolationError {
        violation_type: String,
        attempted_operation: String,
        blocked_path: Option<String>,
        message: String,
        code: ErrorCode,
        context: ErrorContext,
        recovery: RecoveryStrategy,
    },

    #[error("Configuration error in [{section}]: {message}")]
    ConfigurationError {
        section: String,
        key: Option<String>,
        expected_type: Option<String>,
        actual_value: Option<String>,
        message: String,
        code: ErrorCode,
        context: ErrorContext,
        recovery: RecoveryStrategy,
    },

    #[error("Semantic index error during {operation}: {message}")]
    SemanticIndexError {
        operation: String,
        index_name: Option<String>,
        document_count: Option<usize>,
        message: String,
        code: ErrorCode,
        context: ErrorContext,
        recovery: RecoveryStrategy,
    },
}

// -----------------------------------------------------------------
// Error Construction Helpers and Context Extensions
// -----------------------------------------------------------------

impl CodexErr {
    /// Create a new AST parsing error with rich context
    pub fn ast_parse_error(language: LanguageInfo, message: impl Into<String>) -> Self {
        Self::AstParseError {
            language,
            location: None,
            message: message.into(),
            source_code: None,
            code: ErrorCode::AstParseFailure,
            context: ErrorContext::new("AST parsing"),
            recovery: RecoveryStrategy::Fallback(
                "Skip AST analysis, use text-based fallback".to_string(),
            ),
        }
    }

    /// Create a new tool execution error with detailed context
    pub fn tool_execution_error(tool: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ToolExecutionError {
            tool: tool.into(),
            command: None,
            exit_code: None,
            stdout: None,
            stderr: None,
            message: message.into(),
            code: ErrorCode::ToolExecutionFailed,
            context: ErrorContext::new("Tool execution"),
            recovery: RecoveryStrategy::Retry(RetryConfig::default()),
        }
    }

    /// Create a new context retrieval error
    pub fn context_retrieval_error(
        operation: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::ContextRetrievalError {
            operation: operation.into(),
            query: None,
            chunks_retrieved: 0,
            relevance_score: None,
            message: message.into(),
            code: ErrorCode::ContextRetrievalFailed,
            context: ErrorContext::new("Context retrieval"),
            recovery: RecoveryStrategy::Degrade("Continue without enhanced context".to_string()),
        }
    }

    /// Create a new sandbox violation error
    pub fn sandbox_violation_error(
        violation_type: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::SandboxViolationError {
            violation_type: violation_type.into(),
            attempted_operation: "unknown".to_string(),
            blocked_path: None,
            message: message.into(),
            code: ErrorCode::SandboxViolation,
            context: ErrorContext::new("Sandbox security check"),
            recovery: RecoveryStrategy::FailFast,
        }
    }

    /// Create a new configuration error
    pub fn configuration_error(section: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            section: section.into(),
            key: None,
            expected_type: None,
            actual_value: None,
            message: message.into(),
            code: ErrorCode::ConfigurationInvalid,
            context: ErrorContext::new("Configuration validation"),
            recovery: RecoveryStrategy::Fallback("Use default configuration values".to_string()),
        }
    }

    /// Create a new semantic index error
    pub fn semantic_index_error(operation: impl Into<String>, message: impl Into<String>) -> Self {
        Self::SemanticIndexError {
            operation: operation.into(),
            index_name: None,
            document_count: None,
            message: message.into(),
            code: ErrorCode::SemanticIndexQueryFailed,
            context: ErrorContext::new("Semantic indexing"),
            recovery: RecoveryStrategy::Degrade("Use basic text search instead".to_string()),
        }
    }

    /// Get the error code for this error
    pub fn error_code(&self) -> ErrorCode {
        match self {
            Self::AstParseError { code, .. } => code.clone(),
            Self::ToolExecutionError { code, .. } => code.clone(),
            Self::ContextRetrievalError { code, .. } => code.clone(),
            Self::SandboxViolationError { code, .. } => code.clone(),
            Self::ConfigurationError { code, .. } => code.clone(),
            Self::SemanticIndexError { code, .. } => code.clone(),
            Self::Timeout => ErrorCode::NetworkTimeout,
            Self::Spawn => ErrorCode::ToolExecutionFailed,
            Self::Interrupted => ErrorCode::OperationAborted,
            Self::UnexpectedStatus(..) => ErrorCode::NetworkConnectionFailed,
            Self::UsageLimitReached(..) => ErrorCode::NetworkRateLimited,
            Self::InternalServerError => ErrorCode::InternalError,
            Self::RetryLimit(..) => ErrorCode::NetworkTimeout,
            Self::InternalAgentDied => ErrorCode::InternalError,
            Self::Sandbox(..) => ErrorCode::SandboxViolation,
            Self::InvalidConfig(..) => ErrorCode::ConfigurationInvalid,
            Self::ModeRestriction(..) => ErrorCode::OperationAborted,
            Self::MemoryLimitExceeded { .. } => ErrorCode::ResourceExhausted,
            _ => ErrorCode::UnknownError,
        }
    }

    /// Get the recovery strategy for this error
    pub fn recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            Self::AstParseError { recovery, .. } => recovery.clone(),
            Self::ToolExecutionError { recovery, .. } => recovery.clone(),
            Self::ContextRetrievalError { recovery, .. } => recovery.clone(),
            Self::SandboxViolationError { recovery, .. } => recovery.clone(),
            Self::ConfigurationError { recovery, .. } => recovery.clone(),
            Self::SemanticIndexError { recovery, .. } => recovery.clone(),
            Self::Stream(..) => RecoveryStrategy::Retry(RetryConfig::default()),
            Self::Timeout => RecoveryStrategy::Retry(RetryConfig::default()),
            Self::UnexpectedStatus(..) => RecoveryStrategy::Retry(RetryConfig::default()),
            Self::UsageLimitReached(..) => RecoveryStrategy::FailFast,
            Self::InternalServerError => RecoveryStrategy::Retry(RetryConfig::default()),
            Self::RetryLimit(..) => RecoveryStrategy::FailFast,
            Self::Sandbox(..) => RecoveryStrategy::FailFast,
            Self::InvalidConfig(..) => {
                RecoveryStrategy::Fallback("Use default configuration".to_string())
            }
            Self::ModeRestriction(..) => RecoveryStrategy::FailFast,
            _ => RecoveryStrategy::FailFast,
        }
    }

    /// Get a user-friendly error message with actionable information
    pub fn user_message(&self) -> Cow<'static, str> {
        match self {
            Self::AstParseError {
                language, message, ..
            } => Cow::Owned(format!(
                "Failed to parse {} code: {}. Try checking syntax or using a different language mode.",
                language.name, message
            )),
            Self::ToolExecutionError { tool, message, .. } => Cow::Owned(format!(
                "Tool '{}' failed: {}. Ensure the tool is installed and accessible.",
                tool, message
            )),
            Self::ContextRetrievalError {
                operation, message, ..
            } => Cow::Owned(format!(
                "Context retrieval failed during {}: {}. Some features may be limited.",
                operation, message
            )),
            Self::SandboxViolationError {
                violation_type,
                message,
                ..
            } => Cow::Owned(format!(
                "Security violation ({}): {}. Operation blocked for safety.",
                violation_type, message
            )),
            Self::ConfigurationError {
                section, message, ..
            } => Cow::Owned(format!(
                "Configuration error in [{}]: {}. Check your config file.",
                section, message
            )),
            Self::SemanticIndexError {
                operation, message, ..
            } => Cow::Owned(format!(
                "Semantic index error during {}: {}. Falling back to basic search.",
                operation, message
            )),
            Self::UsageLimitReached(err) => Cow::Owned(err.to_string()),
            Self::UsageNotIncluded => {
                Cow::Borrowed("Upgrade required: This feature requires a Plus subscription.")
            }
            Self::ModeRestriction(msg) => {
                Cow::Owned(format!("Mode restriction: {}. Try switching modes.", msg))
            }
            _ => Cow::Owned(self.to_string()),
        }
    }

    /// Get the error context if available
    pub const fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::AstParseError { context, .. } => Some(context),
            Self::ToolExecutionError { context, .. } => Some(context),
            Self::ContextRetrievalError { context, .. } => Some(context),
            Self::SandboxViolationError { context, .. } => Some(context),
            Self::ConfigurationError { context, .. } => Some(context),
            Self::SemanticIndexError { context, .. } => Some(context),
            _ => None,
        }
    }

    /// Get all error codes in the error chain
    pub fn error_chain(&self) -> Vec<ErrorCode> {
        let mut chain = vec![self.error_code()];

        // Add source error codes if they have them
        let mut current_source = self.source();
        while let Some(source) = current_source {
            if let Some(codex_err) = source.downcast_ref::<CodexErr>() {
                chain.push(codex_err.error_code());
            }
            current_source = source.source();
        }

        chain
    }

    /// Add context to this error using anyhow-style chaining
    pub fn with_context<C>(self, additional_context: C) -> Self
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        match self {
            Self::AstParseError {
                mut context,
                language,
                location,
                message,
                source_code,
                code,
                recovery,
            } => {
                context.additional_info.insert(
                    "additional_context".to_string(),
                    additional_context.to_string(),
                );
                Self::AstParseError {
                    context,
                    language,
                    location,
                    message,
                    source_code,
                    code,
                    recovery,
                }
            }
            Self::ToolExecutionError {
                mut context,
                tool,
                command,
                exit_code,
                stdout,
                stderr,
                message,
                code,
                recovery,
            } => {
                context.additional_info.insert(
                    "additional_context".to_string(),
                    additional_context.to_string(),
                );
                Self::ToolExecutionError {
                    context,
                    tool,
                    command,
                    exit_code,
                    stdout,
                    stderr,
                    message,
                    code,
                    recovery,
                }
            }
            Self::ContextRetrievalError {
                mut context,
                operation,
                query,
                chunks_retrieved,
                relevance_score,
                message,
                code,
                recovery,
            } => {
                context.additional_info.insert(
                    "additional_context".to_string(),
                    additional_context.to_string(),
                );
                Self::ContextRetrievalError {
                    context,
                    operation,
                    query,
                    chunks_retrieved,
                    relevance_score,
                    message,
                    code,
                    recovery,
                }
            }
            Self::SandboxViolationError {
                mut context,
                violation_type,
                attempted_operation,
                blocked_path,
                message,
                code,
                recovery,
            } => {
                context.additional_info.insert(
                    "additional_context".to_string(),
                    additional_context.to_string(),
                );
                Self::SandboxViolationError {
                    context,
                    violation_type,
                    attempted_operation,
                    blocked_path,
                    message,
                    code,
                    recovery,
                }
            }
            Self::ConfigurationError {
                mut context,
                section,
                key,
                expected_type,
                actual_value,
                message,
                code,
                recovery,
            } => {
                context.additional_info.insert(
                    "additional_context".to_string(),
                    additional_context.to_string(),
                );
                Self::ConfigurationError {
                    context,
                    section,
                    key,
                    expected_type,
                    actual_value,
                    message,
                    code,
                    recovery,
                }
            }
            Self::SemanticIndexError {
                mut context,
                operation,
                index_name,
                document_count,
                message,
                code,
                recovery,
            } => {
                context.additional_info.insert(
                    "additional_context".to_string(),
                    additional_context.to_string(),
                );
                Self::SemanticIndexError {
                    context,
                    operation,
                    index_name,
                    document_count,
                    message,
                    code,
                    recovery,
                }
            }
            // For legacy errors, wrap in General with context
            other => Self::General(format!("{}: {}", additional_context, other)),
        }
    }
}

#[derive(Debug)]
pub struct UsageLimitReachedError {
    pub plan_type: Option<String>,
}

impl std::fmt::Display for UsageLimitReachedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(plan_type) = &self.plan_type
            && plan_type == "plus"
        {
            write!(
                f,
                "You've hit your usage limit. Upgrade to Pro (https://openai.com/chatgpt/pricing), or wait for limits to reset (every 5h and every week.)."
            )?;
        } else {
            write!(
                f,
                "You've hit your usage limit. Limits reset every 5h and every week."
            )?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct EnvVarError {
    /// Name of the environment variable that is missing.
    pub var: String,

    /// Optional instructions to help the user get a valid value for the
    /// variable and set it.
    pub instructions: Option<String>,
}

impl std::fmt::Display for EnvVarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Missing environment variable: `{}`.", self.var)?;
        if let Some(instructions) = &self.instructions {
            write!(f, " {instructions}")?;
        }
        Ok(())
    }
}

pub fn get_error_message_ui(e: &CodexErr) -> String {
    match e {
        CodexErr::Sandbox(SandboxErr::Denied(_, _, stderr)) => stderr.to_string(),
        _ => e.to_string(),
    }
}

// -----------------------------------------------------------------
// Error Reporting and Recovery Utilities
// -----------------------------------------------------------------

/// Error reporting utilities for better UX and debugging
pub mod reporting {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;
    use tracing::error;
    use tracing::info;
    use tracing::warn;

    /// Global error reporter for collecting and analyzing error patterns
    #[derive(Debug, Default)]
    pub struct ErrorReporter {
        error_counts: Arc<Mutex<HashMap<ErrorCode, usize>>>,
        recent_errors: Arc<Mutex<Vec<(SystemTime, ErrorCode, String)>>>,
    }

    impl ErrorReporter {
        pub fn new() -> Self {
            Self::default()
        }

        /// Report an error and update statistics
        pub fn report_error(&self, error: &CodexErr) {
            let error_code = error.error_code();
            let message = error.to_string();
            let timestamp = SystemTime::now();

            // Update error counts
            {
                let mut counts = self.error_counts.lock().unwrap();
                *counts.entry(error_code.clone()).or_insert(0) += 1;
            }

            // Add to recent errors (keep last 100)
            {
                let mut recent = self.recent_errors.lock().unwrap();
                recent.push((timestamp, error_code.clone(), message.clone()));
                if recent.len() > 100 {
                    recent.remove(0);
                }
            }

            // Log based on error severity
            match error_code.category() {
                "Sandbox/Security" => {
                    error!("[{}] Security issue: {}", error_code.as_str(), message)
                }
                "Configuration" => {
                    warn!("[{}] Configuration issue: {}", error_code.as_str(), message)
                }
                "AST/Parsing" | "Context/RAG" | "Semantic Indexing" => {
                    info!("[{}] Processing issue: {}", error_code.as_str(), message)
                }
                _ => error!("[{}] Error: {}", error_code.as_str(), message),
            }
        }

        /// Get error frequency statistics
        pub fn get_error_stats(&self) -> HashMap<ErrorCode, usize> {
            self.error_counts.lock().unwrap().clone()
        }

        /// Get recent error patterns for debugging
        pub fn get_recent_errors(&self, limit: usize) -> Vec<(SystemTime, ErrorCode, String)> {
            let recent = self.recent_errors.lock().unwrap();
            recent.iter().rev().take(limit).cloned().collect()
        }

        /// Check if an error pattern suggests system issues
        pub fn analyze_error_patterns(&self) -> Vec<String> {
            let mut suggestions = Vec::new();
            let stats = self.get_error_stats();

            // Check for high frequency errors
            for (code, count) in &stats {
                if *count > 10 {
                    match code {
                        ErrorCode::ToolExecutionFailed => {
                            suggestions.push("High tool execution failures - check if required tools are installed and accessible".to_string());
                        }
                        ErrorCode::NetworkTimeout => {
                            suggestions.push("Frequent network timeouts - check internet connection or increase timeout values".to_string());
                        }
                        ErrorCode::ConfigurationInvalid => {
                            suggestions.push("Multiple configuration errors - review and validate configuration files".to_string());
                        }
                        ErrorCode::AstParseFailure => {
                            suggestions.push("Repeated AST parse failures - check if code files are valid or consider language detection improvements".to_string());
                        }
                        _ => {}
                    }
                }
            }

            suggestions
        }
    }
}

/// Error recovery mechanisms with retry logic and fallback strategies
pub mod recovery {
    use super::*;
    use rand::Rng;
    use std::future::Future;
    use tokio::time::Duration;
    use tokio::time::sleep;

    /// Execute an operation with retry logic based on error recovery strategy
    pub async fn execute_with_recovery<F, Fut, T>(
        mut operation: F,
        max_attempts: Option<usize>,
    ) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut attempts = 0;
        let default_max = max_attempts.unwrap_or(3);

        loop {
            attempts += 1;

            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    let recovery = error.recovery_strategy();

                    match recovery {
                        RecoveryStrategy::FailFast => return Err(error),
                        RecoveryStrategy::Retry(config)
                            if attempts < config.max_attempts.min(default_max) =>
                        {
                            let delay = calculate_backoff_delay(&config, attempts);
                            tracing::warn!(
                                "Attempt {} failed, retrying in {:?}: {}",
                                attempts,
                                delay,
                                error
                            );
                            sleep(delay).await;
                            continue;
                        }
                        RecoveryStrategy::Retry(_) => {
                            tracing::error!(
                                "Max retry attempts ({}) exceeded: {}",
                                default_max,
                                error
                            );
                            return Err(error);
                        }
                        RecoveryStrategy::Fallback(fallback_msg) => {
                            tracing::warn!(
                                "Using fallback strategy: {} (original error: {})",
                                fallback_msg,
                                error
                            );
                            return Err(error.with_context(format!("Fallback: {}", fallback_msg)));
                        }
                        RecoveryStrategy::Degrade(degrade_msg) => {
                            tracing::info!(
                                "Degrading functionality: {} (original error: {})",
                                degrade_msg,
                                error
                            );
                            return Err(error.with_context(format!("Degraded: {}", degrade_msg)));
                        }
                    }
                }
            }
        }
    }

    /// Calculate exponential backoff delay with jitter
    fn calculate_backoff_delay(config: &RetryConfig, attempt: usize) -> Duration {
        let base_delay_ms = config.base_delay.as_millis() as f64;
        let exponential_delay = base_delay_ms * config.exponential_base.powi(attempt as i32 - 1);

        let mut delay = Duration::from_millis(exponential_delay as u64);

        // Cap at max_delay
        if delay > config.max_delay {
            delay = config.max_delay;
        }

        // Add jitter to prevent thundering herd
        if config.jitter {
            let mut rng = rand::thread_rng();
            let jitter_factor = rng.gen_range(0.5..1.5);
            delay = Duration::from_millis((delay.as_millis() as f64 * jitter_factor) as u64);
        }

        delay
    }

    /// Check if an error is transient and should be retried
    pub fn is_transient_error(error: &CodexErr) -> bool {
        matches!(
            error.error_code(),
            ErrorCode::NetworkTimeout
                | ErrorCode::NetworkConnectionFailed
                | ErrorCode::ToolExecutionTimeout
                | ErrorCode::ContextRetrievalFailed
                | ErrorCode::SemanticIndexQueryFailed
        )
    }

    /// Get recommended recovery strategy for an error code
    pub fn get_recovery_strategy(code: &ErrorCode) -> RecoveryStrategy {
        match code {
            // Retry strategies for transient errors
            ErrorCode::NetworkTimeout
            | ErrorCode::NetworkConnectionFailed
            | ErrorCode::ToolExecutionTimeout
            | ErrorCode::ContextRetrievalFailed => RecoveryStrategy::Retry(RetryConfig::default()),

            // Fallback strategies for tool failures
            ErrorCode::ToolNotFound | ErrorCode::ToolExecutionFailed => {
                RecoveryStrategy::Fallback("Use alternative tool or manual processing".to_string())
            }

            // Degradation for non-critical features
            ErrorCode::SemanticIndexQueryFailed | ErrorCode::ContextEmbeddingError => {
                RecoveryStrategy::Degrade("Continue with reduced functionality".to_string())
            }

            // Security errors must fail fast
            ErrorCode::SandboxViolation | ErrorCode::SandboxPermissionDenied => {
                RecoveryStrategy::FailFast
            }

            // Configuration errors can use defaults
            ErrorCode::ConfigurationMissing | ErrorCode::ConfigurationInvalid => {
                RecoveryStrategy::Fallback("Use default configuration".to_string())
            }

            // Default to fail fast for unknown errors
            _ => RecoveryStrategy::FailFast,
        }
    }
}

// -----------------------------------------------------------------
// Context Extensions for anyhow-style error handling
// -----------------------------------------------------------------

/// Extension trait to add context to Results
pub trait ErrorContextExt<T> {
    /// Add context to an error using anyhow-style chaining
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static;

    /// Add context to an error with a closure (lazy evaluation)
    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C;
}

impl<T> ErrorContextExt<T> for Result<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
    {
        self.map_err(|e| e.with_context(context))
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display + Send + Sync + 'static,
        F: FnOnce() -> C,
    {
        self.map_err(|e| e.with_context(f()))
    }
}

/// Helper macro for creating errors with context
#[macro_export]
macro_rules! codex_error {
    (ast_parse, $lang:expr, $msg:expr) => {
        $crate::error::CodexErr::ast_parse_error(
            $crate::error::LanguageInfo {
                name: $lang.to_string(),
                file_extension: None,
                mime_type: None,
            },
            $msg,
        )
    };

    (tool_exec, $tool:expr, $msg:expr) => {
        $crate::error::CodexErr::tool_execution_error($tool, $msg)
    };

    (context, $op:expr, $msg:expr) => {
        $crate::error::CodexErr::context_retrieval_error($op, $msg)
    };

    (sandbox, $violation:expr, $msg:expr) => {
        $crate::error::CodexErr::sandbox_violation_error($violation, $msg)
    };

    (config, $section:expr, $msg:expr) => {
        $crate::error::CodexErr::configuration_error($section, $msg)
    };

    (index, $op:expr, $msg:expr) => {
        $crate::error::CodexErr::semantic_index_error($op, $msg)
    };
}

/// Global error reporter instance
static GLOBAL_ERROR_REPORTER: std::sync::OnceLock<Arc<reporting::ErrorReporter>> =
    std::sync::OnceLock::new();

/// Get or initialize the global error reporter
pub fn get_error_reporter() -> Arc<reporting::ErrorReporter> {
    GLOBAL_ERROR_REPORTER
        .get_or_init(|| Arc::new(reporting::ErrorReporter::new()))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_limit_reached_error_formats_plus_plan() {
        let err = UsageLimitReachedError {
            plan_type: Some("plus".to_string()),
        };
        assert_eq!(
            err.to_string(),
            "You've hit your usage limit. Upgrade to Pro (https://openai.com/chatgpt/pricing), or wait for limits to reset (every 5h and every week.)."
        );
    }

    #[test]
    fn usage_limit_reached_error_formats_default_when_none() {
        let err = UsageLimitReachedError { plan_type: None };
        assert_eq!(
            err.to_string(),
            "You've hit your usage limit. Limits reset every 5h and every week."
        );
    }

    #[test]
    fn usage_limit_reached_error_formats_default_for_other_plans() {
        let err = UsageLimitReachedError {
            plan_type: Some("pro".to_string()),
        };
        assert_eq!(
            err.to_string(),
            "You've hit your usage limit. Limits reset every 5h and every week."
        );
    }

    #[test]
    fn error_code_mapping_works() {
        let err = CodexErr::ast_parse_error(
            LanguageInfo {
                name: "rust".to_string(),
                file_extension: Some("rs".to_string()),
                mime_type: Some("text/rust".to_string()),
            },
            "Syntax error",
        );

        assert_eq!(err.error_code(), ErrorCode::AstParseFailure);
        assert_eq!(err.error_code().as_str(), "A001");
        assert_eq!(err.error_code().category(), "AST/Parsing");
    }

    #[test]
    fn recovery_strategy_assignment() {
        let err = CodexErr::tool_execution_error("ast-grep", "Command not found");

        match err.recovery_strategy() {
            RecoveryStrategy::Retry(_) => {}
            _ => panic!("Tool execution errors should have retry strategy"),
        }
    }

    #[test]
    fn user_message_provides_actionable_info() {
        let err = CodexErr::configuration_error("database", "Invalid URL format");
        let user_msg = err.user_message();

        assert!(user_msg.contains("Configuration error"));
        assert!(user_msg.contains("database"));
        assert!(user_msg.contains("Check your config file"));
    }

    #[test]
    fn error_context_tracking() {
        let mut ctx = ErrorContext::new("test operation");
        ctx = ctx.with_location("src/main.rs:42");
        ctx = ctx.with_info("user_id", "12345");

        assert_eq!(ctx.operation, "test operation");
        assert_eq!(ctx.location, Some("src/main.rs:42".to_string()));
        assert_eq!(
            ctx.additional_info.get("user_id"),
            Some(&"12345".to_string())
        );
        assert!(ctx.error_id > 0);
    }

    #[test]
    fn error_chain_traversal() {
        let base_err = CodexErr::tool_execution_error("ast-grep", "Command failed");
        let chain = base_err.error_chain();

        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0], ErrorCode::ToolExecutionFailed);
    }

    #[test]
    fn error_reporting_stats() {
        let reporter = reporting::ErrorReporter::new();
        let err1 = CodexErr::ast_parse_error(
            LanguageInfo {
                name: "rust".to_string(),
                file_extension: Some("rs".to_string()),
                mime_type: None,
            },
            "Syntax error",
        );
        let err2 = CodexErr::tool_execution_error("ast-grep", "Not found");

        reporter.report_error(&err1);
        reporter.report_error(&err2);
        reporter.report_error(&err1); // Report same type again

        let stats = reporter.get_error_stats();
        assert_eq!(stats.get(&ErrorCode::AstParseFailure), Some(&2));
        assert_eq!(stats.get(&ErrorCode::ToolExecutionFailed), Some(&1));
    }

    #[tokio::test]
    async fn recovery_retry_logic() {
        let attempt_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count_clone = attempt_count.clone();

        let result = recovery::execute_with_recovery(
            || {
                let count = count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async move {
                    if count < 1 {
                        Err(CodexErr::tool_execution_error("test", "Temporary failure"))
                    } else {
                        Ok("Success")
                    }
                }
            },
            Some(3),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(attempt_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn recovery_strategy_assignment_by_code() {
        let strategy = recovery::get_recovery_strategy(&ErrorCode::NetworkTimeout);
        match strategy {
            RecoveryStrategy::Retry(_) => {}
            _ => panic!("Network timeout should have retry strategy"),
        }

        let strategy = recovery::get_recovery_strategy(&ErrorCode::SandboxViolation);
        match strategy {
            RecoveryStrategy::FailFast => {}
            _ => panic!("Sandbox violations should fail fast"),
        }
    }

    #[test]
    fn transient_error_detection() {
        let timeout_err = CodexErr::Timeout;
        assert!(recovery::is_transient_error(&timeout_err));

        let config_err = CodexErr::InvalidConfig("test".to_string());
        assert!(!recovery::is_transient_error(&config_err));
    }

    #[test]
    fn macro_error_creation() {
        let err = codex_error!(ast_parse, "rust", "Syntax error");
        assert_eq!(err.error_code(), ErrorCode::AstParseFailure);

        let err = codex_error!(tool_exec, "ast-grep", "Command failed");
        assert_eq!(err.error_code(), ErrorCode::ToolExecutionFailed);
    }
}
