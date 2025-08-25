# AGCodex Query Library Usage Guide

## Overview

The AGCodex Query Library provides a comprehensive, high-performance tree-sitter query system for structural code analysis. It supports 10+ programming languages out of the box with pre-compiled, cached queries for optimal performance.

## Core Features

### 🔍 Language Support
- **Rust**: Functions, structs, traits, impl blocks, modules
- **Python**: Functions, classes, methods, imports
- **JavaScript/TypeScript**: Functions, classes, imports, symbols
- **Go**: Functions, structs, interfaces, packages
- **Java**: Methods, classes, interfaces, constructors
- **C/C++**: Functions, structs, unions, enums, includes

### ⚡ Performance Optimizations
- **Query Caching**: DashMap-based lock-free cache
- **Precompilation**: Common queries compiled at startup
- **Arc Sharing**: Zero-copy query sharing across threads
- **Target Performance**: <10ms compilation, >90% cache hit rate

## Usage Examples

### Basic Query Library Usage

```rust
use agcodex_core::code_tools::queries::{QueryLibrary, QueryType};
use ast::Language;

// Initialize the library (with precompiled queries)
let library = QueryLibrary::new();

// Get a compiled query for Rust functions
let rust_functions = library.get_query(Language::Rust, QueryType::Functions)?;
println!("Query description: {}", rust_functions.description);
println!("Capture names: {:?}", rust_functions.capture_names);

// Check language support
if library.supports_query(Language::Python, &QueryType::Classes) {
    let python_classes = library.get_query(Language::Python, QueryType::Classes)?;
    // Use the compiled query...
}

// Get supported query types for a language
let rust_queries = library.supported_query_types(Language::Rust);
println!("Rust supports: {:?}", rust_queries);
```

### Custom Query Templates

```rust
// Build a custom parameterized query
let custom_template = r#"
(function_declaration
  name: (identifier) @func_name (#match? @func_name "test_.*")
  body: (block) @body) @test_function
"#;

let test_functions = library.get_custom_query(
    Language::JavaScript,
    QueryType::Functions,
    custom_template,
    "test_functions"
)?;
```

### Integration with TreeSitterTool

```rust
use agcodex_core::code_tools::tree_sitter::TreeSitterTool;

let tool = TreeSitterTool::new();

// Execute structured queries
let matches = tool.search_structured(
    Language::Rust,
    QueryType::Functions,
    vec![PathBuf::from("src/main.rs")]
).await?;

// Check query support
if tool.supports_query(Language::Python, &QueryType::Classes) {
    // Execute Python class queries...
}

// Get performance statistics
let stats = tool.query_stats();
println!("Cache hit rate: {:.2}%", stats.hit_rate() * 100.0);
```

## Query Types

### Functions (`QueryType::Functions`)
Extract function definitions, declarations, and method signatures:

**Rust**: `function_item`, method declarations in `impl` blocks
**Python**: `function_definition`, `async_function_definition`
**JavaScript**: `function_declaration`, `arrow_function`, `function_expression`
**Go**: `function_declaration`, `method_declaration`
**Java**: `method_declaration`
**C/C++**: `function_definition`, `function_declaration`

### Classes (`QueryType::Classes`)
Extract type definitions and object-oriented constructs:

**Rust**: `struct_item`, `enum_item`, `trait_item`, `impl_item`
**Python**: `class_definition`
**JavaScript**: `class_declaration`, `class_expression`
**Go**: `struct_type`, `interface_type`
**Java**: `class_declaration`, `interface_declaration`, `enum_declaration`
**C/C++**: `struct_specifier`, `union_specifier`, `class_specifier` (C++)

### Imports (`QueryType::Imports`)
Extract module dependencies and import statements:

**Rust**: `use_declaration`, `extern_crate_declaration`, `mod_item`
**Python**: `import_statement`, `import_from_statement`
**JavaScript**: `import_statement`, `export_statement`
**Go**: `import_declaration`, `package_clause`
**Java**: `import_declaration`, `package_declaration`
**C/C++**: `preproc_include`, `using_declaration` (C++), `namespace_definition` (C++)

### Symbols (`QueryType::Symbols`)
Extract variable declarations and symbol definitions:

