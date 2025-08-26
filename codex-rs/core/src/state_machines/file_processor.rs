//! File processor state machine with type-safe processing pipeline.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;

use super::StateMachine;
use super::StateMachineError;
use super::StateMachineResult;
use super::StateMachineState;
use super::StateTransition;
use super::TrackedStateMachine;
use crate::define_states;
use crate::types::FilePath;

// Define file processor states
define_states! {
    FileIdle,
    FileScanning,
    FileProcessing,
    FileCompleted,
    FileError,
}

/// File processing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessingMode {
    /// Read file contents only
    ReadOnly,
    /// Parse file structure (AST when possible)
    Parse,
    /// Full analysis including semantics
    Analyze,
    /// Transform/modify file contents
    Transform,
}

/// File type classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileType {
    SourceCode { language: String },
    Configuration { format: String },
    Documentation { format: String },
    Data { format: String },
    Binary,
    Unknown,
}

/// Processed file information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessedFile {
    pub path: FilePath,
    pub file_type: FileType,
    pub size: usize,
    pub content: Option<String>,
    pub metadata: HashMap<String, String>,
    pub processing_time: std::time::Duration,
    pub issues: Vec<String>,
}

/// File processing statistics
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub files_scanned: usize,
    pub files_processed: usize,
    pub files_skipped: usize,
    pub files_failed: usize,
    pub total_size: usize,
    pub total_duration: std::time::Duration,
    pub by_type: HashMap<FileType, usize>,
}

/// File processing configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessorConfig {
    pub mode: ProcessingMode,
    pub max_file_size: usize,
    pub include_hidden: bool,
    pub follow_symlinks: bool,
    pub parallel_processing: bool,
    pub supported_extensions: Vec<String>,
    pub excluded_patterns: Vec<String>,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            mode: ProcessingMode::ReadOnly,
            max_file_size: 10 * 1024 * 1024, // 10MB
            include_hidden: false,
            follow_symlinks: false,
            parallel_processing: true,
            supported_extensions: vec![
                "rs".to_string(),
                "ts".to_string(),
                "js".to_string(),
                "py".to_string(),
                "go".to_string(),
                "java".to_string(),
                "cpp".to_string(),
                "c".to_string(),
                "h".to_string(),
            ],
            excluded_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                "build".to_string(),
            ],
        }
    }
}

/// Type-safe file processor with state machine
#[derive(Debug)]
pub struct FileProcessor<S: StateMachineState> {
    state_machine: TrackedStateMachine<S>,
    config: ProcessorConfig,
    processed_files: Vec<ProcessedFile>,
    stats: ProcessingStats,
    current_directory: Option<PathBuf>,
}

impl FileProcessor<FileIdle> {
    /// Create a new file processor
    pub fn new(config: ProcessorConfig) -> Self {
        Self {
            state_machine: TrackedStateMachine::new(),
            config,
            processed_files: Vec::new(),
            stats: ProcessingStats::default(),
            current_directory: None,
        }
    }

    /// Start scanning a directory (transition to FileScanning)
    pub fn start_scanning(
        mut self,
        directory: impl Into<PathBuf>,
    ) -> StateMachineResult<FileProcessor<FileScanning>> {
        let directory = directory.into();

        if !directory.exists() || !directory.is_dir() {
            return Err(StateMachineError::PreconditionFailed {
                condition: format!(
                    "Directory does not exist or is not a directory: {:?}",
                    directory
                ),
            });
        }

        let transition = StateTransition::new("FileIdle", "FileScanning")
            .with_metadata("directory", directory.display().to_string());

        self.state_machine.record_transition(transition);
        self.current_directory = Some(directory);

        Ok(FileProcessor {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            processed_files: self.processed_files,
            stats: self.stats,
            current_directory: self.current_directory,
        })
    }
}

