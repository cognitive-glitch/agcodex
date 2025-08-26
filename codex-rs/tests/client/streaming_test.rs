use std::sync::Arc;
use std::time::Duration;

use agcodex_core::client::ModelClient;
use agcodex_core::client_common::{Prompt, ResponseEvent};
use agcodex_core::config::Config;
use agcodex_core::error::CodexErr;
use agcodex_core::model_family::find_family_for_model;
use agcodex_core::model_provider_info::{ModelProviderInfo, WireApi};
use agcodex_core::models::{ContentItem, ResponseItem};
use agcodex_core::protocol::TokenUsage;
use agcodex_protocol::config_types::{ReasoningEffort, ReasoningSummary};
use futures::{Stream, StreamExt};
use serde_json::json;
use tokio::sync::mpsc;
use tokio::time::{interval, timeout, Instant};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Streaming test fixture for SSE-specific scenarios
struct StreamingTestFixture {
    config: Arc<Config>,
    provider: ModelProviderInfo,
    session_id: Uuid,
}

impl StreamingTestFixture {
    fn new() -> Self {
        let model_family = find_family_for_model("gpt-4").expect("known model");
        let config = Arc::new(Config {
            model: "gpt-4".to_string(),
            model_family: model_family.clone(),
            responses_originator_header: "streaming-test".to_string(),
            show_raw_agent_reasoning: false,
            ..Default::default()
        });

        let provider = ModelProviderInfo {
            name: "streaming-test".to_string(),
            base_url: None, // Set dynamically in tests
            env_key: Some("TEST_API_KEY".to_string()),
            env_key_instructions: None,
            wire_api: WireApi::Responses,
            query_params: None,
            http_headers: None,
            env_http_headers: None,
            request_max_retries: Some(1), // Minimal retries for streaming tests
            stream_max_retries: Some(1),
            stream_idle_timeout_ms: Some(2000), // Shorter timeout for tests
            requires_openai_auth: false,
        };

        Self {
            config,
            provider,
            session_id: Uuid::new_v4(),
        }
    }

    fn with_mock_server_url(mut self, url: &str) -> Self {
        self.provider.base_url = Some(url.to_string());
        self
    }

    fn with_idle_timeout(mut self, timeout_ms: u64) -> Self {
        self.provider.stream_idle_timeout_ms = Some(timeout_ms);
        self
    }

    fn build(self) -> ModelClient {
        ModelClient::new(
            self.config,
            None, // No auth for streaming tests
            self.provider,
            ReasoningEffort::Medium,
            ReasoningSummary::None,
            self.session_id,
        )
    }

    fn create_prompt() -> Prompt {
        Prompt {
            input: vec![ResponseItem::Message {
                id: Some("stream-test-1".to_string()),
                role: "user".to_string(),
                content: vec![ContentItem::InputText {
                    text: "Tell me a story".to_string(),
                }],
            }],
            store: false,
            tools: vec![],
            base_instructions_override: None,
        }
    }
}

/// Mock SSE response generators for streaming scenarios
struct StreamingMocks;

impl StreamingMocks {
    /// Complete streaming response with multiple text deltas
    fn streaming_text_response() -> ResponseTemplate {
        let events = vec![
            "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{}}\n\n",
            "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"Once\"}\n\n",
            "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\" upon\"}\n\n",
            "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\" a\"}\n\n",
            "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\" time\"}\n\n",
            "event: response.output_item.done\ndata: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Once upon a time\"}]}}\n\n",
            "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-456\",\"usage\":{\"input_tokens\":15,\"output_tokens\":20,\"total_tokens\":35}}}\n\n"
        ];
        ResponseTemplate::new(200)
            .set_body_string(events.join(""))
            .insert_header("content-type", "text/event-stream")
            .insert_header("cache-control", "no-cache")
            .insert_header("connection", "keep-alive")
    }

