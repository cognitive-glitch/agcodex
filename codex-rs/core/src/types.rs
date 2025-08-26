//! Type-safe newtype wrappers and core types for the codex-rs system.
//!
//! This module provides compile-time safety through:
//! - Newtype wrappers with validation
//! - Zero-cost abstractions
//! - Const generics for fixed-size constraints
//! - NonZero types for size validation

use std::fmt;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;
use serde::Serialize;

// ============================================================================
// Core Error Types
// ============================================================================

/// Errors that can occur during type construction and validation
#[derive(Debug, thiserror::Error)]
pub enum TypeValidationError {
    #[error("Invalid file path: {0}")]
    InvalidFilePath(String),

    #[error("Invalid AST node ID: {0}")]
    InvalidAstNodeId(String),

    #[error("Invalid query pattern: {0}")]
    InvalidQueryPattern(String),

    #[error("Context window too large: {size} bytes (max: {max_size})")]
    ContextWindowTooLarge { size: usize, max_size: usize },

    #[error("Sandbox path outside allowed boundaries: {0}")]
    SandboxPathViolation(String),

    #[error("Empty value not allowed")]
    EmptyValue,

    #[error("Size must be non-zero")]
    ZeroSize,
}

/// Result type for this module
pub type Result<T> = std::result::Result<T, TypeValidationError>;

// ============================================================================
// Newtype Wrappers
// ============================================================================

/// Type-safe wrapper for file paths with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FilePath(PathBuf);

impl FilePath {
    /// Maximum allowed path length for safety
    pub const MAX_PATH_LENGTH: usize = 4096;

    /// Create a new FilePath with validation
    pub fn new(path: impl Into<PathBuf>) -> Result<Self> {
        let path_buf = path.into();
        Self::validate_path(&path_buf)?;
        Ok(FilePath(path_buf))
    }

    /// Create a FilePath from a trusted source (skip validation)
    ///
    /// # Safety
    /// The caller must ensure the path is valid and safe
    pub unsafe fn new_unchecked(path: impl Into<PathBuf>) -> Self {
        FilePath(path.into())
    }

    /// Get the inner PathBuf
    pub const fn as_path_buf(&self) -> &PathBuf {
        &self.0
    }

    /// Get the path as a string slice
    pub fn as_str(&self) -> Option<&str> {
        self.0.to_str()
    }

    /// Check if the path exists
    pub fn exists(&self) -> bool {
        self.0.exists()
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.0.is_dir()
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.0.is_file()
    }

    /// Join with another path component
    pub fn join(&self, path: impl AsRef<std::path::Path>) -> Self {
        // Note: We trust join operations are safe if the base path was validated
        FilePath(self.0.join(path))
    }

    /// Get the parent directory
    pub fn parent(&self) -> Option<FilePath> {
        self.0.parent().map(|p| FilePath(p.to_path_buf()))
    }

    fn validate_path(path: &PathBuf) -> Result<()> {
        let path_str = path.to_string_lossy();

        // Check length
        if path_str.len() > Self::MAX_PATH_LENGTH {
            return Err(TypeValidationError::InvalidFilePath(format!(
                "Path too long: {} chars",
                path_str.len()
            )));
        }

        // Check for dangerous patterns
        if path_str.contains("..") {
            return Err(TypeValidationError::InvalidFilePath(
                "Path traversal not allowed".to_string(),
            ));
        }

        // Check for null bytes
        if path_str.contains('\0') {
            return Err(TypeValidationError::InvalidFilePath(
                "Null bytes not allowed".to_string(),
            ));
        }

        Ok(())
    }
}

impl TryFrom<PathBuf> for FilePath {
    type Error = TypeValidationError;

    fn try_from(path: PathBuf) -> Result<Self> {
        Self::new(path)
    }
}

impl TryFrom<String> for FilePath {
    type Error = TypeValidationError;

    fn try_from(path: String) -> Result<Self> {
        Self::new(PathBuf::from(path))
    }
}

impl TryFrom<&str> for FilePath {
    type Error = TypeValidationError;

