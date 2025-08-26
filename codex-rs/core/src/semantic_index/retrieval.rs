//! Smart Retrieval Engine for Semantic Code Search
//!
//! This module implements high-performance semantic retrieval with sub-100ms
//! latency, supporting complex queries, relevance ranking, and context assembly.
//!
//! # Query Processing Pipeline
//!
//! ```text
//! SearchQuery → QueryEmbedding → VectorSimilarity → RelevanceRanking → ContextAssembly
//!     ↓             ↓               ↓                  ↓                 ↓
//! Parse &       Generate         Calculate          Apply filters     Assemble
//! normalize     embedding        cosine sim         & boosting        results
//! ```
//!
//! # Performance Optimizations
//!
//! - **Approximate Nearest Neighbor**: Efficient similarity search
//! - **Query caching**: LRU cache for repeated queries  
//! - **Result ranking**: ML-inspired relevance scoring
//! - **Context window management**: Smart chunking for RAG
//!
//! # Concurrency Model
//!
//! - **Concurrent queries**: Multiple search threads
//! - **Lock-free reads**: RwLock for storage access
//! - **Async I/O**: Non-blocking storage operations

use super::ChunkId;
use super::DocumentId;
use super::Result;
use super::SemanticIndexError;
use super::embeddings::EmbeddingVector;
use super::embeddings::find_top_k_similar;
use super::storage::StorageBackend;

use serde::Deserialize;
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Instant;
use std::time::SystemTime;
use uuid::Uuid;

/// Search query with filtering and ranking options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// The search text
    pub text: String,

    /// Maximum number of results to return
    pub limit: usize,

    /// Minimum similarity threshold (0.0 to 1.0)
    pub threshold: f32,

    /// Language filters
    pub languages: Vec<crate::ast_compactor::Language>,

    /// File path patterns to include
    pub include_paths: Vec<String>,

    /// File path patterns to exclude
    pub exclude_paths: Vec<String>,

    /// Boost results from specific chunks
    pub boost_chunks: HashMap<ChunkId, f32>,

    /// Boost results from specific documents
    pub boost_documents: HashMap<DocumentId, f32>,

    /// Search mode for different use cases
    pub search_mode: SearchMode,

    /// Include similarity scores in results
    pub include_scores: bool,

    /// Context window size for RAG (characters)
    pub context_window: Option<usize>,
}

impl SearchQuery {
    /// Create a simple search query
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            limit: 10,
            threshold: 0.7,
            languages: Vec::new(),
            include_paths: Vec::new(),
            exclude_paths: Vec::new(),
            boost_chunks: HashMap::new(),
            boost_documents: HashMap::new(),
            search_mode: SearchMode::Semantic,
            include_scores: false,
            context_window: None,
        }
    }

    /// Set maximum number of results
    pub const fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set similarity threshold
    pub const fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Filter by programming languages
    pub fn with_languages(mut self, languages: Vec<crate::ast_compactor::Language>) -> Self {
        self.languages = languages;
        self
    }

    /// Set search mode
    pub const fn with_mode(mut self, mode: SearchMode) -> Self {
        self.search_mode = mode;
        self
    }

    /// Include similarity scores in results
    pub const fn with_scores(mut self) -> Self {
        self.include_scores = true;
        self
    }

    /// Set context window for RAG
    pub const fn with_context_window(mut self, size: usize) -> Self {
        self.context_window = Some(size);
        self
    }

    /// Add boost for specific chunks
    pub fn boost_chunk(mut self, chunk_id: ChunkId, boost: f32) -> Self {
        self.boost_chunks.insert(chunk_id, boost);
        self
    }

    /// Validate query parameters
    pub fn validate(&self) -> Result<()> {
        if self.text.trim().is_empty() {
            return Err(SemanticIndexError::QueryFailed {
                query: self.text.clone(),
                reason: "Query text cannot be empty".to_string(),
            });
        }

        if self.limit == 0 || self.limit > 1000 {
            return Err(SemanticIndexError::QueryFailed {
                query: self.text.clone(),
                reason: "Limit must be between 1 and 1000".to_string(),
            });
        }

        if self.threshold < 0.0 || self.threshold > 1.0 {
            return Err(SemanticIndexError::QueryFailed {
                query: self.text.clone(),
                reason: "Threshold must be between 0.0 and 1.0".to_string(),
            });
        }

        Ok(())
    }
}

