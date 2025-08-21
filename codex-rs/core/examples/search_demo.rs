//! Multi-Layer Search Engine Demo
//!
//! This example demonstrates the complete usage of AGCodex's sophisticated
//! multi-layer search engine with Tantivy integration.

use agcodex_core::code_tools::search::Location;
use agcodex_core::code_tools::search::MultiLayerSearchEngine;
use agcodex_core::code_tools::search::QueryType;
use agcodex_core::code_tools::search::Scope;
use agcodex_core::code_tools::search::SearchConfig;
use agcodex_core::code_tools::search::SearchQuery;
use agcodex_core::code_tools::search::SearchScope;
use agcodex_core::code_tools::search::Symbol;
use agcodex_core::code_tools::search::SymbolKind;
use agcodex_core::code_tools::search::Visibility;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 AGCodex Multi-Layer Search Engine Demo");
    println!("==========================================\n");

    // 1. Create search engine with custom configuration
    let config = SearchConfig {
        max_cache_size: 500,
        cache_ttl: Duration::from_secs(600), // 10 minutes
        enable_symbol_index: true,
        enable_tantivy: true,
        enable_ast_cache: true,
        enable_ripgrep_fallback: true,
        max_results: 50,
        timeout: Duration::from_secs(30),
    };

    let search_engine = MultiLayerSearchEngine::new(config)?;
    println!("✅ Multi-layer search engine initialized");

    // 2. Add some test symbols to Layer 1 (Symbol Index)
    add_sample_symbols(&search_engine).await;
    println!("✅ Sample symbols added to Layer 1 index");

    // 3. Demonstrate different search strategies
    demonstrate_search_strategies(&search_engine).await;

    // 4. Show rich context output
    demonstrate_rich_context(&search_engine).await;

    // 5. Performance demonstration
    demonstrate_performance(&search_engine).await;

    println!("\n🎉 Search engine demo completed successfully!");
    Ok(())
}

