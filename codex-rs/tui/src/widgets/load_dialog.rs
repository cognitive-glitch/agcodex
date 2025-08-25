//! Load session dialog widget for AGCodex TUI

use agcodex_persistence::types::OperatingMode;
use agcodex_persistence::types::SessionMetadata;
use chrono::DateTime;
use chrono::Local;
use ratatui::buffer::Buffer;
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
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;
use uuid::Uuid;

use crate::bottom_pane::popup_consts::MAX_POPUP_ROWS;
use crate::bottom_pane::scroll_state::ScrollState;
use crate::bottom_pane::selection_popup_common::GenericDisplayRow;
use crate::bottom_pane::selection_popup_common::render_rows;

/// Session item for display in the load dialog
#[derive(Debug, Clone)]
pub struct SessionDisplayItem {
    pub metadata: SessionMetadata,
    pub display_name: String,
    pub formatted_date: String,
    pub mode_indicator: String,
    pub preview_text: String,
}

impl SessionDisplayItem {
    fn new(metadata: SessionMetadata) -> Self {
        let local_time: DateTime<Local> = metadata.updated_at.into();
        let formatted_date = if local_time.date_naive() == Local::now().date_naive() {
            format!("Today {}", local_time.format("%H:%M"))
        } else {
            local_time.format("%b %d %H:%M").to_string()
        };

        let mode_indicator = match metadata.current_mode {
            OperatingMode::Plan => "📋",
            OperatingMode::Build => "🔨",
            OperatingMode::Review => "🔍",
        };

        let preview_text = format!(
            "{} messages • {} • Model: {}",
            metadata.message_count,
            format_file_size(metadata.file_size),
            metadata.model
        );

        let display_name = if metadata.title.is_empty() {
            format!("Session {}", &metadata.id.to_string()[0..8])
        } else {
            metadata.title.clone()
        };

        Self {
            metadata,
            display_name,
            formatted_date,
            mode_indicator: mode_indicator.to_string(),
            preview_text,
        }
    }
}

/// Visual state for the load session dialog
pub struct LoadDialog {
    /// Query string for filtering sessions
    search_query: String,
    /// All available sessions
    all_sessions: Vec<SessionDisplayItem>,
    /// Filtered sessions based on search query
    filtered_sessions: Vec<SessionDisplayItem>,
    /// Shared selection/scroll state
    state: ScrollState,
    /// Whether we're currently loading session data
    loading: bool,
    /// Error message if loading failed
    error_message: Option<String>,
}

impl Default for LoadDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadDialog {
    pub const fn new() -> Self {
        Self {
            search_query: String::new(),
            all_sessions: Vec::new(),
            filtered_sessions: Vec::new(),
            state: ScrollState::new(),
            loading: true,
            error_message: None,
        }
    }

    /// Update the search query and filter sessions
    pub fn set_search_query(&mut self, query: &str) {
        self.search_query = query.to_string();
        self.filter_sessions();
    }

    /// Set the list of sessions from SessionManager
    pub fn set_sessions(&mut self, sessions: Vec<SessionMetadata>) {
        self.all_sessions = sessions.into_iter().map(SessionDisplayItem::new).collect();

        // Sort by most recently updated first
        self.all_sessions
            .sort_by(|a, b| b.metadata.updated_at.cmp(&a.metadata.updated_at));

        self.loading = false;
        self.filter_sessions();
    }

    /// Set error state
    pub fn set_error(&mut self, error: String) {
        self.loading = false;
        self.error_message = Some(error);
    }

    /// Filter sessions based on current search query
    fn filter_sessions(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_sessions = self.all_sessions.clone();
        } else {
            let query_lower = self.search_query.to_lowercase();
            self.filtered_sessions = self
                .all_sessions
                .iter()
                .filter(|session| {
                    session.display_name.to_lowercase().contains(&query_lower)
                        || session.metadata.model.to_lowercase().contains(&query_lower)
                        || session.formatted_date.to_lowercase().contains(&query_lower)
                })
                .cloned()
                .collect();
        }

