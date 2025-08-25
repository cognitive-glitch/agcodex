//! Benchmarks for session persistence with Zstd compression.
//! Tests save/load times, compression ratios, and checkpoint operations.

use agcodex_persistence::compression::CompressionLevel;
use agcodex_persistence::session_manager::SessionManager;
use agcodex_persistence::session_manager::SessionManagerConfig;
use agcodex_persistence::storage::SessionStorage;
use agcodex_persistence::storage::StorageBackend;
use agcodex_persistence::types::ConversationContext;
use agcodex_persistence::types::ConversationSnapshot;
use agcodex_persistence::types::MessageMetadata;
use agcodex_persistence::types::MessageSnapshot;
use agcodex_persistence::types::OperatingMode;
use agcodex_persistence::types::ResponseItem;
use agcodex_persistence::types::SessionMetadata;
use agcodex_persistence::types::SessionState;
use chrono::Utc;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::black_box;
use criterion::criterion_group;
use criterion::criterion_main;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Generate a mock message with varying content size
fn generate_mock_message(size: usize) -> MessageSnapshot {
    MessageSnapshot {
        item: ResponseItem::Message {
            id: Some("msg_".to_string() + &Uuid::new_v4().to_string()),
            role: "assistant".to_string(),
            content: vec![agcodex_persistence::types::ContentItem::OutputText {
                text: "x".repeat(size),
            }],
        },
        timestamp: Utc::now(),
        turn_index: 0,
        metadata: MessageMetadata {
            edited: false,
            edit_history: Vec::new(),
            branch_point: false,
            branch_id: None,
            parent_message_id: None,
            message_id: Uuid::new_v4(),
            file_context: Vec::new(),
            tool_calls: Vec::new(),
        },
    }
}

/// Generate a conversation with specified number of messages
fn generate_conversation(message_count: usize, message_size: usize) -> ConversationSnapshot {
    let mut messages = Vec::new();
    for i in 0..message_count {
        let mut msg = generate_mock_message(message_size);
        msg.turn_index = i;
        messages.push(msg);
    }

    ConversationSnapshot {
        id: Uuid::new_v4(),
        messages,
        context: ConversationContext {
            working_directory: PathBuf::from("/test/project"),
            environment_variables: std::collections::HashMap::from([
                ("USER".to_string(), "test_user".to_string()),
                ("HOME".to_string(), "/home/test".to_string()),
            ]),
            open_files: vec![PathBuf::from("src/main.rs"), PathBuf::from("src/lib.rs")],
            ast_index_state: None,
            embedding_cache: None,
        },
        mode_history: vec![(OperatingMode::Build, Utc::now())],
    }
}

