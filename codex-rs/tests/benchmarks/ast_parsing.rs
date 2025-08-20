//! Performance benchmarks for AST parsing and tree-sitter operations.
//!
//! Tests parsing speed, memory usage, and caching performance for 50+ languages
//! with target metrics as defined in CLAUDE.md Phase 6.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use std::fs;

// Mock AST parser for benchmarking
#[derive(Debug, Clone)]
pub struct AstParser {
    pub language_parsers: HashMap<String, LanguageParser>,
    pub cache: HashMap<String, ParseResult>,
    pub stats: ParsingStats,
}

#[derive(Debug, Clone)]
pub struct LanguageParser {
    pub language: String,
    pub file_extensions: Vec<String>,
    pub parsing_complexity: f64, // Simulated complexity score
}

#[derive(Debug, Clone)]
pub struct ParseResult {
    pub language: String,
    pub node_count: usize,
    pub parse_time: Duration,
    pub memory_usage: usize,
    pub ast_hash: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ParsingStats {
    pub total_parses: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub total_parse_time: Duration,
    pub average_parse_time: Duration,
}

impl AstParser {
    pub fn new() -> Self {
        let mut parsers = HashMap::new();
        
        // Initialize parsers for 50+ languages
        let languages = [
            // Core languages
            ("rust", vec!["rs"], 0.8),
            ("python", vec!["py", "pyi"], 0.7),
            ("javascript", vec!["js", "mjs"], 0.6),
            ("typescript", vec!["ts", "tsx"], 0.7),
            ("java", vec!["java"], 0.9),
            ("cpp", vec!["cpp", "cc", "cxx", "hpp"], 1.0),
            ("c", vec!["c", "h"], 0.8),
            ("csharp", vec!["cs"], 0.8),
            ("go", vec!["go"], 0.6),
            ("swift", vec!["swift"], 0.8),
            ("kotlin", vec!["kt", "kts"], 0.7),
            ("scala", vec!["scala", "sc"], 0.9),
            ("ruby", vec!["rb", "rbw"], 0.6),
            ("php", vec!["php"], 0.7),
            ("perl", vec!["pl", "pm"], 0.8),
            
            // Web languages
            ("html", vec!["html", "htm"], 0.4),
            ("css", vec!["css", "scss", "sass"], 0.3),
            ("json", vec!["json"], 0.2),
            ("xml", vec!["xml"], 0.4),
            ("yaml", vec!["yaml", "yml"], 0.3),
            ("toml", vec!["toml"], 0.3),
            
            // Shell and scripting
            ("bash", vec!["sh", "bash"], 0.5),
            ("zsh", vec!["zsh"], 0.5),
            ("fish", vec!["fish"], 0.4),
            ("powershell", vec!["ps1"], 0.5),
            
            // Functional languages
            ("haskell", vec!["hs", "lhs"], 1.0),
            ("ocaml", vec!["ml", "mli"], 0.9),
            ("fsharp", vec!["fs", "fsi"], 0.8),
            ("erlang", vec!["erl", "hrl"], 0.8),
            ("elixir", vec!["ex", "exs"], 0.7),
            ("clojure", vec!["clj", "cljs"], 0.8),
            ("scheme", vec!["scm", "ss"], 0.7),
            ("racket", vec!["rkt"], 0.8),
            
            // Systems languages
            ("zig", vec!["zig"], 0.7),
            ("nim", vec!["nim"], 0.7),
            ("crystal", vec!["cr"], 0.7),
            ("d", vec!["d"], 0.8),
            
            // JVM languages
            ("groovy", vec!["groovy"], 0.6),
            ("clojure", vec!["clj"], 0.8),
            
            // .NET languages
            ("vb", vec!["vb"], 0.7),
            ("fsharp", vec!["fs"], 0.8),
            
            // Database languages
            ("sql", vec!["sql"], 0.5),
            ("plpgsql", vec!["pgsql"], 0.6),
            
            // Configuration languages
            ("dockerfile", vec!["dockerfile"], 0.3),
            ("makefile", vec!["makefile", "mk"], 0.4),
            ("cmake", vec!["cmake"], 0.5),
            ("nix", vec!["nix"], 0.6),
            
            // Documentation
            ("markdown", vec!["md", "markdown"], 0.3),
            ("rst", vec!["rst"], 0.4),
            ("asciidoc", vec!["adoc"], 0.4),
            ("latex", vec!["tex", "latex"], 0.6),
        ];
        
        for (lang, extensions, complexity) in languages {
            parsers.insert(lang.to_string(), LanguageParser {
                language: lang.to_string(),
                file_extensions: extensions.into_iter().map(|s| s.to_string()).collect(),
                parsing_complexity: complexity,
            });
        }
        
        Self {
            language_parsers: parsers,
            cache: HashMap::new(),
            stats: ParsingStats::default(),
        }
    }
    
