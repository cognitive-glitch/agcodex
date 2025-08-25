# Tree-sitter Integration Implementation Plan

## Overview
This document outlines the comprehensive implementation strategy for adding tree-sitter support to AGCodex, enabling AST-based code intelligence for 50+ languages with hierarchical retrieval and AI Distiller-style compaction.

## Current State Analysis

### What Exists
- **Module Structure**: Scaffolding in place for both `code_tools` and `context_engine` modules
- **Basic Dependencies**: `tree-sitter` and `tree-sitter-bash` already in workspace
- **Interface Definitions**: `CodeTool` trait and basic types defined
- **Bash Support**: Working tree-sitter bash parser in `apply-patch` and `core/src/bash.rs`

### What's Missing
- **Language Parsers**: Only bash parser present, need 49+ more
- **Implementation**: All modules are scaffolds with `NotImplemented` errors
- **AST Intelligence**: No actual AST parsing, compaction, or retrieval
- **Location Metadata**: No file:line:column tracking
- **Caching**: No AST cache implementation
- **Integration**: Not integrated with file-search or TUI

## Architecture Design

### 1. Crate Structure
```
ast/                    # New crate for AST operations
├── src/
│   ├── lib.rs                 # Public API
│   ├── languages/             # Language-specific modules
│   │   ├── mod.rs
│   │   ├── registry.rs       # Language detection & management
│   │   └── parsers.rs        # Parser initialization
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── pool.rs           # Concurrent parser pool
│   │   └── cache.rs          # Parsed AST cache
│   ├── compactor/
│   │   ├── mod.rs
│   │   ├── distiller.rs      # AI Distiller algorithm
│   │   └── strategies.rs     # Language-specific strategies
│   └── location/
│       ├── mod.rs
│       └── metadata.rs       # Location tracking types
```

### 2. Core Types

```rust
// Location-aware metadata for all operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file_path: PathBuf,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub byte_range: Range<usize>,
}

// Hierarchical code chunk for RAG
#[derive(Debug, Clone)]
pub struct CodeChunk {
    pub id: ChunkId,
    pub level: ChunkLevel,
    pub content: String,           // Compacted code
    pub original: String,           // Original code
    pub location: SourceLocation,
    pub metadata: ChunkMetadata,
}

#[derive(Debug, Clone)]
pub enum ChunkLevel {
    File,      // File overview and imports
    Module,    // Module/namespace level
    Class,     // Class/struct/trait level
    Function,  // Function/method level
    Block,     // Complex code blocks
}

#[derive(Debug, Clone)]
pub struct ChunkMetadata {
    pub language: String,
    pub symbols: Vec<String>,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub complexity: f32,
    pub compressed_size: usize,
    pub original_size: usize,
    pub compression_ratio: f32,
}
```

## Implementation Phases

### Phase 1: Language Support (Days 1-2)

#### Task 1.1: Add Language Dependencies
Update `workspace Cargo.toml`:
```toml
[workspace.dependencies]
# Core languages
tree-sitter-rust = "0.23"
tree-sitter-python = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-go = "0.23"
tree-sitter-java = "0.23"
tree-sitter-cpp = "0.23"
tree-sitter-c = "0.23"
tree-sitter-c-sharp = "0.23"

# Web languages
tree-sitter-html = "0.23"
tree-sitter-css = "0.23"
tree-sitter-json = "0.24"
tree-sitter-yaml = "0.6"
tree-sitter-toml = "0.6"

# Add all 50+ languages...
```

#### Task 1.2: Create Language Registry
```rust
// ast/src/languages/registry.rs
pub struct LanguageRegistry {
    languages: HashMap<String, LanguageInfo>,
}

pub struct LanguageInfo {
    pub name: String,
    pub extensions: Vec<String>,
    pub language: Language,
    pub display_name: String,
    pub parser_config: ParserConfig,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_all_languages();
        registry
    }
    
    pub fn detect_language(&self, path: &Path) -> Option<&LanguageInfo> {
        // Smart detection: extension, shebang, content
    }
}
```

### Phase 2: Parser Infrastructure (Day 3)