    fn try_from(path: &str) -> Result<Self> {
        Self::new(PathBuf::from(path))
    }
}

impl FromStr for FilePath {
    type Err = TypeValidationError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(PathBuf::from(s))
    }
}

impl fmt::Display for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.display())
    }
}

// ============================================================================

/// Type-safe AST node identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AstNodeId(NonZeroUsize);

impl AstNodeId {
    /// Create a new AST node ID
    pub fn new(id: usize) -> Result<Self> {
        NonZeroUsize::new(id)
            .map(AstNodeId)
            .ok_or(TypeValidationError::ZeroSize)
    }

    /// Get the inner value
    pub const fn get(&self) -> usize {
        self.0.get()
    }

    /// Get the raw NonZeroUsize
    pub const fn as_non_zero(&self) -> NonZeroUsize {
        self.0
    }
}

impl TryFrom<usize> for AstNodeId {
    type Error = TypeValidationError;

    fn try_from(id: usize) -> Result<Self> {
        Self::new(id)
    }
}

impl From<NonZeroUsize> for AstNodeId {
    fn from(id: NonZeroUsize) -> Self {
        AstNodeId(id)
    }
}

impl fmt::Display for AstNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================

/// Type-safe query pattern with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryPattern(String);

impl QueryPattern {
    /// Maximum pattern length for performance
    pub const MAX_PATTERN_LENGTH: usize = 8192;

    /// Create a new query pattern with validation
    pub fn new(pattern: impl Into<String>) -> Result<Self> {
        let pattern = pattern.into();
        Self::validate_pattern(&pattern)?;
        Ok(QueryPattern(pattern))
    }

    /// Get the pattern as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the pattern length
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the pattern is empty
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Check if this is a regex pattern
    pub fn is_regex(&self) -> bool {
        self.0.contains('*') || self.0.contains('?') || self.0.contains('[')
    }

    /// Check if this is a literal pattern
    pub fn is_literal(&self) -> bool {
        !self.is_regex()
    }

    fn validate_pattern(pattern: &str) -> Result<()> {
        if pattern.is_empty() {
            return Err(TypeValidationError::EmptyValue);
        }

        if pattern.len() > Self::MAX_PATTERN_LENGTH {
            return Err(TypeValidationError::InvalidQueryPattern(format!(
                "Pattern too long: {} chars",
                pattern.len()
            )));
        }

        // Basic regex validation for common errors
        if pattern.contains("**") && !pattern.contains("**/") {
            return Err(TypeValidationError::InvalidQueryPattern(
                "Invalid glob pattern: use **/ for recursive matching".to_string(),
            ));
        }

        Ok(())
    }
}

impl TryFrom<String> for QueryPattern {
    type Error = TypeValidationError;

    fn try_from(pattern: String) -> Result<Self> {
        Self::new(pattern)
    }
}

impl TryFrom<&str> for QueryPattern {
    type Error = TypeValidationError;

    fn try_from(pattern: &str) -> Result<Self> {
        Self::new(pattern.to_string())
    }
}

impl FromStr for QueryPattern {
    type Err = TypeValidationError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s.to_string())
    }
}

impl fmt::Display for QueryPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================

/// Type-safe context window with size limits
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextWindow(String);

impl ContextWindow {
    /// Default maximum context window size (1MB)
    pub const DEFAULT_MAX_SIZE: usize = 1024 * 1024;

    /// Create a new context window with default size limit
    pub fn new(content: impl Into<String>) -> Result<Self> {
        Self::new_with_limit(content, Self::DEFAULT_MAX_SIZE)
    }

    /// Create a new context window with custom size limit
    pub fn new_with_limit(content: impl Into<String>, max_size: usize) -> Result<Self> {
        let content = content.into();

        if content.len() > max_size {
            return Err(TypeValidationError::ContextWindowTooLarge {
                size: content.len(),
                max_size,
            });
        }

        Ok(ContextWindow(content))
    }

    /// Get the content
    pub fn content(&self) -> &str {
        &self.0
    }

