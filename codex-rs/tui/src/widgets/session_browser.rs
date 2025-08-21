//! Session Browser widget for browsing and managing AGCodex sessions
//!
//! Provides a comprehensive interface for:
//! - Timeline view of all sessions
//! - Session metadata display
//! - Branch visualization
//! - Search across sessions
//! - Export and management operations

use agcodex_core::modes::OperatingMode;
use agcodex_persistence::types::{SessionIndex, SessionMetadata};
use chrono::{DateTime, Utc};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Table, Row, Cell, Widget, WidgetRef,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::bottom_pane::scroll_state::ScrollState;

/// Different view modes for the session browser
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// Tree view showing session hierarchy and branches
    Tree,
    /// Simple list view sorted by criteria
    List,
    /// Timeline view showing sessions chronologically
    Timeline,
}

/// Sort criteria for session list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    /// Sort by last accessed time (most recent first)
    LastAccessed,
    /// Sort by creation time (newest first)
    Created,
    /// Sort by name alphabetically
    Name,
    /// Sort by message count (highest first)
    MessageCount,
    /// Sort by file size (largest first)
    Size,
}

/// Panel focus state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    /// Left panel: session list/tree
    SessionList,
    /// Right panel: preview/details
    Preview,
    /// Bottom panel: actions
    Actions,
    /// Search input
    Search,
}

/// Session browser widget state
#[derive(Debug, Clone)]
pub struct SessionBrowser {
    /// Current view mode
    view_mode: ViewMode,
    /// Current sort criteria
    sort_by: SortBy,
    /// Which panel is currently focused
    focused_panel: FocusedPanel,
    /// Session index with all session metadata
    session_index: SessionIndex,
    /// Filtered sessions based on search
    filtered_sessions: Vec<Uuid>,
    /// Search query
    search_query: String,
    /// Selection state for session list
    session_scroll_state: ScrollState,
    /// Currently selected session for preview
    selected_session: Option<Uuid>,
    /// Actions available (Open, Delete, Export, etc.)
    actions: Vec<SessionAction>,
    /// Selected action index
    action_scroll_state: ScrollState,
    /// Whether to show confirmation dialog
    show_confirmation: bool,
    /// Confirmation message
    confirmation_message: String,
    /// Show export options
    show_export_options: bool,
    /// Show favorites only
    favorites_only: bool,
    /// Show advanced filters
    show_advanced_filters: bool,
    /// Date filter range
    date_filter: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Mode filter
    mode_filter: Option<OperatingMode>,
}

/// Available actions for sessions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionAction {
    Open,
    Delete,
    Duplicate,
    Export,
    Rename,
    AddToFavorites,
    RemoveFromFavorites,
    Archive,
    AddTags,
    RemoveTags,
    ViewBranches,
    CompareWith,
    RestoreFromCheckpoint,
}

impl SessionAction {
    fn display_name(&self) -> &'static str {
        match self {
            SessionAction::Open => "Open Session",
            SessionAction::Delete => "Delete Session",
            SessionAction::Duplicate => "Duplicate Session",
            SessionAction::Export => "Export as Markdown",
            SessionAction::Rename => "Rename Session",
            SessionAction::AddToFavorites => "Add to Favorites",
            SessionAction::RemoveFromFavorites => "Remove from Favorites",
            SessionAction::Archive => "Archive Session",
            SessionAction::AddTags => "Add Tags",
            SessionAction::RemoveTags => "Remove Tags",
            SessionAction::ViewBranches => "View Branches",
            SessionAction::CompareWith => "Compare with Another",
            SessionAction::RestoreFromCheckpoint => "Restore from Checkpoint",
        }
    }

    fn shortcut(&self) -> Option<&'static str> {
        match self {
            SessionAction::Open => Some("Enter"),
            SessionAction::Delete => Some("Del"),
            SessionAction::Export => Some("E"),
            SessionAction::Rename => Some("F2"),
            SessionAction::AddToFavorites => Some("F"),
            _ => None,
        }
    }
}