#### Task 2.1: Parser Pool
```rust
// ast/src/parser/pool.rs
use dashmap::DashMap;

pub struct ParserPool {
    parsers: Arc<DashMap<String, Parser>>,
    max_parsers: usize,
}

impl ParserPool {
    pub async fn parse_file(&self, path: &Path) -> Result<ParsedFile> {
        let content = tokio::fs::read_to_string(path).await?;
        let language = self.detect_language(path)?;
        let parser = self.get_or_create_parser(language)?;
        
        let tree = parser.parse(&content, None)
            .ok_or(ParseError::Failed)?;
        
        Ok(ParsedFile {
            path: path.to_path_buf(),
            content,
            tree,
            language: language.name,
            metadata: self.extract_metadata(&tree, &content),
        })
    }
    
    pub async fn parse_parallel(&self, paths: Vec<PathBuf>) -> Vec<Result<ParsedFile>> {
        use futures::stream::{self, StreamExt};
        
        stream::iter(paths)
            .map(|path| self.parse_file(&path))
            .buffer_unordered(self.max_parsers)
            .collect()
            .await
    }
}
```

#### Task 2.2: AST Cache
```rust
// ast/src/parser/cache.rs
use dashmap::DashMap;
use lru::LruCache;

pub struct AstCache {
    parsed_files: Arc<DashMap<PathBuf, CachedAst>>,
    lru: Arc<Mutex<LruCache<PathBuf, ()>>>,
    max_cache_size: usize,
}

#[derive(Clone)]
struct CachedAst {
    tree: Tree,
    content: String,
    parsed_at: SystemTime,
    file_hash: u64,
}

impl AstCache {
    pub async fn get_or_parse(&self, path: &Path) -> Result<ParsedFile> {
        // Check if file changed
        let current_hash = self.compute_file_hash(path).await?;
        
        if let Some(cached) = self.parsed_files.get(path) {
            if cached.file_hash == current_hash {
                self.touch_lru(path);
                return Ok(cached.to_parsed_file());
            }
        }
        
        // Parse and cache
        let parsed = self.parser_pool.parse_file(path).await?;
        self.insert_with_eviction(path, parsed.clone());
        Ok(parsed)
    }
}
```

### Phase 3: AST Compaction (Day 4)

#### Task 3.1: AI Distiller Implementation
```rust
// ast/src/compactor/distiller.rs
pub struct AiDistiller {
    compression_level: CompressionLevel,
    preserve_semantics: bool,
}

pub enum CompressionLevel {
    Light,   // 70% compression - Keep most structure
    Medium,  // 85% compression - Balanced
    Hard,    // 95% compression - Maximum reduction
}

impl AiDistiller {
    pub fn distill_ast(&self, tree: &Tree, source: &str) -> DistillResult {
        let mut visitor = DistillVisitor::new(self.compression_level);
        visitor.visit_tree(tree, source);
        
        DistillResult {
            compacted: visitor.output,
            original_size: source.len(),
            compressed_size: visitor.output.len(),
            compression_ratio: 1.0 - (visitor.output.len() as f32 / source.len() as f32),
            location_map: visitor.location_map,
        }
    }
    
    fn distill_function(&self, node: Node, source: &str) -> String {
        // Extract signature, remove body details
        let signature = self.extract_signature(node, source);
        let docstring = self.extract_docstring(node, source);
        
        format!("{}\n{}\n    # {} lines of implementation",
            signature,
            docstring.unwrap_or_default(),
            self.count_lines(node))
    }
    
    fn distill_class(&self, node: Node, source: &str) -> String {
        // Keep class structure, method signatures
        let mut output = String::new();
        output.push_str(&self.extract_class_header(node, source));
        
        for method in self.find_methods(node) {
            output.push_str(&self.distill_function(method, source));
        }
        
        output
    }
}
```

