//! Agent Panel Widget
//!
//! Displays running agents with progress bars, status indicators, and controls.
//! Features real-time progress updates, cancellation buttons, and execution history.

use agcodex_core::subagents::SubagentExecution;
use agcodex_core::subagents::SubagentStatus;
use ratatui::buffer::Buffer;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;
use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;

/// Agent panel state and data
#[derive(Debug, Clone, Default)]
pub struct AgentPanel {
    /// Currently running agents
    running_agents: HashMap<Uuid, AgentExecution>,
    /// Completed agents (limited history)
    completed_agents: Vec<AgentExecution>,
    /// Current selection in the agent list
    selected_index: usize,
    /// Whether the panel is visible
    visible: bool,
    /// Maximum completed agents to keep
    max_history: usize,
    /// Progress updates for streaming agents
    progress_updates: HashMap<Uuid, ProgressInfo>,
}

/// Extended agent execution with UI state
#[derive(Debug, Clone)]
pub struct AgentExecution {
    /// Core execution data
    pub execution: SubagentExecution,
    /// UI-specific progress information
    pub progress: f32,
    /// Current status message
    pub status_message: String,
    /// Whether this agent can be cancelled
    pub cancellable: bool,
    /// Output chunks for streaming display
    pub output_chunks: Vec<String>,
    /// Total output length (for truncation)
    pub total_output_length: usize,
    /// Execution start time for UI display
    pub ui_started_at: SystemTime,
}

/// Progress information for streaming updates
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub progress: f32,
    pub message: String,
    pub last_update: SystemTime,
}

impl AgentPanel {
    /// Create a new agent panel
    pub fn new() -> Self {
        Self {
            running_agents: HashMap::new(),
            completed_agents: Vec::new(),
            selected_index: 0,
            visible: false,
            max_history: 10,
            progress_updates: HashMap::new(),
        }
    }

    /// Toggle panel visibility
    pub const fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    /// Set panel visibility
    pub const fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Check if panel is visible
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    /// Add a new running agent
    pub fn add_agent(&mut self, execution: SubagentExecution) {
        let agent_execution = AgentExecution {
            progress: 0.0,
            status_message: "Starting...".to_string(),
            cancellable: true,
            output_chunks: Vec::new(),
            total_output_length: 0,
            ui_started_at: SystemTime::now(),
            execution,
        };

        self.running_agents
            .insert(agent_execution.execution.id, agent_execution);
    }

    /// Update agent progress
    pub fn update_progress(&mut self, agent_id: Uuid, progress: f32, message: String) {
        if let Some(agent) = self.running_agents.get_mut(&agent_id) {
            agent.progress = progress.clamp(0.0, 1.0);
            agent.status_message = message.clone();
        }

        self.progress_updates.insert(
            agent_id,
            ProgressInfo {
                progress,
                message,
                last_update: SystemTime::now(),
            },
        );
    }

    /// Add output chunk for streaming agent
    pub fn add_output_chunk(&mut self, agent_id: Uuid, chunk: String) {
        if let Some(agent) = self.running_agents.get_mut(&agent_id) {
            agent.total_output_length += chunk.len();
            agent.output_chunks.push(chunk);

            // Limit chunks to prevent excessive memory usage
            if agent.output_chunks.len() > 100 {
                let removed = agent.output_chunks.remove(0);
                agent.total_output_length -= removed.len();
            }
        }
    }

    /// Complete an agent execution
    pub fn complete_agent(&mut self, agent_id: Uuid, execution: SubagentExecution) {
        if let Some(mut agent) = self.running_agents.remove(&agent_id) {
            agent.execution = execution;
            agent.progress = 1.0;
            agent.status_message = "Completed".to_string();
            agent.cancellable = false;

            // Move to completed list
            self.completed_agents.push(agent);

            // Limit history
            if self.completed_agents.len() > self.max_history {
                self.completed_agents.remove(0);
            }
        }

        self.progress_updates.remove(&agent_id);
    }

    /// Fail an agent execution
    pub fn fail_agent(&mut self, agent_id: Uuid, error: String) {
        if let Some(mut agent) = self.running_agents.remove(&agent_id) {
            agent.execution.fail(error.clone());
            agent.progress = 0.0;
            agent.status_message = format!("Failed: {}", error);
            agent.cancellable = false;

            // Move to completed list
            self.completed_agents.push(agent);

            // Limit history
            if self.completed_agents.len() > self.max_history {
                self.completed_agents.remove(0);
            }
        }

        self.progress_updates.remove(&agent_id);
    }

