//! Type-safe builder for context window construction with memory management.

use std::marker::PhantomData;
use std::num::NonZeroUsize;

use serde::Deserialize;
use serde::Serialize;

use super::BuilderError;
use super::BuilderResult;
use super::BuilderState;
use super::Init;
use super::Ready;
use super::Validated;
use crate::types::ContextWindow;
use crate::types::FilePath;

/// Priority levels for context items
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ContextPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Context item with metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextItem {
    pub content: ContextWindow,
    pub priority: ContextPriority,
    pub source: ContextSource,
    pub weight: f32,
}

/// Source of context information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextSource {
    /// Direct user input
    UserInput,
    /// File content
    File { path: FilePath },
    /// Search results
    SearchResult { query: String },
    /// AST analysis
    AstAnalysis { language: String },
    /// Historical context
    History { timestamp: u64 },
    /// System message
    System,
}

/// Context compression strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionStrategy {
    /// No compression - keep everything
    None,
    /// Remove low-priority items first
    PriorityBased,
    /// Keep recent items, remove old ones
    TimeBasedLru,
    /// Use semantic similarity to deduplicate
    SemanticDedup,
    /// Hybrid approach using multiple strategies
    Hybrid,
}

/// Context window configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Maximum total size in bytes
    pub max_size: usize,
    /// Maximum number of items
    pub max_items: Option<NonZeroUsize>,
    /// Compression strategy when limits are exceeded
    pub compression_strategy: CompressionStrategy,
    /// Minimum priority to include
    pub min_priority: ContextPriority,
    /// Reserve space for system messages
    pub system_reserve: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_size: 32_768, // 32KB default
            max_items: NonZeroUsize::new(50),
            compression_strategy: CompressionStrategy::PriorityBased,
            min_priority: ContextPriority::Low,
            system_reserve: 1024, // 1KB for system messages
        }
    }
}

/// Final context window with all items
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Context {
    items: Vec<ContextItem>,
    config: ContextConfig,
    total_size: usize,
}

impl Context {
    /// Create a new builder
    pub fn builder() -> ContextBuilder<Init> {
        ContextBuilder::new()
    }

    /// Get all context items
    pub fn items(&self) -> &[ContextItem] {
        &self.items
    }

    /// Get configuration
    pub const fn config(&self) -> &ContextConfig {
        &self.config
    }

    /// Get total size in bytes
    pub const fn total_size(&self) -> usize {
        self.total_size
    }

    /// Get available space
    pub const fn available_space(&self) -> usize {
        self.config.max_size.saturating_sub(self.total_size)
    }

    /// Check if context is at capacity
    pub fn is_at_capacity(&self) -> bool {
        self.available_space() == 0
            || (self
                .config
                .max_items
                .is_some_and(|max| self.items.len() >= max.get()))
    }

    /// Get items by priority
    pub fn items_by_priority(&self, priority: ContextPriority) -> Vec<&ContextItem> {
        self.items
            .iter()
            .filter(|item| item.priority >= priority)
            .collect()
    }

    /// Compress context using configured strategy
    pub fn compress(&mut self) -> BuilderResult<usize> {
        let original_size = self.total_size;

        match self.config.compression_strategy {
            CompressionStrategy::None => { /* no compression */ }
            CompressionStrategy::PriorityBased => self.compress_by_priority()?,
            CompressionStrategy::TimeBasedLru => self.compress_by_time()?,
            CompressionStrategy::SemanticDedup => self.compress_semantic()?,
            CompressionStrategy::Hybrid => self.compress_hybrid()?,
        }

        Ok(original_size - self.total_size)
    }

    fn compress_by_priority(&mut self) -> BuilderResult<()> {
        // Sort by priority (lowest first) and remove until under limit
        self.items.sort_by(|a, b| a.priority.cmp(&b.priority));

        while self.total_size > self.config.max_size && !self.items.is_empty() {
            if !self.items.is_empty() {
                let item = self.items.remove(0);
                self.total_size = self.total_size.saturating_sub(item.content.len());
            }
        }

        Ok(())
    }