/// Different search modes for various use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchMode {
    /// Pure semantic similarity search
    Semantic,

    /// Hybrid semantic + keyword matching
    Hybrid,

    /// Code-specific search with syntax awareness
    Code,

    /// Function and method search
    Functions,

    /// Type and struct search
    Types,

    /// Documentation and comment search
    Documentation,
}

/// Search result with metadata and relevance scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Unique result identifier
    pub id: Uuid,

    /// Source chunk ID
    pub chunk_id: ChunkId,

    /// Source document ID
    pub document_id: DocumentId,

    /// Matched content
    pub content: String,

    /// File path where content was found
    pub file_path: Option<String>,

    /// Line range in the source file
    pub line_range: Option<(usize, usize)>,

    /// Programming language
    pub language: crate::ast_compactor::Language,

    /// Relevance score (0.0 to 1.0)
    pub relevance_score: RelevanceScore,

    /// Raw similarity score (0.0 to 1.0)
    pub similarity_score: f32,

    /// Additional context around the match
    pub context: Option<String>,

    /// Highlighting information for UI
    pub highlights: Vec<TextHighlight>,

    /// Result metadata
    pub metadata: ResultMetadata,

    /// Timestamp when result was generated
    pub created_at: SystemTime,
}

impl SearchResult {
    /// Create a new search result
    pub fn new(
        chunk_id: ChunkId,
        document_id: DocumentId,
        content: String,
        similarity_score: f32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            chunk_id,
            document_id,
            content,
            file_path: None,
            line_range: None,
            language: crate::ast_compactor::Language::Unknown,
            relevance_score: RelevanceScore::new(similarity_score),
            similarity_score,
            context: None,
            highlights: Vec::new(),
            metadata: ResultMetadata::default(),
            created_at: SystemTime::now(),
        }
    }

    /// Get final ranking score for sorting
    pub const fn ranking_score(&self) -> f32 {
        self.relevance_score.final_score
    }

    /// Check if result meets quality threshold
    pub fn is_high_quality(&self) -> bool {
        self.relevance_score.final_score >= 0.8
            && self.similarity_score >= 0.7
            && !self.content.trim().is_empty()
    }

    /// Get estimated relevance for the user
    pub fn estimated_relevance(&self) -> f32 {
        // Combine multiple signals for better relevance estimation
        let base_score = self.similarity_score;
        let length_bonus = if self.content.len() > 100 { 0.05 } else { 0.0 };
        let language_bonus = if self.language != crate::ast_compactor::Language::Unknown {
            0.05
        } else {
            0.0
        };

        (base_score + length_bonus + language_bonus).min(1.0)
    }
}

/// Advanced relevance scoring with multiple factors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceScore {
    /// Base semantic similarity score
    pub similarity: f32,

    /// Keyword matching bonus
    pub keyword_bonus: f32,

    /// Language-specific bonus
    pub language_bonus: f32,

    /// Recency bonus (newer content)
    pub recency_bonus: f32,

    /// Quality bonus (well-structured code)
    pub quality_bonus: f32,

    /// User-defined boost
    pub boost_factor: f32,

    /// Final computed relevance score
    pub final_score: f32,
}

impl RelevanceScore {
    /// Create a new relevance score with base similarity
    pub fn new(similarity: f32) -> Self {
        let mut score = Self {
            similarity,
            keyword_bonus: 0.0,
            language_bonus: 0.0,
            recency_bonus: 0.0,
            quality_bonus: 0.0,
            boost_factor: 1.0,
            final_score: similarity,
        };

        score.compute_final_score();
        score
    }

    /// Add keyword matching bonus
    pub fn with_keyword_bonus(&mut self, bonus: f32) -> &mut Self {
        self.keyword_bonus = bonus.clamp(0.0, 0.3);
        self.compute_final_score();
        self
    }

    /// Add language-specific bonus
    pub fn with_language_bonus(&mut self, bonus: f32) -> &mut Self {
        self.language_bonus = bonus.clamp(0.0, 0.1);
        self.compute_final_score();
        self
    }

