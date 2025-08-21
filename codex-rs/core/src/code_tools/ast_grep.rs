//! AST-Grep integration for structural code search.
//! Provides pattern-based AST search using ast-grep-core.
//! Note: Tree-sitter is the primary structural engine; AST-Grep is offered as
//! internal optional tooling.

use super::CodeTool;
use super::ToolError;
use ast_grep_language::SupportLang;
use dashmap::DashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

/// AST-Grep search tool using command-line interface
#[derive(Debug, Clone)]
pub struct AstGrep {
    /// Cache for search results to avoid recomputation
    result_cache: Arc<DashMap<String, Vec<AgMatch>>>,
}

/// Query types for AST-Grep
#[derive(Debug, Clone)]
pub enum AgQuery {
    /// Simple pattern search (e.g., "console.log($_)")
    Pattern {
        language: Option<String>,
        pattern: String,
        paths: Vec<PathBuf>,
    },
    /// YAML rule-based search for complex patterns
    Rule {
        yaml_rule: String,
        paths: Vec<PathBuf>,
    },
}

/// Match result from AST-Grep search
#[derive(Debug, Clone)]
pub struct AgMatch {
    pub file: PathBuf,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub matched_text: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    /// Captured metavariables (e.g., $_ -> captured value)
    pub metavariables: std::collections::HashMap<String, String>,
}

impl Default for AstGrep {
    fn default() -> Self {
        Self::new()
    }
}

impl AstGrep {
    /// Create a new AST-Grep instance with empty caches
    pub fn new() -> Self {
        Self {
            result_cache: Arc::new(DashMap::new()),
        }
    }

    /// Detect ast-grep language from file extension or language string
    fn detect_language(
        &self,
        lang_hint: Option<&str>,
        file_path: &Path,
    ) -> Result<SupportLang, ToolError> {
        // If language hint is provided, try to use it
        if let Some(lang) = lang_hint {
            return self.parse_language_string(lang);
        }

        // Otherwise detect from file extension
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| {
                ToolError::InvalidQuery("Cannot detect language from file path".to_string())
            })?;

        match extension {
            "rs" => Ok(SupportLang::Rust),
            "py" | "pyi" => Ok(SupportLang::Python),
            "js" | "mjs" | "cjs" => Ok(SupportLang::JavaScript),
            "ts" | "mts" | "cts" | "tsx" | "jsx" => Ok(SupportLang::TypeScript),
            "go" => Ok(SupportLang::Go),
            "java" => Ok(SupportLang::Java),
            "c" | "h" => Ok(SupportLang::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "c++" => Ok(SupportLang::Cpp),
            "cs" => Ok(SupportLang::CSharp),
            "sh" | "bash" | "zsh" => Ok(SupportLang::Bash),
            "rb" => Ok(SupportLang::Ruby),
            "php" => Ok(SupportLang::Php),
            "lua" => Ok(SupportLang::Lua),
            "hs" | "lhs" => Ok(SupportLang::Haskell),
            "ex" | "exs" => Ok(SupportLang::Elixir),
            "scala" | "sc" => Ok(SupportLang::Scala),
            "swift" => Ok(SupportLang::Swift),
            "kt" | "kts" => Ok(SupportLang::Kotlin),
            "html" | "htm" => Ok(SupportLang::Html),
            "css" | "scss" | "sass" => Ok(SupportLang::Css),
            "json" => Ok(SupportLang::Json),
            _ => Err(ToolError::UnsupportedLanguage(extension.to_string())),
        }
    }

    /// Parse language string to SupportLang
    fn parse_language_string(&self, lang: &str) -> Result<SupportLang, ToolError> {
        match lang.to_lowercase().as_str() {
            "rust" | "rs" => Ok(SupportLang::Rust),
            "python" | "py" => Ok(SupportLang::Python),
            "javascript" | "js" => Ok(SupportLang::JavaScript),
            "typescript" | "ts" => Ok(SupportLang::TypeScript),
            "go" | "golang" => Ok(SupportLang::Go),
            "java" => Ok(SupportLang::Java),
            "c" => Ok(SupportLang::C),
            "cpp" | "c++" | "cxx" => Ok(SupportLang::Cpp),
            "csharp" | "c#" | "cs" => Ok(SupportLang::CSharp),
            "bash" | "sh" => Ok(SupportLang::Bash),
            "ruby" | "rb" => Ok(SupportLang::Ruby),
            "php" => Ok(SupportLang::Php),
            "lua" => Ok(SupportLang::Lua),
            "haskell" | "hs" => Ok(SupportLang::Haskell),
            "elixir" | "ex" => Ok(SupportLang::Elixir),
            "scala" => Ok(SupportLang::Scala),
            "swift" => Ok(SupportLang::Swift),
            "kotlin" | "kt" => Ok(SupportLang::Kotlin),
            "html" => Ok(SupportLang::Html),
            "css" => Ok(SupportLang::Css),
            "json" => Ok(SupportLang::Json),
            _ => Err(ToolError::UnsupportedLanguage(lang.to_string())),
        }
    }