impl FileProcessor<FileScanning> {
    /// Scan directory and collect files
    pub fn scan_files(&mut self) -> StateMachineResult<Vec<FilePath>> {
        let directory = self.current_directory.as_ref().ok_or_else(|| {
            StateMachineError::PreconditionFailed {
                condition: "No directory set for scanning".to_string(),
            }
        })?;

        let mut files = Vec::new();

        match self.scan_directory_recursive(directory) {
            Ok(discovered_files) => {
                files.extend(discovered_files);
                self.stats.files_scanned = files.len();
                Ok(files)
            }
            Err(e) => Err(StateMachineError::PreconditionFailed {
                condition: format!("Failed to scan directory: {}", e),
            }),
        }
    }

    /// Complete scanning and transition to processing
    pub fn start_processing(
        mut self,
        files: Vec<FilePath>,
    ) -> StateMachineResult<FileProcessor<FileProcessing>> {
        if files.is_empty() {
            return Err(StateMachineError::PreconditionFailed {
                condition: "No files to process".to_string(),
            });
        }

        let transition = StateTransition::new("FileScanning", "FileProcessing")
            .with_metadata("file_count", files.len().to_string());

        self.state_machine.record_transition(transition);

        Ok(FileProcessor {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            processed_files: self.processed_files,
            stats: self.stats,
            current_directory: self.current_directory,
        })
    }

    /// Handle scanning error
    pub fn handle_scan_error(mut self, error: String) -> FileProcessor<FileError> {
        let transition =
            StateTransition::new("FileScanning", "FileError").with_metadata("error", error);

        self.state_machine.record_transition(transition);

        FileProcessor {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            processed_files: self.processed_files,
            stats: self.stats,
            current_directory: self.current_directory,
        }
    }

    fn scan_directory_recursive(&self, dir: &PathBuf) -> std::io::Result<Vec<FilePath>> {
        let mut files = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if self.should_process_directory(&path)
                    && (self.config.follow_symlinks || !path.is_symlink()) {
                        let sub_files = self.scan_directory_recursive(&path)?;
                        files.extend(sub_files);
                    }
            } else if path.is_file() && self.should_process_file(&path)
                && let Ok(file_path) = FilePath::new(path) {
                    files.push(file_path);
                }
        }

        Ok(files)
    }

    fn should_process_directory(&self, path: &PathBuf) -> bool {
        let path_str = path.to_string_lossy();

        // Skip hidden directories unless configured to include them
        if !self.config.include_hidden
            && path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'))
        {
            return false;
        }

        // Check excluded patterns
        for pattern in &self.config.excluded_patterns {
            if path_str.contains(pattern) {
                return false;
            }
        }

        true
    }

    fn should_process_file(&self, path: &PathBuf) -> bool {
        let path_str = path.to_string_lossy();

        // Skip hidden files unless configured to include them
        if !self.config.include_hidden
            && path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with('.'))
        {
            return false;
        }

        // Check file size limit
        if let Ok(metadata) = std::fs::metadata(path)
            && metadata.len() > self.config.max_file_size as u64 {
                return false;
            }

        // Check supported extensions
        if !self.config.supported_extensions.is_empty() {
            let has_supported_extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| {
                    self.config
                        .supported_extensions
                        .iter()
                        .any(|supported| supported == ext)
                })
                .unwrap_or(false);

            if !has_supported_extension {
                return false;
            }
        }

        // Check excluded patterns
        for pattern in &self.config.excluded_patterns {
            if path_str.contains(pattern) {
                return false;
            }
        }

        true
    }
}