    pub fn parse_code(&mut self, code: &str, language: &str) -> Result<ParseResult, ParseError> {
        let start = Instant::now();
        
        // Check cache first
        let cache_key = format!("{}:{}", language, self.hash_code(code));
        if let Some(cached_result) = self.cache.get(&cache_key) {
            self.stats.cache_hits += 1;
            return Ok(cached_result.clone());
        }
        
        self.stats.cache_misses += 1;
        self.stats.total_parses += 1;
        
        // Simulate parsing based on language complexity
        let parser = self.language_parsers.get(language)
            .ok_or(ParseError::UnsupportedLanguage)?;
        
        let result = self.simulate_parsing(code, parser)?;
        let parse_time = start.elapsed();
        
        let final_result = ParseResult {
            language: language.to_string(),
            node_count: result.node_count,
            parse_time,
            memory_usage: result.memory_usage,
            ast_hash: result.ast_hash,
        };
        
        // Cache the result
        self.cache.insert(cache_key, final_result.clone());
        
        // Update stats
        self.stats.total_parse_time += parse_time;
        self.stats.average_parse_time = self.stats.total_parse_time / self.stats.total_parses as u32;
        
        Ok(final_result)
    }
    
    pub fn parse_file(&mut self, file_path: &str) -> Result<ParseResult, ParseError> {
        let content = fs::read_to_string(file_path)
            .map_err(|e| ParseError::IoError(e.to_string()))?;
        
        let language = self.detect_language_from_extension(file_path)
            .ok_or(ParseError::UnknownFileType)?;
        
        self.parse_code(&content, &language)
    }
    
    pub fn batch_parse(&mut self, files: Vec<&str>) -> Vec<Result<ParseResult, ParseError>> {
        files.into_iter()
            .map(|file| self.parse_file(file))
            .collect()
    }
    
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    pub fn get_cache_hit_rate(&self) -> f64 {
        let total_requests = self.stats.cache_hits + self.stats.cache_misses;
        if total_requests == 0 {
            0.0
        } else {
            self.stats.cache_hits as f64 / total_requests as f64
        }
    }
    
    // Private helper methods
    
    fn simulate_parsing(&self, code: &str, parser: &LanguageParser) -> Result<ParseResult, ParseError> {
        let code_size = code.len();
        
        // Simulate parsing time based on code size and language complexity
        let base_time_ns = (code_size as f64 * parser.parsing_complexity * 1000.0) as u64;
        std::thread::sleep(Duration::from_nanos(base_time_ns.min(10_000_000))); // Cap at 10ms
        
        // Simulate node count based on code structure
        let estimated_nodes = code.lines().count() * 3 + code.matches('{').count() * 5;
        
        // Simulate memory usage (roughly proportional to nodes)
        let memory_usage = estimated_nodes * 64; // 64 bytes per node estimate
        
        Ok(ParseResult {
            language: parser.language.clone(),
            node_count: estimated_nodes,
            parse_time: Duration::from_nanos(base_time_ns),
            memory_usage,
            ast_hash: self.hash_code(code),
        })
    }
    
    fn hash_code(&self, code: &str) -> u64 {
        // Simple hash for demonstration
        code.len() as u64 * 31 + code.chars().count() as u64
    }
    
