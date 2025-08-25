# Practical Implementation Guide: AST-Based RAG for AGCodex-RS

## Quick Start Implementation Plan

This guide provides a concrete, step-by-step implementation plan for adding AST-based code intelligence to AGCodex-RS.

## Week 1: Foundation Setup

### Day 1-2: Add Tree-sitter Dependencies

```toml
# core/Cargo.toml
[dependencies]
# Parser and grammars
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-python = "0.23"
tree-sitter-javascript = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-go = "0.23"
tree-sitter-java = "0.23"
tree-sitter-cpp = "0.23"

# Embedding and vector storage
candle-core = "0.8"  # For local embeddings
candle-transformers = "0.8"
lancedb = "0.13"  # Embedded vector DB

# Search and indexing
tantivy = "0.22"  # Full-text search
dashmap = "6.1"  # Concurrent caching

# Serialization
bincode = "1.3"  # Fast binary serialization
zstd = "0.13"  # Compression
```

### Day 3-4: Create Module Structure

```bash
# Create the code intelligence module structure
mkdir -p core/src/code_intelligence/{parser,chunker,compactor,embedder,indexer,retriever}
```

### Day 5: Basic Tree-sitter Parser

```rust
// core/src/code_intelligence/parser.rs
use tree_sitter::{Parser, Tree, Language};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("unsupported language: {0}")]
    UnsupportedLanguage(String),
    
    #[error("parse failed")]
    ParseFailed,
    
    #[error("invalid UTF-8 in source")]
    InvalidUtf8(#[from] std::str::Utf8Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodeLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Go,
    Java,
    Cpp,
}

impl CodeLanguage {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "py" => Some(Self::Python),
            "js" => Some(Self::JavaScript),
            "ts" | "tsx" => Some(Self::TypeScript),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "cpp" | "cc" | "cxx" => Some(Self::Cpp),
            _ => None,
        }
    }
    
    pub fn tree_sitter_language(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Go => tree_sitter_go::LANGUAGE.into(),
            Self::Java => tree_sitter_java::LANGUAGE.into(),
            Self::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        }
    }
}

pub struct CodeParser {
    parsers: HashMap<CodeLanguage, Parser>,
}

impl CodeParser {
    pub fn new() -> Self {
        let mut parsers = HashMap::new();
        
        for lang in [
            CodeLanguage::Rust,
            CodeLanguage::Python,
            CodeLanguage::JavaScript,
            CodeLanguage::TypeScript,
            CodeLanguage::Go,
            CodeLanguage::Java,
            CodeLanguage::Cpp,
        ] {
            let mut parser = Parser::new();
            parser.set_language(&lang.tree_sitter_language())
                .expect("Failed to set language");
            parsers.insert(lang, parser);
        }
        
        Self { parsers }
    }
    
    pub fn parse(&mut self, code: &str, lang: CodeLanguage) -> Result<Tree, ParseError> {
        let parser = self.parsers.get_mut(&lang)
            .ok_or_else(|| ParseError::UnsupportedLanguage(format!("{:?}", lang)))?;
        
        parser.parse(code, None)
            .ok_or(ParseError::ParseFailed)
    }
}
```

## Week 2: Code Chunking & Compaction

### Day 6-7: AST-Based Chunker