    fn compress_by_time(&mut self) -> BuilderResult<()> {
        // For simplicity, remove items with History source first (oldest first)
        let mut history_indices: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                if let ContextSource::History { timestamp } = &item.source {
                    Some((i, *timestamp))
                } else {
                    None
                }
            })
            .collect();

        // Sort by timestamp (oldest first)
        history_indices.sort_by_key(|(_, timestamp)| *timestamp);

        // Remove oldest history items first
        for (index, _) in history_indices.into_iter().rev() {
            if self.total_size <= self.config.max_size {
                break;
            }
            let item = self.items.remove(index);
            self.total_size = self.total_size.saturating_sub(item.content.len());
        }

        Ok(())
    }

    fn compress_semantic(&mut self) -> BuilderResult<()> {
        // Simplified semantic deduplication - remove exact duplicates
        let mut unique_items = Vec::new();
        let mut seen_content = std::collections::HashSet::new();

        for item in self.items.drain(..) {
            let content_hash = item.content.content();
            if seen_content.insert(content_hash.to_string()) {
                unique_items.push(item);
            } else {
                self.total_size = self.total_size.saturating_sub(item.content.len());
            }
        }

        self.items = unique_items;
        Ok(())
    }

    fn compress_hybrid(&mut self) -> BuilderResult<()> {
        // First remove duplicates, then by priority
        self.compress_semantic()?;
        if self.total_size > self.config.max_size {
            self.compress_by_priority()?;
        }
        Ok(())
    }
}

/// Type-safe builder for Context
#[derive(Debug)]
pub struct ContextBuilder<S: BuilderState> {
    items: Vec<ContextItem>,
    config: ContextConfig,
    total_size: usize,
    _state: PhantomData<S>,
}

impl ContextBuilder<Init> {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            config: ContextConfig::default(),
            total_size: 0,
            _state: PhantomData,
        }
    }

    /// Set maximum size (transitions to Validated state)
    pub fn max_size(mut self, max_size: usize) -> BuilderResult<ContextBuilder<Validated>> {
        if max_size == 0 {
            return Err(BuilderError::InvalidField {
                field: "max_size",
                value: "0".to_string(),
            });
        }

        self.config.max_size = max_size;

        Ok(ContextBuilder {
            items: self.items,
            config: self.config,
            total_size: self.total_size,
            _state: PhantomData,
        })
    }
}

impl<S: BuilderState> ContextBuilder<S> {
    /// Set maximum number of items
    pub const fn max_items(mut self, max_items: Option<NonZeroUsize>) -> Self {
        self.config.max_items = max_items;
        self
    }

    /// Set compression strategy
    pub const fn compression_strategy(mut self, strategy: CompressionStrategy) -> Self {
        self.config.compression_strategy = strategy;
        self
    }

    /// Set minimum priority
    pub const fn min_priority(mut self, priority: ContextPriority) -> Self {
        self.config.min_priority = priority;
        self
    }

    /// Set system reserve space
    pub const fn system_reserve(mut self, reserve: usize) -> Self {
        self.config.system_reserve = reserve;
        self
    }
}

impl ContextBuilder<Validated> {
    /// Add a context item (transitions to Ready state on first item)
    pub fn add_item(
        mut self,
        content: impl TryInto<ContextWindow>,
        priority: ContextPriority,
        source: ContextSource,
    ) -> BuilderResult<ContextBuilder<Ready>> {
        let content = content.try_into().map_err(|_| BuilderError::InvalidField {
            field: "content",
            value: "invalid content".to_string(),
        })?;

        let item = ContextItem {
            content,
            priority,
            source,
            weight: priority as u8 as f32,
        };

        self.total_size += item.content.len();
        self.items.push(item);

        Ok(ContextBuilder {
            items: self.items,
            config: self.config,
            total_size: self.total_size,
            _state: PhantomData,
        })
    }
}

impl ContextBuilder<Ready> {
    /// Add more context items
    pub fn add_item(
        mut self,
        content: impl TryInto<ContextWindow>,
        priority: ContextPriority,
        source: ContextSource,
    ) -> BuilderResult<Self> {
        let content = content.try_into().map_err(|_| BuilderError::InvalidField {
            field: "content",
            value: "invalid content".to_string(),
        })?;

        let item = ContextItem {
            content,
            priority,
            source,
            weight: priority as u8 as f32,
        };

        self.total_size += item.content.len();
        self.items.push(item);

        Ok(self)
    }

