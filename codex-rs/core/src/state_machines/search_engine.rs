//! Search engine state machine with type-safe state transitions.

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use super::StateMachine;
use super::StateMachineError;
use super::StateMachineResult;
use super::StateMachineState;
use super::StateTransition;
use super::TrackedStateMachine;
use crate::builders::SearchQuery;
use crate::builders::SearchScope;
use crate::define_states;
use crate::types::FilePath;
use crate::types::QueryPattern;

// Define search engine specific states
define_states! {
    SearchIdle,
    SearchIndexing,
    SearchQuerying,
    SearchCompleted,
    SearchError,
}

/// Search result with metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: FilePath,
    pub line_number: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub matched_text: String,
    pub context_before: String,
    pub context_after: String,
    pub relevance_score: f32,
}

/// Search statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchStats {
    pub files_searched: usize,
    pub matches_found: usize,
    pub search_duration: std::time::Duration,
    pub index_size: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

/// Type-safe search engine with state machine
#[derive(Debug)]
pub struct SearchEngine<S: StateMachineState> {
    state_machine: TrackedStateMachine<S>,
    index: SearchIndex,
    query_cache: HashMap<String, Vec<SearchResult>>,
    config: SearchConfig,
    stats: SearchStats,
}

/// Search index structure
#[derive(Debug, Clone, Default)]
struct SearchIndex {
    files: HashMap<FilePath, FileIndex>,
    word_index: HashMap<String, Vec<FileLocation>>,
    last_updated: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone)]
struct FileIndex {
    content: String,
    lines: Vec<String>,
    word_positions: HashMap<String, Vec<usize>>,
    last_modified: std::time::SystemTime,
}

#[derive(Debug, Clone)]
struct FileLocation {
    file_path: FilePath,
    line: usize,
    column: usize,
}

/// Search engine configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchConfig {
    pub max_results: usize,
    pub case_sensitive: bool,
    pub regex_enabled: bool,
    pub cache_enabled: bool,
    pub index_refresh_interval: std::time::Duration,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: 1000,
            case_sensitive: false,
            regex_enabled: true,
            cache_enabled: true,
            index_refresh_interval: std::time::Duration::from_secs(300), // 5 minutes
        }
    }
}

impl SearchEngine<SearchIdle> {
    /// Create a new search engine in idle state
    pub fn new(config: SearchConfig) -> Self {
        Self {
            state_machine: TrackedStateMachine::new(),
            index: SearchIndex::default(),
            query_cache: HashMap::new(),
            config,
            stats: SearchStats {
                files_searched: 0,
                matches_found: 0,
                search_duration: std::time::Duration::ZERO,
                index_size: 0,
                cache_hits: 0,
                cache_misses: 0,
            },
        }
    }

    /// Start indexing files (transition to SearchIndexing state)
    pub fn start_indexing(
        mut self,
        files: Vec<FilePath>,
    ) -> StateMachineResult<SearchEngine<SearchIndexing>> {
        let transition = StateTransition::new("SearchIdle", "SearchIndexing")
            .with_metadata("file_count", files.len().to_string());

        self.state_machine.record_transition(transition);

        Ok(SearchEngine {
            state_machine: TrackedStateMachine::new(),
            index: self.index,
            query_cache: self.query_cache,
            config: self.config,
            stats: self.stats,
        })
    }
}

impl SearchEngine<SearchIndexing> {
    /// Add a file to the index
    pub fn index_file(&mut self, file_path: FilePath) -> StateMachineResult<()> {
        if !file_path.exists() {
            return Err(StateMachineError::PreconditionFailed {
                condition: format!("File does not exist: {}", file_path),
            });
        }

        let content = std::fs::read_to_string(file_path.as_path_buf()).map_err(|e| {
            StateMachineError::PreconditionFailed {
                condition: format!("Cannot read file: {}", e),
            }
        })?;

        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let word_positions = Self::build_word_index(&content);

        let file_index = FileIndex {
            content,
            lines,
            word_positions,
            last_modified: std::time::SystemTime::now(),
        };

        self.index.files.insert(file_path, file_index);
        self.stats.files_searched += 1;
        self.stats.index_size += 1;

        Ok(())
    }