```rust
// core/src/code_intelligence/chunker.rs
use tree_sitter::{Node, Tree, TreeCursor};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub id: String,
    pub file_path: String,
    pub language: CodeLanguage,
    pub chunk_type: ChunkType,
    pub name: String,
    pub signature: String,
    pub full_code: String,
    pub start_line: usize,
    pub end_line: usize,
    pub parent_id: Option<String>,
    pub references: Vec<Reference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkType {
    Module,
    Class,
    Function,
    Method,
    Interface,
    Struct,
    Enum,
    Trait,
    Implementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    pub target: String,
    pub ref_type: ReferenceType,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReferenceType {
    Call,
    Import,
    Inheritance,
    Implementation,
    Usage,
}

pub struct AstChunker {
    language_queries: HashMap<CodeLanguage, String>,
}

impl AstChunker {
    pub fn new() -> Self {
        let mut queries = HashMap::new();
        
        // Rust queries
        queries.insert(CodeLanguage::Rust, r#"
            (function_item
                name: (identifier) @function.name
            ) @function.def
            
            (impl_item
                type: (type_identifier) @impl.type
                body: (declaration_list) @impl.body
            ) @impl.def
            
            (struct_item
                name: (type_identifier) @struct.name
            ) @struct.def
            
            (enum_item
                name: (type_identifier) @enum.name
            ) @enum.def
            
            (trait_item
                name: (type_identifier) @trait.name
            ) @trait.def
        "#.to_string());
        
        // Python queries
        queries.insert(CodeLanguage::Python, r#"
            (class_definition
                name: (identifier) @class.name
                body: (block) @class.body
            ) @class.def
            
            (function_definition
                name: (identifier) @function.name
                parameters: (parameters) @function.params
                body: (block) @function.body
            ) @function.def
        "#.to_string());
        
        // Add more language queries...
        
        Self { language_queries: queries }
    }
    
    pub fn extract_chunks(&self, tree: &Tree, code: &str, file_path: &str, lang: CodeLanguage) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let mut cursor = tree.walk();
        
        self.visit_node(&mut cursor, code.as_bytes(), &mut chunks, file_path, lang, None);
        
        // Extract references
        for chunk in &mut chunks {
            chunk.references = self.extract_references(&tree, code.as_bytes(), &chunk);
        }
        
        chunks
    }
    
    fn visit_node(
        &self,
        cursor: &mut TreeCursor,
        source: &[u8],
        chunks: &mut Vec<CodeChunk>,
        file_path: &str,
        lang: CodeLanguage,
        parent_id: Option<String>,
    ) {
        let node = cursor.node();
        
        if let Some(chunk) = self.node_to_chunk(node, source, file_path, lang, parent_id.clone()) {
            let chunk_id = chunk.id.clone();
            chunks.push(chunk);
            
            // Recursively process children with this chunk as parent
            if cursor.goto_first_child() {
                loop {
                    self.visit_node(cursor, source, chunks, file_path, lang, Some(chunk_id.clone()));
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        } else {
            // Not a chunk node, but still process children
            if cursor.goto_first_child() {
                loop {
                    self.visit_node(cursor, source, chunks, file_path, lang, parent_id.clone());
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }
    }
    
    fn node_to_chunk(
        &self,
        node: Node,
        source: &[u8],
        file_path: &str,
        lang: CodeLanguage,
        parent_id: Option<String>,
    ) -> Option<CodeChunk> {
        let chunk_type = match (lang, node.kind()) {
            (CodeLanguage::Rust, "function_item") => ChunkType::Function,
            (CodeLanguage::Rust, "impl_item") => ChunkType::Implementation,
            (CodeLanguage::Rust, "struct_item") => ChunkType::Struct,
            (CodeLanguage::Rust, "enum_item") => ChunkType::Enum,
            (CodeLanguage::Rust, "trait_item") => ChunkType::Trait,
            (CodeLanguage::Python, "class_definition") => ChunkType::Class,
            (CodeLanguage::Python, "function_definition") => ChunkType::Function,
            _ => return None,
        };
        
        let name = self.extract_name(node, source)?;
        let signature = self.extract_signature(node, source);
        let full_code = node.utf8_text(source).ok()?.to_string();
        
        Some(CodeChunk {
            id: format!("{}:{}:{}", file_path, node.start_position().row, name),
            file_path: file_path.to_string(),
            language: lang,
            chunk_type,
            name,
            signature,
            full_code,
            start_line: node.start_position().row,
            end_line: node.end_position().row,
            parent_id,
            references: Vec::new(),
        })
    }
    
    fn extract_name(&self, node: Node, source: &[u8]) -> Option<String> {
        // Find name node based on node type
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "identifier" || child.kind() == "type_identifier" {
                    return child.utf8_text(source).ok().map(|s| s.to_string());
                }
            }
        }
        None
    }
    
    fn extract_signature(&self, node: Node, source: &[u8]) -> String {
        // Extract just the signature (first line typically)
        let full = node.utf8_text(source).unwrap_or("");
        full.lines().next().unwrap_or("").to_string()
    }
    
    fn extract_references(&self, tree: &Tree, source: &[u8], chunk: &CodeChunk) -> Vec<Reference> {
        // TODO: Implement reference extraction
        // This would walk the AST within the chunk and find:
        // - Function calls
        // - Type references
        // - Import statements
        Vec::new()
    }
}
```

