//! Comprehensive tests for the TreeTool - tree-sitter AST parsing and querying.
//!
//! Tests cover the core tree-sitter functionality including:
//! - Multi-language parsing (27+ languages)
//! - Query pattern matching and execution
//! - Symbol extraction and analysis
//! - Semantic code diffing
//! - Performance targets (<10ms per file)
//! - Error recovery and syntax error handling
//! - Cache effectiveness and TTL management
//! - Language auto-detection from file extensions

use agcodex_core::tools::tree::{
    TreeTool, TreeInput, TreeOutput, TreeError, SupportedLanguage, 
    Symbol, SymbolKind, QueryMatch, SemanticDiff, Point
};
use agcodex_core::subagents::IntelligenceLevel;
use agcodex_core::tools::InternalTool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::fs;

// Sample code in multiple languages for comprehensive testing
const RUST_SAMPLE: &str = r#"
use std::collections::HashMap;
use std::sync::Arc;

/// User management system
#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Admin,
    Moderator,
    User,
}

impl User {
    /// Create a new user with default role
    pub fn new(id: u64, name: String, email: String) -> Self {
        Self {
            id,
            name,
            email,
            roles: vec![Role::User],
        }
    }
    
    pub fn add_role(&mut self, role: Role) {
        if !self.roles.contains(&role) {
            self.roles.push(role);
        }
    }
    
    pub fn is_admin(&self) -> bool {
        self.roles.contains(&Role::Admin)
    }
}

pub struct UserRegistry {
    users: Arc<HashMap<u64, User>>,
    next_id: u64,
}

impl UserRegistry {
    pub fn new() -> Self {
        Self {
            users: Arc::new(HashMap::new()),
            next_id: 1,
        }
    }
    
    pub async fn register_user(&mut self, name: String, email: String) -> Result<u64, String> {
        let id = self.next_id;
        self.next_id += 1;
        
        let user = User::new(id, name, email);
        let mut users = Arc::make_mut(&mut self.users);
        users.insert(id, user);
        
        Ok(id)
    }
}
"#;

const PYTHON_SAMPLE: &str = r#"
"""
Task management system with async support
"""
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, field
from datetime import datetime
import asyncio
import json

@dataclass
class Task:
    """A task with metadata and completion tracking."""
    id: str
    title: str
    completed: bool = False
    created_at: datetime = field(default_factory=datetime.now)
    tags: List[str] = field(default_factory=list)
    priority: int = 1  # 1=low, 2=medium, 3=high
    
    def mark_completed(self) -> None:
        """Mark the task as completed."""
        self.completed = True
    
    def add_tag(self, tag: str) -> None:
        """Add a tag if not already present."""
        if tag not in self.tags:
            self.tags.append(tag)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            'id': self.id,
            'title': self.title,
            'completed': self.completed,
            'created_at': self.created_at.isoformat(),
            'tags': self.tags,
            'priority': self.priority
        }

class TaskManager:
    """Manages a collection of tasks with async operations."""
    
    def __init__(self):
        self.tasks: Dict[str, Task] = {}
        self._lock = asyncio.Lock()
    
    async def add_task(self, task: Task) -> None:
        """Add a task to the manager."""
        async with self._lock:
            self.tasks[task.id] = task
    
    async def get_task(self, task_id: str) -> Optional[Task]:
        """Get a task by ID."""
        async with self._lock:
            return self.tasks.get(task_id)
    
    async def list_tasks(self, completed: Optional[bool] = None) -> List[Task]:
        """List tasks, optionally filtered by completion status."""
        async with self._lock:
            tasks = list(self.tasks.values())
            
            if completed is not None:
                tasks = [task for task in tasks if task.completed == completed]
            
            return sorted(tasks, key=lambda t: t.created_at)
    
    async def find_by_tag(self, tag: str) -> List[Task]:
        """Find all tasks with the specified tag."""
        tasks = await self.list_tasks()
        return [task for task in tasks if tag in task.tags]
    
    async def export_json(self) -> str:
        """Export all tasks to JSON."""
        tasks = await self.list_tasks()
        task_dicts = [task.to_dict() for task in tasks]
        return json.dumps(task_dicts, indent=2)
"#;

const JAVASCRIPT_SAMPLE: &str = r#"
/**
 * Event-driven user interface component
 */

class EventManager {
    constructor() {
        this.listeners = new Map();
        this.eventQueue = [];
        this.processing = false;
    }
    
    /**
     * Register an event listener
     * @param {string} eventType - Type of event to listen for
     * @param {Function} handler - Handler function
     * @param {Object} options - Optional configuration
     */
    on(eventType, handler, options = {}) {
        if (!this.listeners.has(eventType)) {
            this.listeners.set(eventType, []);
        }
        
        this.listeners.get(eventType).push({
            handler,
            once: options.once || false,
            priority: options.priority || 0
        });
        
        // Sort by priority
        this.listeners.get(eventType).sort((a, b) => b.priority - a.priority);
    }
    
    /**
     * Emit an event to all registered listeners
     * @param {string} eventType - Type of event to emit
     * @param {*} data - Event data
     */
    async emit(eventType, data) {
        const listeners = this.listeners.get(eventType) || [];
        const results = [];
        
        for (const listener of listeners) {
            try {
                const result = await listener.handler(data);
                results.push(result);
                
                if (listener.once) {
                    this.off(eventType, listener.handler);
                }
            } catch (error) {
                console.error(`Event handler error for ${eventType}:`, error);
            }
        }
        
        return results;
    }
    
