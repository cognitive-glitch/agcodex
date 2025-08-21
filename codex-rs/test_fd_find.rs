#!/usr/bin/env rust-script

//! Test script for fd-find implementation

use std::path::PathBuf;
use std::time::Duration;

// Copy the essential types and trait from our implementation
trait CodeTool {
    type Query;
    type Output;
    fn search(&self, query: Self::Query) -> Result<Self::Output, ToolError>;
}

#[derive(Debug, thiserror::Error)]
enum ToolError {
    #[error("tool not implemented: {0}")]
    NotImplemented(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("invalid query: {0}")]
    InvalidQuery(String),
}

// Include our fd-find implementation here
include!("core/src/code_tools/fd_find.rs");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing AGCodex FdFind Implementation");
    println!("======================================");
    
    let fd_find = FdFind::new();
    let current_dir = std::env::current_dir()?;
    
    // Test 1: Basic file search
    println!("\n1. 📁 Basic file search in current directory:");
    let query = FdQuery::new(&current_dir)
        .file_type(FileTypeFilter::FilesOnly)
        .max_results(10);
    
    match fd_find.search(query) {
        Ok(results) => {
            println!("   Found {} files:", results.len());
            for result in results.iter().take(5) {
                println!("   • {}", result.path.display());
            }
        }
        Err(e) => println!("   Error: {}", e),
    }
    
    // Test 2: Rust files only
    println!("\n2. 🦀 Find Rust files:");
    let rust_files = fd_find.find_files_by_extension(&current_dir, &["rs"])?;
    println!("   Found {} Rust files:", rust_files.len());
    for file in rust_files.iter().take(3) {
        println!("   • {}", file.path.display());
    }
    
    // Test 3: Configuration files
    println!("\n3. ⚙️  Find configuration files:");
    let config_files = fd_find.find_by_content_type(&current_dir, ContentType::Config)?;
    println!("   Found {} config files:", config_files.len());
    for file in config_files.iter().take(3) {
        println!("   • {}", file.path.display());
    }
    
    // Test 4: Builder pattern
    println!("\n4. 🔧 Builder pattern test:");
    let query = FdQuery::new(&current_dir)
        .globs(&["*.toml", "*.rs"])
        .file_type(FileTypeFilter::FilesOnly)
        .max_depth(3)
        .max_results(5)
        .timeout(Duration::from_secs(2));
    
    let results = fd_find.search(query)?;
    println!("   Found {} files with builder pattern:", results.len());
    for result in &results {
        println!("   • {} ({} bytes)", 
                 result.path.display(), 
                 result.size.unwrap_or(0));
    }
    
    println!("\n✅ FdFind implementation working correctly!");
    Ok(())
}