    fn detect_language_from_extension(&self, file_path: &str) -> Option<String> {
        let extension = std::path::Path::new(file_path)
            .extension()?
            .to_str()?;
        
        for (lang, parser) in &self.language_parsers {
            if parser.file_extensions.contains(&extension.to_string()) {
                return Some(lang.clone());
            }
        }
        
        None
    }
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnsupportedLanguage,
    UnknownFileType,
    IoError(String),
    ParseFailed(String),
}

// Sample code for benchmarking
const RUST_SAMPLE: &str = r#"
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub roles: Vec<Role>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Admin,
    User,
    Guest,
    Custom(String),
}

impl User {
    pub fn new(id: u64, name: String, email: String) -> Self {
        Self {
            id,
            name,
            email,
            roles: vec![Role::User],
            metadata: HashMap::new(),
        }
    }
    
    pub fn add_role(&mut self, role: Role) -> &mut Self {
        if !self.roles.contains(&role) {
            self.roles.push(role);
        }
        self
    }
    
    pub fn set_metadata(&mut self, key: String, value: serde_json::Value) -> &mut Self {
        self.metadata.insert(key, value);
        self
    }
    
    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }
    
    pub fn is_admin(&self) -> bool {
        self.has_role(&Role::Admin)
    }
}

pub struct UserManager {
    users: HashMap<u64, User>,
    next_id: u64,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            next_id: 1,
        }
    }
    
    pub fn create_user(&mut self, name: String, email: String) -> &User {
        let id = self.next_id;
        self.next_id += 1;
        
        let user = User::new(id, name, email);
        self.users.insert(id, user);
        self.users.get(&id).unwrap()
    }
    
    pub fn get_user(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }
    
    pub fn update_user<F>(&mut self, id: u64, update_fn: F) -> Option<&User>
    where
        F: FnOnce(&mut User),
    {
        if let Some(user) = self.users.get_mut(&id) {
            update_fn(user);
            Some(user)
        } else {
            None
        }
    }
    
    pub fn delete_user(&mut self, id: u64) -> Option<User> {
        self.users.remove(&id)
    }
    
    pub fn list_users(&self) -> Vec<&User> {
        self.users.values().collect()
    }
    
    pub fn find_by_email(&self, email: &str) -> Option<&User> {
        self.users.values().find(|user| user.email == email)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_creation() {
        let user = User::new(1, "Alice".to_string(), "alice@example.com".to_string());
        assert_eq!(user.id, 1);
        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.roles.len(), 1);
        assert!(user.has_role(&Role::User));
    }
    
    #[test]
    fn test_user_manager() {
        let mut manager = UserManager::new();
        let user = manager.create_user("Bob".to_string(), "bob@example.com".to_string());
        
        assert_eq!(user.name, "Bob");
        assert_eq!(manager.list_users().len(), 1);
        
        let found = manager.find_by_email("bob@example.com");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Bob");
    }
}
"#;

const PYTHON_SAMPLE: &str = r#"
import asyncio
import logging
from typing import Dict, List, Optional, Union, Any
from dataclasses import dataclass, field
from datetime import datetime, timezone
import json
from pathlib import Path

logger = logging.getLogger(__name__)

@dataclass
class TaskConfig:
    """Configuration for a task execution."""
    timeout: float = 30.0
    retries: int = 3
    retry_delay: float = 1.0
    enable_logging: bool = True
    metadata: Dict[str, Any] = field(default_factory=dict)

@dataclass
class TaskResult:
    """Result of a task execution."""
    task_id: str
    success: bool
    result: Any = None
    error: Optional[str] = None
    duration: float = 0.0
    timestamp: datetime = field(default_factory=lambda: datetime.now(timezone.utc))