### Day 8-9: Code Compactor

```rust
// core/src/code_intelligence/compactor.rs
use super::chunker::{CodeChunk, ChunkType};

#[derive(Debug, Clone)]
pub struct CompactedChunk {
    pub chunk_id: String,
    pub summary: String,
    pub signature: String,
    pub docstring: Option<String>,
    pub imports: Vec<String>,
    pub calls: Vec<String>,
    pub complexity: usize,
    pub token_count: usize,
}

pub struct CodeCompactor {
    max_tokens: usize,
    preserve_docstrings: bool,
}

impl CodeCompactor {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            preserve_docstrings: true,
        }
    }
    
    pub fn compact(&self, chunk: &CodeChunk) -> CompactedChunk {
        let signature = self.clean_signature(&chunk.signature);
        let docstring = if self.preserve_docstrings {
            self.extract_docstring(&chunk.full_code)
        } else {
            None
        };
        
        let imports = self.extract_imports(&chunk.full_code);
        let calls = self.extract_function_calls(&chunk.full_code);
        let complexity = self.calculate_complexity(&chunk.full_code);
        
        let summary = self.generate_summary(chunk);
        let token_count = self.estimate_tokens(&signature, &docstring, &summary);
        
        CompactedChunk {
            chunk_id: chunk.id.clone(),
            summary,
            signature,
            docstring,
            imports,
            calls,
            complexity,
            token_count,
        }
    }
    
    fn clean_signature(&self, sig: &str) -> String {
        // Remove implementation, keep only declaration
        sig.trim_end_matches('{').trim().to_string()
    }
    
    fn extract_docstring(&self, code: &str) -> Option<String> {
        // Language-specific docstring extraction
        // For Python: """...""" or '''...'''
        // For Rust: /// or //!
        // For JS/TS: /** ... */
        None // TODO: Implement
    }
    
    fn extract_imports(&self, code: &str) -> Vec<String> {
        // Extract import/use statements
        Vec::new() // TODO: Implement
    }
    
    fn extract_function_calls(&self, code: &str) -> Vec<String> {
        // Extract function calls made within this chunk
        Vec::new() // TODO: Implement
    }
    
    fn calculate_complexity(&self, code: &str) -> usize {
        // Simple cyclomatic complexity estimate
        code.matches("if ").count() +
        code.matches("for ").count() +
        code.matches("while ").count() +
        code.matches("match ").count() +
        code.matches("?").count()
    }
    
    fn generate_summary(&self, chunk: &CodeChunk) -> String {
        format!(
            "{:?} '{}' in {} (lines {}-{})",
            chunk.chunk_type,
            chunk.name,
            chunk.file_path,
            chunk.start_line,
            chunk.end_line
        )
    }
    
    fn estimate_tokens(&self, signature: &str, docstring: &Option<String>, summary: &str) -> usize {
        let mut count = signature.split_whitespace().count() + summary.split_whitespace().count();
        if let Some(doc) = docstring {
            count += doc.split_whitespace().count();
        }
        count
    }
}
```

## Week 3: Embeddings & Vector Storage

### Day 10-11: Embedding Generator