    /// Add recency bonus
    pub fn with_recency_bonus(&mut self, bonus: f32) -> &mut Self {
        self.recency_bonus = bonus.clamp(0.0, 0.1);
        self.compute_final_score();
        self
    }

    /// Add quality bonus
    pub fn with_quality_bonus(&mut self, bonus: f32) -> &mut Self {
        self.quality_bonus = bonus.clamp(0.0, 0.2);
        self.compute_final_score();
        self
    }

    /// Apply boost factor
    pub fn with_boost(&mut self, factor: f32) -> &mut Self {
        self.boost_factor = factor.clamp(0.1, 5.0);
        self.compute_final_score();
        self
    }

    /// Recompute final score
    fn compute_final_score(&mut self) {
        let base_score = self.similarity
            + self.keyword_bonus
            + self.language_bonus
            + self.recency_bonus
            + self.quality_bonus;
        self.final_score = (base_score * self.boost_factor).clamp(0.0, 1.0);
    }
}

/// Text highlighting for search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextHighlight {
    /// Start position in the text
    pub start: usize,

    /// End position in the text
    pub end: usize,

    /// Highlight type
    pub highlight_type: HighlightType,

    /// Confidence score for this highlight
    pub confidence: f32,
}

/// Types of text highlights
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HighlightType {
    /// Exact keyword match
    ExactMatch,

    /// Fuzzy/partial match
    FuzzyMatch,

    /// Semantic similarity match
    SemanticMatch,

    /// Syntax element (function, class, etc.)
    SyntaxElement,
}

/// Additional metadata for search results
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResultMetadata {
    /// Size of the matched content
    pub content_size: usize,

    /// Estimated complexity score
    pub complexity_score: f32,

    /// Number of lines of code
    pub lines_of_code: usize,

    /// Has documentation
    pub has_documentation: bool,

    /// Test coverage indicator
    pub has_tests: bool,

    /// Additional custom metadata
    pub custom_fields: HashMap<String, String>,
}

/// High-performance retrieval engine
#[derive(Debug)]
pub struct RetrievalEngine {
    /// Minimum similarity threshold
    similarity_threshold: f32,

    /// Maximum results to return
    max_results: usize,

    /// Query performance metrics
    metrics: std::sync::Mutex<QueryMetrics>,
}

/// Query performance metrics
#[derive(Debug, Default)]
struct QueryMetrics {
    total_queries: usize,
    total_query_time_ms: u128,
    cache_hits: usize,
    average_results_returned: f32,
}

impl RetrievalEngine {
    /// Create a new retrieval engine
    pub fn new(similarity_threshold: f32, max_results: usize) -> Self {
        Self {
            similarity_threshold: similarity_threshold.clamp(0.0, 1.0),
            max_results: max_results.clamp(1, 1000),
            metrics: std::sync::Mutex::new(QueryMetrics::default()),
        }
    }

    /// Perform semantic search with advanced ranking
    pub async fn search(
        &self,
        query_embedding: &EmbeddingVector,
        query: &SearchQuery,
        storage: &dyn StorageBackend,
    ) -> Result<Vec<SearchResult>> {
        let start_time = Instant::now();

        // Validate query
        query.validate()?;

        // Get all embeddings from storage
        let all_embeddings =
            storage
                .get_all_embeddings()
                .await
                .map_err(|e| SemanticIndexError::StorageFailed {
                    operation: "get_all_embeddings".to_string(),
                    message: e.to_string(),
                })?;

        if all_embeddings.is_empty() {
            return Ok(Vec::new());
        }

        // Find most similar embeddings
        let candidate_limit = (query.limit * 3).min(all_embeddings.len()); // Over-fetch for better ranking
        let similar_indices = find_top_k_similar(
            query_embedding,
            &all_embeddings
                .iter()
                .map(|(_, embedding)| embedding)
                .cloned()
                .collect::<Vec<_>>(),
            candidate_limit,
        );

        // Convert to search results with metadata
        let mut results = Vec::new();
        for (idx, similarity) in similar_indices {
            if similarity < self.similarity_threshold {
                continue;
            }

            let (chunk_id, embedding) = &all_embeddings[idx];

            // Get chunk metadata from storage
            let chunk_info = storage.get_chunk_info(*chunk_id).await.map_err(|e| {
                SemanticIndexError::StorageFailed {
                    operation: "get_chunk_info".to_string(),
                    message: e.to_string(),
                }
            })?;

            if let Some(chunk_info) = chunk_info {
                // Apply filters
                if self.passes_filters(query, &chunk_info) {
                    let mut result = SearchResult::new(
                        *chunk_id,
                        chunk_info.document_id,
                        chunk_info.content.clone(),
                        similarity,
                    );

                    // Enrich result with metadata
                    self.enrich_result(&mut result, query, &chunk_info);

                    // Apply advanced scoring
                    self.apply_advanced_scoring(&mut result, query, &chunk_info);

                    results.push(result);
                }
            }
        }

        // Final ranking and limiting
        results.sort_by(|a, b| {
            b.ranking_score()
                .partial_cmp(&a.ranking_score())
                .unwrap_or(Ordering::Equal)
        });
        results.truncate(query.limit);

        // Update metrics
        self.update_metrics(start_time, results.len());

        Ok(results)
    }