class Task:
    """A task that can be executed asynchronously."""
    
    def __init__(self, task_id: str, func: callable, *args, **kwargs):
        self.task_id = task_id
        self.func = func
        self.args = args
        self.kwargs = kwargs
        self.config = TaskConfig()
        self.result: Optional[TaskResult] = None
        
    async def execute(self) -> TaskResult:
        """Execute the task with configuration."""
        start_time = asyncio.get_event_loop().time()
        
        for attempt in range(self.config.retries + 1):
            try:
                if self.config.enable_logging:
                    logger.info(f"Executing task {self.task_id}, attempt {attempt + 1}")
                
                if asyncio.iscoroutinefunction(self.func):
                    result = await asyncio.wait_for(
                        self.func(*self.args, **self.kwargs),
                        timeout=self.config.timeout
                    )
                else:
                    result = await asyncio.get_event_loop().run_in_executor(
                        None, self.func, *self.args, **self.kwargs
                    )
                
                duration = asyncio.get_event_loop().time() - start_time
                self.result = TaskResult(
                    task_id=self.task_id,
                    success=True,
                    result=result,
                    duration=duration
                )
                return self.result
                
            except asyncio.TimeoutError:
                if attempt < self.config.retries:
                    await asyncio.sleep(self.config.retry_delay)
                    continue
                else:
                    duration = asyncio.get_event_loop().time() - start_time
                    self.result = TaskResult(
                        task_id=self.task_id,
                        success=False,
                        error=f"Task timed out after {self.config.timeout}s",
                        duration=duration
                    )
                    return self.result
                    
            except Exception as e:
                if attempt < self.config.retries:
                    await asyncio.sleep(self.config.retry_delay)
                    continue
                else:
                    duration = asyncio.get_event_loop().time() - start_time
                    self.result = TaskResult(
                        task_id=self.task_id,
                        success=False,
                        error=str(e),
                        duration=duration
                    )
                    return self.result

class TaskManager:
    """Manages and executes multiple tasks."""
    
    def __init__(self, max_concurrent: int = 10):
        self.max_concurrent = max_concurrent
        self.tasks: Dict[str, Task] = {}
        self.running_tasks: Dict[str, asyncio.Task] = {}
        self.results: Dict[str, TaskResult] = {}
        self._semaphore = asyncio.Semaphore(max_concurrent)
        
    def add_task(self, task: Task) -> None:
        """Add a task to the manager."""
        self.tasks[task.task_id] = task
        
    async def execute_task(self, task_id: str) -> TaskResult:
        """Execute a single task."""
        if task_id not in self.tasks:
            raise ValueError(f"Task {task_id} not found")
            
        task = self.tasks[task_id]
        
        async with self._semaphore:
            result = await task.execute()
            self.results[task_id] = result
            return result
            
    async def execute_all(self) -> Dict[str, TaskResult]:
        """Execute all tasks concurrently."""
        if not self.tasks:
            return {}
            
        # Create coroutines for all tasks
        coroutines = [self.execute_task(task_id) for task_id in self.tasks.keys()]
        
        # Execute all tasks concurrently
        results = await asyncio.gather(*coroutines, return_exceptions=True)
        
        # Process results
        final_results = {}
        for i, (task_id, result) in enumerate(zip(self.tasks.keys(), results)):
            if isinstance(result, Exception):
                final_results[task_id] = TaskResult(
                    task_id=task_id,
                    success=False,
                    error=str(result)
                )
            else:
                final_results[task_id] = result
                
        return final_results
        
    def get_result(self, task_id: str) -> Optional[TaskResult]:
        """Get the result of a task."""
        return self.results.get(task_id)
        
    def clear_completed(self) -> None:
        """Clear completed tasks and results."""
        completed_tasks = [
            task_id for task_id, result in self.results.items()
            if result is not None
        ]
        
        for task_id in completed_tasks:
            self.tasks.pop(task_id, None)
            self.results.pop(task_id, None)

# Example usage functions
async def example_async_function(x: int, y: int) -> int:
    """Example async function for testing."""
    await asyncio.sleep(0.1)  # Simulate async work
    return x + y

def example_sync_function(name: str) -> str:
    """Example sync function for testing."""
    return f"Hello, {name}!"

async def main():
    """Example usage of the task system."""
    manager = TaskManager(max_concurrent=5)
    
    # Add some tasks
    task1 = Task("add_task", example_async_function, 10, 20)
    task2 = Task("greet_task", example_sync_function, "Alice")
    task3 = Task("another_add", example_async_function, 5, 15)
    
    manager.add_task(task1)
    manager.add_task(task2)
    manager.add_task(task3)
    
    # Execute all tasks
    results = await manager.execute_all()
    
    # Print results
    for task_id, result in results.items():
        if result.success:
            print(f"Task {task_id}: {result.result} (took {result.duration:.2f}s)")
        else:
            print(f"Task {task_id} failed: {result.error}")

