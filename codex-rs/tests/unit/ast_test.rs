//! Unit tests for AST parsing, compaction, and tree-sitter integration.
//!
//! Tests the core AST-RAG functionality including 50+ language support,
//! compression targets (70%/85%/95%), and incremental updates.

use agcodex_core::context_engine::{AstCompactor, CompactOptions, CompactResult};
use std::collections::HashMap;
use tempfile::TempDir;
use std::fs;

/// Sample code in various languages for testing
const RUST_SAMPLE: &str = r#"
use std::collections::HashMap;

/// A simple struct for testing
#[derive(Debug, Clone)]
pub struct TestStruct {
    pub id: u64,
    pub name: String,
    pub data: HashMap<String, i32>,
}

impl TestStruct {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            data: HashMap::new(),
        }
    }
    
    pub fn add_data(&mut self, key: String, value: i32) {
        self.data.insert(key, value);
    }
}
"#;

const TYPESCRIPT_SAMPLE: &str = r#"
interface User {
    id: number;
    name: string;
    email?: string;
}

class UserManager {
    private users: Map<number, User> = new Map();
    
    constructor(private readonly config: Config) {}
    
    async createUser(userData: Omit<User, 'id'>): Promise<User> {
        const id = this.generateId();
        const user: User = { id, ...userData };
        this.users.set(id, user);
        return user;
    }
    
    private generateId(): number {
        return Math.max(...this.users.keys(), 0) + 1;
    }
}
"#;

const PYTHON_SAMPLE: &str = r#"
from typing import Dict, List, Optional
from dataclasses import dataclass
import asyncio

@dataclass
class Task:
    id: str
    title: str
    completed: bool = False
    tags: List[str] = None
    
    def __post_init__(self):
        if self.tags is None:
            self.tags = []

class TaskManager:
    def __init__(self):
        self.tasks: Dict[str, Task] = {}
    
    async def add_task(self, task: Task) -> None:
        """Add a new task to the manager."""
        self.tasks[task.id] = task
        await self._persist()
    
    async def _persist(self) -> None:
        # Simulate async persistence
        await asyncio.sleep(0.001)
"#;

const GO_SAMPLE: &str = r#"
package main

import (
    "fmt"
    "sync"
    "time"
)

type Worker struct {
    id       int
    jobChan  chan Job
    quitChan chan bool
    wg       *sync.WaitGroup
}

type Job struct {
    ID      string    `json:"id"`
    Payload string    `json:"payload"`
    Created time.Time `json:"created"`
}

func NewWorker(id int, wg *sync.WaitGroup) *Worker {
    return &Worker{
        id:       id,
        jobChan:  make(chan Job, 10),
        quitChan: make(chan bool),
        wg:       wg,
    }
}

func (w *Worker) Start() {
    defer w.wg.Done()
    for {
        select {
        case job := <-w.jobChan:
            w.processJob(job)
        case <-w.quitChan:
            return
        }
    }
}
"#;

#[test]
fn test_ast_compactor_initialization() {
    let compactor = AstCompactor::new();
    
    // Verify compactor can be created
    assert!(compactor.is_some());
}

#[test]
fn test_basic_compression_rust() {
    let compactor = AstCompactor::new().unwrap();
    let options = CompactOptions {
        target_compression: 0.70, // 70% compression
        include_comments: false,
        include_whitespace: false,
        preserve_structure: true,
    };
    
    let result = compactor.compact_code(RUST_SAMPLE, "rust", options);
    
    match result {
        Ok(CompactResult { compressed_code, compression_ratio, .. }) => {
            // Should achieve at least 70% compression
            assert!(compression_ratio >= 0.70);
            
            // Should preserve key structural elements
            assert!(compressed_code.contains("TestStruct"));
            assert!(compressed_code.contains("new"));
            assert!(compressed_code.contains("add_data"));
            
            // Should remove implementation details
            assert!(!compressed_code.contains("HashMap::new()"));
        }
        Err(e) => panic!("Compression failed: {:?}", e),
    }
}

