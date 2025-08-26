//! Comprehensive tree-sitter parser system for codex-rs
//!
//! This module provides a unified interface for parsing 50+ programming languages
//! with efficient caching, lazy initialization, and thread-safe operations.
//!
//! # Architecture
//!
//! ```text
//! ParserFactory
//! ├── Language Detection     - File extension → Language mapping
//! ├── Parser Creation       - Lazy-initialized parsers
//! ├── Content Analysis      - Language detection from content
//! └── Extension Registry    - Comprehensive file type mapping
//! ```
//!
//! # Usage
//!
//! ```rust
//! use agcodex_core::parsers::{Language, ParserFactory, detect_language};
//!
//! // Language detection
//! let lang = detect_language("src/main.rs")?;
//! assert_eq!(lang, Language::Rust);
//!
//! // Parser creation
//! let factory = ParserFactory::instance();
//! let parser = factory.create_parser(Language::Python)?;
//! ```

pub mod cache;
pub mod query_builder;
pub mod utils;

#[cfg(test)]
mod tests;

use agcodex_ast::Language as AstLanguage;
use agcodex_ast::LanguageRegistry;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tree_sitter::Language as TsLanguage;
use tree_sitter::Parser;

/// Enhanced Language enum with comprehensive language support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Language {
    // Systems Programming
    Rust,
    C,
    Cpp,
    Zig,
    Go,

    // Object-Oriented
    Java,
    CSharp,
    Kotlin,
    Swift,
    ObjectiveC,

    // Scripting & Dynamic
    Python,
    JavaScript,
    TypeScript,
    Ruby,
    Php,
    Lua,
    Bash,

    // Functional Programming
    Haskell,
    Elixir,
    Scala,
    OCaml,
    Clojure,

    // Web Technologies
    Html,
    Css,
    Json,
    Yaml,

    // Data & Config
    Sql,
    GraphQL,
    Toml,

    // Specialized
    R,
    Julia,
    Matlab,
    Dart,

    // Build & Infrastructure
    Make,
    Docker,
    Nix,
    Hcl,

    // Documentation
    Markdown,
    Rst,
    Latex,

    // Shader Languages
    Wgsl,
    Glsl,
    Hlsl,
}