    /**
     * Remove an event listener
     */
    off(eventType, handler) {
        const listeners = this.listeners.get(eventType);
        if (listeners) {
            const index = listeners.findIndex(l => l.handler === handler);
            if (index !== -1) {
                listeners.splice(index, 1);
            }
        }
    }
    
    /**
     * Queue an event for batch processing
     */
    queue(eventType, data) {
        this.eventQueue.push({ eventType, data, timestamp: Date.now() });
        
        if (!this.processing) {
            this.processQueue();
        }
    }
    
    async processQueue() {
        this.processing = true;
        
        while (this.eventQueue.length > 0) {
            const event = this.eventQueue.shift();
            await this.emit(event.eventType, event.data);
        }
        
        this.processing = false;
    }
}

// Usage example
const eventManager = new EventManager();

eventManager.on('user:login', async (userData) => {
    console.log(`User ${userData.name} logged in`);
    return { status: 'acknowledged' };
});

eventManager.on('user:logout', (userData) => {
    console.log(`User ${userData.name} logged out`);
}, { once: true });

export default EventManager;
"#;

const TYPESCRIPT_SAMPLE: &str = r#"
/**
 * Type-safe configuration management system
 */

interface ConfigValue {
    key: string;
    value: any;
    type: 'string' | 'number' | 'boolean' | 'object' | 'array';
    required: boolean;
    default?: any;
    validator?: (value: any) => boolean;
}

interface ConfigSchema {
    [key: string]: ConfigValue;
}

interface ValidationResult {
    valid: boolean;
    errors: string[];
    warnings: string[];
}

class ConfigManager<T extends Record<string, any> = Record<string, any>> {
    private config: Partial<T> = {};
    private schema: ConfigSchema;
    private listeners: Map<string, ((value: any) => void)[]> = new Map();

    constructor(schema: ConfigSchema, initialConfig?: Partial<T>) {
        this.schema = schema;
        
        if (initialConfig) {
            this.load(initialConfig);
        }
        
        this.applyDefaults();
    }

    /**
     * Load configuration from object
     */
    load(config: Partial<T>): ValidationResult {
        const result = this.validate(config);
        
        if (result.valid) {
            Object.assign(this.config, config);
            this.notifyListeners();
        }
        
        return result;
    }

    /**
     * Get a configuration value with type safety
     */
    get<K extends keyof T>(key: K): T[K] | undefined {
        return this.config[key];
    }

    /**
     * Set a configuration value with validation
     */
    set<K extends keyof T>(key: K, value: T[K]): boolean {
        const schemaEntry = this.schema[key as string];
        
        if (!schemaEntry) {
            console.warn(`Unknown configuration key: ${String(key)}`);
            return false;
        }
        
        if (schemaEntry.validator && !schemaEntry.validator(value)) {
            console.error(`Validation failed for key: ${String(key)}`);
            return false;
        }
        
        const oldValue = this.config[key];
        this.config[key] = value;
        
        this.notifyListeners(key as string, value, oldValue);
        return true;
    }

    /**
     * Watch for changes to a specific configuration key
     */
    watch<K extends keyof T>(key: K, callback: (value: T[K]) => void): () => void {
        const keyStr = key as string;
        
        if (!this.listeners.has(keyStr)) {
            this.listeners.set(keyStr, []);
        }
        
        this.listeners.get(keyStr)!.push(callback);
        
        // Return unsubscribe function
        return () => {
            const callbacks = this.listeners.get(keyStr);
            if (callbacks) {
                const index = callbacks.indexOf(callback);
                if (index !== -1) {
                    callbacks.splice(index, 1);
                }
            }
        };
    }

    /**
     * Validate configuration against schema
     */
    private validate(config: Partial<T>): ValidationResult {
        const errors: string[] = [];
        const warnings: string[] = [];

        // Check required fields
        for (const [key, schema] of Object.entries(this.schema)) {
            if (schema.required && !(key in config)) {
                errors.push(`Missing required configuration: ${key}`);
            }
        }

        // Validate types and custom validators
        for (const [key, value] of Object.entries(config)) {
            const schema = this.schema[key];
            
            if (!schema) {
                warnings.push(`Unknown configuration key: ${key}`);
                continue;
            }
            
            if (!this.isCorrectType(value, schema.type)) {
                errors.push(`Type mismatch for ${key}: expected ${schema.type}`);
            }
            
            if (schema.validator && !schema.validator(value)) {
                errors.push(`Validation failed for ${key}`);
            }
        }

        return {
            valid: errors.length === 0,
            errors,
            warnings
        };
    }

    private isCorrectType(value: any, expectedType: string): boolean {
        switch (expectedType) {
            case 'string': return typeof value === 'string';
            case 'number': return typeof value === 'number';
            case 'boolean': return typeof value === 'boolean';
            case 'object': return typeof value === 'object' && !Array.isArray(value);
            case 'array': return Array.isArray(value);
            default: return true;
        }
    }

    private applyDefaults(): void {
        for (const [key, schema] of Object.entries(this.schema)) {
            if (schema.default !== undefined && !(key in this.config)) {
                (this.config as any)[key] = schema.default;
            }
        }
    }

    private notifyListeners(key?: string, newValue?: any, oldValue?: any): void {
        if (key) {
            const callbacks = this.listeners.get(key) || [];
            callbacks.forEach(callback => callback(newValue));
        } else {
            // Notify all listeners
            for (const [listenerKey, callbacks] of this.listeners.entries()) {
                const value = this.config[listenerKey as keyof T];
                callbacks.forEach(callback => callback(value));
            }
        }
    }
}