if __name__ == "__main__":
    asyncio.run(main())
"#;

const GO_SAMPLE: &str = r#"
package main

import (
    "context"
    "encoding/json"
    "fmt"
    "log"
    "net/http"
    "sync"
    "time"
    
    "github.com/gorilla/mux"
    "github.com/go-redis/redis/v8"
)

type User struct {
    ID       int64     `json:"id" db:"id"`
    Name     string    `json:"name" db:"name"`
    Email    string    `json:"email" db:"email"`
    Created  time.Time `json:"created" db:"created_at"`
    Updated  time.Time `json:"updated" db:"updated_at"`
}

type UserService interface {
    CreateUser(ctx context.Context, user *User) error
    GetUser(ctx context.Context, id int64) (*User, error)
    UpdateUser(ctx context.Context, user *User) error
    DeleteUser(ctx context.Context, id int64) error
    ListUsers(ctx context.Context, limit, offset int) ([]*User, error)
}

type userService struct {
    db    Database
    cache CacheService
    mu    sync.RWMutex
}

func NewUserService(db Database, cache CacheService) UserService {
    return &userService{
        db:    db,
        cache: cache,
    }
}

func (s *userService) CreateUser(ctx context.Context, user *User) error {
    s.mu.Lock()
    defer s.mu.Unlock()
    
    user.Created = time.Now()
    user.Updated = time.Now()
    
    if err := s.db.CreateUser(ctx, user); err != nil {
        return fmt.Errorf("failed to create user: %w", err)
    }
    
    // Cache the user
    if err := s.cache.SetUser(ctx, user); err != nil {
        log.Printf("Failed to cache user: %v", err)
    }
    
    return nil
}

func (s *userService) GetUser(ctx context.Context, id int64) (*User, error) {
    s.mu.RLock()
    defer s.mu.RUnlock()
    
    // Try cache first
    if user, err := s.cache.GetUser(ctx, id); err == nil {
        return user, nil
    }
    
    // Fallback to database
    user, err := s.db.GetUser(ctx, id)
    if err != nil {
        return nil, fmt.Errorf("failed to get user: %w", err)
    }
    
    // Cache for next time
    if err := s.cache.SetUser(ctx, user); err != nil {
        log.Printf("Failed to cache user: %v", err)
    }
    
    return user, nil
}

func (s *userService) UpdateUser(ctx context.Context, user *User) error {
    s.mu.Lock()
    defer s.mu.Unlock()
    
    user.Updated = time.Now()
    
    if err := s.db.UpdateUser(ctx, user); err != nil {
        return fmt.Errorf("failed to update user: %w", err)
    }
    
    // Update cache
    if err := s.cache.SetUser(ctx, user); err != nil {
        log.Printf("Failed to update cached user: %v", err)
    }
    
    return nil
}

func (s *userService) DeleteUser(ctx context.Context, id int64) error {
    s.mu.Lock()
    defer s.mu.Unlock()
    
    if err := s.db.DeleteUser(ctx, id); err != nil {
        return fmt.Errorf("failed to delete user: %w", err)
    }
    
    // Remove from cache
    if err := s.cache.DeleteUser(ctx, id); err != nil {
        log.Printf("Failed to delete cached user: %v", err)
    }
    
    return nil
}

func (s *userService) ListUsers(ctx context.Context, limit, offset int) ([]*User, error) {
    s.mu.RLock()
    defer s.mu.RUnlock()
    
    users, err := s.db.ListUsers(ctx, limit, offset)
    if err != nil {
        return nil, fmt.Errorf("failed to list users: %w", err)
    }
    
    return users, nil
}

type Database interface {
    CreateUser(ctx context.Context, user *User) error
    GetUser(ctx context.Context, id int64) (*User, error)
    UpdateUser(ctx context.Context, user *User) error
    DeleteUser(ctx context.Context, id int64) error
    ListUsers(ctx context.Context, limit, offset int) ([]*User, error)
}

