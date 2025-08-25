# Client API Communication Tests

This directory contains comprehensive tests for AGCodex's client API communication layer, covering both general API functionality and streaming-specific scenarios.

## Test Architecture

The tests follow SLEEK principles with mandatory diagrams and systematic testing approaches:

### Architecture Δ
- **Components**: ModelClient, Mock HTTP servers (wiremock), Authentication providers, Provider configurations
- **Interfaces/contracts**: HTTP REST APIs, Server-Sent Events (SSE), Authentication flows, Error response formats  
- **Data flows**: Request → Authentication → Provider → HTTP client → Mock server → Response → SSE parser → Events
- **Security boundaries**: API key isolation, Network timeout enforcement, Rate limiting compliance

### Memory & Concurrency Δ
- **Ownership**: Tests own mock servers, Client owns HTTP connections, Streams own response channels
- **Concurrency**: Test runner, Mock server, SSE stream processor, Authentication refresher
- **Synchronization**: tokio::test coordination, Mock server lifecycle, Stream completion signaling

## Test Files

### `api_test.rs` - General API Testing
Comprehensive tests covering core API functionality:

#### Authentication Tests
- **API key authentication**: Verifies proper header formatting and environment variable handling
- **ChatGPT authentication**: Tests session-based auth with account ID headers
- **Token refresh on 401**: Validates automatic token refresh mechanism

#### Error Handling Tests  
- **Rate limiting**: Tests 429 responses with retry-after parsing
- **Usage limits**: Validates usage_limit_reached error handling
- **Validation errors**: Tests 400 Bad Request error propagation
- **Network timeouts**: Simulates connection timeouts and proper error handling
- **Server errors with retry**: Tests exponential backoff retry logic

#### Multi-Provider Support
- **OpenAI Responses API**: Tests WireApi::Responses endpoint
- **Anthropic Chat API**: Tests WireApi::Chat completions endpoint
- **Provider configuration**: Validates different provider settings

#### Request/Response Validation
- **Header verification**: Ensures proper User-Agent, originator, session_id headers
- **JSON payload validation**: Tests request serialization against expected schema
- **Concurrent requests**: Validates thread safety with multiple simultaneous requests
- **Memory usage**: Tests resource management under load

### `streaming_test.rs` - Streaming-Specific Testing
Focuses on Server-Sent Events (SSE) streaming scenarios:

#### Streaming Response Tests
- **Incremental text deltas**: Verifies real-time text streaming with OutputTextDelta events
- **Reasoning content**: Tests o1-style models with ReasoningContentDelta and ReasoningSummaryDelta
- **Event ordering**: Validates chronological event delivery

#### Stream Reliability Tests
- **Backpressure handling**: Tests system behavior with fast-producing streams and slow consumers
- **Stream interruption**: Validates recovery from connection drops and incomplete responses
- **Malformed events**: Tests resilience against invalid SSE JSON data
- **Empty streams**: Handles streams with no events

#### Performance & Resource Tests
- **Idle timeout**: Tests stream idle timeout behavior with configurable timeouts
- **Concurrent streams**: Validates multiple simultaneous streaming connections
- **Resource cleanup**: Ensures proper cleanup when streams are dropped early
- **Token counting**: Verifies accurate token usage reporting in completion events

## Test Infrastructure

### Mock Server Architecture
Uses `wiremock` for deterministic HTTP mocking:

```rust
// Example mock setup
Mock::given(method("POST"))
    .and(path("/responses"))
    .and(header("authorization", "Bearer test-key"))
    .respond_with(MockResponses::successful_stream())
    .expect(1)
    .mount(&mock_server)
    .await;
```

### Test Fixtures
- **ClientTestFixture**: Builder pattern for consistent ModelClient setup
- **StreamingTestFixture**: Specialized setup for SSE testing scenarios
- **MockResponses**: Pre-built response templates for common scenarios

### Deterministic Testing
All tests are designed to be:
- **Fast**: Most tests complete in <100ms
- **Deterministic**: Same inputs always produce same outputs  
- **Isolated**: No cross-test dependencies or shared state
- **Reliable**: No flaky timeouts or race conditions

## Running the Tests

```bash
# Run all client tests
cargo test client:: --no-fail-fast

# Run specific test files
cargo test -p agcodex-core tests::client::api_test --no-fail-fast
cargo test -p agcodex-core tests::client::streaming_test --no-fail-fast

# Run with output for debugging
cargo test client:: --no-fail-fast -- --nocapture
```

## Test Coverage

### API Test Coverage (api_test.rs)
- ✅ Successful API request with streaming
- ✅ API key authentication 
- ✅ ChatGPT authentication mode
- ✅ Rate limiting with retry-after
- ✅ Usage limit reached error
- ✅ Validation error handling
- ✅ Unauthorized error with token refresh
- ✅ Retry behavior on server errors
- ✅ Network timeout handling
- ✅ Multi-provider support (Responses & Chat APIs)
- ✅ Request headers and custom params
- ✅ JSON payload serialization
- ✅ Concurrent requests
- ✅ Memory usage under load

### Streaming Test Coverage (streaming_test.rs)
- ✅ Streaming vs non-streaming responses
- ✅ Reasoning content streaming (o1-style models)
- ✅ Backpressure handling with slow consumers
- ✅ Stream interruption recovery
- ✅ Malformed SSE event handling
- ✅ Empty stream handling
- ✅ Idle timeout behavior
- ✅ Token counting during streaming
- ✅ Concurrent stream handling
- ✅ Stream resource cleanup
- ✅ Partial response handling

## Design Principles

### SLEEK Adherence
- **Diagrams First**: All tests designed with comprehensive architecture diagrams
- **Concurrent Orchestration**: Tests use parallel execution where possible
- **Precise Context**: Tests target specific functionality boundaries
- **Error Handling**: Uses `thiserror` exclusively, comprehensive error scenarios
- **Performance Targets**: All tests meet <100ms execution time goals

### Testing Best Practices
- **Arrange-Act-Assert**: Consistent test structure
- **One assertion per test**: Each test proves one specific behavior
- **Deterministic**: No time-based assertions or random data
- **Independent**: Tests can run in any order without affecting each other
- **Fast**: Quick feedback with minimal setup overhead

## Future Enhancements

### Additional Test Scenarios
- **WebSocket streaming**: If/when WebSocket support is added
- **Compression testing**: gzip/deflate response handling
- **IPv6 connectivity**: Network stack variations
- **Proxy support**: HTTP/HTTPS proxy scenarios

### Performance Testing
- **Load testing**: High-volume concurrent request handling
- **Memory profiling**: Detailed memory usage analysis
- **Latency measurements**: Response time percentiles
- **Resource leak detection**: Long-running connection tests

### Security Testing
- **TLS certificate validation**: Certificate chain verification
- **Request signing**: Authentication signature validation
- **Rate limit bypassing**: Security boundary testing
- **Injection attacks**: Input validation security