```rust
// core/src/code_intelligence/embedder.rs
use candle_core::{Device, Tensor};
use candle_transformers::models::bert::{BertModel, Config};
use tokenizers::Tokenizer;
use async_trait::async_trait;

#[async_trait]
pub trait CodeEmbedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError>;
    fn dimension(&self) -> usize;
}

#[derive(Error, Debug)]
pub enum EmbedError {
    #[error("model loading failed: {0}")]
    ModelLoad(String),
    
    #[error("tokenization failed: {0}")]
    Tokenization(String),
    
    #[error("inference failed: {0}")]
    Inference(String),
}

// Local embedding with CodeBERT or similar
pub struct LocalCodeEmbedder {
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl LocalCodeEmbedder {
    pub async fn new(model_path: &str) -> Result<Self, EmbedError> {
        // Load model and tokenizer
        // This is simplified - actual implementation would need proper model loading
        todo!("Implement model loading")
    }
}

#[async_trait]
impl CodeEmbedder for LocalCodeEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        // Tokenize
        let encoding = self.tokenizer.encode(text, true)
            .map_err(|e| EmbedError::Tokenization(e.to_string()))?;
        
        // Convert to tensor
        let input_ids = Tensor::new(encoding.get_ids(), &self.device)
            .map_err(|e| EmbedError::Inference(e.to_string()))?;
        
        // Run through model
        let output = self.model.forward(&input_ids)
            .map_err(|e| EmbedError::Inference(e.to_string()))?;
        
        // Pool and convert to Vec<f32>
        // This is simplified - actual implementation would need proper pooling
        Ok(vec![0.0; 768]) // Placeholder
    }
    
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        // Batch processing for efficiency
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }
    
    fn dimension(&self) -> usize {
        768 // BERT-base dimension
    }
}

// OpenAI embedding fallback
pub struct OpenAICodeEmbedder {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAICodeEmbedder {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: "text-embedding-3-small".to_string(),
        }
    }
}

#[async_trait]
impl CodeEmbedder for OpenAICodeEmbedder {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        // Call OpenAI API
        todo!("Implement OpenAI API call")
    }
    
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbedError> {
        // OpenAI supports batch embedding
        todo!("Implement batch API call")
    }
    
    fn dimension(&self) -> usize {
        1536 // text-embedding-3-small dimension
    }
}
```

### Day 12-13: Vector Database Integration

```rust
// core/src/code_intelligence/vector_store.rs
use lancedb::{Connection, Table};
use arrow::array::{Float32Array, StringArray};
use arrow::record_batch::RecordBatch;
use dashmap::DashMap;

pub struct CodeVectorStore {
    connection: Connection,
    table: Table,
    cache: DashMap<String, Vec<f32>>,
}

impl CodeVectorStore {
    pub async fn new(path: &str) -> Result<Self, VectorStoreError> {
        let connection = lancedb::connect(path).await?;
        
        // Create or open table
        let table = if connection.table_names().await?.contains(&"code_embeddings".to_string()) {
            connection.open_table("code_embeddings").await?
        } else {
            connection.create_table(
                "code_embeddings",
                vec![
                    ("chunk_id", arrow::datatypes::DataType::Utf8),
                    ("file_path", arrow::datatypes::DataType::Utf8),
                    ("chunk_type", arrow::datatypes::DataType::Utf8),
                    ("name", arrow::datatypes::DataType::Utf8),
                    ("signature", arrow::datatypes::DataType::Utf8),
                    ("embedding", arrow::datatypes::DataType::FixedSizeList(
                        Box::new(arrow::datatypes::Field::new("item", arrow::datatypes::DataType::Float32, true)),
                        768,
                    )),
                    ("metadata", arrow::datatypes::DataType::Utf8),
                ],
            ).await?
        };
        
        Ok(Self {
            connection,
            table,
            cache: DashMap::new(),
        })
    }
    
    pub async fn insert(&self, chunk: &CompactedChunk, embedding: Vec<f32>) -> Result<(), VectorStoreError> {
        // Cache the embedding
        self.cache.insert(chunk.chunk_id.clone(), embedding.clone());
        
        // Insert into LanceDB
        let batch = RecordBatch::try_from_iter(vec![
            ("chunk_id", Arc::new(StringArray::from(vec![chunk.chunk_id.clone()]))),
            ("embedding", Arc::new(Float32Array::from(embedding))),
            // ... other fields
        ])?;
        
        self.table.add(batch).await?;
        Ok(())
    }
    
    pub async fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<SearchResult>, VectorStoreError> {
        // Vector similarity search
        let results = self.table
            .query()
            .nearest_to(query_embedding)
            .limit(k)
            .execute()
            .await?;
        
        // Convert to SearchResult
        Ok(results.into_iter().map(|r| SearchResult {
            chunk_id: r.get("chunk_id"),
            score: r.get("_distance"),
            // ... other fields
        }).collect())
    }
}
```

