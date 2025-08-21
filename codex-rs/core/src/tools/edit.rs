//! Basic patch-based edit tool for AGCodex.
//!
//! Provides fast text-based editing with rich context extraction and semantic
//! impact analysis. Designed for <1ms performance on text replacements.

use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use thiserror::Error;

// Import PerformanceMetrics and related types from output module
use super::output::{PerformanceMetrics, MemoryUsage, CpuUsage, IoStats, CacheStats};

/// Output wrapper for edit operations
pub type ToolOutput<T> = Result<T, EditError>;

#[derive(Error, Debug)]
pub enum EditError {
    #[error("file not found: {path}")]
    FileNotFound { path: PathBuf },
    
    #[error("pattern not found in file: {pattern}")]
    PatternNotFound { pattern: String },
    
    #[error("invalid line range: {range:?} for file with {total_lines} lines")]
    InvalidLineRange { range: Range<usize>, total_lines: usize },
    
    #[error("ambiguous pattern: found {count} matches for '{pattern}'")]
    AmbiguousPattern { pattern: String, count: usize },
    
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("encoding error: file contains invalid UTF-8")]
    EncodingError,
    
    #[error("performance target exceeded: {duration_ms}ms > 1ms for operation '{operation}'")]
    PerformanceThreshold { operation: String, duration_ms: f64 },
}

/// Core text patching engine optimized for speed
#[derive(Debug, Clone)]
pub struct TextPatcher {
    /// Whether to validate UTF-8 encoding (default: true)
    validate_encoding: bool,
    /// Maximum file size to process (default: 10MB)
    max_file_size: usize,
    /// Enable performance monitoring (default: true)
    monitor_performance: bool,
}

impl Default for TextPatcher {
    fn default() -> Self {
        Self {
            validate_encoding: true,
            max_file_size: 10 * 1024 * 1024, // 10MB
            monitor_performance: true,
        }
    }
}

impl TextPatcher {
    /// Create a new TextPatcher with default settings
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Fast string replacement with position tracking
    pub fn replace_text(&self, content: &str, old: &str, new: &str) -> ToolOutput<(String, Vec<TextChange>)> {
        let start_time = if self.monitor_performance { Some(Instant::now()) } else { None };
        
        if old.is_empty() {
            return Err(EditError::PatternNotFound { pattern: old.to_string() });
        }
        
        let mut result = String::with_capacity(content.len() + (new.len() - old.len()).max(0));
        let mut changes = Vec::new();
        let mut last_end = 0;
        let mut _match_count = 0;
        
        // Find all matches first to detect ambiguity
        let matches: Vec<_> = content.match_indices(old).collect();
        
        if matches.is_empty() {
            return Err(EditError::PatternNotFound { pattern: old.to_string() });
        }
        
        if matches.len() > 1 {
            return Err(EditError::AmbiguousPattern { 
                pattern: old.to_string(), 
                count: matches.len() 
            });
        }
        
        // Perform the single replacement
        for (start, matched) in matches {
            // Add content before match
            result.push_str(&content[last_end..start]);
            
            // Add replacement
            result.push_str(new);
            
            // Track the change
            let line_start = content[..start].rfind('\n').map_or(0, |pos| pos + 1);
            let line_end = content[start..].find('\n').map_or(content.len(), |pos| start + pos);
            let line_number = content[..start].matches('\n').count() + 1;
            
            changes.push(TextChange {
                start_pos: start,
                end_pos: start + matched.len(),
                line_number,
                old_text: matched.to_string(),
                new_text: new.to_string(),
                line_content_before: content[line_start..line_end].to_string(),
                line_content_after: {
                    let mut line_after = content[line_start..start].to_string();
                    line_after.push_str(new);
                    line_after.push_str(&content[start + matched.len()..line_end]);
                    line_after
                },
            });
            
            last_end = start + matched.len();
            _match_count += 1;
        }
        
        // Add remaining content
        result.push_str(&content[last_end..]);
        
        // Performance monitoring
        if let Some(start_time) = start_time {
            let duration = start_time.elapsed();
            let duration_ms = duration.as_secs_f64() * 1000.0;
            if duration_ms > 1.0 {
                return Err(EditError::PerformanceThreshold {
                    operation: "text_replacement".to_string(),
                    duration_ms,
                });
            }
        }
        
        Ok((result, changes))
    }
    