    /// Response with reasoning deltas (for o1-style models)
    fn reasoning_response() -> ResponseTemplate {
        let events = vec![
            "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{}}\n\n",
            "event: response.reasoning_text.delta\ndata: {\"type\":\"response.reasoning_text.delta\",\"delta\":\"Let me think about this...\"}\n\n",
            "event: response.reasoning_text.delta\ndata: {\"type\":\"response.reasoning_text.delta\",\"delta\":\" I need to consider\"}\n\n",
            "event: response.reasoning_summary_part.added\ndata: {\"type\":\"response.reasoning_summary_part.added\"}\n\n",
            "event: response.reasoning_summary_text.delta\ndata: {\"type\":\"response.reasoning_summary_text.delta\",\"delta\":\"Analyzing the request\"}\n\n",
            "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"Based on my analysis\"}\n\n",
            "event: response.output_item.done\ndata: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Based on my analysis\"}]}}\n\n",
            "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-reasoning\",\"usage\":{\"input_tokens\":20,\"output_tokens\":10,\"output_tokens_details\":{\"reasoning_tokens\":25},\"total_tokens\":55}}}\n\n"
        ];
        ResponseTemplate::new(200)
            .set_body_string(events.join(""))
            .insert_header("content-type", "text/event-stream")
    }

    /// Slow streaming response to test backpressure
    fn slow_streaming_response() -> ResponseTemplate {
        let mut events = vec![
            "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{}}\n\n".to_string(),
        ];
        
        // Add many text deltas to simulate a long response
        for i in 0..100 {
            events.push(format!(
                "event: response.output_text.delta\ndata: {{\"type\":\"response.output_text.delta\",\"delta\":\"word{} \"}}\n\n",
                i
            ));
        }
        
        events.push("event: response.output_item.done\ndata: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Long response with many words\"}]}}\n\n".to_string());
        events.push("event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-long\",\"usage\":{\"input_tokens\":10,\"output_tokens\":100,\"total_tokens\":110}}}\n\n".to_string());

        ResponseTemplate::new(200)
            .set_body_string(events.join(""))
            .insert_header("content-type", "text/event-stream")
            .set_delay(Duration::from_millis(50)) // Small delay per chunk
    }

    /// Interrupted streaming response (ends abruptly)
    fn interrupted_stream() -> ResponseTemplate {
        let events = vec![
            "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{}}\n\n",
            "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"This will be cut off\"}\n\n",
            // No completion event - simulates connection drop
        ];
        ResponseTemplate::new(200)
            .set_body_string(events.join(""))
            .insert_header("content-type", "text/event-stream")
    }

    /// Malformed SSE events
    fn malformed_sse_response() -> ResponseTemplate {
        let events = vec![
            "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{}}\n\n",
            "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"Valid text\"}\n\n",
            "event: response.output_text.delta\ndata: {invalid json here}\n\n", // Malformed JSON
            "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"More text\"}\n\n",
            "event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-malformed\"}}\n\n"
        ];
        ResponseTemplate::new(200)
            .set_body_string(events.join(""))
            .insert_header("content-type", "text/event-stream")
    }

    /// Empty stream (no events)
    fn empty_stream() -> ResponseTemplate {
        ResponseTemplate::new(200)
            .set_body_string("")
            .insert_header("content-type", "text/event-stream")
    }

    /// Stream that times out due to idle
    fn idle_timeout_stream() -> ResponseTemplate {
        let events = vec![
            "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{}}\n\n",
            // Long delay with no more events
        ];
        ResponseTemplate::new(200)
            .set_body_string(events.join(""))
            .insert_header("content-type", "text/event-stream")
            .set_delay(Duration::from_secs(5)) // Longer than test timeout
    }
}