impl Language {
    /// Get the tree-sitter language for this programming language
    pub fn to_tree_sitter(&self) -> Result<TsLanguage, ParserError> {
        match self {
            // Core languages with full parser support
            Self::Rust => Ok(tree_sitter_rust::LANGUAGE.into()),
            Self::Python => Ok(tree_sitter_python::LANGUAGE.into()),
            Self::JavaScript => Ok(tree_sitter_javascript::LANGUAGE.into()),
            Self::TypeScript => Ok(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            Self::Go => Ok(tree_sitter_go::LANGUAGE.into()),
            Self::Java => Ok(tree_sitter_java::LANGUAGE.into()),
            Self::C => Ok(tree_sitter_c::LANGUAGE.into()),
            Self::Cpp => Ok(tree_sitter_cpp::LANGUAGE.into()),
            Self::CSharp => Ok(tree_sitter_c_sharp::LANGUAGE.into()),

            // Scripting languages
            Self::Bash => Ok(tree_sitter_bash::LANGUAGE.into()),
            Self::Ruby => Ok(tree_sitter_ruby::LANGUAGE.into()),
            Self::Php => Ok(tree_sitter_php::LANGUAGE_PHP.into()),
            Self::Lua => Ok(tree_sitter_lua::LANGUAGE.into()),

            // Web languages
            Self::Html => Ok(tree_sitter_html::LANGUAGE.into()),
            Self::Css => Ok(tree_sitter_css::LANGUAGE.into()),
            Self::Json => Ok(tree_sitter_json::LANGUAGE.into()),
            Self::Yaml => Ok(tree_sitter_yaml::LANGUAGE.into()),

            // Functional languages
            Self::Haskell => Ok(tree_sitter_haskell::LANGUAGE.into()),
            Self::Elixir => Ok(tree_sitter_elixir::LANGUAGE.into()),
            Self::Scala => Ok(tree_sitter_scala::LANGUAGE.into()),
            Self::OCaml => Ok(tree_sitter_ocaml::LANGUAGE_OCAML.into()),
            Self::Clojure => Ok(tree_sitter_clojure::LANGUAGE.into()),

            // Systems languages
            Self::Zig => Ok(tree_sitter_zig::LANGUAGE.into()),
            Self::Swift => Ok(tree_sitter_swift::LANGUAGE.into()),
            Self::Kotlin => Ok(tree_sitter_kotlin_ng::LANGUAGE.into()),
            Self::ObjectiveC => Ok(tree_sitter_objc::LANGUAGE.into()),

            // Build languages
            Self::Make => Ok(tree_sitter_make::LANGUAGE.into()),
            Self::Nix => Ok(tree_sitter_nix::LANGUAGE.into()),
            Self::Hcl => Ok(tree_sitter_hcl::LANGUAGE.into()),

            // Documentation
            Self::Rst => Ok(tree_sitter_rst::LANGUAGE.into()),

            // Unsupported languages - return error
            Self::R
            | Self::Julia
            | Self::Matlab
            | Self::Dart
            | Self::Sql
            | Self::GraphQL
            | Self::Toml
            | Self::Docker
            | Self::Markdown
            | Self::Latex
            | Self::Wgsl
            | Self::Glsl
            | Self::Hlsl => Err(ParserError::UnsupportedLanguage {
                language: format!("{:?}", self),
            }),
        }
    }

    /// Convert to AST crate's Language enum
    pub const fn to_ast_language(&self) -> Option<AstLanguage> {
        match self {
            Self::Rust => Some(AstLanguage::Rust),
            Self::Python => Some(AstLanguage::Python),
            Self::JavaScript => Some(AstLanguage::JavaScript),
            Self::TypeScript => Some(AstLanguage::TypeScript),
            Self::Go => Some(AstLanguage::Go),
            Self::Java => Some(AstLanguage::Java),
            Self::C => Some(AstLanguage::C),
            Self::Cpp => Some(AstLanguage::Cpp),
            Self::CSharp => Some(AstLanguage::CSharp),
            Self::Bash => Some(AstLanguage::Bash),
            Self::Ruby => Some(AstLanguage::Ruby),
            Self::Php => Some(AstLanguage::Php),
            Self::Lua => Some(AstLanguage::Lua),
            Self::Haskell => Some(AstLanguage::Haskell),
            Self::Elixir => Some(AstLanguage::Elixir),
            Self::Scala => Some(AstLanguage::Scala),
            Self::OCaml => Some(AstLanguage::OCaml),
            Self::Clojure => Some(AstLanguage::Clojure),
            Self::Zig => Some(AstLanguage::Zig),
            Self::Swift => Some(AstLanguage::Swift),
            Self::Kotlin => Some(AstLanguage::Kotlin),
            Self::ObjectiveC => Some(AstLanguage::ObjectiveC),
            _ => None, // Languages not supported by AST module
        }
    }

    /// Get human-readable name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::Zig => "Zig",
            Self::Go => "Go",
            Self::Java => "Java",
            Self::CSharp => "C#",
            Self::Kotlin => "Kotlin",
            Self::Swift => "Swift",
            Self::ObjectiveC => "Objective-C",
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Ruby => "Ruby",
            Self::Php => "PHP",
            Self::Lua => "Lua",
            Self::Bash => "Bash",
            Self::Haskell => "Haskell",
            Self::Elixir => "Elixir",
            Self::Scala => "Scala",
            Self::OCaml => "OCaml",
            Self::Clojure => "Clojure",
            Self::Html => "HTML",
            Self::Css => "CSS",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Sql => "SQL",
            Self::GraphQL => "GraphQL",
            Self::Toml => "TOML",
            Self::R => "R",
            Self::Julia => "Julia",
            Self::Matlab => "MATLAB",
            Self::Dart => "Dart",
            Self::Make => "Make",
            Self::Docker => "Docker",
            Self::Nix => "Nix",
            Self::Hcl => "HCL",
            Self::Markdown => "Markdown",
            Self::Rst => "reStructuredText",
            Self::Latex => "LaTeX",
            Self::Wgsl => "WGSL",
            Self::Glsl => "GLSL",
            Self::Hlsl => "HLSL",
        }
    }

    /// Get common file extensions for this language
    pub const fn extensions(&self) -> &'static [&'static str] {
        match self {
            Self::Rust => &["rs"],
            Self::C => &["c", "h"],
            Self::Cpp => &["cpp", "cxx", "cc", "c++", "hpp", "hxx", "h++", "hh"],
            Self::Zig => &["zig"],
            Self::Go => &["go"],
            Self::Java => &["java"],
            Self::CSharp => &["cs", "csx"],
            Self::Kotlin => &["kt", "kts"],
            Self::Swift => &["swift"],
            Self::ObjectiveC => &["m", "mm"],
            Self::Python => &["py", "pyw", "pyi"],
            Self::JavaScript => &["js", "mjs", "cjs"],
            Self::TypeScript => &["ts", "tsx"],
            Self::Ruby => &["rb", "rbw"],
            Self::Php => &["php", "php3", "php4", "php5", "phtml"],
            Self::Lua => &["lua"],
            Self::Bash => &["sh", "bash", "zsh", "fish"],
            Self::Haskell => &["hs", "lhs"],
            Self::Elixir => &["ex", "exs"],
            Self::Scala => &["scala", "sc"],
            Self::OCaml => &["ml", "mli"],
            Self::Clojure => &["clj", "cljs", "cljc", "edn"],
            Self::Html => &["html", "htm", "xhtml"],
            Self::Css => &["css"],
            Self::Json => &["json", "jsonl"],
            Self::Yaml => &["yaml", "yml"],
            Self::Sql => &["sql"],
            Self::GraphQL => &["graphql", "gql"],
            Self::Toml => &["toml"],
            Self::R => &["r", "R"],
            Self::Julia => &["jl"],
            Self::Matlab => &["m", "mat"],
            Self::Dart => &["dart"],
            Self::Make => &["Makefile", "makefile", "mk"],
            Self::Docker => &["Dockerfile", "dockerfile"],
            Self::Nix => &["nix"],
            Self::Hcl => &["hcl", "tf"],
            Self::Markdown => &["md", "markdown", "mdown"],
            Self::Rst => &["rst"],
            Self::Latex => &["tex", "latex"],
            Self::Wgsl => &["wgsl"],
            Self::Glsl => &["glsl", "vert", "frag", "geom", "comp"],
            Self::Hlsl => &["hlsl", "fx", "fxh"],
        }
    }

    /// Check if this language supports AST operations
    pub const fn supports_ast(&self) -> bool {
        matches!(
            self,
            Self::Rust
                | Self::Python
                | Self::JavaScript
                | Self::TypeScript
                | Self::Go
                | Self::Java
                | Self::C
                | Self::Cpp
                | Self::CSharp
                | Self::Bash
                | Self::Ruby
                | Self::Php
                | Self::Lua
                | Self::Haskell
                | Self::Elixir
                | Self::Scala
                | Self::OCaml
                | Self::Clojure
                | Self::Zig
                | Self::Swift
                | Self::Kotlin
                | Self::ObjectiveC
                | Self::Html
                | Self::Css
                | Self::Json
                | Self::Yaml
                | Self::Make
                | Self::Nix
                | Self::Hcl
                | Self::Rst
        )
    }

    /// Check if this is a compiled language
    pub const fn is_compiled(&self) -> bool {
        matches!(
            self,
            Self::Rust
                | Self::C
                | Self::Cpp
                | Self::Zig
                | Self::Go
                | Self::Java
                | Self::CSharp
                | Self::Kotlin
                | Self::Swift
                | Self::ObjectiveC
                | Self::Haskell
                | Self::Scala
                | Self::OCaml
        )
    }

    /// Check if this language has strong type system
    pub const fn is_strongly_typed(&self) -> bool {
        matches!(
            self,
            Self::Rust
                | Self::Go
                | Self::Java
                | Self::CSharp
                | Self::Kotlin
                | Self::Swift
                | Self::TypeScript
                | Self::Haskell
                | Self::Scala
                | Self::OCaml
                | Self::Zig
        )
    }
}