/// Benchmark session save operations
fn bench_session_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_save");
    let rt = Runtime::new().unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Different session sizes
    let test_cases = vec![
        ("small", 10, 100),    // 10 messages, 100 bytes each
        ("medium", 50, 500),   // 50 messages, 500 bytes each
        ("large", 100, 1000),  // 100 messages, 1KB each
        ("xlarge", 500, 5000), // 500 messages, 5KB each
    ];

    for (name, msg_count, msg_size) in test_cases {
        let conversation = generate_conversation(msg_count, msg_size);
        let total_size = (msg_count * msg_size) as u64;
        group.throughput(Throughput::Bytes(total_size));

        // Benchmark with Light compression
        group.bench_function(BenchmarkId::new("light_compression", name), |b| {
            let rt = Runtime::new().unwrap();
            let temp_dir = TempDir::new().unwrap();
            b.iter(|| {
                rt.block_on(async {
                    let storage =
                        SessionStorage::new(temp_dir.path().to_path_buf(), CompressionLevel::Fast)
                            .unwrap();
                    let session_id = Uuid::new_v4();
                    let metadata = SessionMetadata {
                        id: session_id,
                        title: "Benchmark test".to_string(),
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                        last_accessed: Utc::now(),
                        message_count: conversation.messages.len(),
                        turn_count: conversation.messages.len(),
                        current_mode: OperatingMode::Build,
                        model: "test".to_string(),
                        tags: Vec::new(),
                        is_favorite: false,
                        file_size: 0,
                        compression_ratio: 1.0,
                        format_version: 1,
                        checkpoints: Vec::new(),
                    };
                    let state = SessionState::default();
                    storage
                        .save_session(session_id, &metadata, &conversation, &state)
                        .await
                        .unwrap();
                    black_box(session_id)
                })
            });
        });

        // Benchmark with Balanced compression
        group.bench_function(BenchmarkId::new("balanced_compression", name), |b| {
            let rt = Runtime::new().unwrap();
            let temp_dir = TempDir::new().unwrap();
            b.iter(|| {
                rt.block_on(async {
                    let storage = SessionStorage::new(
                        temp_dir.path().to_path_buf(),
                        CompressionLevel::Balanced,
                    )
                    .unwrap();
                    let session_id = Uuid::new_v4();
                    let metadata = SessionMetadata {
                        id: session_id,
                        title: "Benchmark test".to_string(),
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                        last_accessed: Utc::now(),
                        message_count: conversation.messages.len(),
                        turn_count: conversation.messages.len(),
                        current_mode: OperatingMode::Build,
                        model: "test".to_string(),
                        tags: Vec::new(),
                        is_favorite: false,
                        file_size: 0,
                        compression_ratio: 1.0,
                        format_version: 1,
                        checkpoints: Vec::new(),
                    };
                    let state = SessionState::default();
                    storage
                        .save_session(session_id, &metadata, &conversation, &state)
                        .await
                        .unwrap();
                    black_box(session_id)
                })
            });
        });

        // Benchmark with Aggressive compression
        group.bench_function(BenchmarkId::new("aggressive_compression", name), |b| {
            let rt = Runtime::new().unwrap();
            let temp_dir = TempDir::new().unwrap();
            b.iter(|| {
                rt.block_on(async {
                    let storage = SessionStorage::new(
                        temp_dir.path().to_path_buf(),
                        CompressionLevel::Maximum,
                    )
                    .unwrap();
                    let session_id = Uuid::new_v4();
                    let metadata = SessionMetadata {
                        id: session_id,
                        title: "Benchmark test".to_string(),
                        created_at: Utc::now(),
                        updated_at: Utc::now(),
                        last_accessed: Utc::now(),
                        message_count: conversation.messages.len(),
                        turn_count: conversation.messages.len(),
                        current_mode: OperatingMode::Build,
                        model: "test".to_string(),
                        tags: Vec::new(),
                        is_favorite: false,
                        file_size: 0,
                        compression_ratio: 1.0,
                        format_version: 1,
                        checkpoints: Vec::new(),
                    };
                    let state = SessionState::default();
                    storage
                        .save_session(session_id, &metadata, &conversation, &state)
                        .await
                        .unwrap();
                    black_box(session_id)
                })
            });
        });
    }
    group.finish();
}

/// Benchmark session load operations
fn bench_session_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_load");
    let rt = Runtime::new().unwrap();

    // Prepare test sessions
    let test_cases = vec![
        ("small", 10, 100),
        ("medium", 50, 500),
        ("large", 100, 1000),
    ];

    for (name, msg_count, msg_size) in test_cases {
        let temp_dir = TempDir::new().unwrap();
        let storage =
            SessionStorage::new(temp_dir.path().to_path_buf(), CompressionLevel::Balanced).unwrap();

        // Pre-save sessions
        let session_id = Uuid::new_v4();
        let conversation = generate_conversation(msg_count, msg_size);
        rt.block_on(async {
            let metadata = SessionMetadata {
                id: session_id,
                title: "Load benchmark test".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                last_accessed: Utc::now(),
                message_count: conversation.messages.len(),
                turn_count: conversation.messages.len(),
                current_mode: OperatingMode::Build,
                model: "test".to_string(),
                tags: Vec::new(),
                is_favorite: false,
                file_size: 0,
                compression_ratio: 1.0,
                format_version: 1,
                checkpoints: Vec::new(),
            };
            let state = SessionState::default();
            storage
                .save_session(session_id, &metadata, &conversation, &state)
                .await
                .unwrap();
        });

        group.bench_function(BenchmarkId::new("load_session", name), |b| {
            let rt = Runtime::new().unwrap();
            let temp_dir_path = temp_dir.path().to_path_buf();
            b.iter(|| {
                rt.block_on(async {
                    let storage =
                        SessionStorage::new(temp_dir_path.clone(), CompressionLevel::Balanced)
                            .unwrap();
                    let loaded = storage.load_session(session_id).await.unwrap();
                    black_box(loaded)
                })
            });
        });

        // Benchmark full session reload (for comparison)
        group.bench_function(BenchmarkId::new("reload_session", name), |b| {
            let rt = Runtime::new().unwrap();
            let temp_dir_path = temp_dir.path().to_path_buf();
            b.iter(|| {
                rt.block_on(async {
                    let storage =
                        SessionStorage::new(temp_dir_path.clone(), CompressionLevel::Balanced)
                            .unwrap();
                    let loaded = storage.load_session(session_id).await.unwrap();
                    black_box(loaded)
                })
            });
        });
    }
    group.finish();
}