#### Task 3.2: Language-Specific Strategies
```rust
// ast/src/compactor/strategies.rs
pub trait CompactionStrategy {
    fn should_keep_node(&self, node: &Node) -> bool;
    fn transform_node(&self, node: &Node, source: &str) -> Option<String>;
    fn get_importance(&self, node: &Node) -> f32;
}

pub struct RustStrategy;
impl CompactionStrategy for RustStrategy {
    fn should_keep_node(&self, node: &Node) -> bool {
        matches!(node.kind(), 
            "function_item" | "impl_item" | "struct_item" | 
            "trait_item" | "enum_item" | "type_alias")
    }
}

pub struct PythonStrategy;
impl CompactionStrategy for PythonStrategy {
    fn should_keep_node(&self, node: &Node) -> bool {
        matches!(node.kind(),
            "function_definition" | "class_definition" |
            "decorated_definition" | "import_statement")
    }
}
```

### Phase 4: Semantic Indexing (Day 5)

#### Task 4.1: Semantic Index
```rust
// core/src/context_engine/semantic_index.rs
use tantivy::{schema::*, Index, IndexWriter};

pub struct SemanticIndex {
    index: Index,
    writer: Arc<Mutex<IndexWriter>>,
    chunks: Arc<DashMap<ChunkId, CodeChunk>>,
    symbol_graph: Arc<SymbolGraph>,
}

impl SemanticIndex {
    pub async fn index_codebase(&self, root: &Path) -> Result<IndexStats> {
        let files = self.discover_files(root)?;
        
        // Parse in parallel
        let parsed_files = self.parser_pool
            .parse_parallel(files)
            .await;
        
        // Chunk and compress
        let chunks = self.create_chunks(&parsed_files)?;
        
        // Generate embeddings with location metadata
        let embeddings = self.generate_embeddings(&chunks).await?;
        
        // Store in index
        self.store_chunks(chunks, embeddings).await?;
        
        Ok(IndexStats {
            files_indexed: parsed_files.len(),
            chunks_created: chunks.len(),
            compression_ratio: self.calculate_compression(&chunks),
        })
    }
    
    fn create_chunks(&self, files: &[ParsedFile]) -> Result<Vec<CodeChunk>> {
        let mut all_chunks = Vec::new();
        
        for file in files {
            // File-level chunk
            all_chunks.push(self.create_file_chunk(file));
            
            // Class-level chunks
            for class in self.extract_classes(&file.tree, &file.content) {
                all_chunks.push(self.create_class_chunk(class, file));
            }
            
            // Function-level chunks
            for function in self.extract_functions(&file.tree, &file.content) {
                all_chunks.push(self.create_function_chunk(function, file));
            }
        }
        
        Ok(all_chunks)
    }
}
```

#### Task 4.2: Symbol Graph
```rust
// core/src/context_engine/semantic_index.rs
pub struct SymbolGraph {
    nodes: HashMap<SymbolId, SymbolNode>,
    edges: Vec<SymbolEdge>,
}

pub struct SymbolNode {
    pub id: SymbolId,
    pub name: String,
    pub kind: SymbolKind,
    pub location: SourceLocation,
    pub visibility: Visibility,
}

pub enum SymbolEdge {
    Imports { from: SymbolId, to: SymbolId },
    Extends { child: SymbolId, parent: SymbolId },
    Implements { impl: SymbolId, trait_: SymbolId },
    Calls { caller: SymbolId, callee: SymbolId },
    Uses { user: SymbolId, used: SymbolId },
}
```

### Phase 5: Context Retrieval (Day 6)

#### Task 5.1: Hierarchical Retrieval
```rust
// core/src/context_engine/retrieval.rs
pub struct ContextRetriever {
    index: SemanticIndex,
    embedder: EmbeddingModel,
    strategy: RetrievalStrategy,
}

pub enum RetrievalStrategy {
    Hierarchical,  // File → Class → Function
    Semantic,      // Pure embedding similarity
    Hybrid,        // Combine both
}

impl ContextRetriever {
    pub async fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<RetrievalResult>> {
        // Generate query embedding
        let query_embedding = self.embedder.embed(query).await?;
        
        // Search at multiple levels
        let file_results = self.search_files(&query_embedding, limit / 3)?;
        let class_results = self.search_classes(&query_embedding, limit / 3)?;
        let function_results = self.search_functions(&query_embedding, limit / 3)?;
        
        // Combine and re-rank
        let mut all_results = Vec::new();
        all_results.extend(file_results);
        all_results.extend(class_results);
        all_results.extend(function_results);
        
        // Re-rank by relevance and diversity
        self.rerank_results(&mut all_results, limit);
        
        Ok(all_results)
    }
    
    fn rerank_results(&self, results: &mut Vec<RetrievalResult>, limit: usize) {
        // Score based on:
        // - Semantic similarity
        // - Structural relevance (same file/module)
        // - Recency
        // - Diversity (avoid too many chunks from same file)
        
        results.sort_by(|a, b| {
            let a_score = self.compute_final_score(a);
            let b_score = self.compute_final_score(b);
            b_score.partial_cmp(&a_score).unwrap()
        });
        
        results.truncate(limit);
    }
}
```