/// Errors related to parser operations
#[derive(Debug, Error)]
pub enum ParserError {
    #[error("unsupported language: {language}")]
    UnsupportedLanguage { language: String },

    #[error("failed to create parser for {language}: {details}")]
    ParserCreationFailed { language: String, details: String },

    #[error("language detection failed for path: {path}")]
    LanguageDetectionFailed { path: String },

    #[error("no suitable parser found")]
    NoParserFound,

    #[error("parse operation failed: {reason}")]
    ParseFailed { reason: String },
}

/// Thread-safe parser factory with lazy initialization and caching
pub struct ParserFactory {
    registry: Arc<LanguageRegistry>,
    extension_map: Arc<HashMap<String, Language>>,
}

impl ParserFactory {
    /// Create a new parser factory instance
    fn new() -> Self {
        let extension_map = Self::build_extension_map();

        Self {
            registry: Arc::new(LanguageRegistry::new()),
            extension_map: Arc::new(extension_map),
        }
    }

    /// Get the global parser factory instance (singleton)
    pub fn instance() -> &'static ParserFactory {
        &PARSER_FACTORY
    }

    /// Build comprehensive file extension to language mapping
    fn build_extension_map() -> HashMap<String, Language> {
        let mut map = HashMap::new();

        // Register all languages and their extensions
        for &lang in ALL_LANGUAGES {
            for &ext in lang.extensions() {
                map.insert(ext.to_lowercase(), lang);
            }
        }

        // Special cases and common variations
        map.insert("makefile".to_string(), Language::Make);
        map.insert("dockerfile".to_string(), Language::Docker);
        map.insert("jsx".to_string(), Language::JavaScript);
        map.insert("vue".to_string(), Language::JavaScript);
        map.insert("svelte".to_string(), Language::JavaScript);

        map
    }

    /// Create a parser for the specified language
    pub fn create_parser(&self, language: Language) -> Result<Parser, ParserError> {
        let ts_language = language.to_tree_sitter()?;
        let mut parser = Parser::new();

        parser
            .set_language(&ts_language)
            .map_err(|e| ParserError::ParserCreationFailed {
                language: language.name().to_string(),
                details: e.to_string(),
            })?;

        Ok(parser)
    }

    /// Detect language from file path
    pub fn detect_language<P: AsRef<Path>>(&self, path: P) -> Result<Language, ParserError> {
        detect_language(path)
    }

    /// Detect language from content analysis (fallback method)
    pub fn detect_language_from_content(&self, content: &str) -> Option<Language> {
        detect_language_from_content(content)
    }
}