impl SessionBrowser {
    /// Create a new session browser
    pub fn new(session_index: SessionIndex) -> Self {
        let filtered_sessions = session_index.recent_sessions.clone();
        let actions = vec![
            SessionAction::Open,
            SessionAction::Delete,
            SessionAction::Export,
            SessionAction::Rename,
            SessionAction::AddToFavorites,
            SessionAction::Duplicate,
            SessionAction::Archive,
        ];

        let mut browser = Self {
            view_mode: ViewMode::List,
            sort_by: SortBy::LastAccessed,
            focused_panel: FocusedPanel::SessionList,
            session_index,
            filtered_sessions,
            search_query: String::new(),
            session_scroll_state: ScrollState::new(),
            selected_session: None,
            actions,
            action_scroll_state: ScrollState::new(),
            show_confirmation: false,
            confirmation_message: String::new(),
            show_export_options: false,
            favorites_only: false,
            show_advanced_filters: false,
            date_filter: None,
            mode_filter: None,
        };

        browser.refresh_filtered_sessions();
        browser.update_selection();
        browser
    }

    /// Update search query and refresh filtered sessions
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.refresh_filtered_sessions();
        self.session_scroll_state.reset();
        self.update_selection();
    }

    /// Toggle view mode
    pub fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            ViewMode::Tree => ViewMode::List,
            ViewMode::List => ViewMode::Timeline,
            ViewMode::Timeline => ViewMode::Tree,
        };
        self.refresh_filtered_sessions();
    }

    /// Cycle through sort options
    pub fn cycle_sort(&mut self) {
        self.sort_by = match self.sort_by {
            SortBy::LastAccessed => SortBy::Created,
            SortBy::Created => SortBy::Name,
            SortBy::Name => SortBy::MessageCount,
            SortBy::MessageCount => SortBy::Size,
            SortBy::Size => SortBy::LastAccessed,
        };
        self.refresh_filtered_sessions();
    }

    /// Move focus to next panel
    pub fn focus_next_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::SessionList => FocusedPanel::Preview,
            FocusedPanel::Preview => FocusedPanel::Actions,
            FocusedPanel::Actions => FocusedPanel::Search,
            FocusedPanel::Search => FocusedPanel::SessionList,
        };
    }

    /// Move focus to previous panel
    pub fn focus_previous_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::SessionList => FocusedPanel::Search,
            FocusedPanel::Search => FocusedPanel::Actions,
            FocusedPanel::Actions => FocusedPanel::Preview,
            FocusedPanel::Preview => FocusedPanel::SessionList,
        };
    }

    /// Move selection up in current focused panel
    pub fn move_up(&mut self) {
        match self.focused_panel {
            FocusedPanel::SessionList => {
                let len = self.filtered_sessions.len();
                self.session_scroll_state.move_up_wrap(len);
                self.update_selection();
            }
            FocusedPanel::Actions => {
                let len = self.actions.len();
                self.action_scroll_state.move_up_wrap(len);
            }
            _ => {}
        }
    }

    /// Move selection down in current focused panel
    pub fn move_down(&mut self) {
        match self.focused_panel {
            FocusedPanel::SessionList => {
                let len = self.filtered_sessions.len();
                self.session_scroll_state.move_down_wrap(len);
                self.update_selection();
            }
            FocusedPanel::Actions => {
                let len = self.actions.len();
                self.action_scroll_state.move_down_wrap(len);
            }
            _ => {}
        }
    }

    /// Get currently selected session ID
    pub fn selected_session_id(&self) -> Option<Uuid> {
        self.selected_session
    }

    /// Get currently selected action
    pub fn selected_action(&self) -> Option<&SessionAction> {
        self.action_scroll_state
            .selected_idx
            .and_then(|idx| self.actions.get(idx))
    }

    /// Toggle favorites filter
    pub fn toggle_favorites_only(&mut self) {
        self.favorites_only = !self.favorites_only;
        self.refresh_filtered_sessions();
        self.session_scroll_state.reset();
        self.update_selection();
    }

    /// Show confirmation dialog
    pub fn show_confirmation(&mut self, message: String) {
        self.confirmation_message = message;
        self.show_confirmation = true;
    }

    /// Hide confirmation dialog
    pub fn hide_confirmation(&mut self) {
        self.show_confirmation = false;
        self.confirmation_message.clear();
    }

    /// Toggle export options
    pub fn toggle_export_options(&mut self) {
        self.show_export_options = !self.show_export_options;
    }

    /// Get session metadata for selected session
    pub fn selected_session_metadata(&self) -> Option<&SessionMetadata> {
        self.selected_session
            .and_then(|id| self.session_index.sessions.get(&id))
    }

    /// Update the session index (e.g., after changes)
    pub fn update_session_index(&mut self, session_index: SessionIndex) {
        self.session_index = session_index;
        self.refresh_filtered_sessions();
        self.update_selection();
    }

    /// Refresh filtered sessions based on current criteria
    fn refresh_filtered_sessions(&mut self) {
        let mut sessions: Vec<Uuid> = if self.search_query.is_empty() {
            if self.favorites_only {
                self.session_index.favorite_sessions.clone()
            } else {
                self.session_index.sessions.keys().copied().collect()
            }
        } else {
            // Search in session titles and tags
            self.session_index
                .search(&self.search_query)
                .into_iter()
                .map(|metadata| metadata.id)
                .collect()
        };

        // Apply filters
        if let Some(mode) = self.mode_filter {
            sessions.retain(|&id| {
                self.session_index
                    .sessions
                    .get(&id)
                    .map_or(false, |metadata| metadata.current_mode == mode)
            });
        }

        if let Some((start, end)) = self.date_filter {
            sessions.retain(|&id| {
                self.session_index
                    .sessions
                    .get(&id)
                    .map_or(false, |metadata| {
                        metadata.last_accessed >= start && metadata.last_accessed <= end
                    })
            });
        }

        // Sort sessions
        sessions.sort_by(|&a, &b| {
            let meta_a = self.session_index.sessions.get(&a);
            let meta_b = self.session_index.sessions.get(&b);

            match (meta_a, meta_b) {
                (Some(a), Some(b)) => match self.sort_by {
                    SortBy::LastAccessed => b.last_accessed.cmp(&a.last_accessed),
                    SortBy::Created => b.created_at.cmp(&a.created_at),
                    SortBy::Name => a.title.cmp(&b.title),
                    SortBy::MessageCount => b.message_count.cmp(&a.message_count),
                    SortBy::Size => b.file_size.cmp(&a.file_size),
                },
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        self.filtered_sessions = sessions;
    }

    /// Update currently selected session
    fn update_selection(&mut self) {
        self.selected_session = self
            .session_scroll_state
            .selected_idx
            .and_then(|idx| self.filtered_sessions.get(idx).copied());

        // Update actions based on selected session
        if let Some(session_id) = self.selected_session {
            if let Some(metadata) = self.session_index.sessions.get(&session_id) {
                self.actions = vec![
                    SessionAction::Open,
                    SessionAction::Export,
                    SessionAction::Duplicate,
                    SessionAction::Rename,
                    if metadata.is_favorite {
                        SessionAction::RemoveFromFavorites
                    } else {
                        SessionAction::AddToFavorites
                    },
                    SessionAction::Delete,
                    SessionAction::Archive,
                ];

                if !metadata.checkpoints.is_empty() {
                    self.actions.push(SessionAction::RestoreFromCheckpoint);
                }
            }
        }

        // Clamp action selection
        let action_len = self.actions.len();
        self.action_scroll_state.clamp_selection(action_len);
    }

    /// Format duration for display
    fn format_duration(start: &DateTime<Utc>, end: &DateTime<Utc>) -> String {
        let duration = *end - *start;
        let total_seconds = duration.num_seconds().max(0);

        if total_seconds < 60 {
            format!("{}s", total_seconds)
        } else if total_seconds < 3600 {
            format!("{}m", total_seconds / 60)
        } else if total_seconds < 86400 {
            format!("{}h", total_seconds / 3600)
        } else {
            format!("{}d", total_seconds / 86400)
        }
    }

    /// Format file size for display
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

    /// Get display string for operating mode
    fn mode_display(mode: OperatingMode) -> (&'static str, Color) {
        match mode {
            OperatingMode::Plan => ("📋 Plan", Color::Blue),
            OperatingMode::Build => ("🔨 Build", Color::Green),
            OperatingMode::Review => ("🔍 Review", Color::Yellow),
        }
    }
}

impl Widget for SessionBrowser {
    fn render(self, area: Rect, buf: &mut Buffer) {
        WidgetRef::render_ref(&self, area, buf);
    }
}

impl WidgetRef for SessionBrowser {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        // Clear the area
        Clear.render(area, buf);

        // Main layout: [Header][Body][Footer]
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(10),   // Body
                Constraint::Length(3), // Footer
            ])
            .split(area);

        // Render header
        self.render_header(main_chunks[0], buf);

        // Body layout: [Left Panel][Right Panel]
        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Session list
                Constraint::Percentage(60), // Preview + Actions
            ])
            .split(main_chunks[1]);

        // Render left panel (session list)
        self.render_session_list(body_chunks[0], buf);

        // Right panel layout: [Preview][Actions]
        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(70), // Preview
                Constraint::Percentage(30), // Actions
            ])
            .split(body_chunks[1]);

        // Render right panels
        self.render_preview(right_chunks[0], buf);
        self.render_actions(right_chunks[1], buf);

        // Render footer
        self.render_footer(main_chunks[2], buf);

        // Render overlays
        if self.show_confirmation {
            self.render_confirmation_dialog(area, buf);
        }

        if self.show_export_options {
            self.render_export_options(area, buf);
        }
    }
}