    /// Convert SupportLang to string for CLI usage
    fn language_to_string(&self, lang: SupportLang) -> &'static str {
        match lang {
            SupportLang::Rust => "rust",
            SupportLang::Python => "python",
            SupportLang::JavaScript => "javascript",
            SupportLang::TypeScript => "typescript",
            SupportLang::Go => "go",
            SupportLang::Java => "java",
            SupportLang::C => "c",
            SupportLang::Cpp => "cpp",
            SupportLang::CSharp => "csharp",
            SupportLang::Bash => "bash",
            SupportLang::Ruby => "ruby",
            SupportLang::Php => "php",
            SupportLang::Lua => "lua",
            SupportLang::Haskell => "haskell",
            SupportLang::Elixir => "elixir",
            SupportLang::Scala => "scala",
            SupportLang::Swift => "swift",
            SupportLang::Kotlin => "kotlin",
            SupportLang::Html => "html",
            SupportLang::Css => "css",
            SupportLang::Json => "json",
            _ => "text", // fallback
        }
    }

    /// Search files using ast-grep command line tool
    fn search_with_pattern(
        &self,
        pattern: &str,
        language: SupportLang,
        paths: &[PathBuf],
    ) -> Result<Vec<AgMatch>, ToolError> {
        if paths.is_empty() {
            return Ok(Vec::new());
        }

        // For now, implement a simple pattern-based search
        // In a full implementation, we would use ast-grep CLI or library directly
        let mut matches = Vec::new();
        let _lang_str = self.language_to_string(language);

        for path in paths {
            if !path.exists() || !path.is_file() {
                continue;
            }

            // Read file content
            let content = std::fs::read_to_string(path).map_err(|e| ToolError::Io(e))?;

            // Simple pattern matching for demonstration
            // In a real implementation, this would use ast-grep-core properly
            if self.simple_pattern_match(&content, pattern) {
                let lines: Vec<&str> = content.lines().collect();

                // Find the matching line(s)
                for (line_idx, line) in lines.iter().enumerate() {
                    if line.contains(&pattern.replace("$_", "")) {
                        // Simple pattern matching
                        let line_num = (line_idx + 1) as u32;

                        // Extract context
                        let context_before = if line_idx > 0 {
                            lines[line_idx.saturating_sub(3)..line_idx]
                                .iter()
                                .map(|s| s.to_string())
                                .collect()
                        } else {
                            Vec::new()
                        };

                        let context_after = if line_idx < lines.len() - 1 {
                            lines[line_idx + 1..std::cmp::min(line_idx + 4, lines.len())]
                                .iter()
                                .map(|s| s.to_string())
                                .collect()
                        } else {
                            Vec::new()
                        };

                        matches.push(AgMatch {
                            file: path.clone(),
                            line: line_num,
                            column: 1, // Simplified column detection
                            end_line: line_num,
                            end_column: line.len() as u32,
                            matched_text: line.to_string(),
                            context_before,
                            context_after,
                            metavariables: std::collections::HashMap::new(),
                        });
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Simple pattern matching (placeholder for real AST-based matching)
    fn simple_pattern_match(&self, content: &str, pattern: &str) -> bool {
        // This is a very simplified pattern matcher
        // In a real implementation, this would use proper AST parsing
        let simplified_pattern = pattern.replace("$_", "").replace("($_)", "()");
        content.contains(&simplified_pattern)
    }

    /// Search using YAML rule (placeholder implementation)
    fn search_with_rule(
        &self,
        _yaml_rule: &str,
        _paths: &[PathBuf],
    ) -> Result<Vec<AgMatch>, ToolError> {
        // YAML rule support would require parsing YAML and converting to ast-grep rules
        // This is a complex feature that would need proper implementation
        Err(ToolError::NotImplemented(
            "YAML rule support - use simple patterns instead",
        ))
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> usize {
        self.result_cache.len()
    }

    /// Clear all caches
    pub fn clear_cache(&self) {
        self.result_cache.clear();
    }
}

impl CodeTool for AstGrep {
    type Query = AgQuery;
    type Output = Vec<AgMatch>;

    fn search(&self, query: Self::Query) -> Result<Self::Output, ToolError> {
        match query {
            AgQuery::Pattern {
                language,
                pattern,
                paths,
            } => {
                // Generate cache key
                let cache_key = format!(
                    "{}:{}:{}",
                    language.as_deref().unwrap_or("auto"),
                    pattern,
                    paths.len()
                );

                // Check cache first
                if let Some(cached) = self.result_cache.get(&cache_key) {
                    return Ok(cached.clone());
                }

                // Determine language from first file if not specified
                let lang = if let Some(lang_str) = language.as_deref() {
                    self.parse_language_string(lang_str)?
                } else if let Some(first_path) = paths.first() {
                    self.detect_language(None, first_path)?
                } else {
                    return Err(ToolError::InvalidQuery(
                        "No language specified and no files provided".to_string(),
                    ));
                };

                // Search files
                let results = self.search_with_pattern(&pattern, lang, &paths)?;

                // Cache results
                self.result_cache.insert(cache_key, results.clone());

                Ok(results)
            }
            AgQuery::Rule { yaml_rule, paths } => self.search_with_rule(&yaml_rule, &paths),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_language_detection() {
        let ast_grep = AstGrep::new();

        // Test detection from file extension
        assert_eq!(
            ast_grep
                .detect_language(None, Path::new("test.rs"))
                .unwrap(),
            SupportLang::Rust
        );
        assert_eq!(
            ast_grep
                .detect_language(None, Path::new("test.py"))
                .unwrap(),
            SupportLang::Python
        );
        assert_eq!(
            ast_grep
                .detect_language(None, Path::new("test.js"))
                .unwrap(),
            SupportLang::JavaScript
        );

        // Test detection from language hint
        assert_eq!(
            ast_grep
                .detect_language(Some("rust"), Path::new("unknown.txt"))
                .unwrap(),
            SupportLang::Rust
        );
    }

    #[test]
    fn test_simple_pattern_matching() {
        let ast_grep = AstGrep::new();

        // Test simple pattern matching
        assert!(ast_grep.simple_pattern_match("console.log('hello')", "console.log"));
        assert!(ast_grep.simple_pattern_match("println!(\"test\")", "println!"));
        assert!(!ast_grep.simple_pattern_match("print('test')", "console.log"));
    }

    #[test]
    fn test_search_pattern() {
        let ast_grep = AstGrep::new();
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.js");

        // Create test JavaScript file
        fs::write(
            &file_path,
            r#"
function test() {
    console.log("hello");
    console.error("error");
    alert("world");
}
"#,
        )
        .unwrap();

        // Search for console.log pattern
        let query = AgQuery::Pattern {
            language: Some("javascript".to_string()),
            pattern: "console.log".to_string(),
            paths: vec![file_path.clone()],
        };

        let results = ast_grep.search(query).unwrap();
        assert!(!results.is_empty());

        let match_result = &results[0];
        assert_eq!(match_result.file, file_path);
        assert!(match_result.matched_text.contains("console.log"));
    }

    #[test]
    fn test_unsupported_language() {
        let ast_grep = AstGrep::new();

        let result = ast_grep.detect_language(None, Path::new("test.unknown"));
        assert!(result.is_err());

        if let Err(ToolError::UnsupportedLanguage(_)) = result {
            // Expected error type
        } else {
            panic!("Expected UnsupportedLanguage error");
        }
    }

    #[test]
    fn test_yaml_rule_placeholder() {
        let ast_grep = AstGrep::new();
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");

        fs::write(&file_path, "fn main() {}").unwrap();

        let query = AgQuery::Rule {
            yaml_rule: "id: test\nlanguage: rust\nrule:\n  pattern: 'fn $_() {}'".to_string(),
            paths: vec![file_path],
        };

        let result = ast_grep.search(query);
        assert!(result.is_err());

        if let Err(ToolError::NotImplemented(_)) = result {
            // Expected - YAML rules are not yet implemented
        } else {
            panic!("Expected NotImplemented error for YAML rules");
        }
    }

    #[test]
    fn test_cache_management() {
        let ast_grep = AstGrep::new();
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");

        fs::write(&file_path, "fn main() {}").unwrap();

        // First search
        let query1 = AgQuery::Pattern {
            language: Some("rust".to_string()),
            pattern: "fn".to_string(),
            paths: vec![file_path.clone()],
        };

        let _ = ast_grep.search(query1).unwrap();
        assert_eq!(ast_grep.cache_stats(), 1);

        // Second search with same parameters should use cache
        let query2 = AgQuery::Pattern {
            language: Some("rust".to_string()),
            pattern: "fn".to_string(),
            paths: vec![file_path],
        };

        let _ = ast_grep.search(query2).unwrap();
        assert_eq!(ast_grep.cache_stats(), 1); // Still 1 because cached

        // Clear cache
        ast_grep.clear_cache();
        assert_eq!(ast_grep.cache_stats(), 0);
    }

    #[test]
    fn test_language_conversion() {
        let ast_grep = AstGrep::new();

        assert_eq!(ast_grep.language_to_string(SupportLang::Rust), "rust");
        assert_eq!(ast_grep.language_to_string(SupportLang::Python), "python");
        assert_eq!(
            ast_grep.language_to_string(SupportLang::JavaScript),
            "javascript"
        );
    }
}
