//! Type-safe builder for search queries with compile-time state validation.

use std::marker::PhantomData;

use serde::Deserialize;
use serde::Serialize;

use super::BuilderError;
use super::BuilderResult;
use super::BuilderState;
use super::Init;
use super::Ready;
use super::Validated;
use crate::types::FilePath;
use crate::types::QueryPattern;

/// Search scope for queries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchScope {
    /// Search in current directory only
    CurrentDirectory,
    /// Search recursively from a root path
    Recursive { root: FilePath },
    /// Search in specific files only
    Files { files: Vec<FilePath> },
    /// Search in files matching a glob pattern
    Glob { pattern: QueryPattern },
}

/// Search query configuration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQueryConfig {
    /// Case-sensitive search
    pub case_sensitive: bool,
    /// Use regex patterns
    pub regex: bool,
    /// Maximum number of results
    pub max_results: Option<usize>,
    /// Search in hidden files
    pub include_hidden: bool,
    /// Follow symbolic links
    pub follow_links: bool,
}

impl Default for SearchQueryConfig {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            regex: false,
            max_results: Some(1000),
            include_hidden: false,
            follow_links: false,
        }
    }
}

/// Final search query object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub pattern: QueryPattern,
    pub scope: SearchScope,
    pub config: SearchQueryConfig,
}

impl SearchQuery {
    /// Create a new builder
    pub fn builder() -> SearchQueryBuilder<Init> {
        SearchQueryBuilder::new()
    }

    /// Get the query pattern
    pub const fn pattern(&self) -> &QueryPattern {
        &self.pattern
    }

    /// Get the search scope
    pub const fn scope(&self) -> &SearchScope {
        &self.scope
    }

    /// Get the configuration
    pub const fn config(&self) -> &SearchQueryConfig {
        &self.config
    }

    /// Check if this query uses regex patterns
    pub fn is_regex_query(&self) -> bool {
        self.config.regex || self.pattern.is_regex()
    }
}

/// Type-safe builder for SearchQuery with compile-time state tracking
#[derive(Debug)]
pub struct SearchQueryBuilder<S: BuilderState> {
    pattern: Option<QueryPattern>,
    scope: Option<SearchScope>,
    config: SearchQueryConfig,
    _state: PhantomData<S>,
}

impl SearchQueryBuilder<Init> {
    /// Create a new builder in initial state
    pub fn new() -> Self {
        Self {
            pattern: None,
            scope: None,
            config: SearchQueryConfig::default(),
            _state: PhantomData,
        }
    }
}

impl<S: BuilderState> SearchQueryBuilder<S> {
    /// Set case sensitivity
    pub const fn case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.config.case_sensitive = case_sensitive;
        self
    }

    /// Enable regex patterns
    pub const fn regex(mut self, regex: bool) -> Self {
        self.config.regex = regex;
        self
    }

    /// Set maximum results
    pub const fn max_results(mut self, max_results: Option<usize>) -> Self {
        self.config.max_results = max_results;
        self
    }

    /// Include hidden files
    pub const fn include_hidden(mut self, include_hidden: bool) -> Self {
        self.config.include_hidden = include_hidden;
        self
    }

    /// Follow symbolic links
    pub const fn follow_links(mut self, follow_links: bool) -> Self {
        self.config.follow_links = follow_links;
        self
    }
}

impl SearchQueryBuilder<Init> {
    /// Set the search pattern (transitions to Validated state)
    pub fn pattern(
        mut self,
        pattern: impl TryInto<QueryPattern>,
    ) -> BuilderResult<SearchQueryBuilder<Validated>> {
        let pattern = pattern.try_into().map_err(|_| BuilderError::InvalidField {
            field: "pattern",
            value: "invalid pattern".to_string(),
        })?;

        self.pattern = Some(pattern);

        Ok(SearchQueryBuilder {
            pattern: self.pattern,
            scope: self.scope,
            config: self.config,
            _state: PhantomData,
        })
    }
}

impl SearchQueryBuilder<Validated> {
    /// Set search scope to current directory
    pub fn scope_current_dir(mut self) -> SearchQueryBuilder<Ready> {
        self.scope = Some(SearchScope::CurrentDirectory);

        SearchQueryBuilder {
            pattern: self.pattern,
            scope: self.scope,
            config: self.config,
            _state: PhantomData,
        }
    }

