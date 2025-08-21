//! Language detection and parser management for programming languages.
//!
//! Note: Configuration and markup languages (TOML, YAML, Markdown, etc.) are not supported
//! for AST-based operations as they don't benefit from AST analysis. Use patch-based editing
//! for those file types instead.

use crate::error::AstError;
use crate::error::AstResult;
use crate::types::AstNode;
use crate::types::ParsedAst;
use dashmap::DashMap;
use std::path::Path;
use tree_sitter::Parser;

/// Supported programming languages for AST-based operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Language {
    // Core languages
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    C,
    Cpp,
    CSharp,
    // Scripting
    Bash,
    Ruby,
    Php,
    Lua,
    // Functional
    Haskell,
    Elixir,
    Scala,
    OCaml,
    Clojure,
    // Systems
    Zig,
    Swift,
    Kotlin,
    ObjectiveC,
    // Statistical/Scientific
    R,
    Julia,
    // Mobile/Web
    Dart,
    // Shader languages
    Wgsl,
    Glsl,
}

impl Language {
    /// Get the tree-sitter language parser
    pub fn parser(&self) -> tree_sitter::Language {
        match self {
            // Core languages with proper parsers
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            // Scripting languages
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
            Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            Self::Lua => tree_sitter_lua::LANGUAGE.into(),
            // Functional languages
            Self::Haskell => tree_sitter_haskell::LANGUAGE.into(),
            Self::Elixir => tree_sitter_elixir::LANGUAGE.into(),
            Self::Scala => tree_sitter_scala::LANGUAGE.into(),
            Self::OCaml => tree_sitter_ocaml::LANGUAGE_OCAML.into(),
            Self::Clojure => tree_sitter_clojure::LANGUAGE.into(),
            // Systems languages
            Self::Zig => tree_sitter_zig::LANGUAGE.into(),
            Self::Swift => tree_sitter_swift::LANGUAGE.into(),
            Self::Kotlin => tree_sitter_kotlin_ng::LANGUAGE.into(),
            Self::ObjectiveC => tree_sitter_objc::LANGUAGE.into(),
            // Languages without crates.io parsers (using fallbacks)
            Self::R => tree_sitter_bash::LANGUAGE.into(), // R parser not available
            Self::Julia => tree_sitter_bash::LANGUAGE.into(), // Julia parser not available
            Self::Dart => tree_sitter_bash::LANGUAGE.into(), // Dart parser not available
            Self::Wgsl => tree_sitter_bash::LANGUAGE.into(), // WGSL parser not available
            Self::Glsl => tree_sitter_bash::LANGUAGE.into(), // GLSL parser not available
        }
    }

    /// Returns true if this language is using a fallback parser
    pub const fn is_fallback(&self) -> bool {
        matches!(
            self,
            Self::R | Self::Julia | Self::Dart | Self::Wgsl | Self::Glsl
        )
    }

    /// Get the actual parser being used (for fallback languages)
    pub const fn actual_parser_name(&self) -> &'static str {
        if !self.is_fallback() {
            return self.name();
        }

        // All current fallbacks use Bash parser
        "Bash (fallback)"
    }

    /// Get language display name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::CSharp => "C#",
            Self::Bash => "Bash",
            Self::Ruby => "Ruby",
            Self::Php => "PHP",
            Self::Lua => "Lua",
            Self::Haskell => "Haskell",
            Self::Elixir => "Elixir",
            Self::Scala => "Scala",
            Self::OCaml => "OCaml",
            Self::Clojure => "Clojure",
            Self::Zig => "Zig",
            Self::Swift => "Swift",
            Self::Kotlin => "Kotlin",
            Self::ObjectiveC => "Objective-C",
            Self::R => "R",
            Self::Julia => "Julia",
            Self::Dart => "Dart",
            Self::Wgsl => "WGSL",
            Self::Glsl => "GLSL",
        }
    }
}

/// Language registry for detection and parsing
pub struct LanguageRegistry {
    parsers: DashMap<Language, Parser>,
}

impl std::fmt::Debug for LanguageRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanguageRegistry")
            .field("parsers_count", &self.parsers.len())
            .finish()
    }
}

impl LanguageRegistry {
    /// Create a new language registry with all parsers initialized
    pub fn new() -> Self {
        let registry = Self {
            parsers: DashMap::new(),
        };

        // Pre-initialize common language parsers
        let common_langs = [
            Language::Rust,
            Language::Python,
            Language::JavaScript,
            Language::TypeScript,
            Language::Go,
            Language::Java,
            Language::C,
            Language::Cpp,
        ];

        for lang in common_langs {
            let _ = registry.get_or_create_parser(lang);
        }

        registry
    }