impl FileProcessor<FileProcessing> {
    /// Process a single file
    pub fn process_file(&mut self, file_path: &FilePath) -> StateMachineResult<ProcessedFile> {
        let start_time = std::time::Instant::now();

        let mut processed = ProcessedFile {
            path: file_path.clone(),
            file_type: self.classify_file(file_path),
            size: 0,
            content: None,
            metadata: HashMap::new(),
            processing_time: std::time::Duration::ZERO,
            issues: Vec::new(),
        };

        // Get file size
        if let Ok(metadata) = std::fs::metadata(file_path.as_path_buf()) {
            processed.size = metadata.len() as usize;
            self.stats.total_size += processed.size;
        }

        // Process based on mode and file type
        match self.config.mode {
            ProcessingMode::ReadOnly => {
                self.read_file_content(&mut processed)?;
            }
            ProcessingMode::Parse => {
                self.read_file_content(&mut processed)?;
                self.parse_file_structure(&mut processed)?;
            }
            ProcessingMode::Analyze => {
                self.read_file_content(&mut processed)?;
                self.parse_file_structure(&mut processed)?;
                self.analyze_file_semantics(&mut processed)?;
            }
            ProcessingMode::Transform => {
                self.read_file_content(&mut processed)?;
                self.parse_file_structure(&mut processed)?;
                self.transform_file_content(&mut processed)?;
            }
        }

        processed.processing_time = start_time.elapsed();
        self.stats.total_duration += processed.processing_time;
        self.stats.files_processed += 1;

        // Update type statistics
        *self
            .stats
            .by_type
            .entry(processed.file_type.clone())
            .or_insert(0) += 1;

        Ok(processed)
    }

    /// Complete processing and transition to completed state
    pub fn complete_processing(mut self) -> FileProcessor<FileCompleted> {
        let transition = StateTransition::new("FileProcessing", "FileCompleted")
            .with_metadata("files_processed", self.stats.files_processed.to_string())
            .with_metadata("total_size", self.stats.total_size.to_string());

        self.state_machine.record_transition(transition);

        FileProcessor {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            processed_files: self.processed_files,
            stats: self.stats,
            current_directory: self.current_directory,
        }
    }

    /// Handle processing error
    pub fn handle_processing_error(mut self, error: String) -> FileProcessor<FileError> {
        let transition =
            StateTransition::new("FileProcessing", "FileError").with_metadata("error", error);

        self.state_machine.record_transition(transition);

        FileProcessor {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            processed_files: self.processed_files,
            stats: self.stats,
            current_directory: self.current_directory,
        }
    }

    fn classify_file(&self, file_path: &FilePath) -> FileType {
        if let Some(extension) = file_path
            .as_path_buf()
            .extension()
            .and_then(|ext| ext.to_str())
        {
            match extension.to_lowercase().as_str() {
                // Source code files
                "rs" => FileType::SourceCode {
                    language: "rust".to_string(),
                },
                "js" | "jsx" | "mjs" => FileType::SourceCode {
                    language: "javascript".to_string(),
                },
                "ts" | "tsx" => FileType::SourceCode {
                    language: "typescript".to_string(),
                },
                "py" | "pyx" => FileType::SourceCode {
                    language: "python".to_string(),
                },
                "go" => FileType::SourceCode {
                    language: "go".to_string(),
                },
                "java" => FileType::SourceCode {
                    language: "java".to_string(),
                },
                "c" | "h" => FileType::SourceCode {
                    language: "c".to_string(),
                },
                "cpp" | "cxx" | "hpp" | "hxx" => FileType::SourceCode {
                    language: "cpp".to_string(),
                },

                // Configuration files
                "json" => FileType::Configuration {
                    format: "json".to_string(),
                },
                "yaml" | "yml" => FileType::Configuration {
                    format: "yaml".to_string(),
                },
                "toml" => FileType::Configuration {
                    format: "toml".to_string(),
                },
                "xml" => FileType::Configuration {
                    format: "xml".to_string(),
                },
                "ini" | "cfg" | "conf" => FileType::Configuration {
                    format: "ini".to_string(),
                },

                // Documentation files
                "md" | "markdown" => FileType::Documentation {
                    format: "markdown".to_string(),
                },
                "txt" => FileType::Documentation {
                    format: "text".to_string(),
                },
                "rst" => FileType::Documentation {
                    format: "restructuredtext".to_string(),
                },
                "adoc" | "asciidoc" => FileType::Documentation {
                    format: "asciidoc".to_string(),
                },

                // Data files
                "csv" => FileType::Data {
                    format: "csv".to_string(),
                },
                "tsv" => FileType::Data {
                    format: "tsv".to_string(),
                },
                "parquet" => FileType::Data {
                    format: "parquet".to_string(),
                },
                "sqlite" | "db" => FileType::Data {
                    format: "sqlite".to_string(),
                },

                // Binary files
                "exe" | "dll" | "so" | "dylib" => FileType::Binary,
                "png" | "jpg" | "jpeg" | "gif" | "svg" => FileType::Binary,
                "zip" | "tar" | "gz" | "bz2" | "xz" => FileType::Binary,

                _ => FileType::Unknown,
            }
        } else {
            FileType::Unknown
        }
    }