## Week 4: Retrieval & Integration

### Day 14-15: Hybrid Retriever

```rust
// core/src/code_intelligence/retriever.rs
use tantivy::{Index, IndexWriter, Document};
use tantivy::query::QueryParser;
use tantivy::collector::TopDocs;

pub struct HybridRetriever {
    vector_store: Arc<CodeVectorStore>,
    text_index: Index,
    embedder: Arc<dyn CodeEmbedder>,
}

impl HybridRetriever {
    pub async fn new(
        vector_store: Arc<CodeVectorStore>,
        embedder: Arc<dyn CodeEmbedder>,
    ) -> Result<Self, RetrieverError> {
        // Create text index schema
        let mut schema_builder = tantivy::schema::Schema::builder();
        schema_builder.add_text_field("chunk_id", tantivy::schema::TEXT | tantivy::schema::STORED);
        schema_builder.add_text_field("content", tantivy::schema::TEXT);
        schema_builder.add_text_field("signature", tantivy::schema::TEXT | tantivy::schema::STORED);
        let schema = schema_builder.build();
        
        let index = Index::create_in_ram(schema);
        
        Ok(Self {
            vector_store,
            text_index: index,
            embedder,
        })
    }
    
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>, RetrieverError> {
        // Parallel search
        let (vector_results, text_results) = tokio::join!(
            self.vector_search(query, k * 2),
            self.text_search(query, k * 2)
        );
        
        // Merge and re-rank
        self.merge_results(vector_results?, text_results?, k)
    }
    
    async fn vector_search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>, RetrieverError> {
        let embedding = self.embedder.embed(query).await?;
        self.vector_store.search(&embedding, k).await
    }
    
    fn text_search(&self, query: &str, k: usize) -> Result<Vec<SearchResult>, RetrieverError> {
        let reader = self.text_index.reader()?;
        let searcher = reader.searcher();
        
        let query_parser = QueryParser::for_index(&self.text_index, vec![/* fields */]);
        let query = query_parser.parse_query(query)?;
        
        let top_docs = searcher.search(&query, &TopDocs::with_limit(k))?;
        
        // Convert to SearchResult
        Ok(top_docs.into_iter().map(|(score, doc_address)| {
            let doc = searcher.doc(doc_address).unwrap();
            SearchResult {
                chunk_id: doc.get_first("chunk_id").unwrap().to_string(),
                score: score as f32,
                // ... other fields
            }
        }).collect())
    }
    
    fn merge_results(
        &self,
        vector_results: Vec<SearchResult>,
        text_results: Vec<SearchResult>,
        k: usize,
    ) -> Result<Vec<SearchResult>, RetrieverError> {
        // Reciprocal Rank Fusion (RRF)
        let mut scores: HashMap<String, f32> = HashMap::new();
        
        for (rank, result) in vector_results.iter().enumerate() {
            let rrf_score = 1.0 / (60.0 + rank as f32);
            *scores.entry(result.chunk_id.clone()).or_insert(0.0) += rrf_score;
        }
        
        for (rank, result) in text_results.iter().enumerate() {
            let rrf_score = 1.0 / (60.0 + rank as f32);
            *scores.entry(result.chunk_id.clone()).or_insert(0.0) += rrf_score;
        }
        
        // Sort by combined score
        let mut results: Vec<_> = scores.into_iter()
            .map(|(chunk_id, score)| SearchResult { chunk_id, score, /* ... */ })
            .collect();
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(k);
        
        Ok(results)
    }
}
```