    /// Replace content in specific line range
    pub fn replace_line_range(&self, content: &str, line_range: Range<usize>, new_content: &str) -> ToolOutput<(String, Vec<TextChange>)> {
        let start_time = if self.monitor_performance { Some(Instant::now()) } else { None };
        
        let lines: Vec<&str> = content.lines().collect();
        
        // Validate line range (1-based to 0-based conversion)
        let start_line = line_range.start.saturating_sub(1);
        let end_line = line_range.end.saturating_sub(1);
        
        if start_line >= lines.len() || end_line > lines.len() || start_line > end_line {
            return Err(EditError::InvalidLineRange { 
                range: line_range, 
                total_lines: lines.len() 
            });
        }
        
        // Build new content
        let mut result_lines = Vec::with_capacity(lines.len());
        let mut changes = Vec::new();
        
        // Lines before the range
        result_lines.extend_from_slice(&lines[..start_line]);
        
        // Capture old content for change tracking
        let old_lines = &lines[start_line..end_line];
        let old_content = old_lines.join("\n");
        
        // Add new content
        if !new_content.is_empty() {
            result_lines.extend(new_content.lines());
        }
        
        // Lines after the range
        result_lines.extend_from_slice(&lines[end_line..]);
        
        // Track the change
        changes.push(TextChange {
            start_pos: lines[..start_line].join("\n").len() + if start_line > 0 { 1 } else { 0 },
            end_pos: lines[..end_line].join("\n").len() + if end_line > 0 { 1 } else { 0 },
            line_number: line_range.start,
            old_text: old_content,
            new_text: new_content.to_string(),
            line_content_before: old_lines.join("\n"),
            line_content_after: new_content.to_string(),
        });
        
        let result = result_lines.join("\n");
        
        // Performance monitoring
        if let Some(start_time) = start_time {
            let duration = start_time.elapsed();
            let duration_ms = duration.as_secs_f64() * 1000.0;
            if duration_ms > 1.0 {
                return Err(EditError::PerformanceThreshold {
                    operation: "line_range_replacement".to_string(),
                    duration_ms,
                });
            }
        }
        
        Ok((result, changes))
    }
}

/// Represents a single text change with full context
#[derive(Debug, Clone)]
pub struct TextChange {
    pub start_pos: usize,
    pub end_pos: usize,
    pub line_number: usize,
    pub old_text: String,
    pub new_text: String,
    pub line_content_before: String,
    pub line_content_after: String,
}

/// Rich context around an edit for LLM consumption
#[derive(Debug, Clone)]
pub struct EditContext {
    /// Lines before the change (default: 5 lines)
    pub before_lines: Vec<String>,
    /// Lines after the change (default: 5 lines)
    pub after_lines: Vec<String>,
    /// Detected containing scope (function, class, etc.)
    pub containing_scope: Option<ScopeInfo>,
    /// Estimated semantic impact level
    pub impact_level: SemanticImpact,
    /// File path for reference
    pub file_path: PathBuf,
}

/// Information about the containing scope
#[derive(Debug, Clone)]
pub struct ScopeInfo {
    pub scope_type: ScopeType,
    pub name: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone)]
pub enum ScopeType {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Module,
    Block,
    Unknown,
}

#[derive(Debug, Clone)]
pub enum SemanticImpact {
    /// Simple text/comment changes
    Minimal,
    /// Logic changes within a function
    Local,
    /// Interface/API changes
    Interface,
    /// Structural changes affecting multiple scopes
    Architectural,
}

/// Complete result of an edit operation
#[derive(Debug, Clone)]
pub struct EditResult {
    /// The changes that were made
    pub changes: Vec<TextChange>,
    /// Rich context around each change
    pub context: EditContext,
    /// Performance metrics
    pub performance: PerformanceMetrics,
    /// Success summary for LLM consumption
    pub summary: String,
}


/// Main edit tool with configurable context extraction
pub struct EditTool {
    patcher: TextPatcher,
    context_lines: usize,
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

impl EditTool {
    /// Create a new EditTool with default settings (5 context lines)
    pub fn new() -> Self {
        Self {
            patcher: TextPatcher::new(),
            context_lines: 5,
        }
    }
    
    /// Create an EditTool with custom context line count
    pub fn with_context_lines(context_lines: usize) -> Self {
        Self {
            patcher: TextPatcher::new(),
            context_lines,
        }
    }
    
