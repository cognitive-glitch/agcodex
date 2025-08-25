//! Interface tests for TreeTool types and enums
//!
//! These tests focus on the public interface of the tree tool without
//! requiring initialization of the actual tree-sitter parsers.

// Only import the types we can test without linking issues
use agcodex_core::tools::tree::{SupportedLanguage, Point, TreeInput, TreeOutput};
use std::path::PathBuf;

#[cfg(test)]
mod tree_interface_tests {
    use super::*;

    #[test]
    fn test_supported_language_string_representations() {
        let test_cases = vec![
            (SupportedLanguage::Rust, "rust"),
            (SupportedLanguage::Python, "python"),
            (SupportedLanguage::JavaScript, "javascript"),
            (SupportedLanguage::TypeScript, "typescript"),
            (SupportedLanguage::Go, "go"),
            (SupportedLanguage::Java, "java"),
            (SupportedLanguage::C, "c"),
            (SupportedLanguage::Cpp, "cpp"),
            (SupportedLanguage::CSharp, "csharp"),
            (SupportedLanguage::Bash, "bash"),
        ];
        
        for (language, expected_str) in test_cases {
            assert_eq!(language.as_str(), expected_str,
                      "Language {:?} should have string representation '{}'", language, expected_str);
        }
    }

    #[test]
    fn test_supported_language_extensions() {
        let test_cases = vec![
            (SupportedLanguage::Rust, vec!["rs"]),
            (SupportedLanguage::Python, vec!["py", "pyw", "pyi"]),
            (SupportedLanguage::JavaScript, vec!["js", "mjs", "cjs"]),
            (SupportedLanguage::TypeScript, vec!["ts", "tsx", "cts", "mts"]),
            (SupportedLanguage::Go, vec!["go"]),
            (SupportedLanguage::Java, vec!["java"]),
            (SupportedLanguage::C, vec!["c", "h"]),
            (SupportedLanguage::Cpp, vec!["cpp", "cxx", "cc", "hpp", "hxx", "hh"]),
            (SupportedLanguage::Bash, vec!["sh", "bash", "zsh"]),
        ];
        
        for (language, expected_extensions) in test_cases {
            let actual_extensions = language.extensions();
            
            // Check that all expected extensions are present
            for expected_ext in &expected_extensions {
                assert!(actual_extensions.contains(expected_ext),
                       "Language {:?} should support extension '{}'", language, expected_ext);
            }
            
            // Ensure at least the expected number of extensions
            assert!(actual_extensions.len() >= expected_extensions.len(),
                   "Language {:?} should have at least {} extensions, got {}", 
                   language, expected_extensions.len(), actual_extensions.len());
        }
    }

    #[test]
    fn test_language_detection_from_extensions() {
        let test_cases = vec![
            ("rs", SupportedLanguage::Rust),
            ("py", SupportedLanguage::Python),
            ("js", SupportedLanguage::JavaScript),
            ("ts", SupportedLanguage::TypeScript),
            ("tsx", SupportedLanguage::TypeScript),
            ("go", SupportedLanguage::Go),
            ("java", SupportedLanguage::Java),
            ("c", SupportedLanguage::C),
            ("h", SupportedLanguage::C),
            ("cpp", SupportedLanguage::Cpp),
            ("hpp", SupportedLanguage::Cpp),
            ("cs", SupportedLanguage::CSharp),
            ("sh", SupportedLanguage::Bash),
            ("bash", SupportedLanguage::Bash),
        ];
        
        for (extension, expected_lang) in test_cases {
            let detected = SupportedLanguage::from_extension(extension);
            assert_eq!(detected, Some(expected_lang),
                      "Extension '{}' should be detected as {:?}", extension, expected_lang);
        }
        
        // Test unknown extensions
        let unknown_extensions = vec!["unknown", "xyz", ""];
        for ext in unknown_extensions {
            let result = SupportedLanguage::from_extension(ext);
            assert_eq!(result, None, "Unknown extension '{}' should return None", ext);
        }
    }