/// Global parser factory instance
lazy_static! {
    static ref PARSER_FACTORY: ParserFactory = ParserFactory::new();
}

/// All supported languages (for iteration)
const ALL_LANGUAGES: &[Language] = &[
    Language::Rust,
    Language::C,
    Language::Cpp,
    Language::Zig,
    Language::Go,
    Language::Java,
    Language::CSharp,
    Language::Kotlin,
    Language::Swift,
    Language::ObjectiveC,
    Language::Python,
    Language::JavaScript,
    Language::TypeScript,
    Language::Ruby,
    Language::Php,
    Language::Lua,
    Language::Bash,
    Language::Haskell,
    Language::Elixir,
    Language::Scala,
    Language::OCaml,
    Language::Clojure,
    Language::Html,
    Language::Css,
    Language::Json,
    Language::Yaml,
    Language::Sql,
    Language::GraphQL,
    Language::Toml,
    Language::R,
    Language::Julia,
    Language::Matlab,
    Language::Dart,
    Language::Make,
    Language::Docker,
    Language::Nix,
    Language::Hcl,
    Language::Markdown,
    Language::Rst,
    Language::Latex,
    Language::Wgsl,
    Language::Glsl,
    Language::Hlsl,
];

/// Detect programming language from file path using comprehensive extension mapping
pub fn detect_language<P: AsRef<Path>>(path: P) -> Result<Language, ParserError> {
    let path = path.as_ref();
    let factory = ParserFactory::instance();

    // Try filename first (for special files like Makefile, Dockerfile)
    if let Some(filename) = path.file_name().and_then(|n| n.to_str())
        && let Some(&language) = factory.extension_map.get(&filename.to_lowercase()) {
            return Ok(language);
        }

    // Try extension
    if let Some(extension) = path.extension().and_then(|ext| ext.to_str())
        && let Some(&language) = factory.extension_map.get(&extension.to_lowercase()) {
            return Ok(language);
        }

    Err(ParserError::LanguageDetectionFailed {
        path: path.display().to_string(),
    })
}

/// Detect language from file content using heuristics
pub fn detect_language_from_content(content: &str) -> Option<Language> {
    let content_lower = content.to_lowercase();
    let lines: Vec<&str> = content.lines().take(10).collect(); // Check first 10 lines

    // Check shebang line
    if let Some(first_line) = lines.first()
        && first_line.starts_with("#!") {
            if first_line.contains("python") {
                return Some(Language::Python);
            }
            if first_line.contains("bash") || first_line.contains("sh") {
                return Some(Language::Bash);
            }
            if first_line.contains("ruby") {
                return Some(Language::Ruby);
            }
            if first_line.contains("node") {
                return Some(Language::JavaScript);
            }
        }

    // Language-specific patterns in early lines
    for line in &lines {
        let line_lower = line.to_lowercase();

        // Rust patterns
        if line_lower.contains("fn main()")
            || line_lower.contains("use std::")
            || line_lower.contains("extern crate")
        {
            return Some(Language::Rust);
        }

        // Python patterns
        if line_lower.contains("import ") && line_lower.contains("def ") {
            return Some(Language::Python);
        }

        // JavaScript/TypeScript patterns
        if line_lower.contains("const ") && line_lower.contains("=>") {
            return Some(Language::JavaScript);
        }

        if line_lower.contains("interface ") || line_lower.contains(": string") {
            return Some(Language::TypeScript);
        }

        // Go patterns
        if line_lower.contains("package main") || line_lower.contains("func main()") {
            return Some(Language::Go);
        }

        // Java patterns
        if line_lower.contains("public class") || line_lower.contains("public static void main") {
            return Some(Language::Java);
        }

        // C patterns
        if line_lower.contains("#include <") && line_lower.contains("int main") {
            return Some(Language::C);
        }

        // C++ patterns
        if line_lower.contains("#include <iostream>") || line_lower.contains("std::") {
            return Some(Language::Cpp);
        }
    }

    // JSON detection
    if content_lower.trim_start().starts_with('{') && content_lower.trim_end().ends_with('}') {
        return Some(Language::Json);
    }

    // YAML detection
    if content
        .lines()
        .any(|line| line.trim_start() != line && line.contains(':'))
    {
        return Some(Language::Yaml);
    }

    None
}