        // Reset selection to first item
        self.state.reset();
        if !self.filtered_sessions.is_empty() {
            self.state.selected_idx = Some(0);
        }
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        let len = self.filtered_sessions.len();
        if len > 0 {
            self.state.move_up_wrap(len);
            self.state.ensure_visible(len, len.min(MAX_POPUP_ROWS));
        }
    }

    /// Move selection down
    pub fn move_down(&mut self) {
        let len = self.filtered_sessions.len();
        if len > 0 {
            self.state.move_down_wrap(len);
            self.state.ensure_visible(len, len.min(MAX_POPUP_ROWS));
        }
    }

    /// Get the currently selected session
    pub fn selected_session(&self) -> Option<&SessionDisplayItem> {
        self.state
            .selected_idx
            .and_then(|idx| self.filtered_sessions.get(idx))
    }

    /// Get the selected session UUID
    pub fn selected_session_id(&self) -> Option<Uuid> {
        self.selected_session().map(|s| s.metadata.id)
    }

    /// Calculate required height for the popup
    pub fn calculate_required_height(&self) -> u16 {
        if self.loading || self.error_message.is_some() {
            return 3; // Minimum height for loading/error messages
        }

        let content_rows = if self.filtered_sessions.is_empty() {
            1 // "No sessions found"
        } else {
            self.filtered_sessions.len().min(MAX_POPUP_ROWS)
        };

        // Add 4 for borders, search bar, and preview
        (content_rows + 4).max(8) as u16
    }

    /// Get the current search query
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Whether the dialog is still loading
    pub const fn is_loading(&self) -> bool {
        self.loading
    }
}

impl WidgetRef for &LoadDialog {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        // Clear the area
        Clear.render(area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Load Session ")
            .title_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let inner = block.inner(area);
        block.render(area, buf);

        if area.width < 50 || area.height < 6 {
            return; // Too small to render properly
        }

        // Split into search bar, session list, and preview
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Search bar
                Constraint::Min(3),    // Session list
                Constraint::Length(3), // Preview pane
            ])
            .split(inner);

        // Render search bar
        let search_text = if self.search_query.is_empty() {
            Line::from(vec![
                Span::styled("Search: ", Style::default().fg(Color::Gray)),
                Span::styled(
                    "(type to filter sessions)",
                    Style::default().fg(Color::DarkGray),
                ),
            ])
        } else {
            Line::from(vec![
                Span::styled("Search: ", Style::default().fg(Color::Gray)),
                Span::styled(&self.search_query, Style::default().fg(Color::White)),
            ])
        };

        Paragraph::new(search_text).render(layout[0], buf);

        // Handle loading/error states
        if self.loading {
            let loading_text = Paragraph::new("Loading sessions...")
                .style(Style::default().fg(Color::Yellow))
                .wrap(Wrap { trim: true });
            loading_text.render(layout[1], buf);
            return;
        }

        if let Some(error) = &self.error_message {
            let error_text = Paragraph::new(format!("Error: {}", error))
                .style(Style::default().fg(Color::Red))
                .wrap(Wrap { trim: true });
            error_text.render(layout[1], buf);
            return;
        }

        // Render session list
        let rows_all: Vec<GenericDisplayRow> = if self.filtered_sessions.is_empty() {
            vec![GenericDisplayRow {
                name: "No sessions found".to_string(),
                match_indices: None,
                is_current: false,
                description: Some("Try adjusting your search".to_string()),
            }]
        } else {
            self.filtered_sessions
                .iter()
                .map(|session| {
                    let display_text = format!(
                        "{} {} - {}",
                        session.mode_indicator, session.display_name, session.formatted_date
                    );

                    GenericDisplayRow {
                        name: display_text,
                        match_indices: None, // TODO: Add fuzzy matching indices
                        is_current: false,
                        description: Some(session.preview_text.clone()),
                    }
                })
                .collect()
        };

        render_rows(
            layout[1],
            buf,
            &rows_all,
            &self.state,
            MAX_POPUP_ROWS,
            false,
        );

        // Render preview pane
        if let Some(selected) = self.selected_session() {
            let preview_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[2]);

            // Left side: Basic info
            let info_lines = vec![
                Line::from(vec![
                    Span::styled("Mode: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format!("{:?}", selected.metadata.current_mode),
                        mode_color(selected.metadata.current_mode),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Messages: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        selected.metadata.message_count.to_string(),
                        Style::default().fg(Color::White),
                    ),
                ]),
            ];

            let info_block = Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Info ");

            Paragraph::new(info_lines)
                .block(info_block)
                .render(preview_layout[0], buf);

            // Right side: Additional details
            let details_lines = vec![
                Line::from(vec![
                    Span::styled("Model: ", Style::default().fg(Color::Gray)),
                    Span::styled(&selected.metadata.model, Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Size: ", Style::default().fg(Color::Gray)),
                    Span::styled(
                        format_file_size(selected.metadata.file_size),
                        Style::default().fg(Color::White),
                    ),
                ]),
            ];

            let details_block = Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Details ");

            Paragraph::new(details_lines)
                .block(details_block)
                .render(preview_layout[1], buf);
        } else {
            // No selection - show help text
            let help_text = Paragraph::new("Press ↑/↓ to select, Enter to load, Esc to cancel")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .borders(Borders::TOP)
                        .border_style(Style::default().fg(Color::DarkGray)),
                );

            help_text.render(layout[2], buf);
        }
    }
}

