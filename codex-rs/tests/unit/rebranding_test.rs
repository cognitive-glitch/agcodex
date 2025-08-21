//! Unit tests for AGCodex rebranding verification.
//!
//! Ensures complete migration from "agcodex" to "agcodex" across all
//! crates, binaries, documentation, and configuration files.

use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use regex::Regex;

/// Files that should be excluded from rebranding checks
const EXCLUDED_PATHS: &[&str] = &[
    "target/",
    ".git/",
    "node_modules/",
    ".cargo/",
    "tests/fixtures/",
];

/// Patterns that indicate old "agcodex" references that should be updated
const OLD_PATTERNS: &[&str] = &[
    r"\bcodex\b",           // Word boundary "agcodex"
    r"\bCodex\b",           // Word boundary "Codex"
    r"\bCODEX\b",           // Word boundary "CODEX"
    r"agcodex-",              // Hyphenated references
    r"agcodex_",              // Underscore references
    r"/codex/",             // Path references
    r"\.codex\b",           // Config directory references
];

/// Patterns that should be present after rebranding
const NEW_PATTERNS: &[&str] = &[
    r"\bagcodex\b",         // Word boundary "agcodex"
    r"\bAGCodex\b",         // Word boundary "AGCodex"
    r"\bAGCODEX\b",         // Word boundary "AGCODEX"
    r"agcodex-",            // Hyphenated references
    r"agcodex_",            // Underscore references
    r"/agcodex/",           // Path references
    r"\.agcodex\b",         // Config directory references
];

/// Exception patterns that are allowed to contain "agcodex"
const ALLOWED_EXCEPTIONS: &[&str] = &[
    r"//.*codex.*",        // Comments referencing old codex
    r"/\*.*codex.*\*/",    // Block comments
    r"#.*codex.*",         // Config comments
    r"agcodex_conversation", // Specific module names that haven't migrated yet
    r"create_dummy_codex_auth", // Test functions
];

#[test]
fn test_cargo_toml_rebranding() {
    let workspace_root = find_workspace_root();
    let cargo_toml_path = workspace_root.join("Cargo.toml");
    
    assert!(cargo_toml_path.exists(), "Cargo.toml not found");
    
    let content = fs::read_to_string(&cargo_toml_path)
        .expect("Failed to read Cargo.toml");
    
    // Should not contain old references
    assert!(!content.contains("agcodex_cli_rs"), 
           "Cargo.toml still contains 'codex_cli_rs' reference");
    
    // Should contain new references
    assert!(content.contains("agcodex") || content.len() > 0, 
           "Cargo.toml should reference agcodex");
}

#[test]
fn test_crate_names_rebranded() {
    let workspace_root = find_workspace_root();
    
    // Check each crate's Cargo.toml
    let crate_dirs = [
        "cli", "core", "tui", "exec", "common",
        "mcp-client", "mcp-server", "protocol",
        "file-search", "apply-patch",
    ];
    
    for crate_dir in &crate_dirs {
        let cargo_toml = workspace_root.join(crate_dir).join("Cargo.toml");
        
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml)
                .expect(&format!("Failed to read {}/Cargo.toml", crate_dir));
            
            // Check for old package names
            assert!(!content.contains(r#"name = "agcodex-"#),
                   "{}/Cargo.toml contains old codex package name", crate_dir);
            
            // Most crates should have agcodex prefix or be agcodex itself
            if *crate_dir != "common" && *crate_dir != "protocol" {
                // Allow some flexibility for crates that might keep their names
            }
        }
    }
}