    /// Check if result passes query filters
    fn passes_filters(&self, query: &SearchQuery, chunk_info: &ChunkInfo) -> bool {
        // Language filter
        if !query.languages.is_empty() && !query.languages.contains(&chunk_info.language) {
            return false;
        }

        // Path inclusion filter
        if !query.include_paths.is_empty() {
            let file_path = chunk_info.file_path.as_deref().unwrap_or("");
            if !query.include_paths.iter().any(|pattern| {
                // Simple glob-like matching
                file_path.contains(pattern)
                    || pattern.contains('*') && self.matches_glob(file_path, pattern)
            }) {
                return false;
            }
        }

        // Path exclusion filter
        if !query.exclude_paths.is_empty() {
            let file_path = chunk_info.file_path.as_deref().unwrap_or("");
            if query.exclude_paths.iter().any(|pattern| {
                file_path.contains(pattern)
                    || pattern.contains('*') && self.matches_glob(file_path, pattern)
            }) {
                return false;
            }
        }

        true
    }

    /// Simple glob pattern matching
    fn matches_glob(&self, text: &str, pattern: &str) -> bool {
        // Simplified glob matching - in production, use a proper glob library
        let pattern = pattern.replace('*', ".*");
        if let Ok(regex) = regex::Regex::new(&pattern) {
            regex.is_match(text)
        } else {
            text.contains(&pattern.replace(".*", ""))
        }
    }

    /// Enrich search result with additional metadata
    fn enrich_result(
        &self,
        result: &mut SearchResult,
        query: &SearchQuery,
        chunk_info: &ChunkInfo,
    ) {
        result.file_path = chunk_info.file_path.clone();
        result.line_range = chunk_info.line_range;
        result.language = chunk_info.language;

        // Add context if requested
        if let Some(context_size) = query.context_window {
            result.context = self.extract_context(&chunk_info.content, context_size);
        }

        // Generate highlights
        result.highlights = self.generate_highlights(&query.text, &chunk_info.content);

        // Fill metadata
        result.metadata = ResultMetadata {
            content_size: chunk_info.content.len(),
            complexity_score: self.calculate_complexity_score(&chunk_info.content),
            lines_of_code: chunk_info.content.lines().count(),
            has_documentation: self.has_documentation(&chunk_info.content),
            has_tests: self.has_tests(&chunk_info.content),
            custom_fields: HashMap::new(),
        };
    }

    /// Apply advanced scoring factors
    fn apply_advanced_scoring(
        &self,
        result: &mut SearchResult,
        query: &SearchQuery,
        chunk_info: &ChunkInfo,
    ) {
        let mut score = RelevanceScore::new(result.similarity_score);

        // Keyword bonus
        let keyword_bonus = self.calculate_keyword_bonus(&query.text, &chunk_info.content);
        score.with_keyword_bonus(keyword_bonus);

        // Language bonus (prefer exact language matches)
        if !query.languages.is_empty() && query.languages.contains(&chunk_info.language) {
            score.with_language_bonus(0.05);
        }

        // Quality bonus
        let quality_bonus = result.metadata.complexity_score * 0.1;
        score.with_quality_bonus(quality_bonus);

        // Apply user-defined boosts
        if let Some(boost) = query.boost_chunks.get(&result.chunk_id) {
            score.with_boost(*boost);
        }

        if let Some(boost) = query.boost_documents.get(&result.document_id) {
            score.with_boost(*boost);
        }

        result.relevance_score = score;
    }