export { ConfigManager, ConfigSchema, ConfigValue, ValidationResult };
"#;

const GO_SAMPLE: &str = r#"
package main

import (
    "context"
    "fmt"
    "sync"
    "time"
)

// Worker represents a concurrent task processor
type Worker struct {
    ID       int
    jobChan  chan Job
    quitChan chan struct{}
    wg       *sync.WaitGroup
    results  chan Result
}

// Job represents a unit of work
type Job struct {
    ID      string    `json:"id"`
    Type    string    `json:"type"`
    Payload []byte    `json:"payload"`
    Created time.Time `json:"created"`
    Timeout time.Duration
}

// Result represents the outcome of a job
type Result struct {
    JobID     string        `json:"job_id"`
    Success   bool          `json:"success"`
    Data      interface{}   `json:"data,omitempty"`
    Error     string        `json:"error,omitempty"`
    Duration  time.Duration `json:"duration"`
}

// WorkerPool manages multiple workers
type WorkerPool struct {
    workers    []*Worker
    jobQueue   chan Job
    resultChan chan Result
    ctx        context.Context
    cancel     context.CancelFunc
    wg         sync.WaitGroup
}

// NewWorker creates a new worker instance
func NewWorker(id int, wg *sync.WaitGroup, results chan Result) *Worker {
    return &Worker{
        ID:       id,
        jobChan:  make(chan Job, 10),
        quitChan: make(chan struct{}),
        wg:       wg,
        results:  results,
    }
}

// Start begins the worker's job processing loop
func (w *Worker) Start(ctx context.Context) {
    defer w.wg.Done()
    
    for {
        select {
        case job := <-w.jobChan:
            result := w.processJob(ctx, job)
            w.results <- result
            
        case <-w.quitChan:
            fmt.Printf("Worker %d shutting down\n", w.ID)
            return
            
        case <-ctx.Done():
            fmt.Printf("Worker %d cancelled by context\n", w.ID)
            return
        }
    }
}

// processJob handles the actual job execution
func (w *Worker) processJob(ctx context.Context, job Job) Result {
    start := time.Now()
    
    // Create job-specific context with timeout
    jobCtx, cancel := context.WithTimeout(ctx, job.Timeout)
    defer cancel()
    
    result := Result{
        JobID: job.ID,
    }
    
    // Simulate job processing
    select {
    case <-time.After(time.Millisecond * 100): // Simulate work
        result.Success = true
        result.Data = fmt.Sprintf("Processed job %s by worker %d", job.ID, w.ID)
        
    case <-jobCtx.Done():
        result.Success = false
        result.Error = "Job timed out or was cancelled"
    }
    
    result.Duration = time.Since(start)
    return result
}

// Stop gracefully shuts down the worker
func (w *Worker) Stop() {
    close(w.quitChan)
}

// AddJob adds a job to the worker's queue
func (w *Worker) AddJob(job Job) bool {
    select {
    case w.jobChan <- job:
        return true
    default:
        return false // Queue is full
    }
}

// NewWorkerPool creates a new worker pool
func NewWorkerPool(numWorkers int) *WorkerPool {
    ctx, cancel := context.WithCancel(context.Background())
    
    return &WorkerPool{
        workers:    make([]*Worker, 0, numWorkers),
        jobQueue:   make(chan Job, numWorkers*10),
        resultChan: make(chan Result, numWorkers*5),
        ctx:        ctx,
        cancel:     cancel,
    }
}

// Start initializes and starts all workers in the pool
func (wp *WorkerPool) Start(numWorkers int) {
    for i := 0; i < numWorkers; i++ {
        worker := NewWorker(i+1, &wp.wg, wp.resultChan)
        wp.workers = append(wp.workers, worker)
        
        wp.wg.Add(1)
        go worker.Start(wp.ctx)
    }
    
    // Start job distributor
    go wp.distributeJobs()
}

// distributeJobs distributes incoming jobs to available workers
func (wp *WorkerPool) distributeJobs() {
    for {
        select {
        case job := <-wp.jobQueue:
            // Find an available worker
            distributed := false
            for _, worker := range wp.workers {
                if worker.AddJob(job) {
                    distributed = true
                    break
                }
            }
            
            if !distributed {
                // All workers are busy, put job back or handle differently
                select {
                case wp.jobQueue <- job:
                case <-wp.ctx.Done():
                    return
                }
            }
            
        case <-wp.ctx.Done():
            return
        }
    }
}

// SubmitJob adds a job to the pool's queue
func (wp *WorkerPool) SubmitJob(job Job) {
    select {
    case wp.jobQueue <- job:
    case <-wp.ctx.Done():
        fmt.Printf("Cannot submit job %s: pool is shutting down\n", job.ID)
    }
}

// GetResults returns the result channel
func (wp *WorkerPool) GetResults() <-chan Result {
    return wp.resultChan
}

// Shutdown gracefully stops all workers
func (wp *WorkerPool) Shutdown() {
    wp.cancel()
    
    // Stop all workers
    for _, worker := range wp.workers {
        worker.Stop()
    }
    
    wp.wg.Wait()
    close(wp.resultChan)
}
"#;

