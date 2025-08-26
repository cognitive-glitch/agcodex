//! Advanced parser caching with lazy initialization and thread safety
//!
//! This module provides high-performance caching for tree-sitter parsers with:
//! - Lazy initialization of parsers (created only when needed)
//! - Thread-safe access with minimal locking overhead
//! - LRU eviction policy for memory management
//! - Parser pooling for high-concurrency scenarios
//!
//! # Architecture
//!
//! ```text
//! ParserPool
//! ├── LazyParser      - Lazy-initialized parser instances
//! ├── ThreadSafeCache - Lock-free parser caching  
//! ├── LRU Eviction    - Memory-efficient eviction
//! └── Pool Management - Parser instance pooling
//! ```
//!
//! # Usage
//!
//! ```rust
//! use agcodex_core::parsers::cache::{ParserPool, LazyParser};
//!
//! // Get a parser from the pool
//! let pool = ParserPool::global();
//! let parser = pool.get_parser(Language::Rust)?;
//!
//! // Use lazy parser for one-off parsing
//! let lazy_parser = LazyParser::new(Language::Python);
//! let tree = lazy_parser.parse("print('hello')")?;
//! ```

use super::Language;
use super::ParserError;
use dashmap::DashMap;
use lazy_static::lazy_static;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use tree_sitter::Parser;
use tree_sitter::Tree;

/// Thread-safe parser pool with lazy initialization
pub struct ParserPool {
    /// Available parsers by language
    parsers: DashMap<Language, Arc<Mutex<Vec<Parser>>>>,
    /// Maximum parsers per language
    max_parsers_per_language: usize,
    /// Statistics
    stats: Arc<RwLock<PoolStats>>,
}

impl ParserPool {
    /// Create a new parser pool
    pub fn new(max_parsers_per_language: usize) -> Self {
        Self {
            parsers: DashMap::new(),
            max_parsers_per_language,
            stats: Arc::new(RwLock::new(PoolStats::default())),
        }
    }

    /// Get the global parser pool instance
    pub fn global() -> &'static ParserPool {
        &GLOBAL_PARSER_POOL
    }

    /// Get a parser for the specified language (borrows from pool)
    pub fn get_parser(&self, language: Language) -> Result<PooledParser, ParserError> {
        // Try to get from pool first
        if let Some(parsers) = self.parsers.get(&language)
            && let Ok(mut parser_vec) = parsers.lock()
                && let Some(parser) = parser_vec.pop() {
                    // Update stats
                    if let Ok(mut stats) = self.stats.write() {
                        stats.hits += 1;
                        stats.active_parsers += 1;
                    }

                    return Ok(PooledParser::new(parser, language, self));
                }

        // Create new parser if pool is empty
        let parser = self.create_parser(language)?;

        // Update stats
        if let Ok(mut stats) = self.stats.write() {
            stats.misses += 1;
            stats.active_parsers += 1;
        }

        Ok(PooledParser::new(parser, language, self))
    }

    /// Create a new parser for the language
    fn create_parser(&self, language: Language) -> Result<Parser, ParserError> {
        let mut parser = Parser::new();
        let ts_language = language.to_tree_sitter()?;

        parser
            .set_language(&ts_language)
            .map_err(|e| ParserError::ParserCreationFailed {
                language: language.name().to_string(),
                details: e.to_string(),
            })?;

        Ok(parser)
    }

    /// Return a parser to the pool
    fn return_parser(&self, parser: Parser, language: Language) {
        let parsers = self
            .parsers
            .entry(language)
            .or_insert_with(|| Arc::new(Mutex::new(Vec::new())));

        if let Ok(mut parser_vec) = parsers.lock()
            && parser_vec.len() < self.max_parsers_per_language {
                parser_vec.push(parser);
            }
            // If pool is full, parser is dropped

        // Update stats
        if let Ok(mut stats) = self.stats.write() {
            stats.active_parsers = stats.active_parsers.saturating_sub(1);
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        self.stats
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Clear all parsers from the pool
    pub fn clear(&self) {
        self.parsers.clear();
        if let Ok(mut stats) = self.stats.write() {
            *stats = PoolStats::default();
        }
    }

    /// Get current pool sizes by language
    pub fn pool_sizes(&self) -> Vec<(Language, usize)> {
        self.parsers
            .iter()
            .map(|entry| {
                let language = *entry.key();
                let size = entry.value().lock().map(|vec| vec.len()).unwrap_or(0);
                (language, size)
            })
            .collect()
    }
}

/// RAII wrapper for pooled parsers
pub struct PooledParser<'a> {
    parser: Option<Parser>,
    language: Language,
    pool: &'a ParserPool,
}