    /// Complete indexing and transition to idle state
    pub fn complete_indexing(mut self) -> SearchEngine<SearchIdle> {
        let transition = StateTransition::new("SearchIndexing", "SearchIdle")
            .with_metadata("indexed_files", self.stats.index_size.to_string());

        self.state_machine.record_transition(transition);
        self.index.last_updated = Some(std::time::SystemTime::now());

        SearchEngine {
            state_machine: TrackedStateMachine::new(),
            index: self.index,
            query_cache: self.query_cache,
            config: self.config,
            stats: self.stats,
        }
    }

    fn build_word_index(content: &str) -> HashMap<String, Vec<usize>> {
        let mut word_positions = HashMap::new();

        for (pos, word) in content.split_whitespace().enumerate() {
            let word = word.to_lowercase();
            word_positions
                .entry(word)
                .or_insert_with(Vec::new)
                .push(pos);
        }

        word_positions
    }
}

impl SearchEngine<SearchIdle> {
    /// Start a search query (transition to SearchQuerying state)
    pub fn start_query(
        mut self,
        query: SearchQuery,
    ) -> StateMachineResult<SearchEngine<SearchQuerying>> {
        // Check cache first
        let cache_key = self.create_cache_key(&query);

        if self.config.cache_enabled && self.query_cache.contains_key(&cache_key) {
            self.stats.cache_hits += 1;
        } else {
            self.stats.cache_misses += 1;
        }

        let transition = StateTransition::new("SearchIdle", "SearchQuerying")
            .with_metadata("pattern", query.pattern().as_str().to_string())
            .with_metadata(
                "cache_hit",
                self.query_cache.contains_key(&cache_key).to_string(),
            );

        self.state_machine.record_transition(transition);

        Ok(SearchEngine {
            state_machine: TrackedStateMachine::new(),
            index: self.index,
            query_cache: self.query_cache,
            config: self.config,
            stats: self.stats,
        })
    }

    fn create_cache_key(&self, query: &SearchQuery) -> String {
        format!("{:?}", query)
    }
}

impl SearchEngine<SearchQuerying> {
    /// Execute the search and return results
    pub fn execute_search(
        mut self,
        query: &SearchQuery,
    ) -> StateMachineResult<SearchEngine<SearchCompleted>> {
        let start_time = std::time::Instant::now();

        let results = match query.scope() {
            SearchScope::CurrentDirectory => self.search_current_directory(query)?,
            SearchScope::Recursive { root } => self.search_recursive(query, root)?,
            SearchScope::Files { files } => self.search_specific_files(query, files)?,
            SearchScope::Glob { pattern } => self.search_glob(query, pattern)?,
        };

        let search_duration = start_time.elapsed();
        self.stats.search_duration = search_duration;
        self.stats.matches_found = results.len();

        // Cache results if enabled
        if self.config.cache_enabled {
            let cache_key = self.create_cache_key(query);
            self.query_cache.insert(cache_key, results);
        }

        let transition = StateTransition::new("SearchQuerying", "SearchCompleted")
            .with_metadata("matches_found", self.stats.matches_found.to_string())
            .with_metadata("duration_ms", search_duration.as_millis().to_string());

        self.state_machine.record_transition(transition);

        Ok(SearchEngine {
            state_machine: TrackedStateMachine::new(),
            index: self.index,
            query_cache: self.query_cache,
            config: self.config,
            stats: self.stats,
        })
    }

