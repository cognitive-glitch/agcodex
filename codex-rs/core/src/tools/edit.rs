//! Simple, practical edit tool for AGCodex.
//!
//! Provides reliable line and text editing with LLM-friendly output.
//! Focuses on predictability over sophistication.

use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EditError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Pattern not found: {0}")]
    PatternNotFound(String),
    
    #[error("Line {line} does not exist (file has {total_lines} lines)")]
    InvalidLine { line: usize, total_lines: usize },
    
    #[error("Ambiguous match: found {count} occurrences of '{pattern}'")]
    AmbiguousMatch { pattern: String, count: usize },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("File contains invalid UTF-8")]
    InvalidUtf8,
}

/// LLM-friendly result of an edit operation
#[derive(Debug, Clone)]
pub struct EditResult {
    pub success: bool,
    pub message: String,        // Human & LLM readable description
    pub file_path: String,
    pub line_changed: Option<usize>,
    pub old_content: String,
    pub new_content: String,
    pub context_before: Vec<String>,  // 3 lines before
    pub context_after: Vec<String>,   // 3 lines after
}

/// Simple edit tool with practical API
pub struct EditTool {
    // Empty for now - all methods are static
}

impl EditTool {
    pub fn new() -> Self {
        Self {}
    }
    
    /// Edit a specific line number in a file
    pub fn edit_line(file_path: &str, line_number: usize, new_content: &str) -> Result<EditResult, EditError> {
        // Validate file exists
        if !Path::new(file_path).exists() {
            return Err(EditError::FileNotFound(file_path.to_string()));
        }
        
        // Read file content
        let content = fs::read_to_string(file_path)
            .map_err(EditError::Io)?
            .replace('\r', ""); // Normalize line endings
        
        // Validate UTF-8
        if content.chars().any(|c| c == '\u{FFFD}') {
            return Err(EditError::InvalidUtf8);
        }
        
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        // Validate line number (1-based)
        if line_number == 0 || line_number > lines.len() {
            return Err(EditError::InvalidLine {
                line: line_number,
                total_lines: lines.len(),
            });
        }
        
        // Get context before change
        let line_index = line_number - 1;
        let old_content = lines[line_index].clone();
        
        // Preserve indentation automatically
        let indentation = Self::detect_indentation(&old_content);
        let new_content_with_indent = if new_content.trim().is_empty() {
            String::new() // Empty line
        } else {
            format!("{}{}", indentation, new_content.trim())
        };
        
        // Make the change
        lines[line_index] = new_content_with_indent.clone();
        
        // Create backup
        let backup_path = format!("{}.backup", file_path);
        fs::write(&backup_path, &content).map_err(EditError::Io)?;
        
        // Write atomically
        let new_file_content = lines.join("\n");
        let temp_path = format!("{}.tmp", file_path);
        fs::write(&temp_path, &new_file_content).map_err(EditError::Io)?;
        fs::rename(&temp_path, file_path).map_err(EditError::Io)?;
        
        // Extract context
        let (context_before, context_after) = Self::extract_context(&lines, line_index);
        
        Ok(EditResult {
            success: true,
            message: format!(
                "Successfully changed line {} from '{}' to '{}'",
                line_number,
                old_content.trim(),
                new_content_with_indent.trim()
            ),
            file_path: file_path.to_string(),
            line_changed: Some(line_number),
            old_content,
            new_content: new_content_with_indent,
            context_before,
            context_after,
        })
    }
    
    /// Replace specific text in a file
    pub fn edit_text(file_path: &str, old_text: &str, new_text: &str) -> Result<EditResult, EditError> {
        // Validate file exists
        if !Path::new(file_path).exists() {
            return Err(EditError::FileNotFound(file_path.to_string()));
        }
        
        // Read file content
        let content = fs::read_to_string(file_path)
            .map_err(EditError::Io)?
            .replace('\r', ""); // Normalize line endings
        
        // Validate UTF-8
        if content.chars().any(|c| c == '\u{FFFD}') {
            return Err(EditError::InvalidUtf8);
        }
        
        // Find all matches
        let matches: Vec<_> = content.match_indices(old_text).collect();
        
        if matches.is_empty() {
            return Err(EditError::PatternNotFound(old_text.to_string()));
        }
        
        // If ambiguous, return all candidates with context
        if matches.len() > 1 {
            return Err(EditError::AmbiguousMatch {
                pattern: old_text.to_string(),
                count: matches.len(),
            });
        }
        
        // Make the replacement
        let new_content = content.replace(old_text, new_text);
        
        // Find which line was changed
        let match_pos = matches[0].0;
        let line_number = content[..match_pos].matches('\n').count() + 1;
        
        // Create backup
        let backup_path = format!("{}.backup", file_path);
        fs::write(&backup_path, &content).map_err(EditError::Io)?;
        
        // Write atomically
        let temp_path = format!("{}.tmp", file_path);
        fs::write(&temp_path, &new_content).map_err(EditError::Io)?;
        fs::rename(&temp_path, file_path).map_err(EditError::Io)?;
        
        // Extract context
        let lines: Vec<String> = new_content.lines().map(|s| s.to_string()).collect();
        let line_index = line_number.saturating_sub(1);
        let (context_before, context_after) = Self::extract_context(&lines, line_index);
        
        Ok(EditResult {
            success: true,
            message: format!(
                "Successfully replaced '{}' with '{}' at line {}",
                old_text.chars().take(50).collect::<String>(),
                new_text.chars().take(50).collect::<String>(),
                line_number
            ),
            file_path: file_path.to_string(),
            line_changed: Some(line_number),
            old_content: old_text.to_string(),
            new_content: new_text.to_string(),
            context_before,
            context_after,
        })
    }
    