    /// Cancel an agent execution
    pub fn cancel_agent(&mut self, agent_id: Uuid) {
        if let Some(mut agent) = self.running_agents.remove(&agent_id) {
            agent.execution.status = SubagentStatus::Cancelled;
            agent.progress = 0.0;
            agent.status_message = "Cancelled".to_string();
            agent.cancellable = false;

            // Move to completed list
            self.completed_agents.push(agent);
        }

        self.progress_updates.remove(&agent_id);
    }

    /// Get the currently selected agent ID
    pub fn selected_agent_id(&self) -> Option<Uuid> {
        let all_agents: Vec<_> = self
            .running_agents
            .values()
            .chain(self.completed_agents.iter())
            .collect();

        all_agents
            .get(self.selected_index)
            .map(|agent| agent.execution.id)
    }

    /// Navigate up in the agent list
    pub const fn navigate_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Navigate down in the agent list
    pub fn navigate_down(&mut self) {
        let total_agents = self.running_agents.len() + self.completed_agents.len();
        if self.selected_index < total_agents.saturating_sub(1) {
            self.selected_index += 1;
        }
    }

    /// Get the total number of agents
    pub fn total_agents(&self) -> usize {
        self.running_agents.len() + self.completed_agents.len()
    }

    /// Get the number of running agents
    pub fn running_count(&self) -> usize {
        self.running_agents.len()
    }

    /// Get the number of completed agents
    pub const fn completed_count(&self) -> usize {
        self.completed_agents.len()
    }

    /// Clear all completed agents
    pub fn clear_completed(&mut self) {
        self.completed_agents.clear();
    }

    /// Get agent by ID
    pub fn get_agent(&self, agent_id: Uuid) -> Option<&AgentExecution> {
        self.running_agents.get(&agent_id).or_else(|| {
            self.completed_agents
                .iter()
                .find(|a| a.execution.id == agent_id)
        })
    }
}

impl WidgetRef for &AgentPanel {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Clear the area
        Clear.render(area, buf);

        // Main panel border
        let block = Block::default()
            .title("󰚩 Agent Panel")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 3 {
            return; // Too small to render content
        }

        // Split into sections: header, agent list, footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Min(1),    // Agent list
                Constraint::Length(2), // Footer
            ])
            .split(inner);

        // Render header with status counts
        self.render_header(chunks[0], buf);

        // Render agent list
        self.render_agent_list(chunks[1], buf);

        // Render footer with help text
        self.render_footer(chunks[2], buf);
    }
}

impl AgentPanel {
    /// Render the header with status counts
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let running = self.running_count();
        let completed = self.completed_count();

        let header_text = if running > 0 {
            format!("󰑮 {} running  󰄬 {} completed", running, completed)
        } else if completed > 0 {
            format!("󰄬 {} completed", completed)
        } else {
            "No agents".to_string()
        };

        Paragraph::new(header_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .render(area, buf);
    }

    /// Render the agent list with progress bars
    fn render_agent_list(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 2 {
            return;
        }

        let mut items = Vec::new();
        let mut list_state = ListState::default();

        // Add running agents
        for agent in self.running_agents.values() {
            items.push(self.format_agent_item(agent, true));
        }

        // Add completed agents
        for agent in &self.completed_agents {
            items.push(self.format_agent_item(agent, false));
        }

        if !items.is_empty() {
            list_state.select(Some(self.selected_index.min(items.len() - 1)));
        }

        let list = List::new(items)
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol("▶ ");

        StatefulWidget::render(list, area, buf, &mut list_state);
    }