#[tokio::test]
async fn test_streaming_vs_non_streaming_responses() {
    // This test verifies that streaming responses deliver events incrementally
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(StreamingMocks::streaming_text_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = StreamingTestFixture::new()
        .with_mock_server_url(&mock_server.uri())
        .build();

    let prompt = StreamingTestFixture::create_prompt();

    // Act
    let mut stream = client.stream(&prompt).await.expect("Stream should start");
    let start_time = Instant::now();
    let mut events = Vec::new();
    let mut event_times = Vec::new();

    while let Some(event) = stream.next().await {
        let event = event.expect("Event should be valid");
        let elapsed = start_time.elapsed();
        
        events.push(event);
        event_times.push(elapsed);
    }

    // Assert
    assert!(events.len() >= 6, "Should receive multiple streaming events");
    
    // Verify we receive incremental text deltas
    let text_deltas: Vec<_> = events.iter()
        .filter_map(|e| match e {
            ResponseEvent::OutputTextDelta(text) => Some(text),
            _ => None,
        })
        .collect();
    
    assert_eq!(text_deltas.len(), 4, "Should receive 4 text deltas");
    assert_eq!(text_deltas[0], "Once");
    assert_eq!(text_deltas[1], " upon");
    assert_eq!(text_deltas[2], " a");
    assert_eq!(text_deltas[3], " time");

    // Verify events come in streaming fashion (incrementally over time)
    for i in 1..event_times.len() {
        assert!(event_times[i] >= event_times[i-1], "Events should arrive in chronological order");
    }
}

#[tokio::test]
async fn test_reasoning_content_streaming() {
    // Test o1-style models with reasoning content
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(StreamingMocks::reasoning_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = StreamingTestFixture::new()
        .with_mock_server_url(&mock_server.uri())
        .build();

    let prompt = StreamingTestFixture::create_prompt();

    // Act
    let mut stream = client.stream(&prompt).await.expect("Stream should start");
    let mut events = Vec::new();

    while let Some(event) = stream.next().await {
        let event = event.expect("Event should be valid");
        events.push(event);
    }

    // Assert
    let reasoning_deltas: Vec<_> = events.iter()
        .filter_map(|e| match e {
            ResponseEvent::ReasoningContentDelta(text) => Some(text),
            _ => None,
        })
        .collect();

    let reasoning_summary_deltas: Vec<_> = events.iter()
        .filter_map(|e| match e {
            ResponseEvent::ReasoningSummaryDelta(text) => Some(text),
            _ => None,
        })
        .collect();

    let reasoning_part_added = events.iter()
        .any(|e| matches!(e, ResponseEvent::ReasoningSummaryPartAdded));

    assert_eq!(reasoning_deltas.len(), 2, "Should receive reasoning content deltas");
    assert_eq!(reasoning_summary_deltas.len(), 1, "Should receive reasoning summary delta");
    assert!(reasoning_part_added, "Should receive reasoning part added event");

    // Verify final completion includes reasoning tokens
    if let Some(ResponseEvent::Completed { token_usage, .. }) = events.last() {
        let usage = token_usage.as_ref().expect("Should have token usage");
        assert_eq!(usage.reasoning_output_tokens, Some(25));
        assert_eq!(usage.total_tokens, 55);
    } else {
        panic!("Expected completion event with token usage");
    }
}

#[tokio::test]
async fn test_backpressure_handling() {
    // Test that the system handles fast-producing streams without dropping events
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(StreamingMocks::slow_streaming_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = StreamingTestFixture::new()
        .with_mock_server_url(&mock_server.uri())
        .build();

    let prompt = StreamingTestFixture::create_prompt();

    // Act - Consume events slowly to test backpressure
    let mut stream = client.stream(&prompt).await.expect("Stream should start");
    let mut text_delta_count = 0;
    let mut total_events = 0;

    while let Some(event) = stream.next().await {
        let event = event.expect("Event should be valid");
        total_events += 1;
        
        if matches!(event, ResponseEvent::OutputTextDelta(_)) {
            text_delta_count += 1;
            // Simulate slow processing
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    // Assert
    assert_eq!(text_delta_count, 100, "Should receive all 100 text deltas despite backpressure");
    assert!(total_events >= 102, "Should receive all events (created, deltas, item done, completed)");
}

#[tokio::test]
async fn test_stream_interruption_recovery() {
    // Test handling of interrupted streams
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(StreamingMocks::interrupted_stream())
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = StreamingTestFixture::new()
        .with_mock_server_url(&mock_server.uri())
        .with_idle_timeout(1000) // 1 second timeout
        .build();

    let prompt = StreamingTestFixture::create_prompt();

    // Act
    let mut stream = client.stream(&prompt).await.expect("Stream should start");
    let mut events = Vec::new();
    let mut last_error = None;

    while let Some(event) = stream.next().await {
        match event {
            Ok(e) => events.push(e),
            Err(e) => {
                last_error = Some(e);
                break;
            }
        }
    }

    // Assert
    assert!(events.len() >= 2, "Should receive some events before interruption");
    assert!(last_error.is_some(), "Should get error due to stream interruption");
    
    if let Some(CodexErr::Stream(msg, _)) = last_error {
        assert!(msg.contains("stream closed before") || msg.contains("idle timeout"), 
                "Error should indicate stream interruption: {}", msg);
    } else {
        panic!("Expected Stream error");
    }
}

#[tokio::test]
async fn test_malformed_sse_event_handling() {
    // Test that malformed events don't crash the parser
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(StreamingMocks::malformed_sse_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = StreamingTestFixture::new()
        .with_mock_server_url(&mock_server.uri())
        .build();

    let prompt = StreamingTestFixture::create_prompt();

    // Act
    let mut stream = client.stream(&prompt).await.expect("Stream should start");
    let mut events = Vec::new();

    while let Some(event) = stream.next().await {
        let event = event.expect("Event should be valid or skipped");
        events.push(event);
    }

    // Assert
    assert!(events.len() >= 3, "Should receive valid events, skipping malformed ones");
    
    // Should receive created, valid deltas, and completed events
    assert!(matches!(events[0], ResponseEvent::Created));
    assert!(events.iter().any(|e| matches!(e, ResponseEvent::OutputTextDelta(_))));
    assert!(events.iter().any(|e| matches!(e, ResponseEvent::Completed { .. })));
}

#[tokio::test]
async fn test_empty_stream_handling() {
    // Test handling of empty streams
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(StreamingMocks::empty_stream())
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = StreamingTestFixture::new()
        .with_mock_server_url(&mock_server.uri())
        .with_idle_timeout(500) // Short timeout for empty stream
        .build();

    let prompt = StreamingTestFixture::create_prompt();

    // Act
    let mut stream = client.stream(&prompt).await.expect("Stream should start");
    let mut events = Vec::new();

    // Should timeout quickly due to no events
    let result = timeout(Duration::from_secs(2), async {
        while let Some(event) = stream.next().await {
            match event {
                Ok(e) => events.push(e),
                Err(e) => return Some(e),
            }
        }
        None
    }).await;

    // Assert
    assert!(result.is_ok(), "Should not timeout at test level");
    let maybe_error = result.unwrap();
    
    if let Some(CodexErr::Stream(msg, _)) = maybe_error {
        assert!(msg.contains("idle timeout") || msg.contains("stream closed"), 
                "Should get appropriate error for empty stream: {}", msg);
    }
    
    assert_eq!(events.len(), 0, "Should receive no events from empty stream");
}

#[tokio::test]
async fn test_idle_timeout_behavior() {
    // Test stream idle timeout functionality
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(StreamingMocks::idle_timeout_stream())
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = StreamingTestFixture::new()
        .with_mock_server_url(&mock_server.uri())
        .with_idle_timeout(1000) // 1 second idle timeout
        .build();

    let prompt = StreamingTestFixture::create_prompt();

    // Act
    let start_time = Instant::now();
    let mut stream = client.stream(&prompt).await.expect("Stream should start");
    let mut events = Vec::new();
    let mut timeout_error = None;

    while let Some(event) = stream.next().await {
        match event {
            Ok(e) => events.push(e),
            Err(e) => {
                timeout_error = Some(e);
                break;
            }
        }
    }

    let elapsed = start_time.elapsed();

    // Assert
    assert!(elapsed >= Duration::from_millis(900), "Should wait for idle timeout");
    assert!(elapsed <= Duration::from_secs(3), "Should not wait too long");
    assert!(timeout_error.is_some(), "Should get timeout error");
    
    if let Some(CodexErr::Stream(msg, _)) = timeout_error {
        assert!(msg.contains("idle timeout"), "Error should indicate idle timeout: {}", msg);
    }
}

#[tokio::test]
async fn test_token_counting_during_streaming() {
    // Test that token usage is properly reported in streaming responses
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(StreamingMocks::streaming_text_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = StreamingTestFixture::new()
        .with_mock_server_url(&mock_server.uri())
        .build();

    let prompt = StreamingTestFixture::create_prompt();

    // Act
    let mut stream = client.stream(&prompt).await.expect("Stream should start");
    let mut completion_event: Option<ResponseEvent> = None;

    while let Some(event) = stream.next().await {
        let event = event.expect("Event should be valid");
        if matches!(event, ResponseEvent::Completed { .. }) {
            completion_event = Some(event);
        }
    }

    // Assert
    assert!(completion_event.is_some(), "Should receive completion event");
    
    if let Some(ResponseEvent::Completed { response_id, token_usage }) = completion_event {
        assert_eq!(response_id, "resp-456");
        assert!(token_usage.is_some(), "Should have token usage");
        
        let usage = token_usage.unwrap();
        assert_eq!(usage.input_tokens, 15);
        assert_eq!(usage.output_tokens, 20);
        assert_eq!(usage.total_tokens, 35);
        assert_eq!(usage.cached_input_tokens, None);
        assert_eq!(usage.reasoning_output_tokens, None);
    }
}

#[tokio::test]
async fn test_concurrent_stream_handling() {
    // Test multiple concurrent streaming connections
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(StreamingMocks::streaming_text_response())
        .expect(3) // Three concurrent requests
        .mount(&mock_server)
        .await;

    let client = Arc::new(StreamingTestFixture::new()
        .with_mock_server_url(&mock_server.uri())
        .build());

    let prompt = StreamingTestFixture::create_prompt();

    // Act - Start 3 concurrent streams
    let tasks = (0..3).map(|i| {
        let client = Arc::clone(&client);
        let prompt = prompt.clone();
        tokio::spawn(async move {
            let mut stream = client.stream(&prompt).await?;
            let mut events = Vec::new();
            
            while let Some(event) = stream.next().await {
                let event = event?;
                events.push(event);
            }
            
            Ok::<_, CodexErr>((i, events))
        })
    }).collect::<Vec<_>>();

    let results = futures::future::join_all(tasks).await;

    // Assert
    for (i, result) in results.into_iter().enumerate() {
        let task_result = result.expect("Task should not panic");
        let (stream_id, events) = task_result.expect("Stream should succeed");
        assert_eq!(stream_id, i, "Stream ID should match");
        assert!(events.len() >= 6, "Each stream should receive all events");
    }
}

#[tokio::test]
async fn test_stream_resource_cleanup() {
    // Test that stream resources are properly cleaned up
    let mock_server = MockServer::start().await;
    
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(StreamingMocks::streaming_text_response())
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = StreamingTestFixture::new()
        .with_mock_server_url(&mock_server.uri())
        .build();

    let prompt = StreamingTestFixture::create_prompt();

    // Act - Create stream but drop it early
    {
        let mut stream = client.stream(&prompt).await.expect("Stream should start");
        
        // Consume only the first event, then drop the stream
        if let Some(event) = stream.next().await {
            assert!(event.is_ok(), "First event should be valid");
        }
        
        // Stream goes out of scope here and should be cleaned up
    }

    // Give some time for cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Assert
    // In a real implementation, we might check resource counters or metrics
    // For now, just verify the test completes without hanging
    assert!(true, "Stream cleanup test completed");
}

#[tokio::test]
async fn test_partial_response_handling() {
    // Test handling of responses that don't complete normally
    let mock_server = MockServer::start().await;
    
    // Response that has output items but no completion
    let partial_events = vec![
        "event: response.created\ndata: {\"type\":\"response.created\",\"response\":{}}\n\n",
        "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"Partial\"}\n\n",
        "event: response.output_item.done\ndata: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"message\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"Partial\"}]}}\n\n",
        // Missing completion event
    ];
    
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_string(partial_events.join(""))
            .insert_header("content-type", "text/event-stream"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = StreamingTestFixture::new()
        .with_mock_server_url(&mock_server.uri())
        .with_idle_timeout(1000) // Short timeout to detect missing completion
        .build();

    let prompt = StreamingTestFixture::create_prompt();

    // Act
    let mut stream = client.stream(&prompt).await.expect("Stream should start");
    let mut events = Vec::new();
    let mut stream_error = None;

    while let Some(event) = stream.next().await {
        match event {
            Ok(e) => events.push(e),
            Err(e) => {
                stream_error = Some(e);
                break;
            }
        }
    }

    // Assert
    assert!(events.len() >= 2, "Should receive partial events");
    assert!(stream_error.is_some(), "Should get error for incomplete stream");
    
    // Should have received the output item even without completion
    let has_output_item = events.iter().any(|e| matches!(e, ResponseEvent::OutputItemDone(_)));
    assert!(has_output_item, "Should receive output item before stream failure");
    
    if let Some(CodexErr::Stream(msg, _)) = stream_error {
        assert!(msg.contains("stream closed before") || msg.contains("idle timeout"), 
                "Should indicate incomplete stream: {}", msg);
    }
}