### Day 16-17: Main Integration

```rust
// core/src/code_intelligence/mod.rs
pub mod parser;
pub mod chunker;
pub mod compactor;
pub mod embedder;
pub mod vector_store;
pub mod retriever;

use std::path::{Path, PathBuf};
use dashmap::DashMap;
use notify::{Watcher, RecursiveMode, Event};

pub struct CodeIntelligenceEngine {
    parser: CodeParser,
    chunker: AstChunker,
    compactor: CodeCompactor,
    embedder: Arc<dyn CodeEmbedder>,
    vector_store: Arc<CodeVectorStore>,
    retriever: Arc<HybridRetriever>,
    file_cache: DashMap<PathBuf, FileMetadata>,
}

#[derive(Clone)]
struct FileMetadata {
    hash: String,
    chunks: Vec<String>, // chunk IDs
    last_indexed: SystemTime,
}

impl CodeIntelligenceEngine {
    pub async fn new(config: CodeIntelligenceConfig) -> Result<Self, Error> {
        let parser = CodeParser::new();
        let chunker = AstChunker::new();
        let compactor = CodeCompactor::new(config.max_tokens);
        
        let embedder: Arc<dyn CodeEmbedder> = if config.use_local_embedder {
            Arc::new(LocalCodeEmbedder::new(&config.model_path).await?)
        } else {
            Arc::new(OpenAICodeEmbedder::new(config.api_key))
        };
        
        let vector_store = Arc::new(CodeVectorStore::new(&config.db_path).await?);
        let retriever = Arc::new(HybridRetriever::new(
            vector_store.clone(),
            embedder.clone(),
        ).await?);
        
        Ok(Self {
            parser,
            chunker,
            compactor,
            embedder,
            vector_store,
            retriever,
            file_cache: DashMap::new(),
        })
    }
    
    pub async fn index_directory(&self, dir: &Path) -> Result<IndexStats, Error> {
        let mut stats = IndexStats::default();
        
        // Walk directory
        for entry in walkdir::WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if let Some(lang) = CodeLanguage::from_extension(ext.to_str().unwrap()) {
                        self.index_file(entry.path(), lang).await?;
                        stats.files_indexed += 1;
                    }
                }
            }
        }
        
        Ok(stats)
    }
    
    pub async fn index_file(&self, path: &Path, lang: CodeLanguage) -> Result<(), Error> {
        // Check if file needs reindexing
        let file_hash = self.calculate_file_hash(path)?;
        
        if let Some(metadata) = self.file_cache.get(path) {
            if metadata.hash == file_hash {
                return Ok(()); // Already up to date
            }
        }
        
        // Read and parse file
        let code = std::fs::read_to_string(path)?;
        let tree = self.parser.parse(&code, lang)?;
        
        // Extract chunks
        let chunks = self.chunker.extract_chunks(&tree, &code, path.to_str().unwrap(), lang);
        
        // Compact and embed chunks
        let mut chunk_ids = Vec::new();
        
        for chunk in chunks {
            let compacted = self.compactor.compact(&chunk);
            
            // Generate embedding
            let text_to_embed = format!(
                "{} {} {}",
                compacted.signature,
                compacted.docstring.as_ref().unwrap_or(&String::new()),
                compacted.summary
            );
            
            let embedding = self.embedder.embed(&text_to_embed).await?;
            
            // Store in vector database
            self.vector_store.insert(&compacted, embedding).await?;
            
            chunk_ids.push(chunk.id);
        }
        
        // Update cache
        self.file_cache.insert(
            path.to_path_buf(),
            FileMetadata {
                hash: file_hash,
                chunks: chunk_ids,
                last_indexed: SystemTime::now(),
            },
        );
        
        Ok(())
    }
    
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<CodeSearchResult>, Error> {
        let results = self.retriever.search(query, k).await?;
        
        // Enrich results with full context
        let mut enriched = Vec::new();
        
        for result in results {
            // Load chunk details from database
            let chunk = self.load_chunk(&result.chunk_id).await?;
            
            enriched.push(CodeSearchResult {
                chunk,
                score: result.score,
                // Add surrounding context if needed
            });
        }
        
        Ok(enriched)
    }
    
    pub fn start_file_watcher(&self, path: &Path) -> Result<(), Error> {
        let (tx, rx) = std::sync::mpsc::channel();
        
        let mut watcher = notify::recommended_watcher(tx)?;
        watcher.watch(path, RecursiveMode::Recursive)?;
        
        // Spawn watcher thread
        let engine = self.clone();
        tokio::spawn(async move {
            for res in rx {
                match res {
                    Ok(Event::Write(paths)) | Ok(Event::Create(paths)) => {
                        for path in paths {
                            if let Some(ext) = path.extension() {
                                if let Some(lang) = CodeLanguage::from_extension(ext.to_str().unwrap()) {
                                    engine.index_file(&path, lang).await.ok();
                                }
                            }
                        }
                    }
                    Ok(Event::Remove(paths)) => {
                        for path in paths {
                            engine.remove_file(&path).await.ok();
                        }
                    }
                    _ => {}
                }
            }
        });
        
        Ok(())
    }
}
```

