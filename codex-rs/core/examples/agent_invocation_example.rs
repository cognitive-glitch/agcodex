//! Example of using @agent-name invocations in conversations
//!
//! This example demonstrates how to use the subagent system to invoke
//! specialized AI assistants within a conversation flow.

use agcodex_core::ConversationManagerExt;
use agcodex_core::InterceptResult;
use agcodex_core::MessageContext;
use agcodex_core::config::Config;
use agcodex_core::config::ConfigOverrides;
use agcodex_core::modes::OperatingMode;
use std::path::PathBuf;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    // tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // Load configuration
    // Note: In a real application, Config would be loaded from config.toml
    // For this example, we use a minimal config load
    let config = Config::load_with_cli_overrides(vec![], ConfigOverrides::default())
        .expect("Failed to load config");

    // Create conversation manager with agent support
    let manager = ConversationManagerExt::new(&config)?;

    info!("AGCodex Agent System initialized");

    // Example 1: Single agent invocation
    demonstrate_single_agent(&manager).await?;

    // Example 2: Parallel agent execution
    demonstrate_parallel_agents(&manager).await?;

    // Example 3: Sequential agent chain
    demonstrate_sequential_chain(&manager).await?;

    // Example 4: Conditional agent execution
    demonstrate_conditional_execution(&manager).await?;

    // Example 5: Mixed content with agents
    demonstrate_mixed_content(&manager).await?;

    Ok(())
}

async fn demonstrate_single_agent(
    manager: &ConversationManagerExt,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Single Agent Invocation ===");

    let message = "@code-reviewer please review this Rust function for best practices";
    let context = MessageContext::default();

    match manager.intercept_message(message, context).await? {
        InterceptResult::PassThrough => {
            info!("No agents invoked, message passed through");
        }
        InterceptResult::Modified { message, metadata } => {
            info!("Message modified with agent results:");
            info!("{}", message);
            if let Some(meta) = metadata {
                info!("Metadata: {:?}", meta);
            }
        }
        InterceptResult::Handled { response, skip_llm } => {
            info!("Agent handled the request:");
            info!("{}", response);
            info!("Skip LLM: {}", skip_llm);
        }
    }

    Ok(())
}

async fn demonstrate_parallel_agents(
    manager: &ConversationManagerExt,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Parallel Agent Execution ===");

    let message = "@performance analyze the hot paths + @security check for vulnerabilities";
    let context = MessageContext {
        should_merge_results: true,
        ..Default::default()
    };

    match manager.intercept_message(message, context).await? {
        InterceptResult::Modified { message, .. } => {
            info!("Parallel agents completed:");
            info!("{}", message);
        }
        _ => info!("Unexpected result type"),
    }

    Ok(())
}

async fn demonstrate_sequential_chain(
    manager: &ConversationManagerExt,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Sequential Agent Chain ===");

    let message = "@refactorer improve code quality → @test-writer add comprehensive tests → @docs generate documentation";
    let context = MessageContext::default();

    match manager.intercept_message(message, context).await? {
        InterceptResult::Modified { message, .. } => {
            info!("Sequential chain completed:");
            info!("{}", message);
        }
        _ => info!("Unexpected result type"),
    }

    Ok(())
}

async fn demonstrate_conditional_execution(
    manager: &ConversationManagerExt,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Conditional Agent Execution ===");

    let message = "@debugger if test failures detected";
    let context = MessageContext::default();

    match manager.intercept_message(message, context).await? {
        InterceptResult::Modified { message, .. } => {
            info!("Conditional agent executed:");
            info!("{}", message);
        }
        InterceptResult::PassThrough => {
            info!("Condition not met, no agent executed");
        }
        _ => info!("Unexpected result type"),
    }

    Ok(())
}

async fn demonstrate_mixed_content(
    manager: &ConversationManagerExt,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Mixed Content with Agents ===");

    let message = r#"
I'm working on a high-performance web server in Rust. 
Can you help me optimize it?

@performance identify bottlenecks in the async runtime
@security audit the request handling for injection attacks
@code-reviewer check for proper error handling

Also, please suggest architectural improvements.
"#;

    let context = MessageContext {
        cwd: PathBuf::from("/projects/web-server"),
        mode: OperatingMode::Build,
        should_merge_results: true,
        fail_on_agent_error: false,
        ..Default::default()
    };

    match manager.intercept_message(message, context).await? {
        InterceptResult::Modified { message, metadata } => {
            info!("Mixed content processed:");
            info!("{}", message);
            if let Some(meta) = metadata {
                info!("Processing metadata: {:?}", meta);
            }
        }
        _ => info!("Unexpected result type"),
    }

    Ok(())
}

// Additional helper to demonstrate real conversation flow integration
async fn demonstrate_conversation_flow(
    manager: &ConversationManagerExt,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n=== Full Conversation Flow ===");

    // Create a new conversation
    let config = Config::load_with_cli_overrides(vec![], ConfigOverrides::default())
        .expect("Failed to load config");
    let new_conv = manager.inner().new_conversation(config).await?;

    info!("Created conversation: {}", new_conv.conversation_id);

    // Process a user message with agent invocations
    let submission_id = manager
        .process_user_turn(
            &new_conv.conversation,
            "@code-reviewer analyze the main.rs file for issues".to_string(),
            PathBuf::from("."),
            "gpt-4o".to_string(),
        )
        .await?;

    info!("Submitted turn with ID: {}", submission_id);

    // In a real application, you would now wait for events from the conversation
    // let event = new_conv.conversation.next_event().await?;

    Ok(())
}