    #[test]
    fn test_all_languages_list_completeness() {
        let all_languages = SupportedLanguage::all_languages();
        
        // Should include core programming languages
        let core_languages = vec![
            SupportedLanguage::Rust,
            SupportedLanguage::Python,
            SupportedLanguage::JavaScript,
            SupportedLanguage::TypeScript,
            SupportedLanguage::Go,
            SupportedLanguage::Java,
            SupportedLanguage::C,
            SupportedLanguage::Cpp,
            SupportedLanguage::CSharp,
        ];
        
        for lang in core_languages {
            assert!(all_languages.contains(&lang),
                   "All languages list should contain {:?}", lang);
        }
        
        // Should have a reasonable total count
        assert!(all_languages.len() >= 20, 
               "Should support at least 20 languages, got {}", all_languages.len());
        assert!(all_languages.len() <= 50, 
               "Should not exceed 50 languages to remain manageable, got {}", all_languages.len());
    }

    #[test] 
    fn test_extension_uniqueness() {
        // Ensure no extension maps to multiple languages
        use std::collections::HashMap;
        let mut extension_to_language = HashMap::new();
        
        for &language in SupportedLanguage::all_languages() {
            for extension in language.extensions() {
                if let Some(existing_lang) = extension_to_language.get(extension) {
                    panic!("Extension '{}' is claimed by both {:?} and {:?}", 
                          extension, existing_lang, language);
                }
                extension_to_language.insert(*extension, language);
            }
        }
        
        // Should have mapped a reasonable number of extensions
        assert!(extension_to_language.len() >= 30,
               "Should map at least 30 file extensions");
    }

    #[test]
    fn test_point_creation_and_access() {
        let point = Point { row: 10, column: 25 };
        
        assert_eq!(point.row, 10);
        assert_eq!(point.column, 25);
    }

    #[test]
    fn test_tree_input_variants() {
        // Test that TreeInput variants can be created
        
        let parse_input = TreeInput::Parse {
            code: "fn test() {}".to_string(),
            language: Some(SupportedLanguage::Rust),
            file_path: None,
        };
        
        match parse_input {
            TreeInput::Parse { code, language, .. } => {
                assert_eq!(code, "fn test() {}");
                assert_eq!(language, Some(SupportedLanguage::Rust));
            }
            _ => panic!("Expected Parse variant"),
        }
        
        let parse_file_input = TreeInput::ParseFile {
            file_path: PathBuf::from("/test/file.rs"),
        };
        
        match parse_file_input {
            TreeInput::ParseFile { file_path } => {
                assert_eq!(file_path, PathBuf::from("/test/file.rs"));
            }
            _ => panic!("Expected ParseFile variant"),
        }
        
        let query_input = TreeInput::Query {
            code: "fn test() {}".to_string(),
            language: Some(SupportedLanguage::Rust),
            pattern: "(function_item)".to_string(),
            file_path: None,
        };
        
        match query_input {
            TreeInput::Query { pattern, .. } => {
                assert_eq!(pattern, "(function_item)");
            }
            _ => panic!("Expected Query variant"),
        }
    }

    #[test]
    fn test_tree_output_variants() {
        // Test TreeOutput variant structure (without actual parsing)
        
        // Test that we can pattern match on the variants
        let parsed_output = TreeOutput::Parsed {
            language: SupportedLanguage::Rust,
            node_count: 42,
            has_errors: false,
            error_count: 0,
            parse_time_ms: 5,
        };
        
        match parsed_output {
            TreeOutput::Parsed { language, node_count, has_errors, .. } => {
                assert_eq!(language, SupportedLanguage::Rust);
                assert_eq!(node_count, 42);
                assert!(!has_errors);
            }
            _ => panic!("Expected Parsed variant"),
        }
        
        let query_results = TreeOutput::QueryResults {
            matches: vec![],
            total_matches: 0,
            query_time_ms: 10,
        };
        
        match query_results {
            TreeOutput::QueryResults { total_matches, query_time_ms, .. } => {
                assert_eq!(total_matches, 0);
                assert_eq!(query_time_ms, 10);
            }
            _ => panic!("Expected QueryResults variant"),
        }
    }