    /// Set search scope to recursive from root
    pub fn scope_recursive(
        mut self,
        root: impl TryInto<FilePath>,
    ) -> BuilderResult<SearchQueryBuilder<Ready>> {
        let root = root.try_into().map_err(|_| BuilderError::InvalidField {
            field: "root",
            value: "invalid path".to_string(),
        })?;

        self.scope = Some(SearchScope::Recursive { root });

        Ok(SearchQueryBuilder {
            pattern: self.pattern,
            scope: self.scope,
            config: self.config,
            _state: PhantomData,
        })
    }

    /// Set search scope to specific files
    pub fn scope_files(mut self, files: Vec<FilePath>) -> SearchQueryBuilder<Ready> {
        self.scope = Some(SearchScope::Files { files });

        SearchQueryBuilder {
            pattern: self.pattern,
            scope: self.scope,
            config: self.config,
            _state: PhantomData,
        }
    }

    /// Set search scope using glob pattern
    pub fn scope_glob(
        mut self,
        pattern: impl TryInto<QueryPattern>,
    ) -> BuilderResult<SearchQueryBuilder<Ready>> {
        let pattern = pattern.try_into().map_err(|_| BuilderError::InvalidField {
            field: "glob_pattern",
            value: "invalid glob pattern".to_string(),
        })?;

        self.scope = Some(SearchScope::Glob { pattern });

        Ok(SearchQueryBuilder {
            pattern: self.pattern,
            scope: self.scope,
            config: self.config,
            _state: PhantomData,
        })
    }
}

impl SearchQueryBuilder<Ready> {
    /// Build the final SearchQuery
    pub fn build(self) -> BuilderResult<SearchQuery> {
        let pattern = self
            .pattern
            .ok_or(BuilderError::MissingField { field: "pattern" })?;
        let scope = self
            .scope
            .ok_or(BuilderError::MissingField { field: "scope" })?;

        Ok(SearchQuery {
            pattern,
            scope,
            config: self.config,
        })
    }
}

impl Default for SearchQueryBuilder<Init> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Convenience Functions
// ============================================================================

/// Create a simple text search query
pub fn simple_text_search(
    pattern: &str,
    root: impl TryInto<FilePath>,
) -> BuilderResult<SearchQuery> {
    SearchQuery::builder()
        .pattern(pattern)?
        .scope_recursive(root)?
        .build()
}

/// Create a regex search query
pub fn regex_search(pattern: &str, root: impl TryInto<FilePath>) -> BuilderResult<SearchQuery> {
    SearchQuery::builder()
        .pattern(pattern)?
        .regex(true)
        .scope_recursive(root)?
        .build()
}

/// Create a glob-based file search
pub fn glob_file_search(
    glob_pattern: &str,
    root: impl TryInto<FilePath>,
) -> BuilderResult<SearchQuery> {
    SearchQuery::builder()
        .pattern(".*")? // Match all content
        .scope_recursive(root)?
        .build()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_query_builder_flow() {
        // This should compile - correct state transitions
        let query = SearchQuery::builder()
            .pattern("test")
            .unwrap()
            .scope_current_dir()
            .case_sensitive(true)
            .build()
            .unwrap();

        assert_eq!(query.pattern().as_str(), "test");
        assert!(matches!(query.scope(), SearchScope::CurrentDirectory));
        assert!(query.config().case_sensitive);
    }

    #[test]
    fn test_search_query_builder_with_path() {
        let root = FilePath::new("/tmp").unwrap();

        let query = SearchQuery::builder()
            .pattern("*.rs")
            .unwrap()
            .scope_recursive(root)
            .unwrap()
            .regex(true)
            .build()
            .unwrap();

        assert!(query.is_regex_query());
    }

    #[test]
    fn test_builder_error_on_invalid_pattern() {
        let result = SearchQuery::builder().pattern(""); // Empty pattern should fail

        assert!(result.is_err());
    }

    #[test]
    fn test_convenience_functions() {
        let root = FilePath::new("/tmp").unwrap();

        let query = simple_text_search("main", root).unwrap();
        assert!(!query.is_regex_query());

        let root2 = FilePath::new("/tmp").unwrap();
        let regex_query = regex_search(r"main\(.*\)", root2).unwrap();
        assert!(regex_query.is_regex_query());
    }

    // These should not compile due to type system:
    //
    // let bad_builder = SearchQuery::builder()
    //     .scope_current_dir()  // Error: can't set scope before pattern
    //     .pattern("test");
    //
    // let bad_build = SearchQuery::builder()
    //     .pattern("test")
    //     .build();  // Error: can't build without setting scope
}