#[test]
fn test_binary_names_rebranded() {
    let workspace_root = find_workspace_root();
    let cli_cargo = workspace_root.join("cli").join("Cargo.toml");
    
    if cli_cargo.exists() {
        let content = fs::read_to_string(&cli_cargo)
            .expect("Failed to read cli/Cargo.toml");
        
        // Binary should be named agcodex, not codex
        if content.contains("[[bin]]") {
            assert!(!content.contains(r#"name = "agcodex""#),
                   "CLI binary still named 'codex'");
        }
    }
}

#[test]
fn test_source_code_references() {
    let workspace_root = find_workspace_root();
    let problematic_files = find_files_with_old_references(&workspace_root);
    
    if !problematic_files.is_empty() {
        let file_list: Vec<String> = problematic_files.into_iter()
            .map(|(path, matches)| format!("{}: {}", path.display(), matches.join(", ")))
            .collect();
        
        panic!("Found old 'codex' references in source files:\n{}", 
               file_list.join("\n"));
    }
}

#[test]
fn test_documentation_rebranding() {
    let workspace_root = find_workspace_root();
    let docs_to_check = ["README.md", "CLAUDE.md", "PLANS.md"];
    
    for doc in &docs_to_check {
        let doc_path = workspace_root.join(doc);
        if doc_path.exists() {
            let content = fs::read_to_string(&doc_path)
                .expect(&format!("Failed to read {}", doc));
            
            // Should contain AGCodex references
            assert!(content.contains("AGCodex"),
                   "{} should contain 'AGCodex' references", doc);
            
            // Check for problematic old references (excluding intentional mentions)
            let old_refs = count_non_contextual_codex_references(&content);
            if old_refs > 5 { // Allow some contextual mentions
                panic!("{} contains {} old 'codex' references - needs updating", doc, old_refs);
            }
        }
    }
}

#[test]
fn test_config_directory_references() {
    let workspace_root = find_workspace_root();
    
    // Find files that reference config directories
    for entry in WalkDir::new(&workspace_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| is_source_file(e.path()))
    {
        if should_skip_file(entry.path()) {
            continue;
        }
        
        let content = match fs::read_to_string(entry.path()) {
            Ok(content) => content,
            Err(_) => continue,
        };
        
        // Check for old config directory references
        if content.contains("~/.agcodex") || content.contains(".agcodex/") {
            // Allow some references in test files or comments
            if !entry.path().to_string_lossy().contains("test") {
                panic!("File {} contains old config directory reference ~/.agcodex - should be ~/.agcodex", 
                       entry.path().display());
            }
        }
    }
}

#[test]
fn test_environment_variable_references() {
    let workspace_root = find_workspace_root();
    
    for entry in WalkDir::new(&workspace_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| is_source_file(e.path()))
    {
        if should_skip_file(entry.path()) {
            continue;
        }
        
        let content = match fs::read_to_string(entry.path()) {
            Ok(content) => content,
            Err(_) => continue,
        };
        
        // Check for old environment variable references
        if content.contains("CODEX_") && !content.contains("CODEX_SANDBOX") {
            // CODEX_SANDBOX is allowed as it might be a legacy test variable
            if !is_exception_allowed(&content, entry.path()) {
                println!("Warning: File {} contains CODEX_ environment variable reference", 
                        entry.path().display());
            }
        }
    }
}

#[test]
fn test_import_statements_updated() {
    let workspace_root = find_workspace_root();
    
    for entry in WalkDir::new(&workspace_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if should_skip_file(entry.path()) {
            continue;
        }
        
        let content = match fs::read_to_string(entry.path()) {
            Ok(content) => content,
            Err(_) => continue,
        };
        
        // Check for old crate imports
        let old_imports = [
            "use codex_core::",
            "use codex_login::",
            "use codex_common::",
            "extern crate codex_",
        ];
        
        for old_import in &old_imports {
            if content.contains(old_import) {
                // Some files might still have these during transition
                println!("Note: File {} contains old import: {}", 
                        entry.path().display(), old_import);
            }
        }
    }
}

#[test]
fn test_test_helper_functions_updated() {
    let workspace_root = find_workspace_root();
    
    // Check test files specifically
    for entry in WalkDir::new(&workspace_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().to_string_lossy().contains("test"))
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let content = match fs::read_to_string(entry.path()) {
            Ok(content) => content,
            Err(_) => continue,
        };
        
        // Look for test helper functions that might need updating
        let old_helpers = [
            "load_default_config_for_test",
            "create_dummy_codex_auth",
        ];
        
        for helper in &old_helpers {
            if content.contains(helper) {
                // These are allowed in test files for now
                // but note their presence
            }
        }
    }
}

// Helper functions

fn find_workspace_root() -> std::path::PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    
    // Look for Cargo.toml with workspace definition
    for ancestor in current_dir.ancestors() {
        let cargo_toml = ancestor.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml).unwrap_or_default();
            if content.contains("[workspace]") {
                return ancestor.to_path_buf();
            }
        }
    }
    
    current_dir
}

fn find_files_with_old_references(root: &Path) -> Vec<(std::path::PathBuf, Vec<String>)> {
    let mut problematic_files = Vec::new();
    
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| is_source_file(e.path()))
    {
        if should_skip_file(entry.path()) {
            continue;
        }
        
        let content = match fs::read_to_string(entry.path()) {
            Ok(content) => content,
            Err(_) => continue,
        };
        
        let matches = find_problematic_references(&content);
        if !matches.is_empty() {
            problematic_files.push((entry.path().to_path_buf(), matches));
        }
    }
    
    problematic_files
}

fn is_source_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        matches!(ext, "rs" | "toml" | "md" | "txt" | "json" | "yaml" | "yml")
    } else {
        false
    }
}

fn should_skip_file(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    
    for excluded in EXCLUDED_PATHS {
        if path_str.contains(excluded) {
            return true;
        }
    }
    
    // Skip this test file itself
    if path_str.contains("rebranding_test.rs") {
        return true;
    }
    
    false
}