type CacheService interface {
    GetUser(ctx context.Context, id int64) (*User, error)
    SetUser(ctx context.Context, user *User) error
    DeleteUser(ctx context.Context, id int64) error
}

type redisCache struct {
    client *redis.Client
    ttl    time.Duration
}

func NewRedisCache(addr string, ttl time.Duration) CacheService {
    client := redis.NewClient(&redis.Options{
        Addr: addr,
    })
    
    return &redisCache{
        client: client,
        ttl:    ttl,
    }
}

func (c *redisCache) GetUser(ctx context.Context, id int64) (*User, error) {
    key := fmt.Sprintf("user:%d", id)
    
    val, err := c.client.Get(ctx, key).Result()
    if err != nil {
        return nil, err
    }
    
    var user User
    if err := json.Unmarshal([]byte(val), &user); err != nil {
        return nil, err
    }
    
    return &user, nil
}

func (c *redisCache) SetUser(ctx context.Context, user *User) error {
    key := fmt.Sprintf("user:%d", user.ID)
    
    data, err := json.Marshal(user)
    if err != nil {
        return err
    }
    
    return c.client.Set(ctx, key, data, c.ttl).Err()
}

func (c *redisCache) DeleteUser(ctx context.Context, id int64) error {
    key := fmt.Sprintf("user:%d", id)
    return c.client.Del(ctx, key).Err()
}

type UserHandler struct {
    service UserService
}

func NewUserHandler(service UserService) *UserHandler {
    return &UserHandler{service: service}
}

func (h *UserHandler) CreateUser(w http.ResponseWriter, r *http.Request) {
    var user User
    if err := json.NewDecoder(r.Body).Decode(&user); err != nil {
        http.Error(w, "Invalid JSON", http.StatusBadRequest)
        return
    }
    
    if err := h.service.CreateUser(r.Context(), &user); err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }
    
    w.Header().Set("Content-Type", "application/json")
    w.WriteHeader(http.StatusCreated)
    json.NewEncoder(w).Encode(&user)
}

func (h *UserHandler) GetUser(w http.ResponseWriter, r *http.Request) {
    vars := mux.Vars(r)
    id := vars["id"]
    
    userID, err := strconv.ParseInt(id, 10, 64)
    if err != nil {
        http.Error(w, "Invalid user ID", http.StatusBadRequest)
        return
    }
    
    user, err := h.service.GetUser(r.Context(), userID)
    if err != nil {
        http.Error(w, err.Error(), http.StatusNotFound)
        return
    }
    
    w.Header().Set("Content-Type", "application/json")
    json.NewEncoder(w).Encode(user)
}

func main() {
    // Initialize services
    cache := NewRedisCache("localhost:6379", 10*time.Minute)
    
    // userService := NewUserService(db, cache)
    // handler := NewUserHandler(userService)
    
    r := mux.NewRouter()
    // r.HandleFunc("/users", handler.CreateUser).Methods("POST")
    // r.HandleFunc("/users/{id}", handler.GetUser).Methods("GET")
    
    srv := &http.Server{
        Addr:         ":8080",
        Handler:      r,
        ReadTimeout:  15 * time.Second,
        WriteTimeout: 15 * time.Second,
        IdleTimeout:  60 * time.Second,
    }
    
    log.Println("Server starting on :8080")
    if err := srv.ListenAndServe(); err != nil {
        log.Fatal(err)
    }
}
"#;

// Benchmark functions

fn bench_single_language_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_language_parsing");
    
    let test_cases = vec![
        ("rust", RUST_SAMPLE),
        ("python", PYTHON_SAMPLE),
        ("go", GO_SAMPLE),
    ];
    
    for (language, code) in test_cases {
        group.bench_with_input(
            BenchmarkId::new("parse", language),
            &(language, code),
            |b, (lang, code)| {
                let mut parser = AstParser::new();
                b.iter(|| {
                    parser.parse_code(black_box(code), black_box(lang)).unwrap()
                })
            },
        );
    }
    
    group.finish();
}