/// Benchmark checkpoint operations
fn bench_checkpoints(c: &mut Criterion) {
    let mut group = c.benchmark_group("checkpoints");
    let rt = Runtime::new().unwrap();

    group.bench_function("create_checkpoint", |b| {
        let rt = Runtime::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let config = SessionManagerConfig {
                    storage_path: temp_dir.path().to_path_buf(),
                    auto_save_interval: Duration::from_secs(300),
                    max_sessions: 100,
                    max_total_size: 1_000_000_000,
                    compression_level: CompressionLevel::Balanced,
                    enable_auto_save: false,
                    enable_mmap: false,
                    max_checkpoints: 10,
                };

                let manager = SessionManager::new(config).await.unwrap();
                let session_id = manager
                    .create_session(
                        "Test Session".to_string(),
                        "gpt-4".to_string(),
                        OperatingMode::Build,
                    )
                    .await
                    .unwrap();

                // Add some messages
                for i in 0..10 {
                    let msg = generate_mock_message(100);
                    manager
                        .add_message(session_id, msg.item, Some(msg.metadata))
                        .await
                        .unwrap();
                }

                // Create checkpoint
                let checkpoint_id = manager
                    .create_checkpoint(
                        session_id,
                        "Test checkpoint".to_string(),
                        Some("Test checkpoint".to_string()),
                    )
                    .await
                    .unwrap();

                black_box(checkpoint_id)
            })
        });
    });

    group.bench_function("restore_checkpoint", |b| {
        let rt = Runtime::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        // Setup: Create session with checkpoint
        let (session_id, checkpoint_id) = rt.block_on(async {
            let config = SessionManagerConfig {
                storage_path: temp_dir.path().to_path_buf(),
                auto_save_interval: Duration::from_secs(300),
                max_sessions: 100,
                max_total_size: 1_000_000_000,
                compression_level: CompressionLevel::Balanced,
                enable_auto_save: false,
                enable_mmap: false,
                max_checkpoints: 10,
            };

            let manager = SessionManager::new(config).await.unwrap();
            let session_id = manager
                .create_session(
                    "Test Session".to_string(),
                    "gpt-4".to_string(),
                    OperatingMode::Build,
                )
                .await
                .unwrap();

            for i in 0..20 {
                let msg = generate_mock_message(200);
                manager
                    .add_message(session_id, msg.item, Some(msg.metadata))
                    .await
                    .unwrap();
            }

            let checkpoint_id = manager
                .create_checkpoint(
                    session_id,
                    "Restore test".to_string(),
                    Some("Restore test".to_string()),
                )
                .await
                .unwrap();

            // Add more messages after checkpoint
            for i in 0..10 {
                let msg = generate_mock_message(200);
                manager
                    .add_message(session_id, msg.item, Some(msg.metadata))
                    .await
                    .unwrap();
            }

            (session_id, checkpoint_id)
        });

        b.iter(|| {
            rt.block_on(async {
                let config = SessionManagerConfig {
                    storage_path: temp_dir.path().to_path_buf(),
                    auto_save_interval: Duration::from_secs(300),
                    max_sessions: 100,
                    max_total_size: 1_000_000_000,
                    compression_level: CompressionLevel::Balanced,
                    enable_auto_save: false,
                    enable_mmap: false,
                    max_checkpoints: 10,
                };

                let manager = SessionManager::new(config).await.unwrap();
                manager
                    .restore_checkpoint(session_id, checkpoint_id)
                    .await
                    .unwrap();
                black_box(checkpoint_id)
            })
        });
    });

    group.finish();
}