    /// Format a single agent item for the list
    fn format_agent_item<'a>(&self, agent: &'a AgentExecution, is_running: bool) -> ListItem<'a> {
        let agent_name = &agent.execution.agent_name;
        let status_icon = match agent.execution.status {
            SubagentStatus::Running => "󰑮",
            SubagentStatus::Completed => "󰄬",
            SubagentStatus::Failed(_) => "󰅙",
            SubagentStatus::Cancelled => "󰜺",
            SubagentStatus::Pending => "󰦖",
        };

        let status_color = match agent.execution.status {
            SubagentStatus::Running => Color::Blue,
            SubagentStatus::Completed => Color::Green,
            SubagentStatus::Failed(_) => Color::Red,
            SubagentStatus::Cancelled => Color::Yellow,
            SubagentStatus::Pending => Color::Gray,
        };

        // Duration calculation
        let duration_text = if let Some(duration) = agent.execution.duration() {
            format!("{}s", duration.as_secs())
        } else {
            let elapsed = agent.ui_started_at.elapsed().unwrap_or(Duration::ZERO);
            format!("{}s", elapsed.as_secs())
        };

        let mut spans = vec![
            Span::styled(status_icon, Style::default().fg(status_color)),
            Span::raw(" "),
            Span::styled(
                agent_name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ];

        // Add progress bar for running agents
        if is_running && agent.progress > 0.0 {
            let progress_text = format!("[{:3.0}%]", agent.progress * 100.0);
            spans.push(Span::styled(
                progress_text,
                Style::default().fg(Color::Cyan),
            ));
            spans.push(Span::raw(" "));
        }

        spans.extend_from_slice(&[
            Span::raw("("),
            Span::styled(duration_text, Style::default().fg(Color::Gray)),
            Span::raw(") "),
            Span::styled(&agent.status_message, Style::default().fg(Color::Gray)),
        ]);

        ListItem::new(Line::from(spans))
    }

    /// Render the footer with help text
    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let help_text = if self.running_count() > 0 {
            "↑/↓ navigate  Enter cancel  Esc close  C clear completed"
        } else {
            "↑/↓ navigate  Esc close  C clear completed"
        };

        Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agcodex_core::subagents::SubagentStatus;

    #[test]
    fn test_agent_panel_creation() {
        let panel = AgentPanel::new();
        assert!(!panel.is_visible());
        assert_eq!(panel.total_agents(), 0);
        assert_eq!(panel.running_count(), 0);
        assert_eq!(panel.completed_count(), 0);
    }

    #[test]
    fn test_agent_panel_visibility_toggle() {
        let mut panel = AgentPanel::new();
        assert!(!panel.is_visible());

        panel.toggle_visibility();
        assert!(panel.is_visible());

        panel.toggle_visibility();
        assert!(!panel.is_visible());

        panel.set_visible(true);
        assert!(panel.is_visible());
    }

    #[test]
    fn test_agent_addition_and_completion() {
        let mut panel = AgentPanel::new();
        let mut execution = SubagentExecution::new("test-agent".to_string());
        execution.start();

        let agent_id = execution.id;
        panel.add_agent(execution);

        assert_eq!(panel.running_count(), 1);
        assert_eq!(panel.completed_count(), 0);

        // Update progress
        panel.update_progress(agent_id, 0.5, "Processing...".to_string());
        let agent = panel.get_agent(agent_id).unwrap();
        assert_eq!(agent.progress, 0.5);
        assert_eq!(agent.status_message, "Processing...");

        // Complete the agent
        let mut completed_execution = SubagentExecution::new("test-agent".to_string());
        completed_execution.complete("Success!".to_string(), vec![]);
        panel.complete_agent(agent_id, completed_execution);

        assert_eq!(panel.running_count(), 0);
        assert_eq!(panel.completed_count(), 1);
    }

    #[test]
    fn test_agent_panel_navigation() {
        let mut panel = AgentPanel::new();

        // Add multiple agents
        for i in 0..3 {
            let mut execution = SubagentExecution::new(format!("agent-{}", i));
            execution.start();
            panel.add_agent(execution);
        }

        assert_eq!(panel.selected_index, 0);

        panel.navigate_down();
        assert_eq!(panel.selected_index, 1);

        panel.navigate_down();
        assert_eq!(panel.selected_index, 2);

        // Should not go beyond bounds
        panel.navigate_down();
        assert_eq!(panel.selected_index, 2);

        panel.navigate_up();
        assert_eq!(panel.selected_index, 1);

        panel.navigate_up();
        assert_eq!(panel.selected_index, 0);

        // Should not go below 0
        panel.navigate_up();
        assert_eq!(panel.selected_index, 0);
    }

    #[test]
    fn test_agent_failure_handling() {
        let mut panel = AgentPanel::new();
        let mut execution = SubagentExecution::new("failing-agent".to_string());
        execution.start();

        let agent_id = execution.id;
        panel.add_agent(execution);

        assert_eq!(panel.running_count(), 1);

        panel.fail_agent(agent_id, "Test error".to_string());

        assert_eq!(panel.running_count(), 0);
        assert_eq!(panel.completed_count(), 1);

        let agent = panel.get_agent(agent_id).unwrap();
        assert!(matches!(agent.execution.status, SubagentStatus::Failed(_)));
        assert!(agent.status_message.contains("Failed"));
    }

    #[test]
    fn test_output_chunking() {
        let mut panel = AgentPanel::new();
        let mut execution = SubagentExecution::new("streaming-agent".to_string());
        execution.start();

        let agent_id = execution.id;
        panel.add_agent(execution);

        panel.add_output_chunk(agent_id, "First chunk".to_string());
        panel.add_output_chunk(agent_id, "Second chunk".to_string());

        let agent = panel.get_agent(agent_id).unwrap();
        assert_eq!(agent.output_chunks.len(), 2);
        assert_eq!(agent.output_chunks[0], "First chunk");
        assert_eq!(agent.output_chunks[1], "Second chunk");
        assert!(agent.total_output_length > 0);
    }
}
