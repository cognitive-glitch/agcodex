//! LRU cache for parsed ASTs with size-based eviction

// use crate::error::{AstError, AstResult}; // unused
use crate::types::ParsedAst;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

/// Parser cache for AST reuse
#[derive(Debug)]
pub struct ParserCache {
    cache: LruCache<PathBuf, Arc<ParsedAst>>,
    max_size_bytes: usize,
    current_size_bytes: usize,
}

impl ParserCache {
    /// Create a new parser cache with maximum size in bytes
    pub fn new(max_size_bytes: usize) -> Self {
        // Default to 100 entries, will evict based on size
        let cap = NonZeroUsize::new(100).unwrap();
        Self {
            cache: LruCache::new(cap),
            max_size_bytes,
            current_size_bytes: 0,
        }
    }

    /// Get a parsed AST from cache
    pub fn get(&mut self, path: &Path) -> Option<Arc<ParsedAst>> {
        self.cache.get(&path.to_path_buf()).cloned()
    }

    /// Insert a parsed AST into cache
    pub fn insert(&mut self, path: PathBuf, ast: ParsedAst) {
        let size = Self::estimate_size(&ast);

        // Evict entries if needed to make space
        while self.current_size_bytes + size > self.max_size_bytes && !self.cache.is_empty() {
            if let Some((_, evicted)) = self.cache.pop_lru() {
                self.current_size_bytes -= Self::estimate_size(&evicted);
            }
        }

        // Insert new entry
        let arc_ast = Arc::new(ast);
        if let Some((_, old)) = self.cache.push(path, arc_ast) {
            self.current_size_bytes -= Self::estimate_size(&old);
        }
        self.current_size_bytes += size;
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.current_size_bytes = 0;
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.cache.len(),
            size_bytes: self.current_size_bytes,
            max_size_bytes: self.max_size_bytes,
            hit_rate: 0.0, // Would need to track hits/misses for this
        }
    }

    /// Estimate size of a parsed AST in bytes
    const fn estimate_size(ast: &ParsedAst) -> usize {
        // Source text size + estimated tree overhead
        ast.source.len() + (ast.root_node.children_count * 64)
    }

    /// Invalidate cache entry for a path
    pub fn invalidate(&mut self, path: &Path) {
        if let Some(removed) = self.cache.pop(&path.to_path_buf()) {
            self.current_size_bytes -= Self::estimate_size(&removed);
        }
    }

    /// Check if a path is cached
    pub fn contains(&self, path: &Path) -> bool {
        self.cache.contains(&path.to_path_buf())
    }
}

impl Clone for ParserCache {
    fn clone(&self) -> Self {
        // Create a new cache with same capacity
        let cap = NonZeroUsize::new(100).unwrap();
        let mut new_cache = LruCache::new(cap);

        // Clone all entries (they're Arc'd so cheap)
        for (k, v) in self.cache.iter() {
            new_cache.push(k.clone(), v.clone());
        }

        Self {
            cache: new_cache,
            max_size_bytes: self.max_size_bytes,
            current_size_bytes: self.current_size_bytes,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub size_bytes: usize,
    pub max_size_bytes: usize,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_registry::Language;
    use crate::language_registry::LanguageRegistry;

    #[test]
    fn test_cache_basic() {
        let mut cache = ParserCache::new(1024 * 1024); // 1MB
        let registry = LanguageRegistry::new();

        let code = "fn main() { println!(\"Hello\"); }";
        let ast = registry.parse(&Language::Rust, code).unwrap();
        let path = PathBuf::from("test.rs");

        // Insert and retrieve
        cache.insert(path.clone(), ast.clone());
        assert!(cache.contains(&path));

        let cached = cache.get(&path).unwrap();
        assert_eq!(cached.source, code);

        // Invalidate
        cache.invalidate(&path);
        assert!(!cache.contains(&path));
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = ParserCache::new(100); // Very small cache
        let registry = LanguageRegistry::new();

        // Insert multiple items that exceed cache size
        for i in 0..10 {
            let code = format!("fn func{}() {{ /* some code */ }}", i);
            let ast = registry.parse(&Language::Rust, &code).unwrap();
            let path = PathBuf::from(format!("test{}.rs", i));
            cache.insert(path, ast);
        }

        // Cache should have evicted some entries
        assert!(cache.cache.len() < 10);
        assert!(cache.current_size_bytes <= cache.max_size_bytes);
    }
}