### Phase 6: Tool Integration (Day 7)

#### Task 6.1: Complete TreeSitterTool
```rust
// core/src/code_tools/tree_sitter.rs
use ast::{ParserPool, AstCache, LanguageRegistry};

pub struct TreeSitterTool {
    parser_pool: Arc<ParserPool>,
    cache: Arc<AstCache>,
    registry: Arc<LanguageRegistry>,
}

impl CodeTool for TreeSitterTool {
    type Query = TsQuery;
    type Output = Vec<TsMatch>;
    
    fn search(&self, query: Self::Query) -> Result<Self::Output, ToolError> {
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(self.search_async(query))
    }
    
    async fn search_async(&self, query: TsQuery) -> Result<Vec<TsMatch>, ToolError> {
        let files = self.find_target_files(&query).await?;
        let mut all_matches = Vec::new();
        
        for file_path in files {
            let parsed = self.cache.get_or_parse(&file_path).await?;
            let matches = self.search_in_tree(&parsed.tree, &query.pattern, &parsed.content)?;
            
            for match_ in matches {
                all_matches.push(TsMatch {
                    file: file_path.display().to_string(),
                    line: match_.start_position.row + 1,
                    column: match_.start_position.column + 1,
                    end_line: match_.end_position.row + 1,
                    end_column: match_.end_position.column + 1,
                    context: Some(self.extract_context(&match_, &parsed.content)),
                    matched_text: self.get_match_text(&match_, &parsed.content),
                    node_kind: match_.node.kind().to_string(),
                });
            }
        }
        
        Ok(all_matches)
    }
}
```

#### Task 6.2: Integrate with File Search
```rust
// file-search/src/lib.rs
use ast::{ParserPool, SemanticIndex};

pub struct EnhancedFileSearch {
    fuzzy_matcher: NucleoMatcher,
    ast_parser: Arc<ParserPool>,
    semantic_index: Arc<SemanticIndex>,
}

impl EnhancedFileSearch {
    pub async fn search_with_ast(&self, query: &str) -> Result<Vec<SearchResult>> {
        // Combine fuzzy matching with AST search
        let fuzzy_results = self.fuzzy_search(query)?;
        let ast_results = self.ast_search(query).await?;
        let semantic_results = self.semantic_search(query).await?;
        
        // Merge and rank results
        self.merge_results(fuzzy_results, ast_results, semantic_results)
    }
    
    async fn ast_search(&self, query: &str) -> Result<Vec<AstSearchResult>> {
        // Search for symbols, functions, classes
        let symbols = self.semantic_index.search_symbols(query).await?;
        
        symbols.into_iter()
            .map(|sym| AstSearchResult {
                file: sym.location.file_path,
                line: sym.location.start_line,
                column: sym.location.start_column,
                symbol_name: sym.name,
                symbol_kind: sym.kind,
                score: sym.relevance_score,
            })
            .collect()
    }
}
```

### Phase 7: Performance Optimization (Day 8)