    /// Get the content length in bytes
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the context is empty
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Estimate token count (rough approximation: 1 token ≈ 4 chars)
    pub const fn estimated_tokens(&self) -> usize {
        self.0.len() / 4
    }

    /// Truncate to fit within a new size limit
    pub fn truncate_to_fit(&mut self, max_size: usize) {
        if self.0.len() > max_size {
            self.0.truncate(max_size);
            // Ensure we don't cut in the middle of a UTF-8 character
            while !self.0.is_char_boundary(self.0.len()) && !self.0.is_empty() {
                self.0.pop();
            }
        }
    }
}

impl TryFrom<String> for ContextWindow {
    type Error = TypeValidationError;

    fn try_from(content: String) -> Result<Self> {
        Self::new(content)
    }
}

impl TryFrom<&str> for ContextWindow {
    type Error = TypeValidationError;

    fn try_from(content: &str) -> Result<Self> {
        Self::new(content.to_string())
    }
}

impl fmt::Display for ContextWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================

/// Type-safe sandbox path with boundary validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SandboxPath {
    inner: PathBuf,
    sandbox_root: PathBuf,
}

impl SandboxPath {
    /// Create a new sandbox path with boundary checking
    pub fn new(path: impl Into<PathBuf>, sandbox_root: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let sandbox_root = sandbox_root.into();

        // Canonicalize paths for proper comparison (if they exist)
        let canonical_path = if path.exists() {
            path.canonicalize().map_err(|e| {
                TypeValidationError::SandboxPathViolation(format!(
                    "Failed to canonicalize path: {}",
                    e
                ))
            })?
        } else {
            path
        };

        let canonical_root = if sandbox_root.exists() {
            sandbox_root.canonicalize().map_err(|e| {
                TypeValidationError::SandboxPathViolation(format!(
                    "Failed to canonicalize sandbox root: {}",
                    e
                ))
            })?
        } else {
            sandbox_root
        };

        // Check if path is within sandbox boundaries
        if !canonical_path.starts_with(&canonical_root) {
            return Err(TypeValidationError::SandboxPathViolation(format!(
                "Path {} is outside sandbox root {}",
                canonical_path.display(),
                canonical_root.display()
            )));
        }

        Ok(SandboxPath {
            inner: canonical_path,
            sandbox_root: canonical_root,
        })
    }

    /// Get the path relative to the sandbox root
    pub fn relative_path(&self) -> Result<&std::path::Path> {
        self.inner.strip_prefix(&self.sandbox_root).map_err(|e| {
            TypeValidationError::SandboxPathViolation(format!("Failed to get relative path: {}", e))
        })
    }

    /// Get the full path
    pub fn full_path(&self) -> &std::path::Path {
        &self.inner
    }

    /// Get the sandbox root
    pub fn sandbox_root(&self) -> &std::path::Path {
        &self.sandbox_root
    }

    /// Check if this path exists
    pub fn exists(&self) -> bool {
        self.inner.exists()
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.inner.is_dir()
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.inner.is_file()
    }
}

impl fmt::Display for SandboxPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.display())
    }
}

// ============================================================================
// Const Generic Types
// ============================================================================

/// Fixed-size buffer with compile-time size validation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedBuffer<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> FixedBuffer<N> {
    /// Create a new empty buffer
    pub const fn new() -> Self {
        Self {
            data: [0; N],
            len: 0,
        }
    }

    /// Create from a slice, returning error if too large
    pub fn from_slice(data: &[u8]) -> Result<Self> {
        if data.len() > N {
            return Err(TypeValidationError::ContextWindowTooLarge {
                size: data.len(),
                max_size: N,
            });
        }

        let mut buffer = Self::new();
        buffer.data[..data.len()].copy_from_slice(data);
        buffer.len = data.len();
        Ok(buffer)
    }

    /// Get the used data
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }

    /// Get the capacity
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Get the current length
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Check if full
    pub const fn is_full(&self) -> bool {
        self.len == N
    }

    /// Push a byte if there's space
    pub const fn push(&mut self, byte: u8) -> Result<()> {
        if self.len >= N {
            return Err(TypeValidationError::ContextWindowTooLarge {
                size: self.len + 1,
                max_size: N,
            });
        }

        self.data[self.len] = byte;
        self.len += 1;
        Ok(())
    }

    /// Clear the buffer
    pub const fn clear(&mut self) {
        self.len = 0;
    }

    /// Write bytes to the buffer
    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        if self.len + data.len() > N {
            return Err(TypeValidationError::ContextWindowTooLarge {
                size: self.len + data.len(),
                max_size: N,
            });
        }

        self.data[self.len..self.len + data.len()].copy_from_slice(data);
        self.len += data.len();
        Ok(())
    }
}

