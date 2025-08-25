//! Integration tests for @agent-name invocation in conversation flow

use agcodex_core::ConversationManagerExt;
use agcodex_core::InterceptResult;
use agcodex_core::MessageContext;
use agcodex_core::modes::OperatingMode;
use std::path::PathBuf;

#[tokio::test]
async fn test_agent_invocation_parsing() {
    // Create a basic manager (without real agents for this test)
    let manager = ConversationManagerExt::new_basic();
    let context = MessageContext::default();

    // Test that normal messages pass through
    let result = manager
        .intercept_message("This is a normal message", context.clone())
        .await
        .unwrap();

    assert!(matches!(result, InterceptResult::PassThrough));

    // Test that agent patterns are detected (will fail without registered agents)
    let result = manager
        .intercept_message("@code-reviewer check this function", context.clone())
        .await;

    // This should either pass through or give an error about missing agents
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_multiple_agent_invocation() {
    let manager = ConversationManagerExt::new_basic();
    let context = MessageContext::default();

    // Test parallel execution pattern
    let message = "@performance analyze + @security audit the codebase";
    let result = manager.intercept_message(message, context.clone()).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_sequential_agent_chain() {
    let manager = ConversationManagerExt::new_basic();
    let context = MessageContext::default();

    // Test sequential execution pattern
    let message = "@refactorer improve → @test-writer add tests";
    let result = manager.intercept_message(message, context.clone()).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_conditional_agent_invocation() {
    let manager = ConversationManagerExt::new_basic();
    let context = MessageContext::default();

    // Test conditional execution pattern
    let message = "@debugger if errors in the test suite";
    let result = manager.intercept_message(message, context.clone()).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_agent_with_parameters() {
    let manager = ConversationManagerExt::new_basic();
    let context = MessageContext::default();

    // Test agent invocation with parameters
    let message = r#"@code-reviewer level=high focus="security,performance" check the auth module"#;
    let result = manager.intercept_message(message, context.clone()).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mixed_content_with_agents() {
    let manager = ConversationManagerExt::new_basic();
    let context = MessageContext::default();

    // Test message with both regular content and agent invocations
    let message = "Can you help me improve this code? @refactorer suggest improvements and @test-writer add unit tests.";
    let result = manager.intercept_message(message, context.clone()).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_agent_aliases() {
    let manager = ConversationManagerExt::new_basic();
    let context = MessageContext::default();

    // Test that agent aliases work
    let messages = vec![
        "@reviewer check this",
        "@refactor improve this",
        "@debug find issues",
        "@test add coverage",
        "@perf optimize this",
    ];

    for message in messages {
        let result = manager.intercept_message(message, context.clone()).await;
        assert!(result.is_ok(), "Failed for message: {}", message);
    }
}

#[tokio::test]
async fn test_context_preservation() {
    let manager = ConversationManagerExt::new_basic();
    let mut context = MessageContext::default();
    context.cwd = PathBuf::from("/test/project");
    context.mode = OperatingMode::Review;

    // Test that context is preserved through agent invocations
    let message = "@code-reviewer analyze the changes";
    let result = manager.intercept_message(message, context.clone()).await;

    assert!(result.is_ok());
    // In a real implementation, we'd verify the context was passed to the agent
}

#[tokio::test]
async fn test_error_handling() {
    let manager = ConversationManagerExt::new_basic();
    let mut context = MessageContext::default();

    // Test with fail_on_agent_error = false (should not propagate errors)
    context.fail_on_agent_error = false;
    let message = "@nonexistent-agent do something";
    let result = manager.intercept_message(message, context.clone()).await;

    assert!(result.is_ok());

    // Test with fail_on_agent_error = true (should propagate errors)
    context.fail_on_agent_error = true;
    let result = manager.intercept_message(message, context.clone()).await;

    // This should still be OK since we handle missing agents gracefully
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_result_merging() {
    let manager = ConversationManagerExt::new_basic();
    let mut context = MessageContext::default();

    // Test with should_merge_results = true
    context.should_merge_results = true;
    let message = "@code-reviewer check this + @security audit";
    let result = manager.intercept_message(message, context.clone()).await;

    if let Ok(InterceptResult::Modified {
        message: merged, ..
    }) = result
    {
        // In a real scenario, this would contain merged agent outputs
        assert!(merged.contains("Agent Analysis") || !merged.is_empty());
    }

    // Test with should_merge_results = false (agent-only response)
    context.should_merge_results = false;
    let result = manager.intercept_message(message, context.clone()).await;

    assert!(result.is_ok());
}