#### Task 7.1: Caching Strategy
```rust
// ast/src/parser/cache.rs
pub struct CacheConfig {
    pub max_memory_mb: usize,      // e.g., 500 MB
    pub max_files: usize,          // e.g., 1000 files
    pub ttl: Duration,             // e.g., 30 minutes
    pub compression: bool,         // Compress cached ASTs
}

pub struct OptimizedCache {
    memory_cache: Arc<DashMap<PathBuf, CachedAst>>,
    disk_cache: Option<DiskCache>,
    stats: Arc<CacheStats>,
}

impl OptimizedCache {
    pub async fn get_with_stats(&self, path: &Path) -> Result<(ParsedFile, CacheHit)> {
        let start = Instant::now();
        
        // Try memory cache first
        if let Some(cached) = self.memory_cache.get(path) {
            self.stats.record_hit(CacheLevel::Memory, start.elapsed());
            return Ok((cached.to_parsed_file(), CacheHit::Memory));
        }
        
        // Try disk cache
        if let Some(ref disk) = self.disk_cache {
            if let Some(cached) = disk.get(path).await? {
                self.promote_to_memory(path, cached.clone());
                self.stats.record_hit(CacheLevel::Disk, start.elapsed());
                return Ok((cached, CacheHit::Disk));
            }
        }
        
        // Cache miss - parse
        let parsed = self.parse_fresh(path).await?;
        self.insert_cascading(path, parsed.clone()).await?;
        self.stats.record_miss(start.elapsed());
        
        Ok((parsed, CacheHit::Miss))
    }
}
```

