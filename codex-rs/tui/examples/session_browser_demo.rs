//! Demo showing the Session Browser widget in action
//!
//! This example creates a sample session index and displays it using the SessionBrowser widget.
//! Run with: cargo run --example session_browser_demo

use agcodex_core::modes::OperatingMode;
use agcodex_persistence::types::CheckpointMetadata;
use agcodex_persistence::types::SessionIndex;
use agcodex_persistence::types::SessionMetadata;
use agcodex_tui::widgets::SessionBrowser;
use chrono::Utc;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use uuid::Uuid;

fn create_sample_session_metadata(
    title: &str,
    mode: OperatingMode,
    favorite: bool,
) -> SessionMetadata {
    SessionMetadata {
        id: Uuid::new_v4(),
        title: title.to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_accessed: Utc::now(),
        message_count: (10..50)
            .into_iter()
            .nth(rand::random::<usize>() % 40)
            .unwrap_or(15),
        turn_count: (5..25)
            .into_iter()
            .nth(rand::random::<usize>() % 20)
            .unwrap_or(10),
        current_mode: mode,
        model: "gpt-4".to_string(),
        tags: if favorite {
            vec!["important".to_string(), "project".to_string()]
        } else {
            vec!["experiment".to_string()]
        },
        is_favorite: favorite,
        file_size: (1024..10240)
            .into_iter()
            .nth(rand::random::<usize>() % 9216)
            .unwrap_or(2048) as u64,
        compression_ratio: 0.75 + (rand::random::<f32>() * 0.2),
        format_version: 1,
        checkpoints: if favorite {
            vec![CheckpointMetadata {
                id: Uuid::new_v4(),
                name: "Important checkpoint".to_string(),
                created_at: Utc::now(),
                message_index: 5,
                description: Some("Before major refactoring".to_string()),
            }]
        } else {
            vec![]
        },
    }
}

fn create_sample_session_index() -> SessionIndex {
    let mut session_index = SessionIndex::new();

    // Add some sample sessions
    let sessions = vec![
        create_sample_session_metadata(
            "Refactor authentication system",
            OperatingMode::Build,
            true,
        ),
        create_sample_session_metadata("Fix CSS layout issues", OperatingMode::Review, false),
        create_sample_session_metadata(
            "Plan new feature: real-time chat",
            OperatingMode::Plan,
            true,
        ),
        create_sample_session_metadata("Debug memory leak in parser", OperatingMode::Build, false),
        create_sample_session_metadata("Review API security", OperatingMode::Review, true),
        create_sample_session_metadata(
            "Experiment with new UI library",
            OperatingMode::Build,
            false,
        ),
        create_sample_session_metadata("Plan database migration", OperatingMode::Plan, false),
        create_sample_session_metadata("Implement user preferences", OperatingMode::Build, true),
    ];

    for session in sessions {
        session_index.add_session(session);
    }

    session_index
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("AGCodex Session Browser Demo");
    println!("============================");

    // Create sample data
    let session_index = create_sample_session_index();
    println!("Created {} sample sessions", session_index.sessions.len());

    // Create the session browser widget
    let browser = SessionBrowser::new(session_index);

    // Create a test terminal for rendering
    let backend = TestBackend::new(100, 40);
    let mut terminal = Terminal::new(backend)?;

    // Render the widget
    terminal.draw(|f| {
        let area = f.area();
        browser.render(area, f.buffer_mut());
    })?;

    // Get the rendered buffer for inspection
    let backend = terminal.backend();
    let buffer = backend.buffer();

    println!("\nRendered Session Browser (first 10 lines):");
    println!("==========================================");

    // Display the first 10 lines of the rendered output
    for y in 0..10.min(buffer.area().height) {
        let mut line = String::new();
        for x in 0..buffer.area().width {
            let cell = &buffer[(x, y)];
            line.push(cell.symbol().chars().next().unwrap_or(' '));
        }
        println!("{:2}: {}", y + 1, line.trim_end());
    }

    // Display session browser functionality
    println!("\nSession Browser Features:");
    println!("=========================");
    println!("✓ Multiple view modes: Tree, List, Timeline");
    println!("✓ Sorting by: Last Accessed, Created, Name, Messages, Size");
    println!("✓ Search across session titles and tags");
    println!("✓ Panel navigation: Session List → Preview → Actions → Search");
    println!("✓ Session metadata display with mode indicators");
    println!("✓ Action support: Open, Delete, Export, Rename, Favorites");
    println!("✓ Confirmation dialogs for destructive actions");
    println!("✓ Export options: Markdown, JSON, Plain text");

    println!("\nKey Bindings (when integrated):");
    println!("===============================");
    println!("↑/↓           Navigate sessions/actions");
    println!("Tab           Switch between panels");
    println!("Enter         Execute selected action/open session");
    println!("/             Start search");
    println!("V             Toggle view mode");
    println!("S             Cycle sort order");
    println!("F             Toggle favorites filter");
    println!("Del           Delete selected session");
    println!("Esc           Cancel operation/close dialog");

    Ok(())
}