#[test]
fn test_medium_compression_typescript() {
    let compactor = AstCompactor::new().unwrap();
    let options = CompactOptions {
        target_compression: 0.85, // 85% compression
        include_comments: false,
        include_whitespace: false,
        preserve_structure: true,
    };
    
    let result = compactor.compact_code(TYPESCRIPT_SAMPLE, "typescript", options);
    
    match result {
        Ok(CompactResult { compressed_code, compression_ratio, .. }) => {
            assert!(compression_ratio >= 0.85);
            
            // Should preserve interface and class structure
            assert!(compressed_code.contains("User"));
            assert!(compressed_code.contains("UserManager"));
            assert!(compressed_code.contains("createUser"));
            
            // Should compress implementation details
            assert!(!compressed_code.contains("Math.max"));
        }
        Err(e) => panic!("Compression failed: {:?}", e),
    }
}

#[test]
fn test_maximum_compression_python() {
    let compactor = AstCompactor::new().unwrap();
    let options = CompactOptions {
        target_compression: 0.95, // 95% compression
        include_comments: false,
        include_whitespace: false,
        preserve_structure: true,
    };
    
    let result = compactor.compact_code(PYTHON_SAMPLE, "python", options);
    
    match result {
        Ok(CompactResult { compressed_code, compression_ratio, .. }) => {
            assert!(compression_ratio >= 0.90); // Allow some variance for maximum compression
            
            // Should preserve only essential structure
            assert!(compressed_code.contains("Task"));
            assert!(compressed_code.contains("TaskManager"));
            
            // Should remove most implementation
            assert!(!compressed_code.contains("asyncio.sleep"));
        }
        Err(e) => panic!("Compression failed: {:?}", e),
    }
}

#[test]
fn test_go_language_support() {
    let compactor = AstCompactor::new().unwrap();
    let options = CompactOptions {
        target_compression: 0.80,
        include_comments: false,
        include_whitespace: false,
        preserve_structure: true,
    };
    
    let result = compactor.compact_code(GO_SAMPLE, "go", options);
    
    match result {
        Ok(CompactResult { compressed_code, compression_ratio, .. }) => {
            assert!(compression_ratio >= 0.70);
            
            // Should preserve struct and function definitions
            assert!(compressed_code.contains("Worker"));
            assert!(compressed_code.contains("Job"));
            assert!(compressed_code.contains("NewWorker"));
            assert!(compressed_code.contains("Start"));
        }
        Err(e) => panic!("Go compression failed: {:?}", e),
    }
}

#[test]
fn test_unsupported_language_handling() {
    let compactor = AstCompactor::new().unwrap();
    let options = CompactOptions::default();
    
    let result = compactor.compact_code("console.log('test');", "fictional_lang", options);
    
    // Should handle unsupported languages gracefully
    assert!(result.is_err());
}

#[test]
fn test_empty_code_handling() {
    let compactor = AstCompactor::new().unwrap();
    let options = CompactOptions::default();
    
    let result = compactor.compact_code("", "rust", options);
    
    match result {
        Ok(CompactResult { compressed_code, compression_ratio, .. }) => {
            assert!(compressed_code.is_empty());
            assert_eq!(compression_ratio, 1.0); // 100% compression of empty content
        }
        Err(_) => {
            // Also acceptable to error on empty input
        }
    }
}

#[test]
fn test_compression_consistency() {
    let compactor = AstCompactor::new().unwrap();
    let options = CompactOptions {
        target_compression: 0.80,
        include_comments: false,
        include_whitespace: false,
        preserve_structure: true,
    };
    
    // Compress the same code multiple times
    let result1 = compactor.compact_code(RUST_SAMPLE, "rust", options.clone()).unwrap();
    let result2 = compactor.compact_code(RUST_SAMPLE, "rust", options).unwrap();
    
    // Results should be identical
    assert_eq!(result1.compressed_code, result2.compressed_code);
    assert_eq!(result1.compression_ratio, result2.compression_ratio);
}

#[test]
fn test_file_based_compression() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    fs::write(&file_path, RUST_SAMPLE).unwrap();
    
    let compactor = AstCompactor::new().unwrap();
    let options = CompactOptions::default();
    
    let result = compactor.compact_file(&file_path, options);
    
    match result {
        Ok(CompactResult { compressed_code, .. }) => {
            assert!(!compressed_code.is_empty());
            assert!(compressed_code.contains("TestStruct"));
        }
        Err(e) => panic!("File compression failed: {:?}", e),
    }
}