#### Task 7.2: Parallel Processing
```rust
// ast/src/parser/pool.rs
use rayon::prelude::*;

pub struct ParallelProcessor {
    thread_pool: ThreadPool,
    chunk_size: usize,
}

impl ParallelProcessor {
    pub fn process_codebase(&self, root: &Path) -> Result<ProcessStats> {
        let files = self.discover_files(root)?;
        
        // Process in chunks for memory efficiency
        let chunks: Vec<_> = files.chunks(self.chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect();
        
        let results: Vec<_> = chunks.par_iter()
            .map(|chunk| self.process_chunk(chunk))
            .collect();
        
        Ok(self.aggregate_stats(results))
    }
    
    fn process_chunk(&self, files: &[PathBuf]) -> ChunkResult {
        let mut parsed = Vec::new();
        let mut errors = Vec::new();
        
        for file in files {
            match self.parse_and_index(file) {
                Ok(result) => parsed.push(result),
                Err(e) => errors.push((file.clone(), e)),
            }
        }
        
        ChunkResult { parsed, errors }
    }
}
```

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_language_detection() {
        let registry = LanguageRegistry::new();
        
        assert_eq!(registry.detect_language(Path::new("test.rs")).unwrap().name, "rust");
        assert_eq!(registry.detect_language(Path::new("test.py")).unwrap().name, "python");
        assert_eq!(registry.detect_language(Path::new("test.js")).unwrap().name, "javascript");
    }
    
    #[tokio::test]
    async fn test_ast_parsing() {
        let pool = ParserPool::new();
        let result = pool.parse_file(Path::new("test_data/sample.rs")).await;
        
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(!parsed.tree.root_node().has_error());
    }
    
    #[test]
    fn test_compression_ratio() {
        let distiller = AiDistiller::new(CompressionLevel::Hard);
        let source = include_str!("test_data/large_file.rs");
        
        let result = distiller.distill_source(source);
        assert!(result.compression_ratio >= 0.90);
        assert!(result.compacted.len() < source.len() / 10);
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_end_to_end_indexing() {
    let indexer = ASTIndexer::new();
    let stats = indexer.index_codebase(Path::new("test_project")).await.unwrap();
    
    assert!(stats.files_indexed > 0);
    assert!(stats.chunks_created > stats.files_indexed);
    assert!(stats.compression_ratio > 0.85);
}

#[tokio::test]
async fn test_context_retrieval() {
    let retriever = ContextRetriever::new();
    let results = retriever.retrieve("function that handles authentication", 10).await.unwrap();
    
    assert!(!results.is_empty());
    assert!(results[0].score > 0.7);
    assert!(results[0].location.file_path.exists());
}
```

### Benchmarks
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_parsing(c: &mut Criterion) {
    let pool = ParserPool::new();
    let file = Path::new("benches/large_file.rs");
    
    c.bench_function("parse_large_file", |b| {
        b.iter(|| {
            black_box(pool.parse_file_sync(file))
        });
    });
}

fn benchmark_compression(c: &mut Criterion) {
    let distiller = AiDistiller::new(CompressionLevel::Medium);
    let source = include_str!("benches/large_file.rs");
    
    c.bench_function("compress_large_file", |b| {
        b.iter(|| {
            black_box(distiller.distill_source(source))
        });
    });
}

criterion_group!(benches, benchmark_parsing, benchmark_compression);
criterion_main!(benches);
```

## Performance Targets

### Parsing Performance
- Single file: <10ms for files under 1000 lines
- Large file (10k lines): <50ms
- Codebase (1M LOC): <5 minutes initial, <1s incremental

### Compression Metrics
- Light mode: 70% compression, <5ms per file
- Medium mode: 85% compression, <10ms per file
- Hard mode: 95% compression, <20ms per file

### Memory Usage
- Cache size: <500MB for 10k files
- Parser pool: <100MB overhead
- Index size: <2GB for 1M LOC

### Retrieval Speed
- Symbol search: <50ms
- Semantic search: <200ms
- Hybrid search: <300ms

## Migration Path

### Step 1: Add Dependencies (Day 1)
1. Update workspace Cargo.toml with all tree-sitter languages
2. Create ast crate with basic structure
3. Update core/Cargo.toml to depend on ast

### Step 2: Implement Core (Days 2-4)
1. Build LanguageRegistry with all 50+ languages
2. Implement ParserPool with caching
3. Create AiDistiller with compression strategies
4. Build location tracking throughout

### Step 3: Wire Integration (Days 5-6)
1. Update TreeSitterTool to use new infrastructure
2. Enhance file-search with AST capabilities
3. Implement context_engine modules
4. Add location metadata everywhere

### Step 4: Test & Optimize (Days 7-8)
1. Add comprehensive test suite
2. Benchmark all operations
3. Optimize cache and parallel processing
4. Document API and usage

## Risk Mitigation

### Memory Management
- **Risk**: Large ASTs consuming too much memory
- **Mitigation**: Implement streaming parser for huge files, aggressive caching with eviction

### Performance Degradation
- **Risk**: AST parsing slowing down operations
- **Mitigation**: Background indexing, incremental updates, extensive caching

### Language Support Complexity
- **Risk**: 50+ languages hard to maintain
- **Mitigation**: Modular design, language-agnostic interfaces, graceful fallbacks

### Compatibility Issues
- **Risk**: tree-sitter version conflicts
- **Mitigation**: Pin versions, extensive testing, feature flags for experimental languages

## Success Criteria

### Functional Requirements
- ✓ All 50+ languages parse without errors
- ✓ 90%+ compression ratio achieved
- ✓ Location metadata present in all operations
- ✓ Cache hit rate >90% for hot paths

### Performance Requirements
- ✓ <10ms parse time for average files
- ✓ <5 minute initial indexing for 1M LOC
- ✓ <200ms semantic search response
- ✓ <500MB memory for typical usage

### Quality Requirements
- ✓ 80%+ test coverage
- ✓ All benchmarks passing targets
- ✓ Zero panics in production code
- ✓ Comprehensive error handling

## Next Steps

1. **Immediate**: Add tree-sitter dependencies to Cargo.toml
2. **Day 1**: Create ast crate structure
3. **Day 2**: Implement LanguageRegistry
4. **Day 3**: Build ParserPool and caching
5. **Day 4**: Implement AiDistiller
6. **Day 5**: Create SemanticIndex
7. **Day 6**: Build ContextRetriever
8. **Day 7**: Integration and testing
9. **Day 8**: Optimization and documentation

## Appendix: Language Priority

### Tier 1 (Must Have - Day 1)
- Rust, Python, JavaScript, TypeScript, Go
- Java, C++, C, C#, Swift

### Tier 2 (Should Have - Day 2)
- Ruby, PHP, Kotlin, Scala, Elixir
- HTML, CSS, JSON, YAML, TOML

### Tier 3 (Nice to Have - Day 3)
- Haskell, OCaml, F#, Clojure, Erlang
- Zig, Nim, Julia, R, Dart

### Tier 4 (Extended Support - Future)
- SQL, GraphQL, Protobuf, Dockerfile
- Nix, HCL, CMake, Make, Bash