impl SessionBrowser {
    /// Render the header with title and controls
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let title = format!(
            "Session Browser - {} {} ({})",
            match self.view_mode {
                ViewMode::Tree => "Tree",
                ViewMode::List => "List",
                ViewMode::Timeline => "Timeline",
            },
            match self.sort_by {
                SortBy::LastAccessed => "Last Accessed",
                SortBy::Created => "Created",
                SortBy::Name => "Name",
                SortBy::MessageCount => "Messages",
                SortBy::Size => "Size",
            },
            self.filtered_sessions.len()
        );

        let style = if self.focused_panel == FocusedPanel::Search {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(style);

        let inner = block.inner(area);
        block.render(area, buf);

        // Search query display
        let search_text = if self.search_query.is_empty() {
            "Search sessions... (Press / to search)".to_string()
        } else {
            format!("Search: {}", self.search_query)
        };

        let search_style = if self.search_query.is_empty() {
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
        } else {
            Style::default().fg(Color::White)
        };

        Paragraph::new(search_text)
            .style(search_style)
            .render(inner, buf);
    }

    /// Render the session list panel
    fn render_session_list(&self, area: Rect, buf: &mut Buffer) {
        let style = if self.focused_panel == FocusedPanel::SessionList {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Sessions")
            .border_style(style);

        let inner = block.inner(area);
        block.render(area, buf);

        if self.filtered_sessions.is_empty() {
            let empty_msg = if self.search_query.is_empty() {
                "No sessions found"
            } else {
                "No sessions match your search"
            };

            Paragraph::new(empty_msg)
                .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
                .render(inner, buf);
            return;
        }

        // Calculate visible range
        let visible_height = inner.height as usize;
        let start_idx = self.session_scroll_state.scroll_top.min(
            self.filtered_sessions.len().saturating_sub(visible_height)
        );
        let end_idx = (start_idx + visible_height).min(self.filtered_sessions.len());

        // Create list items
        let items: Vec<ListItem> = self.filtered_sessions[start_idx..end_idx]
            .iter()
            .enumerate()
            .filter_map(|(local_idx, &session_id)| {
                let global_idx = start_idx + local_idx;
                let metadata = self.session_index.sessions.get(&session_id)?;
                
                let is_selected = Some(global_idx) == self.session_scroll_state.selected_idx;
                let is_favorite = metadata.is_favorite;

                let (mode_icon, mode_color) = Self::mode_display(metadata.current_mode);
                
                let mut spans = vec![
                    Span::styled(
                        if is_favorite { "★ " } else { "  " },
                        Style::default().fg(Color::Yellow)
                    ),
                    Span::styled(mode_icon, Style::default().fg(mode_color)),
                    Span::raw(" "),
                    Span::styled(
                        &metadata.title,
                        Style::default().fg(Color::White).add_modifier(
                            if is_selected { Modifier::BOLD } else { Modifier::empty() }
                        )
                    ),
                ];

                // Add message count and size info
                spans.extend_from_slice(&[
                    Span::raw(" "),
                    Span::styled(
                        format!("({} msgs, {})", 
                            metadata.message_count,
                            Self::format_file_size(metadata.file_size)
                        ),
                        Style::default().fg(Color::DarkGray)
                    ),
                ]);

                let line = Line::from(spans);
                let item_style = if is_selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                Some(ListItem::new(line).style(item_style))
            })
            .collect();

        List::new(items).render(inner, buf);
    }

    /// Render the preview panel
    fn render_preview(&self, area: Rect, buf: &mut Buffer) {
        let style = if self.focused_panel == FocusedPanel::Preview {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Preview")
            .border_style(style);

        let inner = block.inner(area);
        block.render(area, buf);

        if let Some(metadata) = self.selected_session_metadata() {
            let (mode_icon, mode_color) = Self::mode_display(metadata.current_mode);
            
            let lines = vec![
                Line::from(vec![
                    Span::styled("Title: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(&metadata.title, Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("Mode: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(mode_icon, Style::default().fg(mode_color)),
                ]),
                Line::from(vec![
                    Span::styled("Model: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(&metadata.model, Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Created: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        metadata.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                        Style::default().fg(Color::White)
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Last Accessed: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        metadata.last_accessed.format("%Y-%m-%d %H:%M:%S").to_string(),
                        Style::default().fg(Color::White)
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Duration: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        Self::format_duration(&metadata.created_at, &metadata.updated_at),
                        Style::default().fg(Color::White)
                    ),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::styled("Messages: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        metadata.message_count.to_string(),
                        Style::default().fg(Color::White)
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Turns: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        metadata.turn_count.to_string(),
                        Style::default().fg(Color::White)
                    ),
                ]),
                Line::from(vec![
                    Span::styled("File Size: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        Self::format_file_size(metadata.file_size),
                        Style::default().fg(Color::White)
                    ),
                ]),
                Line::from(vec![
                    Span::styled("Compression: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format!("{:.1}%", metadata.compression_ratio * 100.0),
                        Style::default().fg(Color::White)
                    ),
                ]),
            ];

            // Add tags if present
            if !metadata.tags.is_empty() {
                let mut tag_lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Tags: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    ]),
                ];

                for tag in &metadata.tags {
                    tag_lines.push(Line::from(vec![
                        Span::raw("  • "),
                        Span::styled(tag, Style::default().fg(Color::Green)),
                    ]));
                }
                
                let mut all_lines = lines;
                all_lines.extend(tag_lines);
                Paragraph::new(all_lines).render(inner, buf);
            } else {
                Paragraph::new(lines).render(inner, buf);
            }

            // Add checkpoints info if present
            if !metadata.checkpoints.is_empty() {
                // This could be extended to show checkpoint details
            }
        } else {
            Paragraph::new("No session selected")
                .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
                .render(inner, buf);
        }
    }