/// Get color for operating mode
fn mode_color(mode: OperatingMode) -> Style {
    match mode {
        OperatingMode::Plan => Style::default().fg(Color::Blue),
        OperatingMode::Build => Style::default().fg(Color::Green),
        OperatingMode::Review => Style::default().fg(Color::Yellow),
    }
}

/// Format file size in human-readable format
fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_session(title: &str, mode: OperatingMode, messages: usize) -> SessionMetadata {
        SessionMetadata {
            id: Uuid::new_v4(),
            title: title.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed: Utc::now(),
            message_count: messages,
            turn_count: messages / 2,
            current_mode: mode,
            model: "gpt-4".to_string(),
            tags: vec![],
            is_favorite: false,
            file_size: 1024 * messages as u64,
            compression_ratio: 0.7,
            format_version: 1,
            checkpoints: vec![],
        }
    }

    #[test]
    fn test_load_dialog_creation() {
        let dialog = LoadDialog::new();
        assert!(dialog.is_loading());
        assert_eq!(dialog.search_query(), "");
        assert!(dialog.selected_session().is_none());
    }

    #[test]
    fn test_session_filtering() {
        let mut dialog = LoadDialog::new();
        let sessions = vec![
            create_test_session("Project Alpha", OperatingMode::Build, 10),
            create_test_session("Debug Session", OperatingMode::Plan, 5),
            create_test_session("Code Review", OperatingMode::Review, 15),
        ];

        dialog.set_sessions(sessions);
        assert!(!dialog.is_loading());
        assert_eq!(dialog.filtered_sessions.len(), 3);

        dialog.set_search_query("alpha");
        assert_eq!(dialog.filtered_sessions.len(), 1);
        assert_eq!(dialog.filtered_sessions[0].display_name, "Project Alpha");

        dialog.set_search_query("debug");
        assert_eq!(dialog.filtered_sessions.len(), 1);
        assert_eq!(dialog.filtered_sessions[0].display_name, "Debug Session");

        dialog.set_search_query("");
        assert_eq!(dialog.filtered_sessions.len(), 3);
    }

    #[test]
    fn test_file_size_formatting() {
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1048576), "1.0 MB");
        assert_eq!(format_file_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_session_navigation() {
        let mut dialog = LoadDialog::new();
        let sessions = vec![
            create_test_session("Session 1", OperatingMode::Build, 10),
            create_test_session("Session 2", OperatingMode::Plan, 5),
            create_test_session("Session 3", OperatingMode::Review, 15),
        ];

        dialog.set_sessions(sessions);

        // Should start with first item selected
        assert_eq!(dialog.state.selected_idx, Some(0));

        dialog.move_down();
        assert_eq!(dialog.state.selected_idx, Some(1));

        dialog.move_down();
        assert_eq!(dialog.state.selected_idx, Some(2));

        // Should wrap to beginning
        dialog.move_down();
        assert_eq!(dialog.state.selected_idx, Some(0));

        dialog.move_up();
        assert_eq!(dialog.state.selected_idx, Some(2));
    }
}