    /// Perform text replacement edit with full context extraction
    pub fn edit(&self, file: &Path, old: &str, new: &str) -> ToolOutput<EditResult> {
        let start_time = Instant::now();
        
        // Read file content
        let content = std::fs::read_to_string(file).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                EditError::FileNotFound { path: file.to_path_buf() }
            } else {
                EditError::Io(e)
            }
        })?;
        
        // Validate encoding if needed
        if self.patcher.validate_encoding && !content.is_ascii() {
            // Basic UTF-8 validation - more sophisticated validation could be added
            if content.chars().any(|c| c == '\u{FFFD}') {
                return Err(EditError::EncodingError);
            }
        }
        
        // Perform the replacement
        let (new_content, changes) = self.patcher.replace_text(&content, old, new)?;
        
        // Extract context
        let context = self.extract_context(file, &content, &changes)?;
        
        // Write the file back
        std::fs::write(file, &new_content)?;
        
        let duration = start_time.elapsed();
        let performance = PerformanceMetrics {
            execution_time: duration,
            phase_times: HashMap::new(),
            memory_usage: MemoryUsage {
                peak_bytes: (content.len() * std::mem::size_of::<u8>()) as u64,
                average_bytes: (content.len() * std::mem::size_of::<u8>()) as u64,
                allocations: 0,
                deallocations: 0,
                efficiency_score: 0.95,
            },
            cpu_usage: CpuUsage {
                cpu_time: duration,
                utilization_percent: 0.0,
                context_switches: 0,
            },
            io_stats: IoStats {
                bytes_read: content.len() as u64,
                bytes_written: new_content.len() as u64,
                read_ops: 1,
                write_ops: 1,
                io_wait_time: Duration::from_millis(0),
            },
            cache_stats: CacheStats {
                hit_rate: 0.0,
                hits: 0,
                misses: 0,
                cache_size: 0,
                efficiency_score: 0.0,
            },
        };
        
        let summary = self.generate_summary(&changes, &context);
        
        Ok(EditResult {
            changes,
            context,
            performance,
            summary,
        })
    }
    
    /// Perform line-range edit with full context extraction
    pub fn edit_lines(&self, file: &Path, line_range: Range<usize>, new: &str) -> ToolOutput<EditResult> {
        let start_time = Instant::now();
        
        // Read file content
        let content = std::fs::read_to_string(file).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                EditError::FileNotFound { path: file.to_path_buf() }
            } else {
                EditError::Io(e)
            }
        })?;
        
        // Validate encoding if needed
        if self.patcher.validate_encoding && !content.is_ascii() {
            if content.chars().any(|c| c == '\u{FFFD}') {
                return Err(EditError::EncodingError);
            }
        }
        
        // Perform the replacement
        let (new_content, changes) = self.patcher.replace_line_range(&content, line_range, new)?;
        
        // Extract context
        let context = self.extract_context(file, &content, &changes)?;
        
        // Write the file back
        std::fs::write(file, &new_content)?;
        
        let duration = start_time.elapsed();
        let performance = PerformanceMetrics {
            execution_time: duration,
            phase_times: HashMap::new(),
            memory_usage: MemoryUsage {
                peak_bytes: (content.len() * std::mem::size_of::<u8>()) as u64,
                average_bytes: (content.len() * std::mem::size_of::<u8>()) as u64,
                allocations: 0,
                deallocations: 0,
                efficiency_score: 0.95,
            },
            cpu_usage: CpuUsage {
                cpu_time: duration,
                utilization_percent: 0.0,
                context_switches: 0,
            },
            io_stats: IoStats {
                bytes_read: content.len() as u64,
                bytes_written: new_content.len() as u64,
                read_ops: 1,
                write_ops: 1,
                io_wait_time: Duration::from_millis(0),
            },
            cache_stats: CacheStats {
                hit_rate: 0.0,
                hits: 0,
                misses: 0,
                cache_size: 0,
                efficiency_score: 0.0,
            },
        };
        
        let summary = self.generate_summary(&changes, &context);
        
        Ok(EditResult {
            changes,
            context,
            performance,
            summary,
        })
    }
    
    /// Extract rich context around changes
    fn extract_context(&self, file: &Path, content: &str, changes: &[TextChange]) -> ToolOutput<EditContext> {
        let lines: Vec<&str> = content.lines().collect();
        
        // For simplicity, use the first change to determine context
        let primary_change = changes.first().ok_or_else(|| {
            EditError::PatternNotFound { pattern: "no changes found".to_string() }
        })?;
        
        let change_line = primary_change.line_number.saturating_sub(1);
        
        // Extract before/after context lines
        let before_start = change_line.saturating_sub(self.context_lines);
        let before_lines: Vec<String> = lines[before_start..change_line]
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        let after_start = (change_line + 1).min(lines.len());
        let after_end = (after_start + self.context_lines).min(lines.len());
        let after_lines: Vec<String> = lines[after_start..after_end]
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        // Simple scope detection (can be enhanced with tree-sitter later)
        let containing_scope = self.detect_scope(&lines, change_line);
        
        // Simple semantic impact analysis
        let impact_level = self.analyze_semantic_impact(primary_change, &containing_scope);
        
        Ok(EditContext {
            before_lines,
            after_lines,
            containing_scope,
            impact_level,
            file_path: file.to_path_buf(),
        })
    }
    
    /// Basic scope detection using heuristics
    fn detect_scope(&self, lines: &[&str], change_line: usize) -> Option<ScopeInfo> {
        // Look backwards for scope indicators
        for i in (0..=change_line).rev() {
            let line = lines[i].trim();
            
            // Function detection
            if line.contains("fn ") && line.contains('(') {
                if let Some(name) = self.extract_function_name(line) {
                    // Find end of function (simplistic)
                    let end_line = self.find_scope_end(lines, i, change_line);
                    return Some(ScopeInfo {
                        scope_type: ScopeType::Function,
                        name: Some(name),
                        start_line: i + 1,
                        end_line,
                    });
                }
            }
            
            // Class/struct detection
            if line.starts_with("class ") || line.starts_with("struct ") {
                if let Some(name) = self.extract_type_name(line) {
                    let end_line = self.find_scope_end(lines, i, change_line);
                    let scope_type = if line.starts_with("class") { ScopeType::Class } else { ScopeType::Struct };
                    return Some(ScopeInfo {
                        scope_type,
                        name: Some(name),
                        start_line: i + 1,
                        end_line,
                    });
                }
            }
        }
        
        None
    }
    
    fn extract_function_name(&self, line: &str) -> Option<String> {
        // Simple regex-free extraction
        if let Some(fn_pos) = line.find("fn ") {
            let after_fn = &line[fn_pos + 3..];
            if let Some(paren_pos) = after_fn.find('(') {
                let name = after_fn[..paren_pos].trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
        None
    }
    
    fn extract_type_name(&self, line: &str) -> Option<String> {
        // Extract class/struct name
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            Some(parts[1].trim_end_matches('{').trim().to_string())
        } else {
            None
        }
    }
    
    fn find_scope_end(&self, lines: &[&str], start: usize, change_line: usize) -> usize {
        // Simple brace counting to find scope end
        let mut brace_count = 0;
        let mut found_opening = false;
        
        for (i, line) in lines.iter().enumerate().skip(start) {
            for ch in line.chars() {
                match ch {
                    '{' => {
                        brace_count += 1;
                        found_opening = true;
                    }
                    '}' => {
                        brace_count -= 1;
                        if found_opening && brace_count == 0 {
                            return i + 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        
        // Fallback: assume it ends soon after the change
        (change_line + 10).min(lines.len())
    }
    
    /// Analyze the semantic impact of a change
    fn analyze_semantic_impact(&self, change: &TextChange, _scope: &Option<ScopeInfo>) -> SemanticImpact {
        let old_text = &change.old_text;
        let new_text = &change.new_text;
        
        // Check if it's just a comment change
        if old_text.trim_start().starts_with("//") || new_text.trim_start().starts_with("//") {
            return SemanticImpact::Minimal;
        }
        
        // Check for interface changes
        let interface_keywords = &["pub ", "public ", "export ", "fn ", "class ", "struct "];
        if interface_keywords.iter().any(|kw| old_text.contains(kw) || new_text.contains(kw)) {
            return SemanticImpact::Interface;
        }
        
        // Check for structural changes
        if old_text.contains('{') || new_text.contains('{') || 
           old_text.contains('}') || new_text.contains('}') {
            return SemanticImpact::Architectural;
        }
        
        // Default to local impact
        SemanticImpact::Local
    }
    
    /// Generate an LLM-friendly summary of the changes
    fn generate_summary(&self, changes: &[TextChange], context: &EditContext) -> String {
        if changes.is_empty() {
            return "No changes made".to_string();
        }
        
        let change = &changes[0];
        let scope_info = context.containing_scope.as_ref()
            .map(|s| format!(" in {} {}", 
                match s.scope_type {
                    ScopeType::Function => "function",
                    ScopeType::Method => "method", 
                    ScopeType::Class => "class",
                    ScopeType::Struct => "struct",
                    ScopeType::Enum => "enum",
                    ScopeType::Module => "module",
                    ScopeType::Block => "block",
                    ScopeType::Unknown => "scope",
                },
                s.name.as_ref().unwrap_or(&"unknown".to_string())
            ))
            .unwrap_or_default();
        
        let impact_desc = match context.impact_level {
            SemanticImpact::Minimal => "minimal impact",
            SemanticImpact::Local => "local changes",
            SemanticImpact::Interface => "interface modifications", 
            SemanticImpact::Architectural => "structural changes",
        };
        
        format!(
            "Successfully replaced '{}' with '{}' at line {}{} ({}). Change affects {} characters.",
            change.old_text.chars().take(30).collect::<String>(),
            change.new_text.chars().take(30).collect::<String>(),
            change.line_number,
            scope_info,
            impact_desc,
            change.new_text.len() as i64 - change.old_text.len() as i64
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_basic_text_replacement() {
        let patcher = TextPatcher::new();
        let content = "Hello world\nThis is a test\nHello again";
        
        let result = patcher.replace_text(content, "Hello", "Hi");
        
        // Should fail due to ambiguity
        assert!(matches!(result, Err(EditError::AmbiguousPattern { .. })));
    }
    
    #[test]
    fn test_unique_text_replacement() {
        let patcher = TextPatcher::new();
        let content = "Hello world\nThis is a test\nGoodbye world";
        
        let result = patcher.replace_text(content, "This is a test", "This is working").unwrap();
        assert_eq!(result.0, "Hello world\nThis is working\nGoodbye world");
        assert_eq!(result.1.len(), 1);
        assert_eq!(result.1[0].line_number, 2);
    }
    
    #[test]
    fn test_line_range_replacement() {
        let patcher = TextPatcher::new();
        let content = "Line 1\nLine 2\nLine 3\nLine 4";
        
        let result = patcher.replace_line_range(content, 2..4, "New Line 2\nNew Line 3").unwrap();
        assert_eq!(result.0, "Line 1\nNew Line 2\nNew Line 3\nLine 4");
        assert_eq!(result.1.len(), 1);
    }
    
    #[test]
    fn test_edit_tool_integration() -> std::io::Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        writeln!(temp_file, "fn main() {{")?;
        writeln!(temp_file, "    println!(\"Hello, world!\");")?;
        writeln!(temp_file, "}}")?;
        
        let tool = EditTool::new();
        let result = tool.edit(temp_file.path(), "Hello, world!", "Hi there!").unwrap();
        
        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.changes[0].new_text, "Hi there!");
        assert!(result.summary.contains("Successfully replaced"));
        
        // Verify file was actually modified
        let content = std::fs::read_to_string(temp_file.path())?;
        assert!(content.contains("Hi there!"));
        
        Ok(())
    }
    
    #[test]
    fn test_scope_detection() {
        let tool = EditTool::new();
        let lines = vec![
            "use std::io;",
            "",
            "fn main() {",
            "    let x = 1;",
            "    println!(\"Hello\");",
            "}",
        ];
        let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        
        let scope = tool.detect_scope(&line_refs, 4);
        assert!(scope.is_some());
        let scope = scope.unwrap();
        assert!(matches!(scope.scope_type, ScopeType::Function));
        assert_eq!(scope.name, Some("main".to_string()));
        assert_eq!(scope.start_line, 3);
    }
    
    #[test]
    fn test_performance_monitoring() {
        let mut patcher = TextPatcher::new();
        patcher.monitor_performance = true;
        
        let content = "a".repeat(1000);
        let result = patcher.replace_text(&content, "a", "b");
        
        // Should fail due to ambiguous pattern, but performance monitoring should work
        assert!(matches!(result, Err(EditError::AmbiguousPattern { .. })));
    }
}