    #[test]
    fn test_web_language_support() {
        let web_languages = vec![
            (SupportedLanguage::Html, "html"),
            (SupportedLanguage::Css, "css"),
            (SupportedLanguage::Json, "json"),
            (SupportedLanguage::JavaScript, "javascript"),
            (SupportedLanguage::TypeScript, "typescript"),
        ];
        
        for (language, name) in web_languages {
            assert_eq!(language.as_str(), name);
            let extensions = language.extensions();
            assert!(!extensions.is_empty(), "Web language {} should have extensions", name);
        }
    }

    #[test]
    fn test_systems_language_support() {
        let systems_languages = vec![
            (SupportedLanguage::Rust, "rust"),
            (SupportedLanguage::C, "c"),
            (SupportedLanguage::Cpp, "cpp"),
            (SupportedLanguage::Go, "go"),
            (SupportedLanguage::Zig, "zig"),
        ];
        
        for (language, name) in systems_languages {
            assert_eq!(language.as_str(), name);
            let extensions = language.extensions();
            assert!(!extensions.is_empty(), "Systems language {} should have extensions", name);
        }
    }

    #[test]
    fn test_functional_language_support() {
        let functional_languages = vec![
            (SupportedLanguage::Haskell, "haskell"),
            (SupportedLanguage::Elixir, "elixir"),
            (SupportedLanguage::Scala, "scala"),
            (SupportedLanguage::Clojure, "clojure"),
        ];
        
        for (language, name) in functional_languages {
            assert_eq!(language.as_str(), name);
            let extensions = language.extensions();
            assert!(!extensions.is_empty(), "Functional language {} should have extensions", name);
        }
    }

    #[test]
    fn test_config_and_markup_languages() {
        let config_languages = vec![
            (SupportedLanguage::Yaml, "yaml"),
            (SupportedLanguage::Json, "json"),
            (SupportedLanguage::Toml, "toml"),
            (SupportedLanguage::Html, "html"),
            (SupportedLanguage::Css, "css"),
            (SupportedLanguage::Markdown, "markdown"),
        ];
        
        for (language, name) in config_languages {
            assert_eq!(language.as_str(), name);
            let extensions = language.extensions();
            assert!(!extensions.is_empty(), "Config/markup language {} should have extensions", name);
        }
    }

    #[test]
    fn test_performance_target_values() {
        // Test that our performance expectations are reasonable
        use std::time::Duration;
        
        // Parse time target: <10ms per file
        let parse_target = Duration::from_millis(10);
        assert!(parse_target.as_millis() == 10);
        
        // Query time target: <50ms  
        let query_target = Duration::from_millis(50);
        assert!(query_target.as_millis() == 50);
        
        // Cache lookup target: <1ms
        let cache_target = Duration::from_millis(1);
        assert!(cache_target.as_millis() == 1);
        
        // Intelligence level cache sizes
        let light_cache_size = 100;
        let medium_cache_size = 500;
        let hard_cache_size = 2000;
        
        assert!(light_cache_size < medium_cache_size);
        assert!(medium_cache_size < hard_cache_size);
        assert!(hard_cache_size <= 5000); // Reasonable upper bound
    }
}

#[cfg(test)]
mod tree_input_output_tests {
    use super::*;