/// Benchmark session manager operations
fn bench_session_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_manager");
    let rt = Runtime::new().unwrap();

    group.bench_function("list_sessions", |b| {
        let rt = Runtime::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        // Setup: Create multiple sessions
        rt.block_on(async {
            let config = SessionManagerConfig {
                storage_path: temp_dir.path().to_path_buf(),
                auto_save_interval: Duration::from_secs(300),
                max_sessions: 100,
                max_total_size: 1_000_000_000,
                compression_level: CompressionLevel::Balanced,
                enable_auto_save: false,
                enable_mmap: false,
                max_checkpoints: 10,
            };

            let manager = SessionManager::new(config).await.unwrap();

            for i in 0..20 {
                let session_id = manager
                    .create_session(
                        format!("Session {}", i),
                        "gpt-4".to_string(),
                        OperatingMode::Build,
                    )
                    .await
                    .unwrap();
                for j in 0..5 {
                    let msg = generate_mock_message(100);
                    manager
                        .add_message(session_id, msg.item, Some(msg.metadata))
                        .await
                        .unwrap();
                }
            }
        });

        b.iter(|| {
            rt.block_on(async {
                let config = SessionManagerConfig {
                    storage_path: temp_dir.path().to_path_buf(),
                    auto_save_interval: Duration::from_secs(300),
                    max_sessions: 100,
                    max_total_size: 1_000_000_000,
                    compression_level: CompressionLevel::Balanced,
                    enable_auto_save: false,
                    enable_mmap: false,
                    max_checkpoints: 10,
                };

                let manager = SessionManager::new(config).await.unwrap();
                let sessions = manager.list_sessions().await.unwrap();
                black_box(sessions)
            })
        });
    });

    group.bench_function("search_sessions", |b| {
        let rt = Runtime::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        // Setup: Create sessions with searchable content
        rt.block_on(async {
            let config = SessionManagerConfig {
                storage_path: temp_dir.path().to_path_buf(),
                auto_save_interval: Duration::from_secs(300),
                max_sessions: 100,
                max_total_size: 1_000_000_000,
                compression_level: CompressionLevel::Balanced,
                enable_auto_save: false,
                enable_mmap: false,
                max_checkpoints: 10,
            };

            let manager = SessionManager::new(config).await.unwrap();

            for i in 0..20 {
                let session_id = manager
                    .create_session(
                        format!("Project {}", i),
                        "gpt-4".to_string(),
                        OperatingMode::Build,
                    )
                    .await
                    .unwrap();
                // Create message with custom content
                let custom_msg = ResponseItem::Message {
                    id: Some(format!("msg_{}", Uuid::new_v4())),
                    role: "assistant".to_string(),
                    content: vec![agcodex_persistence::types::ContentItem::OutputText {
                        text: format!("Working on feature {} implementation", i),
                    }],
                };
                manager
                    .add_message(session_id, custom_msg, None)
                    .await
                    .unwrap();
            }
        });

        b.iter(|| {
            rt.block_on(async {
                let config = SessionManagerConfig {
                    storage_path: temp_dir.path().to_path_buf(),
                    auto_save_interval: Duration::from_secs(300),
                    max_sessions: 100,
                    max_total_size: 1_000_000_000,
                    compression_level: CompressionLevel::Balanced,
                    enable_auto_save: false,
                    enable_mmap: false,
                    max_checkpoints: 10,
                };

                let manager = SessionManager::new(config).await.unwrap();
                let results = manager.search_sessions("feature").await;
                black_box(results)
            })
        });
    });

    group.finish();
}

/// Benchmark incremental saves (append operations)
fn bench_incremental_saves(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_saves");
    let rt = Runtime::new().unwrap();

    group.bench_function("append_message", |b| {
        let rt = Runtime::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        b.iter(|| {
            rt.block_on(async {
                let config = SessionManagerConfig {
                    storage_path: temp_dir.path().to_path_buf(),
                    auto_save_interval: Duration::from_secs(300),
                    max_sessions: 100,
                    max_total_size: 1_000_000_000,
                    compression_level: CompressionLevel::Balanced,
                    enable_auto_save: true, // Enable auto-save
                    enable_mmap: false,
                    max_checkpoints: 10,
                };

                let manager = SessionManager::new(config).await.unwrap();
                let session_id = manager
                    .create_session(
                        "Incremental Test".to_string(),
                        "gpt-4".to_string(),
                        OperatingMode::Build,
                    )
                    .await
                    .unwrap();

                // Add messages incrementally
                for i in 0..10 {
                    let msg = generate_mock_message(200);
                    manager
                        .add_message(session_id, msg.item, Some(msg.metadata))
                        .await
                        .unwrap();
                }

                black_box(session_id)
            })
        });
    });

    group.bench_function("batch_append", |b| {
        let rt = Runtime::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        b.iter(|| {
            rt.block_on(async {
                let config = SessionManagerConfig {
                    storage_path: temp_dir.path().to_path_buf(),
                    auto_save_interval: Duration::from_secs(300),
                    max_sessions: 100,
                    max_total_size: 1_000_000_000,
                    compression_level: CompressionLevel::Balanced,
                    enable_auto_save: false,
                    enable_mmap: false,
                    max_checkpoints: 10,
                };

                let manager = SessionManager::new(config).await.unwrap();
                let session_id = manager
                    .create_session(
                        "Batch Test".to_string(),
                        "gpt-4".to_string(),
                        OperatingMode::Build,
                    )
                    .await
                    .unwrap();

                // Batch add messages (simulate with individual calls)
                let messages: Vec<_> = (0..10).map(|_| generate_mock_message(200)).collect();

                for msg in messages {
                    manager
                        .add_message(session_id, msg.item, Some(msg.metadata))
                        .await
                        .unwrap();
                }

                black_box(session_id)
            })
        });
    });

    group.finish();
}