    /// Handle search error and transition to error state
    pub fn handle_error(mut self, error: String) -> SearchEngine<SearchError> {
        let transition =
            StateTransition::new("SearchQuerying", "SearchError").with_metadata("error", error);

        self.state_machine.record_transition(transition);

        SearchEngine {
            state_machine: TrackedStateMachine::new(),
            index: self.index,
            query_cache: self.query_cache,
            config: self.config,
            stats: self.stats,
        }
    }

    const fn search_current_directory(
        &self,
        query: &SearchQuery,
    ) -> StateMachineResult<Vec<SearchResult>> {
        // Simplified implementation
        Ok(Vec::new())
    }

    const fn search_recursive(
        &self,
        query: &SearchQuery,
        root: &FilePath,
    ) -> StateMachineResult<Vec<SearchResult>> {
        // Simplified implementation
        Ok(Vec::new())
    }

    fn search_specific_files(
        &self,
        query: &SearchQuery,
        files: &[FilePath],
    ) -> StateMachineResult<Vec<SearchResult>> {
        let mut results = Vec::new();

        for file_path in files {
            if let Some(file_index) = self.index.files.get(file_path) {
                let file_results = self.search_in_file(query, file_path, file_index)?;
                results.extend(file_results);

                if results.len() >= self.config.max_results {
                    break;
                }
            }
        }

        Ok(results)
    }

    const fn search_glob(
        &self,
        query: &SearchQuery,
        pattern: &QueryPattern,
    ) -> StateMachineResult<Vec<SearchResult>> {
        // Simplified implementation
        Ok(Vec::new())
    }

    fn search_in_file(
        &self,
        query: &SearchQuery,
        file_path: &FilePath,
        file_index: &FileIndex,
    ) -> StateMachineResult<Vec<SearchResult>> {
        let mut results = Vec::new();
        let pattern = query.pattern().as_str();

        for (line_num, line) in file_index.lines.iter().enumerate() {
            if let Some(col) = line.find(pattern) {
                let result = SearchResult {
                    file_path: file_path.clone(),
                    line_number: line_num + 1, // 1-based line numbers
                    column_start: col,
                    column_end: col + pattern.len(),
                    matched_text: pattern.to_string(),
                    context_before: self.get_context_before(&file_index.lines, line_num, 2),
                    context_after: self.get_context_after(&file_index.lines, line_num, 2),
                    relevance_score: 1.0, // Simplified scoring
                };
                results.push(result);
            }
        }

        Ok(results)
    }

    fn get_context_before(
        &self,
        lines: &[String],
        line_num: usize,
        context_lines: usize,
    ) -> String {
        let start = line_num.saturating_sub(context_lines);
        lines[start..line_num].join("\n")
    }

    fn get_context_after(&self, lines: &[String], line_num: usize, context_lines: usize) -> String {
        let end = std::cmp::min(line_num + 1 + context_lines, lines.len());
        lines[line_num + 1..end].join("\n")
    }

    fn create_cache_key(&self, query: &SearchQuery) -> String {
        format!("{:?}", query)
    }
}

impl SearchEngine<SearchCompleted> {
    /// Get search results
    pub fn get_results(&self, query: &SearchQuery) -> Option<&Vec<SearchResult>> {
        let cache_key = self.create_cache_key(query);
        self.query_cache.get(&cache_key)
    }

    /// Get search statistics
    pub const fn get_stats(&self) -> &SearchStats {
        &self.stats
    }

    /// Reset to idle state for new search
    pub fn reset(mut self) -> SearchEngine<SearchIdle> {
        let transition = StateTransition::new("SearchCompleted", "SearchIdle");
        self.state_machine.record_transition(transition);

        SearchEngine {
            state_machine: TrackedStateMachine::new(),
            index: self.index,
            query_cache: self.query_cache,
            config: self.config,
            stats: self.stats,
        }
    }

    fn create_cache_key(&self, query: &SearchQuery) -> String {
        format!("{:?}", query)
    }
}

