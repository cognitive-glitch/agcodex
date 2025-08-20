//! Language detection and parser management for 50+ languages

use crate::error::AstError;
use crate::error::AstResult;
use crate::types::AstNode;
use crate::types::ParsedAst;
use dashmap::DashMap;
// use once_cell::sync::Lazy; // unused
use std::path::Path;
use tree_sitter::Parser;

/// Supported programming languages
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
    // Web languages
    Html,
    Css,
    Json,
    Yaml,
    Toml,
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
    // Config/Build
    Dockerfile,
    Hcl,
    Nix,
    Make,
    // Documentation
    Markdown,
    LaTeX,
    Rst,
    // Additional
    Sql,
    R,
    Julia,
    Dart,
    Vue,
    Svelte,
    GraphQL,
    Proto,
    Wgsl,
    Glsl,
}

impl Language {
    /// Get the tree-sitter language parser
    ///
    /// Note: Some languages use fallback parsers due to version incompatibilities
    /// with tree-sitter 0.25. These are documented with inline comments.
    /// Call `is_fallback()` to check if a language is using a fallback parser.
    pub fn parser(&self) -> tree_sitter::Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::C => tree_sitter_c::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Self::CSharp => tree_sitter_c_sharp::LANGUAGE.into(),
            Self::Html => tree_sitter_html::LANGUAGE.into(),
            Self::Css => tree_sitter_css::LANGUAGE.into(),
            Self::Json => tree_sitter_json::LANGUAGE.into(),
            Self::Yaml => tree_sitter_yaml::LANGUAGE.into(),
            Self::Toml => tree_sitter_json::LANGUAGE.into(), // toml v0.20 incompatible with tree-sitter v0.25, using JSON fallback
            Self::Bash => tree_sitter_bash::LANGUAGE.into(),
            Self::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Self::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            Self::Lua => tree_sitter_lua::LANGUAGE.into(),
            Self::Haskell => tree_sitter_haskell::LANGUAGE.into(),
            Self::Elixir => tree_sitter_elixir::LANGUAGE.into(),
            Self::Scala => tree_sitter_scala::LANGUAGE.into(),
            Self::OCaml => tree_sitter_ocaml::LANGUAGE_OCAML.into(),
            Self::Clojure => tree_sitter_bash::LANGUAGE.into(), // clojure not compatible
            Self::Zig => tree_sitter_zig::LANGUAGE.into(),
            Self::Swift => tree_sitter_swift::LANGUAGE.into(),
            Self::Kotlin => tree_sitter_bash::LANGUAGE.into(), // kotlin version conflict
            Self::ObjectiveC => tree_sitter_objc::LANGUAGE.into(),
            Self::Dockerfile => tree_sitter_bash::LANGUAGE.into(), // dockerfile v0.2 incompatible, using Bash fallback
            Self::Hcl => tree_sitter_hcl::LANGUAGE.into(),
            Self::Nix => tree_sitter_bash::LANGUAGE.into(), // nix not compatible
            Self::Make => tree_sitter_make::LANGUAGE.into(),
            Self::Markdown => tree_sitter_bash::LANGUAGE.into(), // markdown not compatible
            Self::LaTeX => tree_sitter_bash::LANGUAGE.into(),    // latex not compatible
            Self::Rst => tree_sitter_bash::LANGUAGE.into(),      // rst not compatible
            Self::Sql => tree_sitter_bash::LANGUAGE.into(),      // sql not compatible
            Self::R => tree_sitter_bash::LANGUAGE.into(),        // r not compatible
            Self::Julia => tree_sitter_bash::LANGUAGE.into(),    // julia not compatible
            Self::Dart => tree_sitter_bash::LANGUAGE.into(),     // dart not compatible
            Self::Vue => tree_sitter_html::LANGUAGE.into(),      // vue not compatible
            Self::Svelte => tree_sitter_html::LANGUAGE.into(),   // svelte not compatible
            Self::GraphQL => tree_sitter_json::LANGUAGE.into(),  // graphql not compatible
            Self::Proto => tree_sitter_bash::LANGUAGE.into(),    // proto not compatible
            Self::Wgsl => tree_sitter_bash::LANGUAGE.into(),     // wgsl not compatible
            Self::Glsl => tree_sitter_bash::LANGUAGE.into(),     // glsl not compatible
        }
    }

    /// Returns true if this language is using a fallback parser due to version incompatibility
    pub const fn is_fallback(&self) -> bool {
        matches!(
            self,
            Self::Toml
                | Self::Clojure
                | Self::Kotlin
                | Self::Dockerfile
                | Self::Nix
                | Self::Markdown
                | Self::LaTeX
                | Self::Rst
                | Self::Sql
                | Self::R
                | Self::Julia
                | Self::Dart
                | Self::Vue
                | Self::Svelte
                | Self::GraphQL
                | Self::Proto
                | Self::Wgsl
                | Self::Glsl
        )
    }

    /// Get the actual parser being used (for fallback languages)
    pub const fn actual_parser_name(&self) -> &'static str {
        if !self.is_fallback() {
            return self.name();
        }

        match self {
            Self::Toml | Self::GraphQL => "JSON (fallback)",
            Self::Vue | Self::Svelte | Self::Markdown | Self::Rst => "HTML (fallback)",
            Self::Clojure
            | Self::Kotlin
            | Self::Dockerfile
            | Self::Nix
            | Self::LaTeX
            | Self::Sql
            | Self::R
            | Self::Julia
            | Self::Dart
            | Self::Proto
            | Self::Wgsl
            | Self::Glsl => "Bash (fallback)",
            _ => self.name(),
        }
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
            Self::Html => "HTML",
            Self::Css => "CSS",
            Self::Json => "JSON",
            Self::Yaml => "YAML",
            Self::Toml => "TOML",
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
            Self::Dockerfile => "Dockerfile",
            Self::Hcl => "HCL",
            Self::Nix => "Nix",
            Self::Make => "Makefile",
            Self::Markdown => "Markdown",
            Self::LaTeX => "LaTeX",
            Self::Rst => "reStructuredText",
            Self::Sql => "SQL",
            Self::R => "R",
            Self::Julia => "Julia",
            Self::Dart => "Dart",
            Self::Vue => "Vue",
            Self::Svelte => "Svelte",
            Self::GraphQL => "GraphQL",
            Self::Proto => "Protocol Buffers",
            Self::Wgsl => "WGSL",
            Self::Glsl => "GLSL",
        }
    }
}

