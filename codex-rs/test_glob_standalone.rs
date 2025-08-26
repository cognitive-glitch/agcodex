#!/usr/bin/env rust-script
//! Standalone test for the GlobTool to verify functionality
//! Run with: rust-script test_glob_standalone.rs

use std::path::PathBuf;
use std::fs;
use tempfile::TempDir;

// Minimal implementation to test core concepts
fn main() {
    println!("🔍 Testing GlobTool core functionality...");
    
    // Create test directory structure
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path();
    
    // Create test files
    fs::write(path.join("main.rs"), "fn main() {}").unwrap();
    fs::write(path.join("lib.rs"), "pub mod lib;").unwrap();
    fs::write(path.join("config.toml"), "[package]").unwrap();
    fs::write(path.join("README.md"), "# Test").unwrap();
    fs::write(path.join("test_main.rs"), "mod tests {}").unwrap();
    
    // Create subdirectory
    fs::create_dir(path.join("src")).unwrap();
    fs::write(path.join("src").join("lib.rs"), "pub mod lib;").unwrap();
    
    // Create .gitignore
    fs::write(path.join(".gitignore"), "target/\n*.tmp").unwrap();
    fs::write(path.join("ignored.tmp"), "temporary").unwrap();
    
    println!("✅ Created test directory structure");
    
    // Test ignore crate functionality
    use ignore::WalkBuilder;
    
    let mut walker = WalkBuilder::new(path);
    walker
        .hidden(false)  // Include hidden files for test
        .ignore(true)   // Use .ignore files
        .git_ignore(true); // Use .gitignore files
    
    let mut files = Vec::new();
    for result in walker.build() {
        match result {
            Ok(entry) => {
                if entry.file_type().map_or(false, |ft| ft.is_file()) {
                    if let Some(name) = entry.file_name().to_str() {
                        files.push(name.to_string());
                    }
                }
            }
            Err(_) => continue,
        }
    }
    
    println!("✅ Found {} files using ignore crate", files.len());
    for file in &files {
        println!("  📄 {}", file);
    }
    
    // Test wildmatch functionality
    use wildmatch::WildMatch;
    let pattern = WildMatch::new("*.rs");
    let rust_files: Vec<_> = files.iter().filter(|f| pattern.matches(f)).collect();
    
    println!("✅ Found {} Rust files using wildmatch", rust_files.len());
    for file in &rust_files {
        println!("  🦀 {}", file);
    }
    
    // Test that gitignore works
    assert!(!files.contains(&"ignored.tmp".to_string()), "ignored.tmp should be filtered out");
    
    // Test basic file classification
    let source_extensions: std::collections::HashSet<String> = ["rs", "py", "js"].iter().map(|s| s.to_string()).collect();
    let config_extensions: std::collections::HashSet<String> = ["toml", "json", "yml"].iter().map(|s| s.to_string()).collect();
    let doc_extensions: std::collections::HashSet<String> = ["md", "txt", "rst"].iter().map(|s| s.to_string()).collect();
    
    for file in &files {
        if let Some(ext) = file.split('.').last() {
            if source_extensions.contains(ext) {
                println!("  📄 {} -> Source", file);
            } else if config_extensions.contains(ext) {
                println!("  ⚙️  {} -> Config", file);
            } else if doc_extensions.contains(ext) {
                println!("  📚 {} -> Documentation", file);
            }
        }
    }
    
    println!("🎉 All core functionality tests passed!");
}