//! Enhanced Agent Panel Interface for AGCodex TUI
//!
//! Provides a comprehensive agent management panel with progress bars,
//! status indicators, and interactive controls.

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
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use uuid::Uuid;

/// Agent panel state with enhanced UI features
#[derive(Debug, Clone)]
pub struct AgentPanel {
    /// Active agents with progress tracking
    agents: Arc<Mutex<HashMap<Uuid, AgentInfo>>>,
    /// Selection state for keyboard navigation
    selected: usize,
    /// Panel visibility
    visible: bool,
    /// Panel dimensions
    area: Rect,
    /// Scroll offset for long lists
    scroll_offset: usize,
    /// Maximum visible items
    max_visible: usize,
    /// Animation frame counter
    animation_frame: usize,
    /// Last update time for animations
    last_update: Instant,
}

/// Enhanced agent information with UI state
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// Core execution data
    pub execution: SubagentExecution,
    /// Progress percentage (0.0 - 1.0)
    pub progress: f32,
    /// Current status message
    pub status_message: String,
    /// Agent start time
    pub started_at: SystemTime,
    /// Last progress update
    pub last_progress_update: Instant,
    /// Whether agent can be cancelled
    pub cancellable: bool,
    /// Output buffer for logs
    pub output_buffer: Vec<String>,
    /// Icon for visual representation
    pub icon: &'static str,
    /// Color for status indicator
    pub color: Color,
}

impl AgentInfo {
    /// Create new agent info from execution
    pub fn new(execution: SubagentExecution) -> Self {
        let (icon, color) = match execution.status {
            SubagentStatus::Pending => ("⏳", Color::Gray),
            SubagentStatus::Running => ("▶", Color::Blue),
            SubagentStatus::Completed => ("✓", Color::Green),
            SubagentStatus::Failed(_) => ("✗", Color::Red),
            SubagentStatus::Cancelled => ("⊘", Color::Yellow),
        };

        Self {
            execution,
            progress: 0.0,
            status_message: "Initializing...".to_string(),
            started_at: SystemTime::now(),
            last_progress_update: Instant::now(),
            cancellable: true,
            output_buffer: Vec::new(),
            icon,
            color,
        }
    }

    /// Update agent status and visual indicators
    pub fn update_status(&mut self, status: SubagentStatus) {
        let (icon, color) = match &status {
            SubagentStatus::Pending => ("⏳", Color::Gray),
            SubagentStatus::Running => ("▶", Color::Blue),
            SubagentStatus::Completed => ("✓", Color::Green),
            SubagentStatus::Failed(_) => ("✗", Color::Red),
            SubagentStatus::Cancelled => ("⊘", Color::Yellow),
        };
        self.icon = icon;
        self.color = color;
        self.cancellable = matches!(status, SubagentStatus::Running | SubagentStatus::Pending);
        self.execution.status = status;
    }

    /// Get elapsed time since agent started
    pub fn elapsed(&self) -> Duration {
        self.started_at
            .elapsed()
            .unwrap_or_else(|_| Duration::from_secs(0))
    }

    /// Format elapsed time for display
    pub fn elapsed_string(&self) -> String {
        let elapsed = self.elapsed();
        let seconds = elapsed.as_secs();
        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m {}s", seconds / 60, seconds % 60)
        } else {
            format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
        }
    }
}

impl Default for AgentPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentPanel {
    /// Create a new agent panel
    pub fn new() -> Self {
        Self {
            agents: Arc::new(Mutex::new(HashMap::new())),
            selected: 0,
            visible: false,
            area: Rect::default(),
            scroll_offset: 0,
            max_visible: 10,
            animation_frame: 0,
            last_update: Instant::now(),
        }
    }