### Day 18-19: TUI Integration

```rust
// tui/src/code_search_widget.rs
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub struct CodeSearchWidget {
    search_query: String,
    search_results: Vec<CodeSearchResult>,
    selected_index: usize,
    engine: Arc<CodeIntelligenceEngine>,
    is_searching: bool,
}

impl CodeSearchWidget {
    pub fn new(engine: Arc<CodeIntelligenceEngine>) -> Self {
        Self {
            search_query: String::new(),
            search_results: Vec::new(),
            selected_index: 0,
            engine,
            is_searching: false,
        }
    }
    
    pub async fn handle_input(&mut self, key: KeyEvent) -> Result<(), Error> {
        match key.code {
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.trigger_search().await?;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.trigger_search().await?;
            }
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.selected_index < self.search_results.len() - 1 {
                    self.selected_index += 1;
                }
            }
            KeyCode::Enter => {
                self.insert_selected_code();
            }
            _ => {}
        }
        Ok(())
    }
    
    async fn trigger_search(&mut self) -> Result<(), Error> {
        if self.search_query.len() < 3 {
            return Ok(());
        }
        
        self.is_searching = true;
        
        // Debounce search
        tokio::time::sleep(Duration::from_millis(300)).await;
        
        let results = self.engine.search(&self.search_query, 10).await?;
        self.search_results = results;
        self.selected_index = 0;
        self.is_searching = false;
        
        Ok(())
    }
    
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Search input
                Constraint::Min(5),     // Results
                Constraint::Length(10), // Preview
            ])
            .split(area);
        
        // Search input
        let input = Paragraph::new(self.search_query.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default()
                .borders(Borders::ALL)
                .title("Code Search (Ctrl+K)"));
        frame.render_widget(input, chunks[0]);
        
        // Search results
        let items: Vec<ListItem> = self.search_results
            .iter()
            .enumerate()
            .map(|(i, result)| {
                let style = if i == self.selected_index {
                    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                
                let content = format!(
                    "{} {} [{}] (score: {:.2})",
                    result.chunk.chunk_type,
                    result.chunk.name,
                    result.chunk.file_path,
                    result.score
                );
                
                ListItem::new(content).style(style)
            })
            .collect();
        
        let results = List::new(items)
            .block(Block::default()
                .borders(Borders::ALL)
                .title(format!("Results ({})", self.search_results.len())));
        frame.render_widget(results, chunks[1]);
        
        // Code preview
        if let Some(selected) = self.search_results.get(self.selected_index) {
            let preview = Paragraph::new(selected.chunk.signature.clone())
                .style(Style::default().fg(Color::Green))
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title("Preview"));
            frame.render_widget(preview, chunks[2]);
        }
    }
}
```

