//! Integration example for Agent Panel and Notifications
//!
//! This module demonstrates how to integrate the agent panel and notification
//! system into the main TUI application.

use super::{AgentPanel, NotificationManager, NotificationType, Notification};
use agcodex_core::subagents::{SubagentExecution, SubagentStatus};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    terminal::Frame,
    widgets::{Block, Borders, Paragraph},
};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Example application state with agent components
pub struct AppWithAgents {
    /// Agent panel for managing agents
    pub agent_panel: AgentPanel,
    /// Notification manager for agent events
    pub notification_manager: NotificationManager,
    /// Main content area
    pub content: String,
    /// Whether to show help
    pub show_help: bool,
}

impl AppWithAgents {
    /// Create a new application with agent components
    pub fn new() -> Self {
        Self {
            agent_panel: AgentPanel::new(),
            notification_manager: NotificationManager::new()
                .with_position(super::NotificationPosition::BottomRight)
                .with_max_visible(3),
            content: String::from("AGCodex TUI with Agent Management"),
            show_help: false,
        }
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        // Check if agent panel handles the key first
        if self.agent_panel.handle_key(key) {
            return true;
        }

        match key {
            // Toggle agent panel with Ctrl+A
            KeyCode::Char('a') if event::KeyModifiers::CONTROL == event::KeyModifiers::CONTROL => {
                self.agent_panel.toggle();
                true
            }
            // Spawn test agent with Ctrl+N
            KeyCode::Char('n') if event::KeyModifiers::CONTROL == event::KeyModifiers::CONTROL => {
                self.spawn_test_agent();
                true
            }
            // Clear all notifications with Ctrl+C
            KeyCode::Char('c') if event::KeyModifiers::CONTROL == event::KeyModifiers::CONTROL => {
                self.notification_manager.clear_all();
                true
            }
            // Toggle help
            KeyCode::Char('?') | KeyCode::F(1) => {
                self.show_help = !self.show_help;
                true
            }
            _ => false,
        }
    }

    /// Spawn a test agent for demonstration
    pub fn spawn_test_agent(&mut self) {
        let agent_name = format!("agent-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
        let mut execution = SubagentExecution::new(agent_name.clone());
        execution.start();
        
        let agent_id = execution.id;
        
        // Add to panel
        self.agent_panel.add_agent(execution);
        
        // Send started notification
        self.notification_manager.agent_started(agent_name.clone());
        
        // Simulate progress updates
        let panel = Arc::new(self.agent_panel.clone());
        let notifier = Arc::new(self.notification_manager.clone());
        
        std::thread::spawn(move || {
            for i in 1..=10 {
                std::thread::sleep(Duration::from_millis(500));
                
                let progress = i as f32 / 10.0;
                let message = format!("Processing step {}/10", i);
                
                // Update panel
                panel.update_progress(agent_id, progress, message.clone());
                
                // Send progress notification every 3 steps
                if i % 3 == 0 {
                    notifier.agent_progress(
                        agent_name.clone(),
                        progress,
                        format!("{}% complete", (progress * 100.0) as u32),
                    );
                }
                
                // Add some output
                panel.add_output(agent_id, format!("[{}] {}", i, message));
            }
            
            // Complete the agent
            panel.complete_agent(agent_id, "Task completed successfully!".to_string());
            notifier.agent_completed(
                agent_name.clone(),
                "All steps processed".to_string(),
            );
        });
    }

    /// Simulate an agent failure for demonstration
    pub fn simulate_agent_failure(&mut self) {
        let agent_name = "failing-agent";
        let mut execution = SubagentExecution::new(agent_name.to_string());
        execution.start();
        
        let agent_id = execution.id;
        
        // Add to panel
        self.agent_panel.add_agent(execution);
        
        // Immediately fail
        self.agent_panel.fail_agent(agent_id, "Connection timeout".to_string());
        
        // Send failure notification
        self.notification_manager.agent_failed(
            agent_name.to_string(),
            "Failed to connect to remote service".to_string(),
        );
    }

    /// Update the application state (called every frame)
    pub fn tick(&mut self) {
        // Update animations
        self.agent_panel.tick();
        self.notification_manager.tick();
    }

    /// Render the application
    pub fn render<B: Backend>(&self, frame: &mut Frame<B>) {
        let size = frame.size();
        
        // Create main layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(10),    // Main content
                Constraint::Length(3),  // Status bar
            ])
            .split(size);
        