    /// Toggle panel visibility
    pub const fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.selected = 0;
            self.scroll_offset = 0;
        }
    }

    /// Show the panel
    pub const fn show(&mut self) {
        self.visible = true;
        self.selected = 0;
        self.scroll_offset = 0;
    }

    /// Hide the panel
    pub const fn hide(&mut self) {
        self.visible = false;
    }

    /// Check if panel is visible
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    /// Add a new agent to track
    pub fn add_agent(&self, execution: SubagentExecution) {
        let mut agents = self.agents.lock().unwrap();
        agents.insert(execution.id, AgentInfo::new(execution));
    }

    /// Update agent progress
    pub fn update_progress(&self, agent_id: Uuid, progress: f32, message: String) {
        let mut agents = self.agents.lock().unwrap();
        if let Some(agent) = agents.get_mut(&agent_id) {
            agent.progress = progress.clamp(0.0, 1.0);
            agent.status_message = message;
            agent.last_progress_update = Instant::now();
        }
    }

    /// Update agent status
    pub fn update_status(&self, agent_id: Uuid, status: SubagentStatus) {
        let mut agents = self.agents.lock().unwrap();
        if let Some(agent) = agents.get_mut(&agent_id) {
            agent.update_status(status);
        }
    }

    /// Add output to agent's buffer
    pub fn add_output(&self, agent_id: Uuid, output: String) {
        let mut agents = self.agents.lock().unwrap();
        if let Some(agent) = agents.get_mut(&agent_id) {
            agent.output_buffer.push(output);
            // Keep only last 100 lines
            if agent.output_buffer.len() > 100 {
                agent
                    .output_buffer
                    .drain(0..agent.output_buffer.len() - 100);
            }
        }
    }

    /// Complete an agent
    pub fn complete_agent(&self, agent_id: Uuid, message: String) {
        self.update_status(agent_id, SubagentStatus::Completed);
        self.update_progress(agent_id, 1.0, message);
    }

    /// Fail an agent
    pub fn fail_agent(&self, agent_id: Uuid, error: String) {
        self.update_status(agent_id, SubagentStatus::Failed(error.clone()));
        self.update_progress(agent_id, 0.0, format!("Failed: {}", error));
    }

    /// Cancel selected agent
    pub fn cancel_selected(&self) {
        let id = {
            let agents = self.agents.lock().unwrap();
            let sorted_agents = self.get_sorted_agents(&agents);
            sorted_agents.get(self.selected).map(|(id, _)| *id)
        };
        if let Some(id) = id {
            self.update_status(id, SubagentStatus::Cancelled);
            self.update_progress(id, 0.0, "Cancelled by user".to_string());
        }
    }

    /// Kill selected agent (forceful termination)
    pub fn kill_selected(&self) {
        self.cancel_selected(); // For now, same as cancel
    }

    /// Navigate selection up
    pub const fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.ensure_visible();
        }
    }

    /// Navigate selection down
    pub fn select_next(&mut self) {
        let agents = self.agents.lock().unwrap();
        let count = agents.len();
        drop(agents);

        if self.selected < count.saturating_sub(1) {
            self.selected += 1;
            self.ensure_visible();
        }
    }

    /// Ensure selected item is visible
    const fn ensure_visible(&mut self) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + self.max_visible {
            self.scroll_offset = self.selected.saturating_sub(self.max_visible - 1);
        }
    }

    /// Get sorted list of agents (running first, then by start time)
    fn get_sorted_agents<'a>(
        &self,
        agents: &'a HashMap<Uuid, AgentInfo>,
    ) -> Vec<(Uuid, &'a AgentInfo)> {
        let mut sorted: Vec<_> = agents.iter().map(|(id, info)| (*id, info)).collect();
        sorted.sort_by(|a, b| {
            // Running agents first
            let a_running = matches!(a.1.execution.status, SubagentStatus::Running);
            let b_running = matches!(b.1.execution.status, SubagentStatus::Running);
            if a_running != b_running {
                return b_running.cmp(&a_running);
            }
            // Then by start time (newer first)
            b.1.started_at.cmp(&a.1.started_at)
        });
        sorted
    }

    /// Update animation frame
    pub fn tick(&mut self) {
        if self.last_update.elapsed() > Duration::from_millis(100) {
            self.animation_frame = (self.animation_frame + 1) % 8;
            self.last_update = Instant::now();
        }
    }

    /// Get progress bar animation character
    const fn get_progress_char(&self, position: usize) -> &'static str {
        const CHARS: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
        CHARS[(position + self.animation_frame) % 8]
    }
}

impl Widget for &AgentPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Clear the area first
        Clear.render(area, buf);

        // Create main block with border
        let block = Block::default()
            .title(" Active Agents ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        block.render(area, buf);

        // Split into header, list, and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Min(3),    // Agent list
                Constraint::Length(2), // Footer/controls
            ])
            .split(inner);

        // Render header with counts
        self.render_header(chunks[0], buf);

        // Render agent list with progress bars
        self.render_agent_list(chunks[1], buf);

        // Render footer with controls
        self.render_footer(chunks[2], buf);
    }
}

impl AgentPanel {
    /// Render the header with agent counts
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let agents = self.agents.lock().unwrap();
        let running = agents
            .values()
            .filter(|a| matches!(a.execution.status, SubagentStatus::Running))
            .count();
        let completed = agents
            .values()
            .filter(|a| matches!(a.execution.status, SubagentStatus::Completed))
            .count();
        let failed = agents
            .values()
            .filter(|a| matches!(a.execution.status, SubagentStatus::Failed(_)))
            .count();

        let mut spans = vec![];