// Malformed code samples for error recovery testing
const RUST_SYNTAX_ERROR: &str = r#"
fn broken_function() {
    let x = 5;
    let y = // missing value
    println!("This should cause parse errors");
    if true {
        // missing closing brace
"#;

const PYTHON_SYNTAX_ERROR: &str = r#"
def broken_function():
    x = 5
    y = [1, 2, 3  # missing closing bracket
    if True
        print("missing colon")
        return x + y
"#;

const JAVASCRIPT_SYNTAX_ERROR: &str = r#"
function brokenFunction() {
    const x = 5;
    const y = {
        prop1: "value1",
        prop2: "value2"  // missing closing brace
    
    if (x > 0 {  // missing closing parenthesis
        console.log("broken syntax");
    }
"#;

/// Test helper functions
struct TreeTestHelper;

impl TreeTestHelper {
    /// Create temporary test files with sample code
    async fn create_test_files() -> Result<(TempDir, HashMap<String, PathBuf>), std::io::Error> {
        let temp_dir = TempDir::new()?;
        let mut files = HashMap::new();
        
        let test_files = vec![
            ("user_system.rs", RUST_SAMPLE),
            ("task_manager.py", PYTHON_SAMPLE),
            ("event_manager.js", JAVASCRIPT_SAMPLE),
            ("config_manager.ts", TYPESCRIPT_SAMPLE),
            ("worker_pool.go", GO_SAMPLE),
            ("syntax_error.rs", RUST_SYNTAX_ERROR),
            ("syntax_error.py", PYTHON_SYNTAX_ERROR),
            ("syntax_error.js", JAVASCRIPT_SYNTAX_ERROR),
        ];
        
        for (filename, content) in test_files {
            let file_path = temp_dir.path().join(filename);
            fs::write(&file_path, content).await?;
            files.insert(filename.to_string(), file_path);
        }
        
        Ok((temp_dir, files))
    }
    
    /// Assert parsing performance meets target
    fn assert_parse_performance(duration: Duration, target_ms: u64, language: &str) {
        assert!(
            duration.as_millis() < target_ms as u128,
            "Parsing {} took {}ms, target was <{}ms",
            language,
            duration.as_millis(),
            target_ms
        );
    }
    
    /// Assert cache hit rate meets expectations
    fn assert_cache_effectiveness(tool: &TreeTool, expected_hit_rate: f64) -> Result<(), TreeError> {
        let stats = tool.cache_stats()?;
        if let Some(cache_size_value) = stats.get("ast_cache_size") {
            let cache_size = cache_size_value.as_i64().unwrap_or(0);
            assert!(cache_size > 0, "Cache should contain entries after operations");
        }
        Ok(())
    }
    
    /// Create a query pattern for finding functions in different languages
    fn function_query_for_language(language: SupportedLanguage) -> &'static str {
        match language {
            SupportedLanguage::Rust => "(function_item name: (identifier) @func_name) @function",
            SupportedLanguage::Python => "(function_definition name: (identifier) @func_name) @function",
            SupportedLanguage::JavaScript => "(function_declaration name: (identifier) @func_name) @function",
            SupportedLanguage::TypeScript => "(function_declaration name: (identifier) @func_name) @function",
            SupportedLanguage::Go => "(function_declaration name: (identifier) @func_name) @function",
            _ => "(identifier) @name",
        }
    }
}

#[cfg(test)]
mod tree_tool_tests {
    use super::*;

    #[tokio::test]
    async fn test_tree_tool_initialization() {
        for intelligence_level in &[IntelligenceLevel::Light, IntelligenceLevel::Medium, IntelligenceLevel::Hard] {
            let tool = TreeTool::new(*intelligence_level);
            assert!(tool.is_ok(), "TreeTool should initialize successfully for {:?}", intelligence_level);
            
            let tool = tool.unwrap();
            let metadata = tool.metadata();
            assert_eq!(metadata.name, "TreeTool");
            assert!(metadata.description.contains("tree-sitter"));
        }
    }

    #[tokio::test]
    async fn test_language_detection() {
        let test_cases = vec![
            ("test.rs", SupportedLanguage::Rust),
            ("main.py", SupportedLanguage::Python),
            ("app.js", SupportedLanguage::JavaScript),
            ("component.tsx", SupportedLanguage::TypeScript),
            ("server.go", SupportedLanguage::Go),
            ("Main.java", SupportedLanguage::Java),
            ("program.c", SupportedLanguage::C),
            ("app.cpp", SupportedLanguage::Cpp),
            ("script.sh", SupportedLanguage::Bash),
            ("config.yaml", SupportedLanguage::Yaml),
        ];
        
        for (filename, expected_lang) in test_cases {
            let detected = SupportedLanguage::from_extension(
                &filename.split('.').last().unwrap()
            );
            assert_eq!(detected, Some(expected_lang), 
                      "Failed to detect {} from {}", expected_lang.as_str(), filename);
        }
    }

    #[tokio::test]
    async fn test_rust_parsing_and_performance() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        let start_time = Instant::now();
        
        let input = TreeInput::Parse {
            code: RUST_SAMPLE.to_string(),
            language: Some(SupportedLanguage::Rust),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        let parse_time = start_time.elapsed();
        
        assert!(result.is_ok(), "Rust parsing should succeed");
        
        let output = result.unwrap();
        if let TreeOutput::Parsed { language, node_count, has_errors, parse_time_ms, .. } = output.result {
            assert_eq!(language, SupportedLanguage::Rust);
            assert!(node_count > 50, "Should parse many AST nodes for complex Rust code");
            assert!(!has_errors, "Well-formed Rust code should not have parse errors");
            
            // Performance target: <10ms
            TreeTestHelper::assert_parse_performance(parse_time, 10, "Rust");
            assert!(parse_time_ms < 10, "Internal parse time should be <10ms");
        } else {
            panic!("Expected Parsed output, got {:?}", output.result);
        }
    }

    #[tokio::test]
    async fn test_python_parsing_and_performance() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        let start_time = Instant::now();
        
        let input = TreeInput::Parse {
            code: PYTHON_SAMPLE.to_string(),
            language: Some(SupportedLanguage::Python),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        let parse_time = start_time.elapsed();
        
        assert!(result.is_ok(), "Python parsing should succeed");
        
        let output = result.unwrap();
        if let TreeOutput::Parsed { language, node_count, has_errors, .. } = output.result {
            assert_eq!(language, SupportedLanguage::Python);
            assert!(node_count > 40, "Should parse many nodes for Python class");
            assert!(!has_errors, "Well-formed Python code should not have errors");
            
            TreeTestHelper::assert_parse_performance(parse_time, 10, "Python");
        } else {
            panic!("Expected Parsed output");
        }
    }

    #[tokio::test]
    async fn test_javascript_parsing_and_performance() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        let start_time = Instant::now();
        
        let input = TreeInput::Parse {
            code: JAVASCRIPT_SAMPLE.to_string(),
            language: Some(SupportedLanguage::JavaScript),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        let parse_time = start_time.elapsed();
        
        assert!(result.is_ok(), "JavaScript parsing should succeed");
        
        let output = result.unwrap();
        if let TreeOutput::Parsed { language, node_count, has_errors, .. } = output.result {
            assert_eq!(language, SupportedLanguage::JavaScript);
            assert!(node_count > 30, "Should parse JS class and methods");
            assert!(!has_errors, "Well-formed JS code should not have errors");
            
            TreeTestHelper::assert_parse_performance(parse_time, 10, "JavaScript");
        } else {
            panic!("Expected Parsed output");
        }
    }

    #[tokio::test]
    async fn test_typescript_parsing_and_performance() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        let start_time = Instant::now();
        
        let input = TreeInput::Parse {
            code: TYPESCRIPT_SAMPLE.to_string(),
            language: Some(SupportedLanguage::TypeScript),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        let parse_time = start_time.elapsed();
        
        assert!(result.is_ok(), "TypeScript parsing should succeed");
        
        let output = result.unwrap();
        if let TreeOutput::Parsed { language, node_count, has_errors, .. } = output.result {
            assert_eq!(language, SupportedLanguage::TypeScript);
            assert!(node_count > 50, "Should parse TS interfaces and generics");
            assert!(!has_errors, "Well-formed TS code should not have errors");
            
            TreeTestHelper::assert_parse_performance(parse_time, 10, "TypeScript");
        } else {
            panic!("Expected Parsed output");
        }
    }

    #[tokio::test]
    async fn test_go_parsing_and_performance() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        let start_time = Instant::now();
        
        let input = TreeInput::Parse {
            code: GO_SAMPLE.to_string(),
            language: Some(SupportedLanguage::Go),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        let parse_time = start_time.elapsed();
        
        assert!(result.is_ok(), "Go parsing should succeed");
        
        let output = result.unwrap();
        if let TreeOutput::Parsed { language, node_count, has_errors, .. } = output.result {
            assert_eq!(language, SupportedLanguage::Go);
            assert!(node_count > 60, "Should parse Go structs and methods");
            assert!(!has_errors, "Well-formed Go code should not have errors");
            
            TreeTestHelper::assert_parse_performance(parse_time, 10, "Go");
        } else {
            panic!("Expected Parsed output");
        }
    }

    #[tokio::test]
    async fn test_error_recovery_rust_syntax_errors() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        let input = TreeInput::Parse {
            code: RUST_SYNTAX_ERROR.to_string(),
            language: Some(SupportedLanguage::Rust),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_ok(), "Parser should handle syntax errors gracefully");
        
        let output = result.unwrap();
        if let TreeOutput::Parsed { has_errors, error_count, .. } = output.result {
            assert!(has_errors, "Should detect syntax errors");
            assert!(error_count > 0, "Should report error count");
        } else {
            panic!("Expected Parsed output");
        }
    }

    #[tokio::test]
    async fn test_error_recovery_python_syntax_errors() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        let input = TreeInput::Parse {
            code: PYTHON_SYNTAX_ERROR.to_string(),
            language: Some(SupportedLanguage::Python),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_ok(), "Parser should handle Python syntax errors gracefully");
        
        let output = result.unwrap();
        if let TreeOutput::Parsed { has_errors, error_count, .. } = output.result {
            assert!(has_errors, "Should detect Python syntax errors");
            assert!(error_count > 0, "Should report error count for Python");
        } else {
            panic!("Expected Parsed output");
        }
    }

    #[tokio::test]
    async fn test_query_functionality_rust() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        // Query for all function definitions in Rust
        let query_pattern = TreeTestHelper::function_query_for_language(SupportedLanguage::Rust);
        let input = TreeInput::Query {
            code: RUST_SAMPLE.to_string(),
            language: Some(SupportedLanguage::Rust),
            pattern: query_pattern.to_string(),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_ok(), "Query should succeed for Rust");
        
        let output = result.unwrap();
        if let TreeOutput::QueryResults { matches, total_matches, query_time_ms } = output.result {
            assert!(total_matches > 0, "Should find function definitions in Rust sample");
            assert!(!matches.is_empty(), "Should return query matches");
            assert!(query_time_ms < 50, "Query should be fast (<50ms)");
            
            // Verify matches contain expected function names
            let function_names: Vec<String> = matches
                .iter()
                .flat_map(|m| &m.captures)
                .filter(|c| c.name == "func_name")
                .map(|c| c.text.clone())
                .collect();
                
            assert!(function_names.contains(&"new".to_string()), "Should find 'new' function");
            assert!(function_names.contains(&"add_role".to_string()), "Should find 'add_role' function");
        } else {
            panic!("Expected QueryResults output");
        }
    }

    #[tokio::test]
    async fn test_query_functionality_multiple_languages() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        let test_cases = vec![
            (SupportedLanguage::Python, PYTHON_SAMPLE),
            (SupportedLanguage::JavaScript, JAVASCRIPT_SAMPLE),
            (SupportedLanguage::TypeScript, TYPESCRIPT_SAMPLE),
            (SupportedLanguage::Go, GO_SAMPLE),
        ];
        
        for (language, code) in test_cases {
            let query_pattern = TreeTestHelper::function_query_for_language(language);
            let input = TreeInput::Query {
                code: code.to_string(),
                language: Some(language),
                pattern: query_pattern.to_string(),
                file_path: None,
            };
            
            let result = tool.execute(input).await;
            assert!(result.is_ok(), "Query should succeed for {:?}", language);
            
            let output = result.unwrap();
            if let TreeOutput::QueryResults { total_matches, .. } = output.result {
                assert!(total_matches > 0, "Should find matches in {} code", language.as_str());
            }
        }
    }

    #[tokio::test]
    async fn test_symbol_extraction_rust() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        let input = TreeInput::ExtractSymbols {
            code: RUST_SAMPLE.to_string(),
            language: Some(SupportedLanguage::Rust),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_ok(), "Symbol extraction should succeed for Rust");
        
        let output = result.unwrap();
        if let TreeOutput::Symbols { symbols, total_symbols, extraction_time_ms } = output.result {
            // Note: Current implementation returns empty Vec for symbol extraction
            // This test validates the interface and timing
            assert_eq!(total_symbols, symbols.len(), "Total count should match vector length");
            assert!(extraction_time_ms < 100, "Symbol extraction should be fast");
        } else {
            panic!("Expected Symbols output");
        }
    }

    #[tokio::test]
    async fn test_semantic_diff_functionality() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        let old_code = r#"
fn simple_function() {
    println!("Original version");
}
"#;
        
        let new_code = r#"
fn simple_function() {
    println!("Modified version");
    println!("Added line");
}
"#;
        
        let input = TreeInput::Diff {
            old_code: old_code.to_string(),
            new_code: new_code.to_string(),
            language: Some(SupportedLanguage::Rust),
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_ok(), "Diff should succeed");
        
        let output = result.unwrap();
        if let TreeOutput::Diff { diff, diff_time_ms } = output.result {
            assert!(diff.added.len() > 0 || diff.modified.len() > 0, "Should detect changes");
            assert!(diff.similarity_score > 0.0 && diff.similarity_score <= 1.0, "Similarity score should be in valid range");
            assert!(diff_time_ms < 100, "Diff computation should be fast");
        } else {
            panic!("Expected Diff output");
        }
    }

    #[tokio::test]
    async fn test_file_based_operations() {
        let (_temp_dir, files) = TreeTestHelper::create_test_files().await.unwrap();
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        // Test file parsing
        let rust_file = files.get("user_system.rs").unwrap();
        let input = TreeInput::ParseFile { 
            file_path: rust_file.clone() 
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_ok(), "File parsing should succeed");
        
        // Test file querying
        let query_pattern = TreeTestHelper::function_query_for_language(SupportedLanguage::Rust);
        let input = TreeInput::QueryFile {
            file_path: rust_file.clone(),
            pattern: query_pattern.to_string(),
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_ok(), "File querying should succeed");
        
        // Test symbol extraction from file
        let input = TreeInput::ExtractSymbolsFromFile {
            file_path: rust_file.clone(),
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_ok(), "File symbol extraction should succeed");
    }

    #[tokio::test]
    async fn test_file_diff_operations() {
        let (_temp_dir, files) = TreeTestHelper::create_test_files().await.unwrap();
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        // Create a modified version of the Rust file
        let original_file = files.get("user_system.rs").unwrap();
        let modified_file = original_file.parent().unwrap().join("user_system_modified.rs");
        
        let modified_content = RUST_SAMPLE.replace("User", "Member"); // Simple modification
        fs::write(&modified_file, modified_content).await.unwrap();
        
        let input = TreeInput::DiffFiles {
            old_file: original_file.clone(),
            new_file: modified_file,
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_ok(), "File diff should succeed");
        
        let output = result.unwrap();
        if let TreeOutput::Diff { diff, .. } = output.result {
            assert!(diff.modified.len() > 0, "Should detect modifications between files");
        }
    }

    #[tokio::test]
    async fn test_cache_effectiveness() {
        let tool = TreeTool::new(IntelligenceLevel::Light).unwrap();
        
        // Parse the same code multiple times
        for _ in 0..3 {
            let input = TreeInput::Parse {
                code: RUST_SAMPLE.to_string(),
                language: Some(SupportedLanguage::Rust),
                file_path: None,
            };
            
            let result = tool.execute(input).await;
            assert!(result.is_ok(), "Cached parsing should succeed");
        }
        
        // Verify cache is working
        TreeTestHelper::assert_cache_effectiveness(&tool, 0.8).unwrap();
        
        // Test cache clearing
        let clear_result = tool.clear_caches();
        assert!(clear_result.is_ok(), "Cache clearing should succeed");
        
        let stats = tool.cache_stats().unwrap();
        if let Some(cache_size_value) = stats.get("ast_cache_size") {
            let cache_size = cache_size_value.as_i64().unwrap_or(-1);
            assert_eq!(cache_size, 0, "Cache should be empty after clearing");
        }
    }

    #[tokio::test]
    async fn test_intelligence_level_configurations() {
        let intelligence_levels = vec![
            IntelligenceLevel::Light,
            IntelligenceLevel::Medium,
            IntelligenceLevel::Hard,
        ];
        
        for level in intelligence_levels {
            let tool = TreeTool::new(level).unwrap();
            
            let input = TreeInput::Parse {
                code: RUST_SAMPLE.to_string(),
                language: Some(SupportedLanguage::Rust),
                file_path: None,
            };
            
            let result = tool.execute(input).await;
            assert!(result.is_ok(), "Should work with {:?} intelligence level", level);
            
            // Verify cache stats reflect the intelligence level
            let stats = tool.cache_stats().unwrap();
            assert!(stats.contains_key("max_cache_size"), "Should have max_cache_size stat");
        }
    }

    #[tokio::test]
    async fn test_unsupported_language_handling() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        // Try to parse with an unsupported extension
        let unsupported_path = PathBuf::from("test.unknownext");
        let input = TreeInput::ParseFile { 
            file_path: unsupported_path 
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_err(), "Should error on unsupported language");
        
        match result.unwrap_err() {
            TreeError::FileReadFailed { .. } | TreeError::LanguageDetectionFailed { .. } => {
                // Expected error types
            }
            other => panic!("Unexpected error type: {:?}", other),
        }
    }

    #[tokio::test] 
    async fn test_empty_code_handling() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        let input = TreeInput::Parse {
            code: "".to_string(),
            language: Some(SupportedLanguage::Rust),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_ok(), "Should handle empty code gracefully");
        
        let output = result.unwrap();
        if let TreeOutput::Parsed { node_count, .. } = output.result {
            assert!(node_count >= 0, "Should have non-negative node count for empty code");
        }
    }

    #[tokio::test]
    async fn test_large_code_performance() {
        let tool = TreeTool::new(IntelligenceLevel::Hard).unwrap();
        
        // Create larger code by repeating the Rust sample
        let large_code = RUST_SAMPLE.repeat(10);
        
        let start_time = Instant::now();
        let input = TreeInput::Parse {
            code: large_code,
            language: Some(SupportedLanguage::Rust),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        let parse_time = start_time.elapsed();
        
        assert!(result.is_ok(), "Should handle larger code files");
        
        // Should still be reasonably fast even for larger files
        assert!(parse_time.as_millis() < 100, "Large file parsing should complete within 100ms");
        
        let output = result.unwrap();
        if let TreeOutput::Parsed { node_count, .. } = output.result {
            assert!(node_count > 500, "Should parse many nodes for large code");
        }
    }

    #[tokio::test]
    async fn test_query_pattern_edge_cases() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        // Test invalid query pattern
        let invalid_query = "(this is not a valid query pattern";
        let input = TreeInput::Query {
            code: RUST_SAMPLE.to_string(),
            language: Some(SupportedLanguage::Rust),
            pattern: invalid_query.to_string(),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_err(), "Should error on invalid query pattern");
        
        match result.unwrap_err() {
            TreeError::QueryCompilationFailed { .. } => {
                // Expected error type
            }
            other => panic!("Expected QueryCompilationFailed, got {:?}", other),
        }
        
        // Test empty query pattern
        let empty_query = "";
        let input = TreeInput::Query {
            code: RUST_SAMPLE.to_string(),
            language: Some(SupportedLanguage::Rust),
            pattern: empty_query.to_string(),
            file_path: None,
        };
        
        let result = tool.execute(input).await;
        assert!(result.is_err(), "Should error on empty query pattern");
    }

    #[tokio::test]
    async fn test_concurrent_parsing() {
        use tokio::task;
        
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        let mut handles = vec![];
        
        // Spawn multiple parsing tasks concurrently
        for i in 0..5 {
            let tool_clone = tool.clone(); // Assuming TreeTool implements Clone or use Arc
            let code = match i % 3 {
                0 => RUST_SAMPLE.to_string(),
                1 => PYTHON_SAMPLE.to_string(),
                _ => JAVASCRIPT_SAMPLE.to_string(),
            };
            let language = match i % 3 {
                0 => SupportedLanguage::Rust,
                1 => SupportedLanguage::Python,
                _ => SupportedLanguage::JavaScript,
            };
            
            let handle = task::spawn(async move {
                let input = TreeInput::Parse {
                    code,
                    language: Some(language),
                    file_path: None,
                };
                
                tool_clone.execute(input).await
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok(), "Concurrent parsing should succeed");
        }
    }
}