impl SearchEngine<SearchError> {
    /// Get error information
    pub fn get_error_info(&self) -> Option<&StateTransition> {
        self.state_machine.last_transition()
    }

    /// Recover from error and return to idle state
    pub fn recover(mut self) -> SearchEngine<SearchIdle> {
        let transition =
            StateTransition::new("SearchError", "SearchIdle").with_metadata("recovery", "manual");

        self.state_machine.record_transition(transition);

        // Clear cache and reset stats on recovery
        SearchEngine {
            state_machine: TrackedStateMachine::new(),
            index: self.index,
            query_cache: HashMap::new(),
            config: self.config,
            stats: SearchStats {
                files_searched: self.stats.files_searched,
                matches_found: 0,
                search_duration: std::time::Duration::ZERO,
                index_size: self.stats.index_size,
                cache_hits: 0,
                cache_misses: 0,
            },
        }
    }
}

// Implement StateMachine trait for all states
impl<S: StateMachineState> StateMachine<S> for SearchEngine<S> {
    type Error = StateMachineError;

    fn state_name(&self) -> &'static str {
        std::any::type_name::<S>()
    }

    fn validate_state(&self) -> Result<(), Self::Error> {
        // Validate that the index is consistent
        if self.stats.index_size != self.index.files.len() {
            return Err(StateMachineError::PreconditionFailed {
                condition: "Index size mismatch".to_string(),
            });
        }

        Ok(())
    }

    fn can_transition_to(&self, target_state: &'static str) -> bool {
        match (self.state_name(), target_state) {
            ("SearchIdle", "SearchIndexing") => true,
            ("SearchIdle", "SearchQuerying") => true,
            ("SearchIndexing", "SearchIdle") => true,
            ("SearchQuerying", "SearchCompleted") => true,
            ("SearchQuerying", "SearchError") => true,
            ("SearchCompleted", "SearchIdle") => true,
            ("SearchError", "SearchIdle") => true,
            _ => false,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_engine_creation() {
        let config = SearchConfig::default();
        let engine = SearchEngine::<SearchIdle>::new(config);

        assert_eq!(engine.state_name(), "search_engine::SearchIdle");
        assert_eq!(engine.stats.files_searched, 0);
    }

    #[test]
    fn test_state_transitions() {
        let config = SearchConfig::default();
        let engine = SearchEngine::<SearchIdle>::new(config);

        // Test valid transitions
        assert!(engine.can_transition_to("SearchIndexing"));
        assert!(engine.can_transition_to("SearchQuerying"));
        assert!(!engine.can_transition_to("SearchCompleted")); // Invalid

        let files = vec![];
        let indexing_engine = engine.start_indexing(files).unwrap();
        assert_eq!(
            indexing_engine.state_name(),
            "search_engine::SearchIndexing"
        );

        let idle_engine = indexing_engine.complete_indexing();
        assert_eq!(idle_engine.state_name(), "search_engine::SearchIdle");
    }

    #[test]
    fn test_transition_history() {
        let config = SearchConfig::default();
        let engine = SearchEngine::<SearchIdle>::new(config);

        let files = vec![];
        let indexing_engine = engine.start_indexing(files).unwrap();
        let idle_engine = indexing_engine.complete_indexing();

        let history = idle_engine.state_machine.transition_history();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].from_state, "SearchIdle");
        assert_eq!(history[0].to_state, "SearchIndexing");
        assert_eq!(history[1].from_state, "SearchIndexing");
        assert_eq!(history[1].to_state, "SearchIdle");
    }

    #[test]
    fn test_search_stats() {
        let config = SearchConfig::default();
        let engine = SearchEngine::<SearchIdle>::new(config);

        assert_eq!(engine.stats.files_searched, 0);
        assert_eq!(engine.stats.matches_found, 0);
        assert_eq!(engine.stats.cache_hits, 0);
        assert_eq!(engine.stats.cache_misses, 0);
    }
}