impl<'a> PooledParser<'a> {
    const fn new(parser: Parser, language: Language, pool: &'a ParserPool) -> Self {
        Self {
            parser: Some(parser),
            language,
            pool,
        }
    }

    /// Parse source code
    pub fn parse(&mut self, source: &str, old_tree: Option<&Tree>) -> Result<Tree, ParserError> {
        self.parser
            .as_mut()
            .ok_or(ParserError::NoParserFound)?
            .parse(source, old_tree)
            .ok_or_else(|| ParserError::ParseFailed {
                reason: "Failed to parse source code".to_string(),
            })
    }

    /// Get the language of this parser
    pub const fn language(&self) -> Language {
        self.language
    }
}

impl<'a> Drop for PooledParser<'a> {
    fn drop(&mut self) {
        if let Some(parser) = self.parser.take() {
            self.pool.return_parser(parser, self.language);
        }
    }
}

/// Lazy-initialized parser for one-off use
/// Note: We create a new parser instance each time since Parser doesn't implement Clone
pub struct LazyParser {
    language: Language,
}

impl LazyParser {
    /// Create a new lazy parser
    pub const fn new(language: Language) -> Self {
        Self { language }
    }

    /// Get or create the parser - returns a new parser instance each time
    /// since tree_sitter::Parser doesn't implement Clone
    fn create_parser(&self) -> Result<Parser, ParserError> {
        let mut parser = Parser::new();
        let ts_language = self.language.to_tree_sitter()?;

        parser
            .set_language(&ts_language)
            .map_err(|e| ParserError::ParserCreationFailed {
                language: self.language.name().to_string(),
                details: e.to_string(),
            })?;

        Ok(parser)
    }

    /// Parse source code (creates parser if needed)
    pub fn parse(&self, source: &str) -> Result<Tree, ParserError> {
        let mut parser = self.create_parser()?;
        parser
            .parse(source, None)
            .ok_or_else(|| ParserError::ParseFailed {
                reason: "Failed to parse source code".to_string(),
            })
    }

    /// Parse with incremental updates
    pub fn parse_incremental(
        &self,
        source: &str,
        old_tree: Option<&Tree>,
    ) -> Result<Tree, ParserError> {
        let mut parser = self.create_parser()?;
        parser
            .parse(source, old_tree)
            .ok_or_else(|| ParserError::ParseFailed {
                reason: "Failed to parse source code incrementally".to_string(),
            })
    }

    /// Get the language
    pub const fn language(&self) -> Language {
        self.language
    }
}

impl Clone for LazyParser {
    fn clone(&self) -> Self {
        Self::new(self.language)
    }
}

/// Thread-safe cache for parsed trees with LRU eviction
pub struct ParseCache {
    /// Cached parsed trees: (Language, content_hash) -> Tree
    cache: Arc<Mutex<LruCache<(Language, u64), Tree>>>,
    /// Cache hit/miss statistics
    stats: Arc<RwLock<CacheStats>>,
}

impl ParseCache {
    /// Create a new parse cache with specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1000).unwrap()),
            ))),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Get cached tree if available
    pub fn get(&self, language: Language, content_hash: u64) -> Option<Tree> {
        let key = (language, content_hash);

        if let Ok(mut cache) = self.cache.lock()
            && let Some(tree) = cache.get(&key) {
                // Update stats
                if let Ok(mut stats) = self.stats.write() {
                    stats.hits += 1;
                }
                return Some(tree.clone());
            }

        // Update stats
        if let Ok(mut stats) = self.stats.write() {
            stats.misses += 1;
        }

        None
    }

    /// Store tree in cache
    pub fn put(&self, language: Language, content_hash: u64, tree: Tree) {
        let key = (language, content_hash);

        if let Ok(mut cache) = self.cache.lock() {
            cache.put(key, tree);

            // Update stats
            if let Ok(mut stats) = self.stats.write() {
                stats.entries = cache.len();
            }
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Clear the cache
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }

        if let Ok(mut stats) = self.stats.write() {
            *stats = CacheStats::default();
        }
    }

    /// Get current cache size
    pub fn len(&self) -> usize {
        self.cache.lock().map(|cache| cache.len()).unwrap_or(0)
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Statistics for parser pool
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of currently active parsers
    pub active_parsers: usize,
}