        if running > 0 {
            spans.push(Span::styled(
                format!("{} running", running),
                Style::default().fg(Color::Blue),
            ));
            spans.push(Span::raw("  "));
        }
        if completed > 0 {
            spans.push(Span::styled(
                format!("{} complete", completed),
                Style::default().fg(Color::Green),
            ));
            spans.push(Span::raw("  "));
        }
        if failed > 0 {
            spans.push(Span::styled(
                format!("{} failed", failed),
                Style::default().fg(Color::Red),
            ));
        }

        if spans.is_empty() {
            spans.push(Span::styled("No agents", Style::default().fg(Color::Gray)));
        }

        Paragraph::new(Line::from(spans))
            .alignment(Alignment::Center)
            .render(area, buf);
    }

    /// Render the agent list with progress bars
    fn render_agent_list(&self, area: Rect, buf: &mut Buffer) {
        let agents = self.agents.lock().unwrap();
        let sorted_agents = self.get_sorted_agents(&agents);

        if sorted_agents.is_empty() {
            let empty_msg = Paragraph::new("No active agents")
                .style(Style::default().fg(Color::Gray))
                .alignment(Alignment::Center);
            empty_msg.render(area, buf);
            return;
        }

        // Calculate visible range
        let visible_start = self.scroll_offset;
        let visible_end = (visible_start + area.height as usize).min(sorted_agents.len());
        let visible_agents = &sorted_agents[visible_start..visible_end];

        // Render each agent with progress bar
        for (idx, (_id, agent)) in visible_agents.iter().enumerate() {
            let y = area.y + idx as u16;
            if y >= area.bottom() {
                break;
            }

            let agent_area = Rect {
                x: area.x,
                y,
                width: area.width,
                height: 1,
            };

            // Check if this agent is selected
            let is_selected = visible_start + idx == self.selected;
            let style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            // Render selection indicator
            if is_selected {
                buf.set_string(agent_area.x, agent_area.y, "▶ ", style.fg(Color::Yellow));
            } else {
                buf.set_string(agent_area.x, agent_area.y, "  ", style);
            }

            // Calculate layout for agent info and progress bar
            let info_area = Rect {
                x: agent_area.x + 2,
                y: agent_area.y,
                width: agent_area.width.saturating_sub(2),
                height: 1,
            };

            // Format agent line
            let agent_name = format!("@{}", agent.execution.agent_name);
            let elapsed = agent.elapsed_string();

            // Render based on status
            match agent.execution.status {
                SubagentStatus::Running => {
                    // Calculate progress bar width
                    let name_width = agent_name.len() as u16 + 2;
                    let elapsed_width = elapsed.len() as u16 + 3;
                    let progress_width = info_area
                        .width
                        .saturating_sub(name_width)
                        .saturating_sub(elapsed_width)
                        .saturating_sub(8); // Space for percentage

                    // Render agent name
                    buf.set_string(
                        info_area.x,
                        info_area.y,
                        &agent_name,
                        style.fg(Color::White).add_modifier(Modifier::BOLD),
                    );

                    // Render progress bar
                    if progress_width > 10 {
                        let bar_start = info_area.x + name_width + 1;
                        self.render_progress_bar(
                            Rect {
                                x: bar_start,
                                y: info_area.y,
                                width: progress_width,
                                height: 1,
                            },
                            buf,
                            agent.progress,
                            style,
                        );

                        // Render percentage
                        let percentage = format!("{:3.0}%", agent.progress * 100.0);
                        buf.set_string(
                            bar_start + progress_width + 1,
                            info_area.y,
                            &percentage,
                            style.fg(Color::Cyan),
                        );
                    }

                    // Render elapsed time
                    buf.set_string(
                        info_area.right().saturating_sub(elapsed.len() as u16),
                        info_area.y,
                        &elapsed,
                        style.fg(Color::Gray),
                    );
                }
                _ => {
                    // For non-running agents, show simple status
                    let status_text = format!(
                        "{} {} {} - {}",
                        agent.icon,
                        agent_name,
                        match agent.execution.status {
                            SubagentStatus::Completed => "Complete",
                            SubagentStatus::Failed(_) => "Failed",
                            SubagentStatus::Cancelled => "Cancelled",
                            SubagentStatus::Pending => "Pending",
                            _ => "Unknown",
                        },
                        elapsed
                    );

                    buf.set_string(
                        info_area.x,
                        info_area.y,
                        &status_text,
                        style.fg(agent.color),
                    );
                }
            }
        }

        // Render scroll indicator if needed
        if sorted_agents.len() > area.height as usize {
            let scroll_percent =
                (self.scroll_offset as f32 / sorted_agents.len() as f32 * 100.0) as u16;
            let scroll_text = format!("▼ {}%", scroll_percent);
            buf.set_string(
                area.right().saturating_sub(scroll_text.len() as u16 + 1),
                area.bottom().saturating_sub(1),
                &scroll_text,
                Style::default().fg(Color::DarkGray),
            );
        }
    }

    /// Render a progress bar
    fn render_progress_bar(&self, area: Rect, buf: &mut Buffer, progress: f32, base_style: Style) {
        let filled = (area.width as f32 * progress) as u16;

        buf.set_string(area.x, area.y, "[", base_style.fg(Color::DarkGray));

        for i in 0..area.width.saturating_sub(2) {
            let x = area.x + i + 1;
            if i < filled {
                buf.set_string(x, area.y, "█", base_style.fg(Color::Green));
            } else if i == filled && progress < 1.0 {
                // Animated progress indicator
                let anim_char = self.get_progress_char(i as usize);
                buf.set_string(x, area.y, anim_char, base_style.fg(Color::Yellow));
            } else {
                buf.set_string(x, area.y, "░", base_style.fg(Color::DarkGray));
            }
        }

        buf.set_string(
            area.right().saturating_sub(1),
            area.y,
            "]",
            base_style.fg(Color::DarkGray),
        );
    }

    /// Render the footer with control hints
    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let controls = vec![
            Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
            Span::raw(" View  "),
            Span::styled("[n]", Style::default().fg(Color::Yellow)),
            Span::raw(" New  "),
            Span::styled("[k]", Style::default().fg(Color::Yellow)),
            Span::raw(" Kill  "),
            Span::styled("[Esc]", Style::default().fg(Color::Yellow)),
            Span::raw(" Close"),
        ];

        Paragraph::new(Line::from(controls))
            .alignment(Alignment::Center)
            .render(area, buf);
    }
}