## Testing & Benchmarking

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_end_to_end() {
        // Create temp directory with test code
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        
        std::fs::write(&test_file, r#"
            fn calculate_sum(a: i32, b: i32) -> i32 {
                a + b
            }
            
            struct Calculator {
                value: i32,
            }
            
            impl Calculator {
                fn new(value: i32) -> Self {
                    Self { value }
                }
                
                fn add(&mut self, other: i32) {
                    self.value += other;
                }
            }
        "#).unwrap();
        
        // Initialize engine
        let config = CodeIntelligenceConfig::default();
        let engine = CodeIntelligenceEngine::new(config).await.unwrap();
        
        // Index the file
        engine.index_file(&test_file, CodeLanguage::Rust).await.unwrap();
        
        // Search for code
        let results = engine.search("calculate sum", 5).await.unwrap();
        
        assert!(!results.is_empty());
        assert_eq!(results[0].chunk.name, "calculate_sum");
        
        // Test struct search
        let results = engine.search("Calculator", 5).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.chunk.name == "Calculator"));
    }
    
    #[tokio::test]
    async fn benchmark_indexing() {
        use std::time::Instant;
        
        let engine = create_test_engine().await;
        let codebase_path = Path::new("/path/to/test/codebase");
        
        let start = Instant::now();
        let stats = engine.index_directory(codebase_path).await.unwrap();
        let duration = start.elapsed();
        
        println!("Indexed {} files in {:?}", stats.files_indexed, duration);
        println!("Rate: {} files/sec", stats.files_indexed as f64 / duration.as_secs_f64());
        
        assert!(stats.files_indexed > 0);
        assert!(duration.as_secs() < 60); // Should index in under a minute
    }
}
```

## Configuration

```toml
# ~/.agcodex/config.toml
[code_intelligence]
enabled = true
auto_index = true
incremental = true

[code_intelligence.parser]
languages = ["rust", "python", "javascript", "typescript", "go", "java", "cpp"]

[code_intelligence.chunking]
strategy = "semantic"  # semantic, function, or class
min_chunk_size = 50
max_chunk_size = 500

[code_intelligence.compaction]
enabled = true
max_tokens = 300
preserve_docstrings = true
preserve_types = true

[code_intelligence.embedding]
provider = "local"  # local, openai, or custom
model = "codebert-base"
batch_size = 32
cache_embeddings = true

[code_intelligence.storage]
backend = "lancedb"  # lancedb, qdrant, or milvus
path = "~/.agcodex/code_index"
compression = true

[code_intelligence.retrieval]
strategy = "hybrid"  # vector, keyword, or hybrid
vector_weight = 0.7
keyword_weight = 0.3
rerank = true
top_k = 10
```

## Performance Metrics

Expected performance with this implementation:
- **Indexing**: 500-1000 files/minute
- **Query latency**: <50ms for 100K chunks
- **Memory usage**: ~500MB for 10K files
- **Storage**: ~1-2KB per chunk
- **Accuracy**: 85-90% relevant results @k=10

## Next Steps

1. **Optimize embeddings**: Use quantization, caching
2. **Add graph relationships**: Build dependency graph
3. **Implement re-ranking**: Use cross-encoder for better relevance
4. **Add streaming updates**: Real-time indexing as code changes
5. **Multi-repo support**: Index multiple projects
6. **Custom language support**: Add more tree-sitter grammars
7. **Query understanding**: Parse natural language to code queries
8. **Context assembly**: Smart context window management

This implementation provides a solid foundation for AST-based code intelligence in AGCodex-RS, with room for optimization and enhancement based on specific use cases and performance requirements.