    /// Add user input item
    pub fn add_user_input(self, content: impl TryInto<ContextWindow>) -> BuilderResult<Self> {
        self.add_item(content, ContextPriority::Critical, ContextSource::UserInput)
    }

    /// Add file content item
    pub fn add_file_content(
        self,
        content: impl TryInto<ContextWindow>,
        path: FilePath,
        priority: ContextPriority,
    ) -> BuilderResult<Self> {
        self.add_item(content, priority, ContextSource::File { path })
    }

    /// Add search result item
    pub fn add_search_result(
        self,
        content: impl TryInto<ContextWindow>,
        query: String,
        priority: ContextPriority,
    ) -> BuilderResult<Self> {
        self.add_item(content, priority, ContextSource::SearchResult { query })
    }

    /// Add system message
    pub fn add_system_message(self, content: impl TryInto<ContextWindow>) -> BuilderResult<Self> {
        self.add_item(content, ContextPriority::Critical, ContextSource::System)
    }

    /// Build the final Context
    pub fn build(self) -> BuilderResult<Context> {
        if self.items.is_empty() {
            return Err(BuilderError::MissingField { field: "items" });
        }

        let mut context = Context {
            items: self.items,
            config: self.config,
            total_size: self.total_size,
        };

        // Apply compression if needed
        if context.total_size > context.config.max_size {
            context.compress()?;
        }

        Ok(context)
    }

    /// Build with automatic compression
    pub fn build_compressed(self) -> BuilderResult<Context> {
        let mut context = self.build()?;
        context.compress()?;
        Ok(context)
    }
}

impl Default for ContextBuilder<Init> {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_builder_flow() {
        let context = Context::builder()
            .max_size(1024)
            .unwrap()
            .add_item(
                "Hello, world!",
                ContextPriority::Critical,
                ContextSource::UserInput,
            )
            .unwrap()
            .add_user_input("More input")
            .unwrap()
            .add_system_message("System ready")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(context.items().len(), 3);
        assert!(context.total_size() > 0);
        assert!(context.total_size() <= 1024);
    }

    #[test]
    fn test_context_priorities() {
        let file_path = FilePath::new("test.txt").unwrap();

        let context = Context::builder()
            .max_size(2048)
            .unwrap()
            .add_item(
                "file content",
                ContextPriority::Medium,
                ContextSource::File {
                    path: file_path.clone(),
                },
            )
            .unwrap()
            .add_file_content("more file content", file_path, ContextPriority::Medium)
            .unwrap()
            .add_user_input("user input")
            .unwrap()
            .build()
            .unwrap();

        let critical_items = context.items_by_priority(ContextPriority::Critical);
        assert_eq!(critical_items.len(), 1);

        let all_items = context.items_by_priority(ContextPriority::Low);
        assert_eq!(all_items.len(), 3);
    }

    #[test]
    fn test_compression() {
        let context = Context::builder()
            .max_size(50) // Very small limit
            .unwrap()
            .compression_strategy(CompressionStrategy::PriorityBased)
            .add_item(
                "low priority content",
                ContextPriority::Low,
                ContextSource::System,
            )
            .unwrap()
            .add_item(
                "high priority content",
                ContextPriority::High,
                ContextSource::System,
            )
            .unwrap()
            .build()
            .unwrap();

        assert!(context.total_size() <= 50);

        // Should keep high priority items
        let high_priority = context.items_by_priority(ContextPriority::High);
        assert!(!high_priority.is_empty());
    }

    #[test]
    fn test_builder_requires_max_size() {
        // This test demonstrates that the builder enforces proper state transitions
        // The following would not compile:
        // let result = Context::builder()
        //     .add_user_input("test"); // Would fail - need max_size first

        // Proper usage requires max_size first:
        let _result = Context::builder()
            .max_size(1024)
            .unwrap()
            .add_item("test", ContextPriority::High, ContextSource::UserInput)
            .unwrap()
            .build()
            .unwrap();
    }

    #[test]
    fn test_context_source_types() {
        let file_path = FilePath::new("example.rs").unwrap();
        let source = ContextSource::File { path: file_path };

        match source {
            ContextSource::File { path } => {
                assert_eq!(path.as_str(), Some("example.rs"));
            }
            _ => panic!("Expected File source"),
        }
    }
}