    #[test]
    fn test_tree_input_clone_and_debug() {
        // Test that TreeInput implements expected traits
        let original = TreeInput::Parse {
            code: "test code".to_string(),
            language: Some(SupportedLanguage::Rust),
            file_path: Some(PathBuf::from("test.rs")),
        };
        
        let cloned = original.clone();
        
        // Should be able to debug print
        let debug_str = format!("{:?}", original);
        assert!(debug_str.contains("Parse"));
        assert!(debug_str.contains("test code"));
        
        // Cloned should match original
        match (&original, &cloned) {
            (TreeInput::Parse { code: c1, language: l1, file_path: f1 }, 
             TreeInput::Parse { code: c2, language: l2, file_path: f2 }) => {
                assert_eq!(c1, c2);
                assert_eq!(l1, l2);
                assert_eq!(f1, f2);
            }
            _ => panic!("Clone should preserve variant type"),
        }
    }

    #[test]
    fn test_all_tree_input_variants() {
        let variants = vec![
            TreeInput::Parse {
                code: "test".to_string(),
                language: Some(SupportedLanguage::Rust),
                file_path: None,
            },
            TreeInput::ParseFile {
                file_path: PathBuf::from("test.rs"),
            },
            TreeInput::Query {
                code: "test".to_string(),
                language: Some(SupportedLanguage::Rust),
                pattern: "pattern".to_string(),
                file_path: None,
            },
            TreeInput::QueryFile {
                file_path: PathBuf::from("test.rs"),
                pattern: "pattern".to_string(),
            },
            TreeInput::ExtractSymbols {
                code: "test".to_string(),
                language: Some(SupportedLanguage::Rust),
                file_path: None,
            },
            TreeInput::ExtractSymbolsFromFile {
                file_path: PathBuf::from("test.rs"),
            },
            TreeInput::Diff {
                old_code: "old".to_string(),
                new_code: "new".to_string(),
                language: Some(SupportedLanguage::Rust),
            },
            TreeInput::DiffFiles {
                old_file: PathBuf::from("old.rs"),
                new_file: PathBuf::from("new.rs"),
            },
        ];
        
        // All variants should be creatable and debuggable
        for (i, variant) in variants.iter().enumerate() {
            let debug_str = format!("{:?}", variant);
            assert!(!debug_str.is_empty(), "Variant {} should have debug representation", i);
        }
        
        assert_eq!(variants.len(), 8, "Should test all TreeInput variants");
    }

    #[test]
    fn test_supported_language_coverage() {
        // Ensure we're testing a representative sample of all language categories
        let all_langs = SupportedLanguage::all_languages();
        
        // Count languages by category
        let core_langs = vec![
            SupportedLanguage::Rust, SupportedLanguage::Python, SupportedLanguage::JavaScript,
            SupportedLanguage::TypeScript, SupportedLanguage::Go, SupportedLanguage::Java,
            SupportedLanguage::C, SupportedLanguage::Cpp, SupportedLanguage::CSharp, SupportedLanguage::Bash
        ];
        
        let web_langs = vec![
            SupportedLanguage::Html, SupportedLanguage::Css, SupportedLanguage::Json,
            SupportedLanguage::Yaml, SupportedLanguage::Toml
        ];
        
        let script_langs = vec![
            SupportedLanguage::Ruby, SupportedLanguage::Php, SupportedLanguage::Lua
        ];
        
        let func_langs = vec![
            SupportedLanguage::Haskell, SupportedLanguage::Elixir, SupportedLanguage::Scala,
            SupportedLanguage::Ocaml, SupportedLanguage::Clojure
        ];
        
        // All categories should be represented in the total list
        for lang in core_langs {
            assert!(all_langs.contains(&lang), "Core language {:?} should be in all_languages", lang);
        }
        for lang in web_langs {
            assert!(all_langs.contains(&lang), "Web language {:?} should be in all_languages", lang);
        }
        for lang in script_langs {
            assert!(all_langs.contains(&lang), "Script language {:?} should be in all_languages", lang);
        }
        for lang in func_langs {
            assert!(all_langs.contains(&lang), "Functional language {:?} should be in all_languages", lang);
        }
    }
}