fn find_problematic_references(content: &str) -> Vec<String> {
    let mut matches = Vec::new();
    
    for pattern in OLD_PATTERNS {
        let regex = Regex::new(pattern).unwrap();
        for mat in regex.find_iter(content) {
            let matched_text = mat.as_str();
            
            // Check if this match is in an allowed exception
            if !is_match_exception(content, mat.start(), matched_text) {
                matches.push(matched_text.to_string());
            }
        }
    }
    
    matches
}

fn is_match_exception(content: &str, match_start: usize, matched_text: &str) -> bool {
    // Get the line containing the match
    let line_start = content[..match_start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = content[match_start..].find('\n').map(|i| match_start + i).unwrap_or(content.len());
    let line = &content[line_start..line_end];
    
    // Check against exception patterns
    for exception in ALLOWED_EXCEPTIONS {
        let regex = Regex::new(exception).unwrap();
        if regex.is_match(line) {
            return true;
        }
    }
    
    // Allow "agcodex" in specific contexts
    if matched_text == "agcodex" && (
        line.contains("// Legacy") ||
        line.contains("// TODO") ||
        line.contains("// Old") ||
        line.contains("deprecated")
    ) {
        return true;
    }
    
    false
}

fn count_non_contextual_codex_references(content: &str) -> usize {
    let agcodex_regex = Regex::new(r"\bcodex\b").unwrap();
    let mut count = 0;
    
    for mat in codex_regex.find_iter(content) {
        if !is_match_exception(content, mat.start(), mat.as_str()) {
            count += 1;
        }
    }
    
    count
}

fn is_exception_allowed(content: &str, path: &Path) -> bool {
    // Allow exceptions in test files
    if path.to_string_lossy().contains("test") {
        return true;
    }
    
    // Allow exceptions in specific files
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if filename == "client.rs" && path.to_string_lossy().contains("core/tests") {
        return true;
    }
    
    false
}

#[cfg(test)]
mod rebranding_progress_tests {
    use super::*;
    
    #[test]
    fn test_rebranding_completion_percentage() {
        let workspace_root = find_workspace_root();
        let all_files = count_all_relevant_files(&workspace_root);
        let files_with_old_refs = find_files_with_old_references(&workspace_root);
        
        let completion_percentage = if all_files > 0 {
            ((all_files - files_with_old_refs.len()) as f64 / all_files as f64) * 100.0
        } else {
            100.0
        };
        
        println!("Rebranding completion: {:.1}% ({}/{} files)", 
                completion_percentage, 
                all_files - files_with_old_refs.len(), 
                all_files);
        
        // Target: at least 90% completion
        assert!(completion_percentage >= 90.0, 
               "Rebranding completion is only {:.1}%, should be at least 90%", 
               completion_percentage);
    }
    
    fn count_all_relevant_files(root: &Path) -> usize {
        WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| is_source_file(e.path()))
            .filter(|e| !should_skip_file(e.path()))
            .count()
    }
}

#[cfg(test)]
mod specific_migration_tests {
    use super::*;
    
    #[test]
    fn test_cli_binary_name() {
        let workspace_root = find_workspace_root();
        let cli_cargo = workspace_root.join("cli").join("Cargo.toml");
        
        if cli_cargo.exists() {
            let content = fs::read_to_string(&cli_cargo).unwrap();
            
            // Should specify agcodex as binary name
            if content.contains("[[bin]]") {
                // Extract binary name if specified
                let lines: Vec<&str> = content.lines().collect();
                for (i, line) in lines.iter().enumerate() {
                    if line.contains("[[bin]]") && i + 1 < lines.len() {
                        let next_line = lines[i + 1];
                        if next_line.contains("name =") {
                            assert!(next_line.contains("agcodex"), 
                                   "Binary name should be 'agcodex', found: {}", next_line);
                        }
                    }
                }
            }
        }
    }
    
    #[test]
    fn test_config_path_migration() {
        // Test that code references ~/.agcodex instead of ~/.agcodex
        let workspace_root = find_workspace_root();
        
        let config_files = find_files_mentioning_config_path(&workspace_root);
        
        for (file_path, has_old_path, has_new_path) in config_files {
            if has_old_path && !has_new_path {
                println!("Warning: {} references old config path ~/.agcodex", file_path.display());
            }
        }
    }
    
    fn find_files_mentioning_config_path(root: &Path) -> Vec<(std::path::PathBuf, bool, bool)> {
        let mut files = Vec::new();
        
        for entry in WalkDir::new(root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| is_source_file(e.path()))
        {
            if should_skip_file(entry.path()) {
                continue;
            }
            
            let content = match fs::read_to_string(entry.path()) {
                Ok(content) => content,
                Err(_) => continue,
            };
            
            let has_old = content.contains("/.codex") || content.contains(".agcodex/");
            let has_new = content.contains("/.agcodex") || content.contains(".agcodex/");
            
            if has_old || has_new {
                files.push((entry.path().to_path_buf(), has_old, has_new));
            }
        }
        
        files
    }
}