    fn read_file_content(&self, processed: &mut ProcessedFile) -> StateMachineResult<()> {
        match std::fs::read_to_string(processed.path.as_path_buf()) {
            Ok(content) => {
                processed.content = Some(content);
                processed
                    .metadata
                    .insert("encoding".to_string(), "utf-8".to_string());
                Ok(())
            }
            Err(e) => {
                // Try reading as binary if UTF-8 fails
                match std::fs::read(processed.path.as_path_buf()) {
                    Ok(bytes) => {
                        processed
                            .metadata
                            .insert("encoding".to_string(), "binary".to_string());
                        processed
                            .metadata
                            .insert("size_bytes".to_string(), bytes.len().to_string());
                        processed
                            .issues
                            .push(format!("File is binary, content not loaded: {}", e));
                        Ok(())
                    }
                    Err(read_err) => Err(StateMachineError::PreconditionFailed {
                        condition: format!("Cannot read file {}: {}", processed.path, read_err),
                    }),
                }
            }
        }
    }

    fn parse_file_structure(&self, processed: &mut ProcessedFile) -> StateMachineResult<()> {
        // Simplified parsing - count lines, functions, etc.
        if let Some(content) = &processed.content {
            let lines = content.lines().count();
            processed
                .metadata
                .insert("line_count".to_string(), lines.to_string());

            // Count some basic patterns
            match &processed.file_type {
                FileType::SourceCode { language } => {
                    let function_count = self.count_functions(content, language);
                    processed
                        .metadata
                        .insert("function_count".to_string(), function_count.to_string());
                }
                FileType::Configuration { .. } => {
                    processed
                        .metadata
                        .insert("type".to_string(), "configuration".to_string());
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn analyze_file_semantics(&self, processed: &mut ProcessedFile) -> StateMachineResult<()> {
        // Placeholder for semantic analysis
        processed
            .metadata
            .insert("analyzed".to_string(), "true".to_string());
        Ok(())
    }

    fn transform_file_content(&self, processed: &mut ProcessedFile) -> StateMachineResult<()> {
        // Placeholder for content transformation
        processed
            .metadata
            .insert("transformed".to_string(), "true".to_string());
        Ok(())
    }

    fn count_functions(&self, content: &str, language: &str) -> usize {
        // Simplified function counting
        match language {
            "rust" => content.matches("fn ").count(),
            "javascript" | "typescript" => {
                content.matches("function ").count()
                    + content.matches(" => ").count()
                    + content.matches("() => {").count()
            }
            "python" => content.matches("def ").count(),
            "java" | "c" | "cpp" => content.matches("() {").count(),
            "go" => content.matches("func ").count(),
            _ => 0,
        }
    }
}

impl FileProcessor<FileCompleted> {
    /// Get processed files
    pub fn get_processed_files(&self) -> &[ProcessedFile] {
        &self.processed_files
    }

    /// Get processing statistics
    pub const fn get_stats(&self) -> &ProcessingStats {
        &self.stats
    }

    /// Reset to idle state for new processing
    pub fn reset(mut self) -> FileProcessor<FileIdle> {
        let transition = StateTransition::new("FileCompleted", "FileIdle");
        self.state_machine.record_transition(transition);

        FileProcessor {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            processed_files: Vec::new(),
            stats: ProcessingStats::default(),
            current_directory: None,
        }
    }
}

impl FileProcessor<FileError> {
    /// Get error information
    pub fn get_error_info(&self) -> Option<&StateTransition> {
        self.state_machine.last_transition()
    }

    /// Recover from error
    pub fn recover(mut self) -> FileProcessor<FileIdle> {
        let transition =
            StateTransition::new("FileError", "FileIdle").with_metadata("recovery", "manual");

        self.state_machine.record_transition(transition);

        FileProcessor {
            state_machine: TrackedStateMachine::new(),
            config: self.config,
            processed_files: Vec::new(),
            stats: ProcessingStats::default(),
            current_directory: None,
        }
    }
}

// Implement StateMachine trait for all states
impl<S: StateMachineState> StateMachine<S> for FileProcessor<S> {
    type Error = StateMachineError;

    fn state_name(&self) -> &'static str {
        std::any::type_name::<S>()
    }

    fn validate_state(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn can_transition_to(&self, target_state: &'static str) -> bool {
        match (self.state_name(), target_state) {
            ("FileIdle", "FileScanning") => true,
            ("FileScanning", "FileProcessing") => true,
            ("FileScanning", "FileError") => true,
            ("FileProcessing", "FileCompleted") => true,
            ("FileProcessing", "FileError") => true,
            ("FileCompleted", "FileIdle") => true,
            ("FileError", "FileIdle") => true,
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
    use std::path::PathBuf;

    #[test]
    fn test_file_processor_creation() {
        let config = ProcessorConfig::default();
        let processor = FileProcessor::<FileIdle>::new(config);

        assert_eq!(processor.state_name(), "file_processor::FileIdle");
        assert_eq!(processor.stats.files_processed, 0);
    }

    #[test]
    fn test_file_type_classification() {
        let config = ProcessorConfig::default();
        let processor = FileProcessor::<FileIdle>::new(config);

        // Create dummy processing state to access classify_file method
        // This is a bit hacky but necessary for testing private methods
        let test_files = vec![
            (
                "test.rs",
                FileType::SourceCode {
                    language: "rust".to_string(),
                },
            ),
            (
                "config.json",
                FileType::Configuration {
                    format: "json".to_string(),
                },
            ),
            (
                "README.md",
                FileType::Documentation {
                    format: "markdown".to_string(),
                },
            ),
            (
                "data.csv",
                FileType::Data {
                    format: "csv".to_string(),
                },
            ),
        ];

        // We can't easily test classify_file as it's private, but we can test
        // the logic through integration
        for (filename, expected_type) in test_files {
            let path = PathBuf::from(filename);
            if let Ok(file_path) = FilePath::new(path) {
                // We would need to access the method through a FileProcessing instance
                // This test demonstrates the structure but can't run without actual files
            }
        }
    }

    #[test]
    fn test_config_defaults() {
        let config = ProcessorConfig::default();

        assert_eq!(config.mode, ProcessingMode::ReadOnly);
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert!(!config.include_hidden);
        assert!(!config.follow_symlinks);
        assert!(config.parallel_processing);
        assert!(config.supported_extensions.contains(&"rs".to_string()));
        assert!(
            config
                .excluded_patterns
                .contains(&"node_modules".to_string())
        );
    }

    #[test]
    fn test_processing_stats() {
        let stats = ProcessingStats::default();

        assert_eq!(stats.files_scanned, 0);
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.files_skipped, 0);
        assert_eq!(stats.files_failed, 0);
        assert_eq!(stats.total_size, 0);
        assert_eq!(stats.by_type.len(), 0);
    }
}