    /// Render the actions panel
    fn render_actions(&self, area: Rect, buf: &mut Buffer) {
        let style = if self.focused_panel == FocusedPanel::Actions {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Actions")
            .border_style(style);

        let inner = block.inner(area);
        block.render(area, buf);

        if self.actions.is_empty() {
            Paragraph::new("No actions available")
                .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC))
                .render(inner, buf);
            return;
        }

        let items: Vec<ListItem> = self.actions
            .iter()
            .enumerate()
            .map(|(idx, action)| {
                let is_selected = Some(idx) == self.action_scroll_state.selected_idx;
                
                let mut spans = vec![
                    Span::styled(
                        action.display_name(),
                        Style::default().fg(Color::White).add_modifier(
                            if is_selected { Modifier::BOLD } else { Modifier::empty() }
                        )
                    ),
                ];

                if let Some(shortcut) = action.shortcut() {
                    spans.extend_from_slice(&[
                        Span::raw(" "),
                        Span::styled(
                            format!("({})", shortcut),
                            Style::default().fg(Color::DarkGray)
                        ),
                    ]);
                }

                let item_style = if is_selected {
                    Style::default().bg(Color::DarkGray)
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(spans)).style(item_style)
            })
            .collect();

        List::new(items).render(inner, buf);
    }

    /// Render the footer with help text
    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let help_text = match self.focused_panel {
            FocusedPanel::SessionList => {
                "↑/↓: Navigate | Enter: Open | Del: Delete | Tab: Next Panel | /: Search | V: View Mode | S: Sort"
            }
            FocusedPanel::Preview => {
                "Tab: Next Panel | Enter: Open Session"
            }
            FocusedPanel::Actions => {
                "↑/↓: Navigate | Enter: Execute Action | Tab: Next Panel"
            }
            FocusedPanel::Search => {
                "Type to search | Enter: Confirm | Esc: Cancel | Tab: Next Panel"
            }
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Help");

        let inner = block.inner(area);
        block.render(area, buf);

        Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .render(inner, buf);
    }

    /// Render confirmation dialog
    fn render_confirmation_dialog(&self, area: Rect, buf: &mut Buffer) {
        // Center the dialog
        let dialog_width = 50.min(area.width.saturating_sub(4));
        let dialog_height = 7.min(area.height.saturating_sub(4));
        
        let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
        
        let dialog_area = Rect {
            x: area.x + dialog_x,
            y: area.y + dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        // Clear background
        Clear.render(dialog_area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Confirm Action")
            .border_style(Style::default().fg(Color::Red));

        let inner = block.inner(dialog_area);
        block.render(dialog_area, buf);

        let lines = vec![
            Line::from(self.confirmation_message.as_str()),
            Line::from(""),
            Line::from(vec![
                Span::styled("Y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw("es / "),
                Span::styled("N", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                Span::raw("o"),
            ]),
        ];

        Paragraph::new(lines)
            .style(Style::default().fg(Color::White))
            .render(inner, buf);
    }

    /// Render export options dialog
    fn render_export_options(&self, area: Rect, buf: &mut Buffer) {
        // Center the dialog
        let dialog_width = 40.min(area.width.saturating_sub(4));
        let dialog_height = 10.min(area.height.saturating_sub(4));
        
        let dialog_x = (area.width.saturating_sub(dialog_width)) / 2;
        let dialog_y = (area.height.saturating_sub(dialog_height)) / 2;
        
        let dialog_area = Rect {
            x: area.x + dialog_x,
            y: area.y + dialog_y,
            width: dialog_width,
            height: dialog_height,
        };

        // Clear background
        Clear.render(dialog_area, buf);

        let block = Block::default()
            .borders(Borders::ALL)
            .title("Export Options")
            .border_style(Style::default().fg(Color::Blue));

        let inner = block.inner(dialog_area);
        block.render(dialog_area, buf);

        let lines = vec![
            Line::from("Choose export format:"),
            Line::from(""),
            Line::from(vec![
                Span::styled("1", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(". Markdown (conversation only)"),
            ]),
            Line::from(vec![
                Span::styled("2", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(". Markdown (with metadata)"),
            ]),
            Line::from(vec![
                Span::styled("3", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(". JSON (complete data)"),
            ]),
            Line::from(vec![
                Span::styled("4", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(". Plain text"),
            ]),
            Line::from(""),
            Line::from("Press Esc to cancel"),
        ];

        Paragraph::new(lines)
            .style(Style::default().fg(Color::White))
            .render(inner, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_session_metadata(title: &str) -> SessionMetadata {
        SessionMetadata {
            id: Uuid::new_v4(),
            title: title.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed: Utc::now(),
            message_count: 10,
            turn_count: 5,
            current_mode: OperatingMode::Build,
            model: "gpt-4".to_string(),
            tags: vec!["test".to_string()],
            is_favorite: false,
            file_size: 1024,
            compression_ratio: 0.85,
            format_version: 1,
            checkpoints: vec![],
        }
    }

    #[test]
    fn test_session_browser_creation() {
        let session_index = SessionIndex::new();
        let browser = SessionBrowser::new(session_index);
        
        assert_eq!(browser.view_mode, ViewMode::List);
        assert_eq!(browser.sort_by, SortBy::LastAccessed);
        assert_eq!(browser.focused_panel, FocusedPanel::SessionList);
        assert!(browser.search_query.is_empty());
    }

    #[test]
    fn test_view_mode_toggle() {
        let session_index = SessionIndex::new();
        let mut browser = SessionBrowser::new(session_index);
        
        assert_eq!(browser.view_mode, ViewMode::List);
        
        browser.toggle_view_mode();
        assert_eq!(browser.view_mode, ViewMode::Timeline);
        
        browser.toggle_view_mode();
        assert_eq!(browser.view_mode, ViewMode::Tree);
        
        browser.toggle_view_mode();
        assert_eq!(browser.view_mode, ViewMode::List);
    }

    #[test]
    fn test_search_functionality() {
        let mut session_index = SessionIndex::new();
        let metadata1 = create_test_session_metadata("Test Session 1");
        let metadata2 = create_test_session_metadata("Another Session");
        
        session_index.add_session(metadata1);
        session_index.add_session(metadata2);
        
        let mut browser = SessionBrowser::new(session_index);
        assert_eq!(browser.filtered_sessions.len(), 2);
        
        browser.set_search_query("Test".to_string());
        assert_eq!(browser.filtered_sessions.len(), 1);
        
        browser.set_search_query("Session".to_string());
        assert_eq!(browser.filtered_sessions.len(), 2);
        
        browser.set_search_query("NonExistent".to_string());
        assert_eq!(browser.filtered_sessions.len(), 0);
    }

    #[test]
    fn test_panel_focus_cycling() {
        let session_index = SessionIndex::new();
        let mut browser = SessionBrowser::new(session_index);
        
        assert_eq!(browser.focused_panel, FocusedPanel::SessionList);
        
        browser.focus_next_panel();
        assert_eq!(browser.focused_panel, FocusedPanel::Preview);
        
        browser.focus_next_panel();
        assert_eq!(browser.focused_panel, FocusedPanel::Actions);
        
        browser.focus_next_panel();
        assert_eq!(browser.focused_panel, FocusedPanel::Search);
        
        browser.focus_next_panel();
        assert_eq!(browser.focused_panel, FocusedPanel::SessionList);
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(SessionBrowser::format_file_size(512), "512 B");
        assert_eq!(SessionBrowser::format_file_size(1024), "1.0 KB");
        assert_eq!(SessionBrowser::format_file_size(1536), "1.5 KB");
        assert_eq!(SessionBrowser::format_file_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_sort_cycling() {
        let session_index = SessionIndex::new();
        let mut browser = SessionBrowser::new(session_index);
        
        assert_eq!(browser.sort_by, SortBy::LastAccessed);
        
        browser.cycle_sort();
        assert_eq!(browser.sort_by, SortBy::Created);
        
        browser.cycle_sort();
        assert_eq!(browser.sort_by, SortBy::Name);
        
        browser.cycle_sort();
        assert_eq!(browser.sort_by, SortBy::MessageCount);
        
        browser.cycle_sort();
        assert_eq!(browser.sort_by, SortBy::Size);
        
        browser.cycle_sort();
        assert_eq!(browser.sort_by, SortBy::LastAccessed);
    }
}