async fn add_sample_symbols(engine: &MultiLayerSearchEngine) {
    let symbols = vec![
        Symbol {
            name: "parse_rust_file".to_string(),
            kind: SymbolKind::Function,
            location: Location {
                file: PathBuf::from("src/parser.rs"),
                line: 45,
                column: 8,
                byte_offset: 1240,
            },
            scope: Scope {
                function: None,
                class: None,
                module: Some("parser".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        },
        Symbol {
            name: "TokenStream".to_string(),
            kind: SymbolKind::Struct,
            location: Location {
                file: PathBuf::from("src/tokens.rs"),
                line: 15,
                column: 12,
                byte_offset: 380,
            },
            scope: Scope {
                function: None,
                class: None,
                module: Some("tokens".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        },
        Symbol {
            name: "SearchEngine".to_string(),
            kind: SymbolKind::Struct,
            location: Location {
                file: PathBuf::from("src/search.rs"),
                line: 25,
                column: 12,
                byte_offset: 650,
            },
            scope: Scope {
                function: None,
                class: None,
                module: Some("search".to_string()),
                namespace: None,
            },
            visibility: Visibility::Public,
        },
    ];

    for symbol in symbols {
        engine.add_symbol(symbol);
    }
}

async fn demonstrate_search_strategies(engine: &MultiLayerSearchEngine) {
    println!("\n📍 Demonstrating Search Strategies");
    println!("----------------------------------");

    // Strategy 1: Symbol lookup (Layer 1 - <1ms)
    println!("1. Symbol Search (Layer 1 - In-Memory):");
    let symbol_query = SearchQuery::symbol("parse_rust_file").with_context_lines(3);

    match engine.search(symbol_query).await {
        Ok(result) => {
            println!(
                "   ✅ Found {} matches in {:?}",
                result.result.len(),
                result.metadata.duration
            );
            println!(
                "   📊 Strategy: {:?} | Layer: {:?}",
                result.metadata.strategy, result.metadata.search_layer
            );
            for match_result in &result.result {
                println!(
                    "   🎯 {}:{}:{} - {}",
                    match_result.file.display(),
                    match_result.line,
                    match_result.column,
                    match_result.content
                );
            }
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }

    // Strategy 2: Definition search (Layer 3 - AST Cache)
    println!("\n2. Definition Search (Layer 3 - AST):");
    let def_query =
        SearchQuery::definition("TokenStream").with_language_filters(vec!["rust".to_string()]);

    match engine.search(def_query).await {
        Ok(result) => {
            println!(
                "   ✅ Found {} definitions in {:?}",
                result.result.len(),
                result.metadata.duration
            );
            println!(
                "   📊 Strategy: {:?} | Layer: {:?}",
                result.metadata.strategy, result.metadata.search_layer
            );
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }

    // Strategy 3: Reference search
    println!("\n3. Reference Search:");
    match engine.find_references("SearchEngine").await {
        Ok(result) => {
            println!(
                "   ✅ Found {} references in {:?}",
                result.result.len(),
                result.metadata.duration
            );
            println!("   📄 Summary: {}", result.summary);
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }

    // Strategy 4: Full-text search with fuzzy matching
    println!("\n4. Fuzzy Full-Text Search:");
    let fuzzy_query = SearchQuery::full_text("parse_rust")
        .fuzzy()
        .case_insensitive()
        .with_limit(10);

    match engine.search(fuzzy_query).await {
        Ok(result) => {
            println!(
                "   ✅ Found {} fuzzy matches in {:?}",
                result.result.len(),
                result.metadata.duration
            );
            for (i, match_result) in result.result.iter().take(3).enumerate() {
                println!(
                    "   {}. Score: {:.2} | {}:{}",
                    i + 1,
                    match_result.score,
                    match_result.file.display(),
                    match_result.line
                );
            }
        }
        Err(e) => println!("   ❌ Error: {}", e),
    }
}

async fn demonstrate_rich_context(engine: &MultiLayerSearchEngine) {
    println!("\n📊 Rich Context Output Example");
    println!("------------------------------");

    let query = SearchQuery::new("SearchEngine")
        .with_context_lines(5)
        .in_directory("src/");

    match engine.search(query).await {
        Ok(result) => {
            println!("🔍 Search Result Analysis:");
            println!("  📈 Total Results: {}", result.metadata.total_results);
            println!("  ⏱️  Execution Time: {:?}", result.metadata.duration);
            println!("  🎯 Search Layer: {:?}", result.metadata.search_layer);
            println!("  🧠 Strategy: {:?}", result.metadata.strategy);

            if let Some(lang) = &result.metadata.language {
                println!("  🔤 Language: {}", lang);
            }

            println!("  📝 Summary: {}", result.summary);

            // Context information
            println!("\n🎯 Context Details:");
            println!(
                "  📂 Location: {}:{}:{}",
                result.context.location.file.display(),
                result.context.location.line,
                result.context.location.column
            );

            if let Some(func) = &result.context.scope.function {
                println!("  🔧 Function: {}", func);
            }
            if let Some(class) = &result.context.scope.class {
                println!("  🏛️  Class: {}", class);
            }
            if let Some(module) = &result.context.scope.module {
                println!("  📦 Module: {}", module);
            }
        }
        Err(e) => println!("❌ Error getting rich context: {}", e),
    }
}

async fn demonstrate_performance(engine: &MultiLayerSearchEngine) {
    println!("\n⚡ Performance Demonstration");
    println!("----------------------------");

    let queries = vec![
        ("Symbol lookup", SearchQuery::symbol("parse_rust_file")),
        ("Fuzzy search", SearchQuery::new("search").fuzzy()),
        ("Definition search", SearchQuery::definition("TokenStream")),
        (
            "Multi-scope search",
            SearchQuery::new("engine").in_files(vec![
                PathBuf::from("src/search.rs"),
                PathBuf::from("src/parser.rs"),
            ]),
        ),
    ];

    for (name, query) in queries {
        let start = std::time::Instant::now();

        match engine.search(query).await {
            Ok(result) => {
                let total_time = start.elapsed();
                println!(
                    "  {} - {} results in {:?} (Total: {:?})",
                    name,
                    result.result.len(),
                    result.metadata.duration,
                    total_time
                );

                // Show layer performance
                match result.metadata.search_layer {
                    agcodex_core::code_tools::search::SearchLayer::SymbolIndex => {
                        println!("    💨 Layer 1 (Symbol Index) - Ultra Fast")
                    }
                    agcodex_core::code_tools::search::SearchLayer::Tantivy => {
                        println!("    🔍 Layer 2 (Tantivy) - Fast Full-Text")
                    }
                    agcodex_core::code_tools::search::SearchLayer::AstCache => {
                        println!("    🧠 Layer 3 (AST Cache) - Semantic Analysis")
                    }
                    agcodex_core::code_tools::search::SearchLayer::RipgrepFallback => {
                        println!("    🔧 Layer 4 (Ripgrep) - Fallback Pattern")
                    }
                    agcodex_core::code_tools::search::SearchLayer::Combined => {
                        println!("    🎯 Combined Layers - Hybrid Approach")
                    }
                }
            }
            Err(e) => println!("  {} - ❌ Error: {}", name, e),
        }
    }
}