impl<const N: usize> Default for FixedBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Common Type Aliases
// ============================================================================

/// Small fixed buffer for IDs and short strings
pub type SmallBuffer = FixedBuffer<256>;

/// Medium fixed buffer for paths and queries
pub type MediumBuffer = FixedBuffer<1024>;

/// Large fixed buffer for content chunks
pub type LargeBuffer = FixedBuffer<4096>;

/// Validated file size
pub type FileSize = NonZeroUsize;

/// Validated line number (1-based)
pub type LineNumber = NonZeroUsize;

/// Validated column number (1-based)
pub type ColumnNumber = NonZeroUsize;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn test_file_path_validation() {
        // Valid paths
        assert!(FilePath::new("/tmp/test.txt").is_ok());
        assert!(FilePath::new("relative/path.rs").is_ok());

        // Invalid paths
        assert!(FilePath::new("../../../etc/passwd").is_err());
        assert!(FilePath::new("path/with\0null").is_err());

        // Long path
        let long_path = "a".repeat(FilePath::MAX_PATH_LENGTH + 1);
        assert!(FilePath::new(long_path).is_err());
    }

    #[test]
    fn test_ast_node_id() {
        assert!(AstNodeId::new(1).is_ok());
        assert!(AstNodeId::new(0).is_err());

        let id = AstNodeId::new(42).unwrap();
        assert_eq!(id.get(), 42);
    }

    #[test]
    fn test_query_pattern() {
        assert!(QueryPattern::new("*.rs").is_ok());
        assert!(QueryPattern::new("").is_err());

        let long_pattern = "a".repeat(QueryPattern::MAX_PATTERN_LENGTH + 1);
        assert!(QueryPattern::new(long_pattern).is_err());

        let pattern = QueryPattern::new("test_*.rs").unwrap();
        assert!(pattern.is_regex());

        let literal = QueryPattern::new("main.rs").unwrap();
        assert!(literal.is_literal());
    }

    #[test]
    fn test_context_window() {
        let small_content = "small content";
        assert!(ContextWindow::new(small_content).is_ok());

        let large_content = "x".repeat(ContextWindow::DEFAULT_MAX_SIZE + 1);
        assert!(ContextWindow::new(large_content).is_err());

        let window = ContextWindow::new("test content").unwrap();
        assert_eq!(window.len(), 12);
        assert!(window.estimated_tokens() > 0);
    }

    #[test]
    fn test_fixed_buffer() {
        let mut buffer = FixedBuffer::<10>::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.capacity(), 10);

        assert!(buffer.push(1).is_ok());
        assert!(!buffer.is_empty());
        assert_eq!(buffer.len(), 1);

        // Fill to capacity
        for i in 2..=10 {
            assert!(buffer.push(i).is_ok());
        }
        assert!(buffer.is_full());

        // Should fail to push when full
        assert!(buffer.push(11).is_err());

        // Test from_slice
        let data = [1, 2, 3, 4, 5];
        let buffer2 = FixedBuffer::<10>::from_slice(&data).unwrap();
        assert_eq!(buffer2.as_slice(), &data);

        // Test oversized slice
        let large_data = [0; 20];
        assert!(FixedBuffer::<10>::from_slice(&large_data).is_err());
    }
}
/// Search scope for search queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchScope {
    CurrentDirectory,
    Recursive,
    SpecificPath(FilePath),
    Glob(String),
}

impl Default for SearchScope {
    fn default() -> Self {
        Self::CurrentDirectory
    }
}

/// Priority levels for context and operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}