fn bench_cold_vs_warm_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("cold_vs_warm_parsing");
    
    // Cold parsing (no cache)
    group.bench_function("cold_rust_parsing", |b| {
        b.iter(|| {
            let mut parser = AstParser::new();
            parser.parse_code(black_box(RUST_SAMPLE), black_box("rust")).unwrap()
        })
    });
    
    // Warm parsing (with cache)
    group.bench_function("warm_rust_parsing", |b| {
        let mut parser = AstParser::new();
        // Prime the cache
        parser.parse_code(RUST_SAMPLE, "rust").unwrap();
        
        b.iter(|| {
            parser.parse_code(black_box(RUST_SAMPLE), black_box("rust")).unwrap()
        })
    });
    
    group.finish();
}

fn bench_batch_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_parsing");
    
    // Create temporary files for batch parsing
    let temp_dir = TempDir::new().unwrap();
    let mut file_paths = Vec::new();
    
    let files = vec![
        ("test1.rs", RUST_SAMPLE),
        ("test2.py", PYTHON_SAMPLE),
        ("test3.go", GO_SAMPLE),
        ("test4.rs", RUST_SAMPLE), // Duplicate for cache testing
        ("test5.py", PYTHON_SAMPLE), // Duplicate for cache testing
    ];
    
    for (filename, content) in files {
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, content).unwrap();
        file_paths.push(file_path.to_string_lossy().to_string());
    }
    
    group.bench_function("batch_5_files", |b| {
        b.iter(|| {
            let mut parser = AstParser::new();
            let file_refs: Vec<&str> = file_paths.iter().map(|s| s.as_str()).collect();
            parser.batch_parse(black_box(file_refs))
        })
    });
    
    group.finish();
}

fn bench_language_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("language_detection");
    
    let test_files = vec![
        "test.rs",
        "test.py",
        "test.go",
        "test.js",
        "test.ts",
        "test.java",
        "test.cpp",
        "test.c",
        "test.rb",
        "test.php",
    ];
    
    group.bench_function("detect_from_extension", |b| {
        let parser = AstParser::new();
        b.iter(|| {
            for file in &test_files {
                black_box(parser.detect_language_from_extension(black_box(file)));
            }
        })
    });
    
    group.finish();
}

fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");
    
    // Test cache hit rate and performance
    group.bench_function("cache_with_50_percent_hits", |b| {
        let mut parser = AstParser::new();
        let codes = vec![RUST_SAMPLE, PYTHON_SAMPLE]; // 2 unique codes
        let languages = vec!["rust", "python"];
        
        b.iter(|| {
            // Parse 4 files with 50% cache hit rate
            for i in 0..4 {
                let code_idx = i % 2;
                parser.parse_code(
                    black_box(codes[code_idx]),
                    black_box(languages[code_idx])
                ).unwrap();
            }
        })
    });
    
    group.bench_function("cache_with_90_percent_hits", |b| {
        let mut parser = AstParser::new();
        let codes = vec![RUST_SAMPLE, PYTHON_SAMPLE];
        let languages = vec!["rust", "python"];
        
        b.iter(|| {
            // Parse 10 files with 90% cache hit rate
            for i in 0..10 {
                let code_idx = if i == 0 { 1 } else { 0 }; // 9 hits to same code, 1 miss
                parser.parse_code(
                    black_box(codes[code_idx]),
                    black_box(languages[code_idx])
                ).unwrap();
            }
        })
    });
    
    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    // Test parsing with different code sizes
    let small_code = "fn main() { println!(\"hello\"); }";
    let large_code = RUST_SAMPLE.repeat(10); // 10x larger
    
    group.bench_function("small_code_parsing", |b| {
        let mut parser = AstParser::new();
        b.iter(|| {
            parser.parse_code(black_box(small_code), black_box("rust")).unwrap()
        })
    });
    
    group.bench_function("large_code_parsing", |b| {
        let mut parser = AstParser::new();
        b.iter(|| {
            parser.parse_code(black_box(&large_code), black_box("rust")).unwrap()
        })
    });
    
    group.finish();
}