**Rust**: `let_declaration`, `const_item`, `static_item`, `type_item`
**Python**: `assignment`, `augmented_assignment`
**JavaScript**: `variable_declaration`, `lexical_declaration`
**Go**: `var_declaration`, `const_declaration`
**Java**: `field_declaration`, `local_variable_declaration`
**C/C++**: Variable declarations, macro definitions

### Methods (`QueryType::Methods`)
Extract method definitions within classes/types:

**Rust**: Methods within `impl` blocks
**Python**: Methods within `class_definition`
**JavaScript**: `method_definition` within classes
**Go**: Methods with receivers
**Java**: Methods within classes/interfaces

## Architecture

### Memory Layout
```
QueryLibrary (Arc)
├── QueryBuilder (cached compilation)
├── QueryTemplates (language-specific patterns)
└── QueryCache (DashMap<CacheKey, Arc<CompiledQuery>>)

CacheKey = (Language, QueryType, Optional<Variant>)
CompiledQuery = Arc<tree_sitter::Query> + metadata
```

### Concurrency Model
- **Lock-free cache**: DashMap for concurrent access
- **Shared queries**: Arc prevents recompilation
- **Thread-safe**: All operations safe across threads
- **Background precompilation**: Async initialization

### Performance Characteristics
```
Operation           | Target Time  | Memory Impact
--------------------|-------------|---------------
Cache hit           | <1ms        | O(1)
Cache miss          | <10ms       | O(query_size)
Precompile all      | <2s         | O(languages × types)
Memory per query    | ~1-5KB      | Shared via Arc
```

## Error Handling

The library uses structured error types with thiserror:

```rust
#[derive(Debug, Error)]
pub enum QueryError {
    #[error("unsupported language: {language}")]
    UnsupportedLanguage { language: String },
    
    #[error("invalid query type: {query_type} for language {language}")]
    InvalidQueryType { query_type: String, language: String },
    
    #[error("query compilation failed: {details}")]
    CompilationFailed { details: String },
    
    // ... more variants
}
```

## Performance Monitoring

```rust
let stats = library.stats();
println!("Cache statistics:");
println!("  Size: {} queries", stats.cache_size);
println!("  Hit rate: {:.2}%", stats.hit_rate() * 100.0);
println!("  Total compilations: {}", stats.total_queries);
println!("  Supported languages: {}", stats.supported_languages);

// Clear cache if needed
library.clear_cache();

// Precompile for specific language
let compiled_count = library.precompile_language(Language::Rust)?;
println!("Precompiled {} queries for Rust", compiled_count);
```

## Best Practices

### 1. Initialize Once
Create the QueryLibrary once and share it:
```rust
// Good: Single instance with precompilation
static QUERY_LIB: Lazy<QueryLibrary> = Lazy::new(|| {
    let lib = QueryLibrary::new();
    lib.precompile_all().expect("Failed to precompile");
    lib
});
```

### 2. Use Structured Queries
Prefer the query library over manual tree-sitter queries:
```rust
// Good: Using query library
let functions = library.get_query(Language::Rust, QueryType::Functions)?;

// Avoid: Manual query compilation
let manual_query = Query::new(&language.parser(), "...")?;
```

### 3. Monitor Performance
Track cache performance in production:
```rust
let stats = library.stats();
if stats.hit_rate() < 0.8 {
    warn!("Low cache hit rate: {:.2}%", stats.hit_rate() * 100.0);
}
```

### 4. Handle Unsupported Languages
Always check support before querying:
```rust
if library.supports_query(language, &QueryType::Functions) {
    let query = library.get_query(language, QueryType::Functions)?;
} else {
    warn!("Language {:?} doesn't support function queries", language);
}
```

## Future Enhancements

- **Additional Languages**: OCaml, Haskell, Kotlin, Swift, Ruby
- **Query Optimization**: AST-specific optimizations
- **Incremental Compilation**: Smart recompilation on language updates
- **Query Analytics**: Detailed performance profiling
- **Custom Captures**: User-defined capture names and patterns

## Integration Points

The Query Library integrates with:
- **TreeSitterTool**: Primary search interface
- **ASTEngine**: Parsed AST processing
- **LanguageRegistry**: Language detection and management
- **MCP Tools**: External tool integration
- **Agent System**: Multi-agent code analysis