impl PoolStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Statistics for parse cache
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Current number of entries
    pub entries: usize,
}

impl CacheStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Global parser pool instance
lazy_static! {
    static ref GLOBAL_PARSER_POOL: ParserPool = ParserPool::new(4); // Max 4 parsers per language
}

/// Global parse cache instance
lazy_static! {
    pub static ref GLOBAL_PARSE_CACHE: ParseCache = ParseCache::new(1000); // Cache 1000 parsed trees
}

/// Utility function to hash source code content
pub fn hash_content(content: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// High-level parsing function with caching
pub fn parse_with_cache(language: Language, source: &str) -> Result<Tree, ParserError> {
    let content_hash = hash_content(source);

    // Try cache first
    if let Some(cached_tree) = GLOBAL_PARSE_CACHE.get(language, content_hash) {
        return Ok(cached_tree);
    }

    // Parse with pooled parser
    let pool = ParserPool::global();
    let mut parser = pool.get_parser(language)?;
    let tree = parser.parse(source, None)?;

    // Cache the result
    GLOBAL_PARSE_CACHE.put(language, content_hash, tree.clone());

    Ok(tree)
}

/// Batch parsing with automatic pooling
pub fn parse_batch(items: Vec<(Language, String)>) -> Vec<Result<Tree, ParserError>> {
    use rayon::prelude::*;

    items
        .into_par_iter()
        .map(|(language, source)| parse_with_cache(language, &source))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_pool() {
        let pool = ParserPool::new(2);

        // Get parsers
        let parser1 = pool.get_parser(Language::Rust).unwrap();
        let parser2 = pool.get_parser(Language::Rust).unwrap();

        assert_eq!(parser1.language(), Language::Rust);
        assert_eq!(parser2.language(), Language::Rust);

        // Check stats
        let stats = pool.stats();
        assert_eq!(stats.misses, 2); // Both were new parsers
        assert_eq!(stats.active_parsers, 2);
    }

    #[test]
    fn test_lazy_parser() {
        let lazy_parser = LazyParser::new(Language::Rust);

        let code = "fn main() {}";
        let tree = lazy_parser.parse(code).unwrap();

        assert!(!tree.root_node().has_error());
        assert_eq!(lazy_parser.language(), Language::Rust);
    }

    #[test]
    fn test_parse_cache() {
        let cache = ParseCache::new(10);

        let code = "fn test() {}";
        let content_hash = hash_content(code);

        // Should miss initially
        assert!(cache.get(Language::Rust, content_hash).is_none());

        // Parse and cache
        let tree = parse_with_cache(Language::Rust, code).unwrap();

        // Should hit now
        let cached = cache.get(Language::Rust, content_hash);
        assert!(cached.is_some());

        let stats = cache.stats();
        assert!(stats.hits > 0);
    }

    #[test]
    fn test_batch_parsing() {
        let items = vec![
            (Language::Rust, "fn main() {}".to_string()),
            (Language::Python, "def main(): pass".to_string()),
        ];

        let results = parse_batch(items);
        assert_eq!(results.len(), 2);

        // First should succeed (Rust)
        assert!(results[0].is_ok());

        // Second should succeed (Python)
        assert!(results[1].is_ok());
    }

    #[test]
    fn test_content_hashing() {
        let code1 = "fn main() {}";
        let code2 = "fn main() {}";
        let code3 = "fn test() {}";

        let hash1 = hash_content(code1);
        let hash2 = hash_content(code2);
        let hash3 = hash_content(code3);

        assert_eq!(hash1, hash2); // Same content, same hash
        assert_ne!(hash1, hash3); // Different content, different hash
    }

    #[test]
    fn test_global_instances() {
        // Test global pool
        let parser = ParserPool::global().get_parser(Language::Rust).unwrap();
        assert_eq!(parser.language(), Language::Rust);

        // Test global cache
        let code = "fn global_test() {}";
        let result = parse_with_cache(Language::Rust, code);
        assert!(result.is_ok());
    }
}