    /// Detect indentation from existing line
    fn detect_indentation(line: &str) -> String {
        let mut indent = String::new();
        for ch in line.chars() {
            if ch == ' ' || ch == '\t' {
                indent.push(ch);
            } else {
                break;
            }
        }
        indent
    }
    
    /// Extract 3 lines before and after for context
    fn extract_context(lines: &[String], changed_line_index: usize) -> (Vec<String>, Vec<String>) {
        let before_start = changed_line_index.saturating_sub(3);
        let before_end = changed_line_index;
        
        let after_start = (changed_line_index + 1).min(lines.len());
        let after_end = (after_start + 3).min(lines.len());
        
        let context_before = lines[before_start..before_end].to_vec();
        let context_after = lines[after_start..after_end].to_vec();
        
        (context_before, context_after)
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Show all ambiguous matches with context for user selection
#[derive(Debug, Clone)]
pub struct AmbiguousMatches {
    pub matches: Vec<MatchCandidate>,
}

#[derive(Debug, Clone)]
pub struct MatchCandidate {
    pub line_number: usize,
    pub context: String,
    pub full_line: String,
}

impl EditTool {
    /// Find all matches of a pattern with context (for handling ambiguity)
    pub fn find_matches(file_path: &str, pattern: &str) -> Result<AmbiguousMatches, EditError> {
        if !Path::new(file_path).exists() {
            return Err(EditError::FileNotFound(file_path.to_string()));
        }
        
        let content = fs::read_to_string(file_path).map_err(EditError::Io)?;
        let lines: Vec<&str> = content.lines().collect();
        
        let mut matches = Vec::new();
        
        for (line_idx, line) in lines.iter().enumerate() {
            if line.contains(pattern) {
                let line_number = line_idx + 1;
                let context_start = line_idx.saturating_sub(2);
                let context_end = (line_idx + 3).min(lines.len());
                
                let context = lines[context_start..context_end]
                    .iter()
                    .enumerate()
                    .map(|(i, l)| {
                        let actual_line = context_start + i + 1;
                        if actual_line == line_number {
                            format!("→ {}: {}", actual_line, l)
                        } else {
                            format!("  {}: {}", actual_line, l)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                
                matches.push(MatchCandidate {
                    line_number,
                    context,
                    full_line: line.to_string(),
                });
            }
        }
        
        Ok(AmbiguousMatches { matches })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_edit_line() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "line 1\nline 2\nline 3\n";
        fs::write(temp_file.path(), content).unwrap();
        
        let result = EditTool::edit_line(
            temp_file.path().to_str().unwrap(),
            2,
            "modified line 2"
        ).unwrap();
        
        assert!(result.success);
        assert_eq!(result.line_changed, Some(2));
        assert_eq!(result.old_content, "line 2");
        assert_eq!(result.new_content, "modified line 2");
        
        // Verify file was changed
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains("modified line 2"));
    }
    
    #[test]
    fn test_edit_text_unique() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "hello world\nthis is unique\ngoodbye world\n";
        fs::write(temp_file.path(), content).unwrap();
        
        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "this is unique",
            "this is modified"
        ).unwrap();
        
        assert!(result.success);
        assert_eq!(result.line_changed, Some(2));
        
        // Verify file was changed
        let new_content = fs::read_to_string(temp_file.path()).unwrap();
        assert!(new_content.contains("this is modified"));
    }
    
    #[test]
    fn test_edit_text_ambiguous() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "hello world\nhello again\nhello there\n";
        fs::write(temp_file.path(), content).unwrap();
        
        let result = EditTool::edit_text(
            temp_file.path().to_str().unwrap(),
            "hello",
            "hi"
        );
        
        assert!(matches!(result, Err(EditError::AmbiguousMatch { count: 3, .. })));
    }
    
    #[test]
    fn test_indentation_preservation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "fn main() {\n    let x = 1;\n    let y = 2;\n}\n";
        fs::write(temp_file.path(), content).unwrap();
        
        let result = EditTool::edit_line(
            temp_file.path().to_str().unwrap(),
            2,
            "let x = 42;"
        ).unwrap();
        
        assert_eq!(result.new_content, "    let x = 42;");
    }
    
    #[test]
    fn test_find_matches() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let content = "hello world\nhello again\nhello there\n";
        fs::write(temp_file.path(), content).unwrap();
        
        let matches = EditTool::find_matches(
            temp_file.path().to_str().unwrap(),
            "hello"
        ).unwrap();
        
        assert_eq!(matches.matches.len(), 3);
        assert_eq!(matches.matches[0].line_number, 1);
        assert_eq!(matches.matches[1].line_number, 2);
        assert_eq!(matches.matches[2].line_number, 3);
    }
}