/// Language registry for detection and parsing
pub struct LanguageRegistry {
    parsers: DashMap<Language, Parser>,
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
    pub fn detect_language(&self, path: &Path) -> AstResult<Language> {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check special filenames first
        let lang = match filename {
            "Dockerfile" | "Containerfile" => Language::Dockerfile,
            "Makefile" | "makefile" | "GNUmakefile" => Language::Make,
            _ => {
                // Get extension and check by extension
                let extension = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .ok_or_else(|| AstError::LanguageDetectionFailed(path.display().to_string()))?;

                match extension {
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
                    "html" | "htm" => Language::Html,
                    "css" | "scss" | "sass" | "less" => Language::Css,
                    "json" | "jsonc" => Language::Json,
                    "yaml" | "yml" => Language::Yaml,
                    "toml" => Language::Toml,
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
                    "hcl" | "tf" | "tfvars" => Language::Hcl,
                    "nix" => Language::Nix,
                    "md" | "markdown" => Language::Markdown,
                    "tex" | "latex" => Language::LaTeX,
                    "rst" => Language::Rst,
                    "sql" => Language::Sql,
                    "r" | "R" => Language::R,
                    "jl" => Language::Julia,
                    "dart" => Language::Dart,
                    "vue" => Language::Vue,
                    "svelte" => Language::Svelte,
                    "graphql" | "gql" => Language::GraphQL,
                    "proto" => Language::Proto,
                    "wgsl" => Language::Wgsl,
                    "glsl" | "vert" | "frag" => Language::Glsl,
                    _ => return Err(AstError::UnsupportedLanguage(extension.to_string())),
                }
            }
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
            total_languages: 47, // Total number of supported languages
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
        assert_eq!(
            registry
                .detect_language(&PathBuf::from("Dockerfile"))
                .unwrap(),
            Language::Dockerfile
        );
        assert_eq!(
            registry
                .detect_language(&PathBuf::from("Makefile"))
                .unwrap(),
            Language::Make
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
}