        // Render main content
        let content_block = Block::default()
            .title(" AGCodex TUI ")
            .borders(Borders::ALL);
        let content = Paragraph::new(self.content.as_str())
            .block(content_block);
        frame.render_widget(content, chunks[0]);
        
        // Render status bar
        let status = if self.show_help {
            "Ctrl+A: Agent Panel | Ctrl+N: New Agent | Ctrl+C: Clear Notifications | ?: Help | Esc: Exit"
        } else {
            "Press ? for help"
        };
        let status_bar = Paragraph::new(status)
            .block(Block::default().borders(Borders::TOP));
        frame.render_widget(status_bar, chunks[1]);
        
        // Render agent panel if visible (overlay)
        if self.agent_panel.is_visible() {
            let panel_area = centered_rect(60, 70, size);
            frame.render_widget(&self.agent_panel, panel_area);
        }
        
        // Render notifications (always visible when active)
        frame.render_widget(&self.notification_manager, size);
    }
}

/// Helper function to create a centered rectangle
fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Example usage in main application loop
pub async fn run_example() -> std::io::Result<()> {
    // This would be integrated into the main TUI app
    let mut app = AppWithAgents::new();
    
    // Spawn some example agents
    app.spawn_test_agent();
    std::thread::sleep(Duration::from_millis(100));
    app.spawn_test_agent();
    
    // Simulate a failure
    app.simulate_agent_failure();
    
    // Main event loop would go here
    // loop {
    //     app.tick();
    //     
    //     if let Event::Key(key) = event::read()? {
    //         if key.code == KeyCode::Esc {
    //             break;
    //         }
    //         app.handle_key(key.code);
    //     }
    //     
    //     terminal.draw(|f| app.render(f))?;
    // }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_app_creation() {
        let app = AppWithAgents::new();
        assert!(!app.agent_panel.is_visible());
        assert!(!app.show_help);
    }
    
    #[test]
    fn test_agent_panel_toggle() {
        let mut app = AppWithAgents::new();
        assert!(!app.agent_panel.is_visible());
        
        // Toggle with Ctrl+A
        app.agent_panel.toggle();
        assert!(app.agent_panel.is_visible());
        
        app.agent_panel.toggle();
        assert!(!app.agent_panel.is_visible());
    }
    
    #[test]
    fn test_spawn_agent() {
        let mut app = AppWithAgents::new();
        app.spawn_test_agent();
        
        // Agent should be added to panel
        // Note: In real implementation, we'd need to wait for the thread
        std::thread::sleep(Duration::from_millis(50));
    }
    
    #[test]
    fn test_notification_manager() {
        let app = AppWithAgents::new();
        
        // Send various notifications
        app.notification_manager.agent_started("test-agent".to_string());
        app.notification_manager.agent_progress(
            "test-agent".to_string(),
            0.5,
            "50% complete".to_string(),
        );
        app.notification_manager.agent_completed(
            "test-agent".to_string(),
            "Success!".to_string(),
        );
        app.notification_manager.agent_failed(
            "test-agent".to_string(),
            "Error occurred".to_string(),
        );
    }
    
    #[test]
    fn test_centered_rect() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 50,
        };
        
        let centered = centered_rect(50, 50, area);
        assert_eq!(centered.width, 50);
        assert_eq!(centered.height, 25);
        assert_eq!(centered.x, 25);
        assert_eq!(centered.y, 12);
    }
}