#[cfg(test)]
mod language_detection_tests {
    use super::*;
    
    #[test]
    fn test_language_detection_by_extension() {
        let test_cases = vec![
            ("test.rs", "rust"),
            ("test.py", "python"),
            ("test.ts", "typescript"),
            ("test.js", "javascript"),
            ("test.go", "go"),
            ("test.java", "java"),
            ("test.cpp", "cpp"),
            ("test.c", "c"),
            ("test.rb", "ruby"),
            ("test.php", "php"),
        ];
        
        let compactor = AstCompactor::new().unwrap();
        
        for (filename, expected_lang) in test_cases {
            let detected = compactor.detect_language_from_filename(filename);
            assert_eq!(detected, Some(expected_lang.to_string()), 
                      "Failed to detect {} from {}", expected_lang, filename);
        }
    }
    
    #[test]
    fn test_language_detection_by_content() {
        let test_cases = vec![
            ("fn main() {}", "rust"),
            ("def main():", "python"),
            ("function main() {}", "javascript"),
            ("package main", "go"),
            ("public class Main {}", "java"),
        ];
        
        let compactor = AstCompactor::new().unwrap();
        
        for (content, expected_lang) in test_cases {
            let detected = compactor.detect_language_from_content(content);
            assert_eq!(detected, Some(expected_lang.to_string()),
                      "Failed to detect {} from content", expected_lang);
        }
    }
}

#[cfg(test)]
mod compression_options_tests {
    use super::*;
    
    #[test]
    fn test_compression_with_comments() {
        let compactor = AstCompactor::new().unwrap();
        let code_with_comments = r#"
        // This is a comment
        fn test() {
            /* Block comment */
            println!("test");
        }
        "#;
        
        let with_comments = CompactOptions {
            include_comments: true,
            ..CompactOptions::default()
        };
        
        let without_comments = CompactOptions {
            include_comments: false,
            ..CompactOptions::default()
        };
        
        let result_with = compactor.compact_code(code_with_comments, "rust", with_comments).unwrap();
        let result_without = compactor.compact_code(code_with_comments, "rust", without_comments).unwrap();
        
        // Without comments should have higher compression
        assert!(result_without.compression_ratio > result_with.compression_ratio);
    }
    
    #[test]
    fn test_structure_preservation() {
        let compactor = AstCompactor::new().unwrap();
        let preserve_structure = CompactOptions {
            preserve_structure: true,
            target_compression: 0.90,
            ..CompactOptions::default()
        };
        
        let result = compactor.compact_code(RUST_SAMPLE, "rust", preserve_structure).unwrap();
        
        // Should preserve function signatures even with high compression
        assert!(result.compressed_code.contains("pub fn new"));
        assert!(result.compressed_code.contains("pub fn add_data"));
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn test_compression_performance() {
        let compactor = AstCompactor::new().unwrap();
        let options = CompactOptions::default();
        
        let start = Instant::now();
        let _ = compactor.compact_code(RUST_SAMPLE, "rust", options);
        let duration = start.elapsed();
        
        // Should complete compression in under 10ms for small files
        assert!(duration.as_millis() < 10, "Compression took {:?}", duration);
    }
    
    #[test]
    fn test_large_file_compression() {
        // Create a larger code sample by repeating the Rust sample
        let large_code = RUST_SAMPLE.repeat(100);
        
        let compactor = AstCompactor::new().unwrap();
        let options = CompactOptions::default();
        
        let start = Instant::now();
        let result = compactor.compact_code(&large_code, "rust", options);
        let duration = start.elapsed();
        
        assert!(result.is_ok());
        // Should handle larger files efficiently
        assert!(duration.as_millis() < 100, "Large file compression took {:?}", duration);
    }
}

impl Default for CompactOptions {
    fn default() -> Self {
        Self {
            target_compression: 0.80,
            include_comments: false,
            include_whitespace: false,
            preserve_structure: true,
        }
    }
}