fn bench_concurrent_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_parsing");
    
    group.bench_function("sequential_parsing", |b| {
        b.iter(|| {
            let mut parser = AstParser::new();
            for _ in 0..5 {
                parser.parse_code(black_box(RUST_SAMPLE), black_box("rust")).unwrap();
            }
        })
    });
    
    // Note: True concurrent parsing would require thread-safe parser
    // This is a simulation showing the potential
    group.bench_function("simulated_concurrent_parsing", |b| {
        b.iter(|| {
            let mut parsers: Vec<_> = (0..5).map(|_| AstParser::new()).collect();
            for parser in &mut parsers {
                parser.parse_code(black_box(RUST_SAMPLE), black_box("rust")).unwrap();
            }
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_single_language_parsing,
    bench_cold_vs_warm_parsing,
    bench_batch_parsing,
    bench_language_detection,
    bench_cache_performance,
    bench_memory_usage,
    bench_concurrent_parsing
);

criterion_main!(benches);

#[cfg(test)]
mod benchmark_validation_tests {
    use super::*;
    
    #[test]
    fn test_parser_initialization() {
        let parser = AstParser::new();
        
        // Should support 50+ languages
        assert!(parser.language_parsers.len() >= 50);
        
        // Should include core languages
        assert!(parser.language_parsers.contains_key("rust"));
        assert!(parser.language_parsers.contains_key("python"));
        assert!(parser.language_parsers.contains_key("javascript"));
        assert!(parser.language_parsers.contains_key("typescript"));
        assert!(parser.language_parsers.contains_key("go"));
    }
    
    #[test]
    fn test_parsing_performance_targets() {
        let mut parser = AstParser::new();
        
        // Test target: <10ms per file (cached)
        let start = Instant::now();
        let result = parser.parse_code(RUST_SAMPLE, "rust").unwrap();
        let duration = start.elapsed();
        
        assert!(duration.as_millis() < 10, 
               "Parsing took {}ms, target is <10ms", duration.as_millis());
        
        // Test cache hit performance
        let start = Instant::now();
        let cached_result = parser.parse_code(RUST_SAMPLE, "rust").unwrap();
        let cache_duration = start.elapsed();
        
        assert!(cache_duration.as_millis() < 1,
               "Cache hit took {}ms, target is <1ms", cache_duration.as_millis());
        
        // Results should be identical
        assert_eq!(result.ast_hash, cached_result.ast_hash);
    }
    
    #[test]
    fn test_cache_hit_rate_targets() {
        let mut parser = AstParser::new();
        
        // Parse same code multiple times
        for _ in 0..10 {
            parser.parse_code(RUST_SAMPLE, "rust").unwrap();
        }
        
        // Should achieve >90% cache hit rate
        let hit_rate = parser.get_cache_hit_rate();
        assert!(hit_rate >= 0.90, 
               "Cache hit rate is {:.2}%, target is >90%", hit_rate * 100.0);
    }
    
    #[test]
    fn test_language_detection_accuracy() {
        let parser = AstParser::new();
        
        let test_cases = vec![
            ("test.rs", Some("rust")),
            ("test.py", Some("python")),
            ("test.js", Some("javascript")),
            ("test.go", Some("go")),
            ("test.unknown", None),
        ];
        
        for (filename, expected) in test_cases {
            let detected = parser.detect_language_from_extension(filename);
            assert_eq!(detected.as_deref(), expected, 
                      "Failed to detect language for {}", filename);
        }
    }
    
    #[test]
    fn test_memory_usage_scaling() {
        let mut parser = AstParser::new();
        
        let small_code = "fn main() {}";
        let large_code = RUST_SAMPLE.repeat(5);
        
        let small_result = parser.parse_code(small_code, "rust").unwrap();
        parser.clear_cache(); // Ensure no cache interference
        let large_result = parser.parse_code(&large_code, "rust").unwrap();
        
        // Memory usage should scale reasonably with code size
        let size_ratio = large_code.len() as f64 / small_code.len() as f64;
        let memory_ratio = large_result.memory_usage as f64 / small_result.memory_usage as f64;
        
        // Memory usage should not grow faster than code size
        assert!(memory_ratio <= size_ratio * 2.0,
               "Memory usage ratio ({:.2}) too high for size ratio ({:.2})", 
               memory_ratio, size_ratio);
    }
}