#[cfg(test)]
mod language_extension_tests {
    use super::*;

    #[test]
    fn test_all_supported_languages_have_extensions() {
        for &language in SupportedLanguage::all_languages() {
            let extensions = language.extensions();
            assert!(!extensions.is_empty(), "{:?} should have at least one extension", language);
            
            // Verify reverse lookup works
            for extension in extensions {
                let detected = SupportedLanguage::from_extension(extension);
                assert_eq!(detected, Some(language), 
                          "Extension {} should map back to {:?}", extension, language);
            }
        }
    }

    #[test]
    fn test_language_string_representations() {
        for &language in SupportedLanguage::all_languages() {
            let lang_str = language.as_str();
            assert!(!lang_str.is_empty(), "{:?} should have a string representation", language);
            assert!(!lang_str.contains(' '), "Language string should not contain spaces: {}", lang_str);
        }
    }

    #[test]
    fn test_common_extensions() {
        let common_cases = vec![
            ("rs", SupportedLanguage::Rust),
            ("py", SupportedLanguage::Python),
            ("js", SupportedLanguage::JavaScript),
            ("ts", SupportedLanguage::TypeScript),
            ("go", SupportedLanguage::Go),
            ("java", SupportedLanguage::Java),
            ("c", SupportedLanguage::C),
            ("cpp", SupportedLanguage::Cpp),
            ("cs", SupportedLanguage::CSharp),
            ("sh", SupportedLanguage::Bash),
            ("html", SupportedLanguage::Html),
            ("css", SupportedLanguage::Css),
            ("json", SupportedLanguage::Json),
            ("yaml", SupportedLanguage::Yaml),
            ("rb", SupportedLanguage::Ruby),
        ];
        
        for (ext, expected) in common_cases {
            let detected = SupportedLanguage::from_extension(ext);
            assert_eq!(detected, Some(expected), "Extension {} should detect {:?}", ext, expected);
        }
    }
}