/// Benchmark worst-case scenarios
fn bench_worst_case_persistence(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case_persistence");
    let rt = Runtime::new().unwrap();

    // Very large single message
    group.bench_function("huge_message", |b| {
        let rt = Runtime::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let huge_message = generate_mock_message(1_000_000); // 1MB message

        b.iter(|| {
            rt.block_on(async {
                let storage =
                    SessionStorage::new(temp_dir.path().to_path_buf(), CompressionLevel::Maximum)
                        .unwrap();
                let session_id = Uuid::new_v4();
                let conversation = ConversationSnapshot {
                    id: session_id,
                    messages: vec![huge_message.clone()],
                    context: ConversationContext {
                        working_directory: PathBuf::from("/test/project"),
                        environment_variables: std::collections::HashMap::new(),
                        open_files: Vec::new(),
                        ast_index_state: None,
                        embedding_cache: None,
                    },
                    mode_history: vec![(OperatingMode::Build, Utc::now())],
                };
                let metadata = SessionMetadata {
                    id: session_id,
                    title: "Huge message test".to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    last_accessed: Utc::now(),
                    message_count: 1,
                    turn_count: 1,
                    current_mode: OperatingMode::Build,
                    model: "test".to_string(),
                    tags: Vec::new(),
                    is_favorite: false,
                    file_size: 0,
                    compression_ratio: 1.0,
                    format_version: 1,
                    checkpoints: Vec::new(),
                };
                let state = SessionState::default();
                storage
                    .save_session(session_id, &metadata, &conversation, &state)
                    .await
                    .unwrap();
                black_box(session_id)
            })
        });
    });

    // Many small messages
    group.bench_function("many_tiny_messages", |b| {
        let rt = Runtime::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let conversation = generate_conversation(1000, 10); // 1000 messages, 10 bytes each

        b.iter(|| {
            rt.block_on(async {
                let storage =
                    SessionStorage::new(temp_dir.path().to_path_buf(), CompressionLevel::Balanced)
                        .unwrap();
                let session_id = Uuid::new_v4();
                let metadata = SessionMetadata {
                    id: session_id,
                    title: "Many tiny messages test".to_string(),
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                    last_accessed: Utc::now(),
                    message_count: conversation.messages.len(),
                    turn_count: conversation.messages.len(),
                    current_mode: OperatingMode::Build,
                    model: "test".to_string(),
                    tags: Vec::new(),
                    is_favorite: false,
                    file_size: 0,
                    compression_ratio: 1.0,
                    format_version: 1,
                    checkpoints: Vec::new(),
                };
                let state = SessionState::default();
                storage
                    .save_session(session_id, &metadata, &conversation, &state)
                    .await
                    .unwrap();
                black_box(session_id)
            })
        });
    });

    // Concurrent access
    group.bench_function("concurrent_access", |b| {
        let rt = Runtime::new().unwrap();
        let temp_dir = TempDir::new().unwrap();

        b.iter(|| {
            rt.block_on(async {
                let config = SessionManagerConfig {
                    storage_path: temp_dir.path().to_path_buf(),
                    auto_save_interval: Duration::from_secs(300),
                    max_sessions: 100,
                    max_total_size: 1_000_000_000,
                    compression_level: CompressionLevel::Balanced,
                    enable_auto_save: false,
                    enable_mmap: false,
                    max_checkpoints: 10,
                };

                let manager = Arc::new(SessionManager::new(config).await.unwrap());
                let mut handles = Vec::new();

                // Spawn multiple concurrent operations
                for i in 0..5 {
                    let mgr = Arc::clone(&manager);
                    let handle = tokio::spawn(async move {
                        let session_id = mgr
                            .create_session(
                                format!("Concurrent {}", i),
                                "gpt-4".to_string(),
                                OperatingMode::Build,
                            )
                            .await
                            .unwrap();
                        for j in 0..10 {
                            let msg = generate_mock_message(100);
                            mgr.add_message(session_id, msg.item, Some(msg.metadata))
                                .await
                                .unwrap();
                        }
                        session_id
                    });
                    handles.push(handle);
                }

                let results = futures::future::join_all(handles).await;
                black_box(results)
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_session_save,
    bench_session_load,
    bench_checkpoints,
    bench_session_manager,
    bench_incremental_saves,
    bench_worst_case_persistence
);
criterion_main!(benches);