/// Handle keyboard input for the agent panel
impl AgentPanel {
    /// Process a key event
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> bool {
        use crossterm::event::KeyCode;

        if !self.visible {
            return false;
        }

        match key {
            KeyCode::Esc => {
                self.hide();
                true
            }
            KeyCode::Up => {
                self.select_previous();
                true
            }
            KeyCode::Down => {
                self.select_next();
                true
            }
            KeyCode::Enter => {
                // View agent details (could open a detail view)
                true
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                // Spawn new agent (emit event)
                true
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                // Kill selected agent
                self.kill_selected();
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_panel_creation() {
        let panel = AgentPanel::new();
        assert!(!panel.is_visible());
        assert_eq!(panel.selected, 0);
    }

    #[test]
    fn test_panel_visibility() {
        let mut panel = AgentPanel::new();
        assert!(!panel.is_visible());

        panel.show();
        assert!(panel.is_visible());

        panel.hide();
        assert!(!panel.is_visible());

        panel.toggle();
        assert!(panel.is_visible());

        panel.toggle();
        assert!(!panel.is_visible());
    }

    #[test]
    fn test_agent_management() {
        let panel = AgentPanel::new();
        let execution = SubagentExecution::new("test-agent".to_string());
        let agent_id = execution.id;

        panel.add_agent(execution);

        // Update progress
        panel.update_progress(agent_id, 0.5, "Processing...".to_string());

        // Update status
        panel.update_status(agent_id, SubagentStatus::Running);

        // Add output
        panel.add_output(agent_id, "Test output line 1".to_string());
        panel.add_output(agent_id, "Test output line 2".to_string());

        // Complete agent
        panel.complete_agent(agent_id, "Success!".to_string());

        let agents = panel.agents.lock().unwrap();
        let agent = agents.get(&agent_id).unwrap();
        assert_eq!(agent.progress, 1.0);
        assert!(matches!(agent.execution.status, SubagentStatus::Completed));
    }

    #[test]
    fn test_navigation() {
        let mut panel = AgentPanel::new();

        // Add multiple agents
        for i in 0..5 {
            let execution = SubagentExecution::new(format!("agent-{}", i));
            panel.add_agent(execution);
        }

        assert_eq!(panel.selected, 0);

        panel.select_next();
        assert_eq!(panel.selected, 1);

        panel.select_next();
        assert_eq!(panel.selected, 2);

        panel.select_previous();
        assert_eq!(panel.selected, 1);

        panel.select_previous();
        assert_eq!(panel.selected, 0);

        // Shouldn't go below 0
        panel.select_previous();
        assert_eq!(panel.selected, 0);
    }

    #[test]
    fn test_elapsed_time_formatting() {
        let execution = SubagentExecution::new("test".to_string());
        let mut agent = AgentInfo::new(execution);

        // Mock different elapsed times
        agent.started_at = SystemTime::now() - Duration::from_secs(30);
        assert_eq!(agent.elapsed_string(), "30s");

        agent.started_at = SystemTime::now() - Duration::from_secs(90);
        assert_eq!(agent.elapsed_string(), "1m 30s");

        agent.started_at = SystemTime::now() - Duration::from_secs(3700);
        assert_eq!(agent.elapsed_string(), "1h 1m");
    }
}