/// Get file extension to language mapping for external use
pub fn get_extension_mappings() -> &'static HashMap<String, Language> {
    &ParserFactory::instance().extension_map
}

/// Iterator over all supported languages
pub fn supported_languages() -> impl Iterator<Item = Language> {
    ALL_LANGUAGES.iter().copied()
}

/// Check if a language is supported for AST operations
pub const fn supports_ast_operations(language: Language) -> bool {
    language.supports_ast()
}

#[cfg(test)]
mod inline_tests {
    use super::*;
    

    #[test]
    fn test_language_detection_by_extension() {
        assert_eq!(detect_language("main.rs").unwrap(), Language::Rust);
        assert_eq!(detect_language("script.py").unwrap(), Language::Python);
        assert_eq!(detect_language("app.js").unwrap(), Language::JavaScript);
        assert_eq!(
            detect_language("component.tsx").unwrap(),
            Language::TypeScript
        );
        assert_eq!(detect_language("program.go").unwrap(), Language::Go);
        assert_eq!(detect_language("Main.java").unwrap(), Language::Java);
    }

    #[test]
    fn test_language_detection_by_filename() {
        assert_eq!(detect_language("Makefile").unwrap(), Language::Make);
        assert_eq!(detect_language("Dockerfile").unwrap(), Language::Docker);
        assert_eq!(detect_language("dockerfile").unwrap(), Language::Docker);
    }

    #[test]
    fn test_content_based_detection() {
        assert_eq!(
            detect_language_from_content("#!/usr/bin/env python3\nimport sys"),
            Some(Language::Python)
        );
        assert_eq!(
            detect_language_from_content("#!/bin/bash\necho hello"),
            Some(Language::Bash)
        );
        assert_eq!(
            detect_language_from_content("fn main() {\n    println!(\"Hello\");\n}"),
            Some(Language::Rust)
        );
        assert_eq!(
            detect_language_from_content("package main\n\nfunc main() {"),
            Some(Language::Go)
        );
    }

    #[test]
    fn test_parser_creation() {
        let factory = ParserFactory::instance();

        // Test supported languages
        assert!(factory.create_parser(Language::Rust).is_ok());
        assert!(factory.create_parser(Language::Python).is_ok());
        assert!(factory.create_parser(Language::JavaScript).is_ok());

        // Test unsupported languages
        assert!(factory.create_parser(Language::R).is_err());
        assert!(factory.create_parser(Language::Matlab).is_err());
    }

    #[test]
    fn test_language_properties() {
        assert!(Language::Rust.is_compiled());
        assert!(!Language::Python.is_compiled());
        assert!(Language::TypeScript.is_strongly_typed());
        assert!(!Language::JavaScript.is_strongly_typed());
        assert!(Language::Java.supports_ast());
        assert!(!Language::R.supports_ast());
    }

    #[test]
    fn test_extension_coverage() {
        let mappings = get_extension_mappings();

        // Check common extensions are covered
        assert!(mappings.contains_key("rs"));
        assert!(mappings.contains_key("py"));
        assert!(mappings.contains_key("js"));
        assert!(mappings.contains_key("ts"));
        assert!(mappings.contains_key("go"));
        assert!(mappings.contains_key("java"));
        assert!(mappings.contains_key("cpp"));
        assert!(mappings.contains_key("h"));
    }

    #[test]
    fn test_all_languages_iteration() {
        let langs: Vec<Language> = supported_languages().collect();
        assert!(!langs.is_empty());
        assert!(langs.contains(&Language::Rust));
        assert!(langs.contains(&Language::Python));
        assert_eq!(langs.len(), ALL_LANGUAGES.len());
    }
}