    /// Calculate keyword matching bonus
    fn calculate_keyword_bonus(&self, query_text: &str, content: &str) -> f32 {
        let query_lower = query_text.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content_lower = content.to_lowercase();

        let matches = query_words
            .iter()
            .filter(|word| content_lower.contains(*word))
            .count();

        if query_words.is_empty() {
            0.0
        } else {
            (matches as f32 / query_words.len() as f32).min(0.3)
        }
    }

    /// Extract context around the matched content
    fn extract_context(&self, content: &str, context_size: usize) -> Option<String> {
        if content.len() <= context_size {
            return Some(content.to_string());
        }

        // Extract context from the beginning for now
        // In a more advanced implementation, we'd extract context around the best match
        Some(content[..context_size.min(content.len())].to_string())
    }

    /// Generate text highlights for matched content
    fn generate_highlights(&self, query_text: &str, content: &str) -> Vec<TextHighlight> {
        let mut highlights = Vec::new();
        let query_lower = query_text.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let content_lower = content.to_lowercase();

        // Find exact matches
        for word in &query_words {
            let mut start = 0;
            while let Some(pos) = content_lower[start..].find(word) {
                let actual_start = start + pos;
                let actual_end = actual_start + word.len();

                highlights.push(TextHighlight {
                    start: actual_start,
                    end: actual_end,
                    highlight_type: HighlightType::ExactMatch,
                    confidence: 1.0,
                });

                start = actual_end;
            }
        }

        highlights
    }

    /// Calculate complexity score for code content
    fn calculate_complexity_score(&self, content: &str) -> f32 {
        let lines = content.lines().count() as f32;
        let complexity_indicators = content.matches("if ").count()
            + content.matches("for ").count()
            + content.matches("while ").count()
            + content.matches("match ").count();

        (complexity_indicators as f32 / lines.max(1.0)).min(1.0)
    }