    /// Detect language from file path
    /// Note: Returns error for config/markup files that should use patch-based editing
    pub fn detect_language(&self, path: &Path) -> AstResult<Language> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| AstError::LanguageDetectionFailed(path.display().to_string()))?;

        let lang = match extension {
            // Programming languages
            "rs" => Language::Rust,
            "py" | "pyi" => Language::Python,
            "js" | "mjs" | "cjs" => Language::JavaScript,
            "ts" | "mts" | "cts" => Language::TypeScript,
            "tsx" | "jsx" => Language::TypeScript,
            "go" => Language::Go,
            "java" => Language::Java,
            "c" | "h" => Language::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "c++" => Language::Cpp,
            "cs" => Language::CSharp,
            "sh" | "bash" | "zsh" | "fish" => Language::Bash,
            "rb" => Language::Ruby,
            "php" => Language::Php,
            "lua" => Language::Lua,
            "hs" | "lhs" => Language::Haskell,
            "ex" | "exs" => Language::Elixir,
            "scala" | "sc" => Language::Scala,
            "ml" | "mli" => Language::OCaml,
            "clj" | "cljs" | "cljc" => Language::Clojure,
            "zig" => Language::Zig,
            "swift" => Language::Swift,
            "kt" | "kts" => Language::Kotlin,
            "m" | "mm" => Language::ObjectiveC,
            "r" | "R" => Language::R,
            "jl" => Language::Julia,
            "dart" => Language::Dart,
            "wgsl" => Language::Wgsl,
            "glsl" | "vert" | "frag" => Language::Glsl,

            // Config/markup files - not supported for AST operations
            "toml" | "yaml" | "yml" | "json" | "jsonc" | "xml" | "html" | "htm" | "css"
            | "scss" | "sass" | "less" | "md" | "markdown" | "tex" | "latex" | "rst" | "sql"
            | "graphql" | "gql" | "proto" | "dockerfile" | "makefile" | "cmake" | "hcl" | "tf"
            | "tfvars" | "nix" => {
                return Err(AstError::UnsupportedLanguage(format!(
                    "{} files should use patch-based editing, not AST operations",
                    extension
                )));
            }

            _ => return Err(AstError::UnsupportedLanguage(extension.to_string())),
        };

        Ok(lang)
    }

    /// Get or create a parser for a language
    fn get_or_create_parser(&self, language: Language) -> Parser {
        // Always create a new parser since Parser doesn't implement Clone
        let mut parser = Parser::new();
        parser.set_language(&language.parser()).unwrap();
        parser
    }

    /// Parse source code for a given language
    pub fn parse(&self, language: &Language, source: &str) -> AstResult<ParsedAst> {
        let mut parser = self.get_or_create_parser(*language);

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| AstError::ParserError("Failed to parse source code".to_string()))?;

        let root = tree.root_node();
        let root_node = AstNode {
            kind: root.kind().to_string(),
            start_byte: root.start_byte(),
            end_byte: root.end_byte(),
            start_position: (root.start_position().row, root.start_position().column),
            end_position: (root.end_position().row, root.end_position().column),
            children_count: root.child_count(),
        };

        Ok(ParsedAst {
            tree,
            source: source.to_string(),
            language: *language,
            root_node,
        })
    }

    /// Get statistics about loaded parsers
    pub fn stats(&self) -> LanguageRegistryStats {
        LanguageRegistryStats {
            loaded_parsers: self.parsers.len(),
            total_languages: 27, // Total number of supported programming languages
        }
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the language registry
#[derive(Debug, Clone)]
pub struct LanguageRegistryStats {
    pub loaded_parsers: usize,
    pub total_languages: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_language_detection() {
        let registry = LanguageRegistry::new();

        // Programming languages should work
        assert_eq!(
            registry.detect_language(&PathBuf::from("test.rs")).unwrap(),
            Language::Rust
        );
        assert_eq!(
            registry.detect_language(&PathBuf::from("test.py")).unwrap(),
            Language::Python
        );
        assert_eq!(
            registry.detect_language(&PathBuf::from("test.js")).unwrap(),
            Language::JavaScript
        );
        assert_eq!(
            registry.detect_language(&PathBuf::from("test.ts")).unwrap(),
            Language::TypeScript
        );

        // Config/markup files should error with helpful message
        assert!(
            registry
                .detect_language(&PathBuf::from("test.toml"))
                .is_err()
        );
        assert!(
            registry
                .detect_language(&PathBuf::from("test.yaml"))
                .is_err()
        );
        assert!(registry.detect_language(&PathBuf::from("test.md")).is_err());
        assert!(
            registry
                .detect_language(&PathBuf::from("Dockerfile"))
                .is_err()
        );
    }

    #[test]
    fn test_parsing() {
        let registry = LanguageRegistry::new();

        let rust_code = r#"
fn main() {
    println!("Hello, world!");
}
"#;

        let ast = registry.parse(&Language::Rust, rust_code).unwrap();
        assert_eq!(ast.language, Language::Rust);
        assert_eq!(ast.root_node.kind, "source_file");
        assert!(ast.root_node.children_count > 0);
    }

    #[test]
    fn test_fallback_detection() {
        assert!(!Language::Rust.is_fallback());
        assert!(!Language::Python.is_fallback());
        assert!(Language::R.is_fallback());
        assert!(Language::Julia.is_fallback());
        assert!(Language::Wgsl.is_fallback());
    }
}