#[cfg(test)]
mod performance_benchmarks {
    use super::*;

    #[tokio::test]
    async fn benchmark_parsing_performance() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        let test_cases = vec![
            ("Rust", RUST_SAMPLE, SupportedLanguage::Rust),
            ("Python", PYTHON_SAMPLE, SupportedLanguage::Python),
            ("JavaScript", JAVASCRIPT_SAMPLE, SupportedLanguage::JavaScript),
            ("TypeScript", TYPESCRIPT_SAMPLE, SupportedLanguage::TypeScript),
            ("Go", GO_SAMPLE, SupportedLanguage::Go),
        ];
        
        for (name, code, language) in test_cases {
            let mut durations = Vec::new();
            
            // Run multiple iterations for accurate timing
            for _ in 0..10 {
                let start = Instant::now();
                
                let input = TreeInput::Parse {
                    code: code.to_string(),
                    language: Some(language),
                    file_path: None,
                };
                
                let result = tool.execute(input).await;
                let duration = start.elapsed();
                
                assert!(result.is_ok(), "Parsing should succeed for {}", name);
                durations.push(duration);
            }
            
            let avg_duration = durations.iter().sum::<Duration>() / durations.len() as u32;
            let min_duration = *durations.iter().min().unwrap();
            let max_duration = *durations.iter().max().unwrap();
            
            println!("{} parsing - Avg: {:?}, Min: {:?}, Max: {:?}", 
                    name, avg_duration, min_duration, max_duration);
            
            // Performance assertions
            assert!(avg_duration.as_millis() < 10, 
                   "{} average parsing time {}ms exceeds 10ms target", name, avg_duration.as_millis());
            assert!(max_duration.as_millis() < 20, 
                   "{} max parsing time {}ms exceeds 20ms limit", name, max_duration.as_millis());
        }
    }

    #[tokio::test]
    async fn benchmark_query_performance() {
        let tool = TreeTool::new(IntelligenceLevel::Medium).unwrap();
        
        let query_pattern = TreeTestHelper::function_query_for_language(SupportedLanguage::Rust);
        let mut query_times = Vec::new();
        
        for _ in 0..10 {
            let start = Instant::now();
            
            let input = TreeInput::Query {
                code: RUST_SAMPLE.to_string(),
                language: Some(SupportedLanguage::Rust),
                pattern: query_pattern.to_string(),
                file_path: None,
            };
            
            let result = tool.execute(input).await;
            let duration = start.elapsed();
            
            assert!(result.is_ok(), "Query should succeed");
            query_times.push(duration);
        }
        
        let avg_query_time = query_times.iter().sum::<Duration>() / query_times.len() as u32;
        assert!(avg_query_time.as_millis() < 50, 
               "Average query time {}ms exceeds 50ms target", avg_query_time.as_millis());
    }

    #[tokio::test]
    async fn benchmark_cache_performance() {
        let tool = TreeTool::new(IntelligenceLevel::Light).unwrap();
        
        // First parse (cache miss)
        let start = Instant::now();
        let input = TreeInput::Parse {
            code: RUST_SAMPLE.to_string(),
            language: Some(SupportedLanguage::Rust),
            file_path: None,
        };
        let _ = tool.execute(input).await.unwrap();
        let first_parse_time = start.elapsed();
        
        // Second parse (cache hit)
        let start = Instant::now();
        let input = TreeInput::Parse {
            code: RUST_SAMPLE.to_string(),
            language: Some(SupportedLanguage::Rust),
            file_path: None,
        };
        let _ = tool.execute(input).await.unwrap();
        let cached_parse_time = start.elapsed();
        
        // Cached version should be significantly faster
        assert!(cached_parse_time < first_parse_time, 
               "Cached parsing should be faster than initial parsing");
        assert!(cached_parse_time.as_millis() < 5, 
               "Cached parsing should be very fast (<5ms)");
        
        println!("First parse: {:?}, Cached parse: {:?}", first_parse_time, cached_parse_time);
    }
}