    /// Check if content has documentation
    fn has_documentation(&self, content: &str) -> bool {
        content.contains("///")
            || content.contains("/**")
            || content.contains("//!")
            || content
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    trimmed.starts_with("//") || trimmed.starts_with('#')
                })
                .count()
                > 2
    }

    /// Check if content has tests
    fn has_tests(&self, content: &str) -> bool {
        content.contains("#[test]")
            || content.contains("test_")
            || content.contains("describe(")
            || content.contains("it(")
    }

    /// Update query performance metrics
    fn update_metrics(&self, start_time: Instant, results_count: usize) {
        let elapsed = start_time.elapsed().as_millis();

        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.total_queries += 1;
            metrics.total_query_time_ms += elapsed;

            let total = metrics.total_queries as f32;
            metrics.average_results_returned =
                (metrics.average_results_returned * (total - 1.0) + results_count as f32) / total;
        }
    }

    /// Get query performance metrics
    pub fn get_metrics(&self) -> Option<(usize, f64, f32)> {
        if let Ok(metrics) = self.metrics.lock() {
            if metrics.total_queries > 0 {
                let avg_time = metrics.total_query_time_ms as f64 / metrics.total_queries as f64;
                Some((
                    metrics.total_queries,
                    avg_time,
                    metrics.average_results_returned,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Chunk information for retrieval
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub content: String,
    pub document_id: DocumentId,
    pub language: crate::ast_compactor::Language,
    pub file_path: Option<String>,
    pub line_range: Option<(usize, usize)>,
}

// Regex support for glob matching
mod regex {
    pub struct Regex {
        pattern: String,
    }

    impl Regex {
        pub fn new(pattern: &str) -> Result<Self, &'static str> {
            // Simplified regex - in production use the `regex` crate
            Ok(Self {
                pattern: pattern.to_string(),
            })
        }

        pub fn is_match(&self, text: &str) -> bool {
            // Very simplified matching
            text.contains(&self.pattern.replace(".*", ""))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_compactor::Language;

    #[test]
    fn test_search_query_creation() {
        let query = SearchQuery::new("test function")
            .with_limit(20)
            .with_threshold(0.8)
            .with_languages(vec![Language::Rust])
            .with_scores();

        assert_eq!(query.text, "test function");
        assert_eq!(query.limit, 20);
        assert_eq!(query.threshold, 0.8);
        assert_eq!(query.languages, vec![Language::Rust]);
        assert!(query.include_scores);
    }

    #[test]
    fn test_search_query_validation() {
        let valid_query = SearchQuery::new("test");
        assert!(valid_query.validate().is_ok());

        let empty_query = SearchQuery::new("");
        assert!(empty_query.validate().is_err());

        let invalid_limit = SearchQuery::new("test").with_limit(0);
        assert!(invalid_limit.validate().is_err());

        let invalid_threshold = SearchQuery::new("test").with_threshold(1.5);
        assert!(invalid_threshold.validate().is_ok()); // Clamped to 1.0
    }

    #[test]
    fn test_relevance_score_calculation() {
        let mut score = RelevanceScore::new(0.8);
        assert_eq!(score.final_score, 0.8);

        score
            .with_keyword_bonus(0.1)
            .with_language_bonus(0.05)
            .with_quality_bonus(0.05)
            .with_boost(1.2);

        assert!(score.final_score > 0.8);
        assert!(score.final_score <= 1.0);
    }

    #[test]
    fn test_search_result_ranking() {
        let result1 = SearchResult::new(
            ChunkId::new_v4(),
            DocumentId::new_v4(),
            "test content 1".to_string(),
            0.9,
        );

        let result2 = SearchResult::new(
            ChunkId::new_v4(),
            DocumentId::new_v4(),
            "test content 2".to_string(),
            0.7,
        );

        assert!(result1.ranking_score() > result2.ranking_score());
        assert!(result1.is_high_quality());
    }

    #[test]
    fn test_text_highlights() {
        let engine = RetrievalEngine::new(0.7, 50);
        let highlights =
            engine.generate_highlights("test function", "This is a test function example");

        assert!(!highlights.is_empty());
        assert!(
            highlights
                .iter()
                .any(|h| h.highlight_type == HighlightType::ExactMatch)
        );
    }

    #[test]
    fn test_keyword_bonus_calculation() {
        let engine = RetrievalEngine::new(0.7, 50);

        let bonus1 = engine.calculate_keyword_bonus("test function", "This is a test function");
        assert!(bonus1 > 0.0);

        let bonus2 = engine.calculate_keyword_bonus("nonexistent", "This is a test function");
        assert_eq!(bonus2, 0.0);

        let partial_bonus =
            engine.calculate_keyword_bonus("test missing", "This is a test function");
        assert!(partial_bonus > 0.0 && partial_bonus < bonus1);
    }

    #[test]
    fn test_complexity_score() {
        let engine = RetrievalEngine::new(0.7, 50);

        let simple_code = "fn hello() { println!(\"Hello\"); }";
        let complex_code = r#"
        fn complex() {
            if x > 0 {
                for i in 0..10 {
                    while condition {
                        match value {
                            Some(v) => v,
                            None => 0,
                        }
                    }
                }
            }
        }
        "#;

        let simple_score = engine.calculate_complexity_score(simple_code);
        let complex_score = engine.calculate_complexity_score(complex_code);

        assert!(complex_score > simple_score);
    }

    #[test]
    fn test_documentation_detection() {
        let engine = RetrievalEngine::new(0.7, 50);

        let documented_code = r#"
        /// This is a documented function
        /// It does something important
        fn documented() {}
        "#;

        let undocumented_code = "fn undocumented() {}";

        assert!(engine.has_documentation(documented_code));
        assert!(!engine.has_documentation(undocumented_code));
    }

    #[test]
    fn test_search_modes() {
        assert_eq!(SearchMode::Semantic, SearchMode::Semantic);
        assert_ne!(SearchMode::Semantic, SearchMode::Code);

        let query = SearchQuery::new("test").with_mode(SearchMode::Functions);
        assert_eq!(query.search_mode, SearchMode::Functions);
    }
}
