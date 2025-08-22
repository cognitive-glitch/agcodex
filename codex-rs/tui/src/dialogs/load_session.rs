//! Enhanced load session browser for AGCodex TUI
//! Provides a rich interface for browsing, searching, and loading saved sessions

use agcodex_persistence::types::OperatingMode;
use agcodex_persistence::types::SessionMetadata;
use chrono::DateTime;
use chrono::Datelike;
use chrono::Local;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyModifiers;
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
use ratatui::widgets::Paragraph;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;
use std::collections::HashMap;
use uuid::Uuid;

/// Session item for display in the load browser
#[derive(Debug, Clone)]
pub struct SessionItem {
    pub metadata: SessionMetadata,
    pub display_name: String,
    pub formatted_date: String,
    pub mode_indicator: String,
    pub mode_color: Color,
    pub preview_lines: Vec<String>,
    pub match_score: Option<i64>,
    pub match_indices: Vec<usize>,
}

impl SessionItem {
    pub fn new(metadata: SessionMetadata) -> Self {
        let local_time: DateTime<Local> = metadata.updated_at.into();
        let formatted_date = format_date(&local_time);

        let (mode_indicator, mode_color) = match metadata.current_mode {
            OperatingMode::Plan => ("📋 Plan", Color::Blue),
            OperatingMode::Build => ("🔨 Build", Color::Green),
            OperatingMode::Review => ("🔍 Review", Color::Yellow),
        };

        let display_name = if metadata.title.is_empty() {
            format!("Session {}", &metadata.id.to_string()[0..8])
        } else {
            metadata.title.clone()
        };

        // Generate preview lines
        let preview_lines = vec![
            format!("Model: {}", metadata.model),
            format!(
                "Messages: {} • Turns: {}",
                metadata.message_count, metadata.turn_count
            ),
            format!(
                "Size: {} • Compression: {:.0}%",
                format_file_size(metadata.file_size),
                metadata.compression_ratio * 100.0
            ),
        ];

        Self {
            metadata,
            display_name,
            formatted_date,
            mode_indicator: mode_indicator.to_string(),
            mode_color,
            preview_lines,
            match_score: None,
            match_indices: Vec::new(),
        }
    }

    /// Update match score and indices for fuzzy search
    pub fn update_match(&mut self, score: i64, indices: Vec<usize>) {
        self.match_score = Some(score);
        self.match_indices = indices;
    }

    /// Clear match information
    pub fn clear_match(&mut self) {
        self.match_score = None;
        self.match_indices.clear();
    }
}

/// State for the load session browser
pub struct LoadSessionState {
    /// Search query for filtering
    pub search_query: String,
    /// Cursor position in search field
    pub search_cursor: usize,
    /// All available sessions
    pub all_sessions: Vec<SessionItem>,
    /// Filtered and sorted sessions
    pub filtered_sessions: Vec<SessionItem>,
    /// Currently selected index
    pub selected_index: usize,
    /// Scroll offset for the list
    pub scroll_offset: usize,
    /// Focus state (0: search, 1: list)
    pub focus: LoadFocus,
    /// Loading state
    pub loading: bool,
    /// Error message
    pub error: Option<String>,
    /// Preview expanded
    pub preview_expanded: bool,
    /// Sort order
    pub sort_by: SortOrder,
    /// Filter by mode
    pub mode_filter: Option<OperatingMode>,
    /// Favorite sessions
    pub favorites: HashMap<Uuid, bool>,
    /// Fuzzy matcher for search
    fuzzy_matcher: SkimMatcherV2,
}

impl std::fmt::Debug for LoadSessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadSessionState")
            .field("search_query", &self.search_query)
            .field("search_cursor", &self.search_cursor)
            .field("all_sessions", &self.all_sessions)
            .field("filtered_sessions", &self.filtered_sessions)
            .field("selected_index", &self.selected_index)
            .field("scroll_offset", &self.scroll_offset)
            .field("focus", &self.focus)
            .field("loading", &self.loading)
            .field("error", &self.error)
            .field("preview_expanded", &self.preview_expanded)
            .field("sort_by", &self.sort_by)
            .field("mode_filter", &self.mode_filter)
            .field("favorites", &self.favorites)
            .field("fuzzy_matcher", &"<SkimMatcherV2>")
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadFocus {
    Search,
    List,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortOrder {
    Recent,
    Name,
    Size,
    Messages,
}

impl LoadSessionState {
    pub fn new() -> Self {
        Self {
            search_query: String::new(),
            search_cursor: 0,
            all_sessions: Vec::new(),
            filtered_sessions: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
            focus: LoadFocus::List,
            loading: false,
            error: None,
            preview_expanded: true,
            sort_by: SortOrder::Recent,
            mode_filter: None,
            favorites: HashMap::new(),
            fuzzy_matcher: SkimMatcherV2::default(),
        }
    }

    /// Set sessions from the manager
    pub fn set_sessions(&mut self, sessions: Vec<SessionMetadata>) {
        self.all_sessions = sessions.into_iter().map(SessionItem::new).collect();
        self.loading = false;
        self.error = None;
        self.apply_filters();
    }

    /// Set loading state
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
        if loading {
            self.error = None;
        }
    }

    /// Set error message
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.loading = false;
    }

    /// Apply search and filters
    pub fn apply_filters(&mut self) {
        let mut filtered = self.all_sessions.clone();

        // Apply mode filter
        if let Some(mode) = self.mode_filter {
            filtered.retain(|s| s.metadata.current_mode == mode);
        }

        // Apply fuzzy search
        if !self.search_query.is_empty() {
            for session in &mut filtered {
                let search_text = format!(
                    "{} {} {} {}",
                    session.display_name,
                    session.metadata.model,
                    session.formatted_date,
                    session.preview_lines.join(" ")
                );

                if let Some(result) = self
                    .fuzzy_matcher
                    .fuzzy_match(&search_text, &self.search_query)
                {
                    let indices = self
                        .fuzzy_matcher
                        .fuzzy_indices(&search_text, &self.search_query)
                        .map(|(_, indices)| indices)
                        .unwrap_or_default();
                    session.update_match(result, indices);
                } else {
                    session.clear_match();
                }
            }

            // Filter out non-matches and sort by score
            filtered.retain(|s| s.match_score.is_some());
            filtered.sort_by(|a, b| b.match_score.unwrap_or(0).cmp(&a.match_score.unwrap_or(0)));
        } else {
            // Clear match scores
            for session in &mut filtered {
                session.clear_match();
            }

            // Apply sort order
            match self.sort_by {
                SortOrder::Recent => {
                    filtered.sort_by(|a, b| b.metadata.updated_at.cmp(&a.metadata.updated_at));
                }
                SortOrder::Name => {
                    filtered.sort_by(|a, b| a.display_name.cmp(&b.display_name));
                }
                SortOrder::Size => {
                    filtered.sort_by(|a, b| b.metadata.file_size.cmp(&a.metadata.file_size));
                }
                SortOrder::Messages => {
                    filtered
                        .sort_by(|a, b| b.metadata.message_count.cmp(&a.metadata.message_count));
                }
            }
        }

        // Move favorites to top
        filtered.sort_by(|a, b| {
            let a_fav = self.favorites.get(&a.metadata.id).copied().unwrap_or(false);
            let b_fav = self.favorites.get(&b.metadata.id).copied().unwrap_or(false);
            b_fav.cmp(&a_fav)
        });

        self.filtered_sessions = filtered;
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Handle key input
    pub fn handle_key_event(&mut self, key: KeyEvent) -> LoadSessionAction {
        match self.focus {
            LoadFocus::Search => self.handle_search_key(key),
            LoadFocus::List => self.handle_list_key(key),
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> LoadSessionAction {
        match key.code {
            KeyCode::Esc => {
                if self.search_query.is_empty() {
                    LoadSessionAction::Cancel
                } else {
                    self.search_query.clear();
                    self.search_cursor = 0;
                    self.apply_filters();
                    LoadSessionAction::None
                }
            }
            KeyCode::Enter | KeyCode::Down | KeyCode::Tab => {
                self.focus = LoadFocus::List;
                LoadSessionAction::None
            }
            KeyCode::Char(c) => {
                if self.search_query.len() < 100 {
                    self.search_query.insert(self.search_cursor, c);
                    self.search_cursor += 1;
                    self.apply_filters();
                }
                LoadSessionAction::None
            }
            KeyCode::Backspace => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                    self.search_query.remove(self.search_cursor);
                    self.apply_filters();
                }
                LoadSessionAction::None
            }
            KeyCode::Delete => {
                if self.search_cursor < self.search_query.len() {
                    self.search_query.remove(self.search_cursor);
                    self.apply_filters();
                }
                LoadSessionAction::None
            }
            KeyCode::Left => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                }
                LoadSessionAction::None
            }
            KeyCode::Right => {
                if self.search_cursor < self.search_query.len() {
                    self.search_cursor += 1;
                }
                LoadSessionAction::None
            }
            KeyCode::Home => {
                self.search_cursor = 0;
                LoadSessionAction::None
            }
            KeyCode::End => {
                self.search_cursor = self.search_query.len();
                LoadSessionAction::None
            }
            _ => LoadSessionAction::None,
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> LoadSessionAction {
        match key.code {
            KeyCode::Esc => LoadSessionAction::Cancel,
            KeyCode::Enter => {
                if let Some(session) = self.get_selected_session() {
                    LoadSessionAction::Load(session.metadata.id)
                } else {
                    LoadSessionAction::None
                }
            }
            KeyCode::Tab | KeyCode::Char('/') => {
                self.focus = LoadFocus::Search;
                LoadSessionAction::None
            }
            KeyCode::Up | KeyCode::Char('k') if key.modifiers.is_empty() => {
                self.move_selection(-1);
                LoadSessionAction::None
            }
            KeyCode::Down | KeyCode::Char('j') if key.modifiers.is_empty() => {
                self.move_selection(1);
                LoadSessionAction::None
            }
            KeyCode::PageUp => {
                self.move_selection(-10);
                LoadSessionAction::None
            }
            KeyCode::PageDown => {
                self.move_selection(10);
                LoadSessionAction::None
            }
            KeyCode::Home => {
                self.selected_index = 0;
                self.scroll_offset = 0;
                LoadSessionAction::None
            }
            KeyCode::End => {
                if !self.filtered_sessions.is_empty() {
                    self.selected_index = self.filtered_sessions.len() - 1;
                    self.update_scroll(10);
                }
                LoadSessionAction::None
            }
            KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                // Toggle favorite
                if let Some(session) = self.get_selected_session() {
                    let is_fav = self
                        .favorites
                        .get(&session.metadata.id)
                        .copied()
                        .unwrap_or(false);
                    self.favorites.insert(session.metadata.id, !is_fav);
                    self.apply_filters();
                }
                LoadSessionAction::None
            }
            KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
                // Cycle sort order
                self.sort_by = match self.sort_by {
                    SortOrder::Recent => SortOrder::Name,
                    SortOrder::Name => SortOrder::Size,
                    SortOrder::Size => SortOrder::Messages,
                    SortOrder::Messages => SortOrder::Recent,
                };
                self.apply_filters();
                LoadSessionAction::None
            }
            KeyCode::Char('m') if key.modifiers == KeyModifiers::CONTROL => {
                // Cycle mode filter
                self.mode_filter = match self.mode_filter {
                    None => Some(OperatingMode::Plan),
                    Some(OperatingMode::Plan) => Some(OperatingMode::Build),
                    Some(OperatingMode::Build) => Some(OperatingMode::Review),
                    Some(OperatingMode::Review) => None,
                };
                self.apply_filters();
                LoadSessionAction::None
            }
            KeyCode::Char('p') if key.modifiers == KeyModifiers::CONTROL => {
                // Toggle preview
                self.preview_expanded = !self.preview_expanded;
                LoadSessionAction::None
            }
            KeyCode::Delete | KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                // Delete session
                if let Some(session) = self.get_selected_session() {
                    LoadSessionAction::Delete(session.metadata.id)
                } else {
                    LoadSessionAction::None
                }
            }
            _ => LoadSessionAction::None,
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if self.filtered_sessions.is_empty() {
            return;
        }

        let len = self.filtered_sessions.len() as i32;
        let new_index = (self.selected_index as i32 + delta).clamp(0, len - 1) as usize;
        self.selected_index = new_index;
        self.update_scroll(10);
    }

    const fn update_scroll(&mut self, visible_items: usize) {
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_items {
            self.scroll_offset = self.selected_index.saturating_sub(visible_items - 1);
        }
    }

    /// Get the currently selected session
    pub fn get_selected_session(&self) -> Option<&SessionItem> {
        self.filtered_sessions.get(self.selected_index)
    }

    /// Get selected session ID
    pub fn get_selected_id(&self) -> Option<Uuid> {
        self.get_selected_session().map(|s| s.metadata.id)
    }
}

impl Default for LoadSessionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions from the load session browser
#[derive(Debug, Clone, PartialEq)]
pub enum LoadSessionAction {
    None,
    Load(Uuid),
    Delete(Uuid),
    Cancel,
}

/// Enhanced Load Session Browser widget
pub struct LoadSessionBrowser<'a> {
    state: &'a LoadSessionState,
}

impl<'a> LoadSessionBrowser<'a> {
    pub const fn new(state: &'a LoadSessionState) -> Self {
        Self { state }
    }

    fn render_search_bar(&self, area: Rect, buf: &mut Buffer) {
        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if self.state.focus == LoadFocus::Search {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            })
            .title(" Search ");

        let inner = search_block.inner(area);
        search_block.render(area, buf);

        let search_text = if self.state.search_query.is_empty() {
            Span::styled(
                "Type to search sessions...",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )
        } else {
            Span::raw(&self.state.search_query)
        };

        let paragraph = Paragraph::new(Line::from(search_text));
        paragraph.render(inner, buf);

        // Render cursor in search mode
        if self.state.focus == LoadFocus::Search && inner.width > 0 {
            let cursor_x = inner.x + (self.state.search_cursor as u16).min(inner.width - 1);
            if cursor_x < inner.right()
                && let Some(cell) = buf.cell_mut((cursor_x, inner.y))
            {
                cell.set_style(Style::default().bg(Color::White).fg(Color::Black));
            }
        }
    }

    fn render_session_list(&self, area: Rect, buf: &mut Buffer) {
        let list_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if self.state.focus == LoadFocus::List {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            })
            .title(format!(
                " Sessions ({}) - Sort: {:?} {}",
                self.state.filtered_sessions.len(),
                self.state.sort_by,
                if let Some(mode) = self.state.mode_filter {
                    format!("- Filter: {:?}", mode)
                } else {
                    String::new()
                }
            ));

        let inner = list_block.inner(area);
        list_block.render(area, buf);

        if self.state.loading {
            let loading = Paragraph::new("Loading sessions...")
                .style(Style::default().fg(Color::Yellow))
                .alignment(Alignment::Center);
            loading.render(inner, buf);
            return;
        }

        if let Some(ref error) = self.state.error {
            let error_text = format!("Error: {}", error);
            let error_paragraph = Paragraph::new(error_text)
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true });
            error_paragraph.render(inner, buf);
            return;
        }

        if self.state.filtered_sessions.is_empty() {
            let empty_text = if self.state.all_sessions.is_empty() {
                "No saved sessions found"
            } else {
                "No sessions match your search"
            };
            let empty = Paragraph::new(empty_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            empty.render(inner, buf);
            return;
        }

        // Calculate visible range
        let visible_height = inner.height as usize;
        let end_index =
            (self.state.scroll_offset + visible_height).min(self.state.filtered_sessions.len());

        // Create list items
        let items: Vec<ListItem> = self.state.filtered_sessions
            [self.state.scroll_offset..end_index]
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let is_selected = self.state.scroll_offset + i == self.state.selected_index;
                let is_favorite = self
                    .state
                    .favorites
                    .get(&session.metadata.id)
                    .copied()
                    .unwrap_or(false);

                let mut spans = vec![];

                // Favorite indicator
                if is_favorite {
                    spans.push(Span::styled("⭐ ", Style::default().fg(Color::Yellow)));
                } else {
                    spans.push(Span::raw("  "));
                }

                // Mode indicator
                spans.push(Span::styled(
                    &session.mode_indicator,
                    Style::default().fg(session.mode_color),
                ));
                spans.push(Span::raw(" "));

                // Session name
                if !session.match_indices.is_empty() && !self.state.search_query.is_empty() {
                    // Highlight matching characters
                    let name_chars: Vec<char> = session.display_name.chars().collect();
                    for (i, ch) in name_chars.iter().enumerate() {
                        if session.match_indices.contains(&i) {
                            spans.push(Span::styled(
                                ch.to_string(),
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ));
                        } else {
                            spans.push(Span::raw(ch.to_string()));
                        }
                    }
                } else {
                    spans.push(Span::raw(&session.display_name));
                }

                // Date
                spans.push(Span::raw(" - "));
                spans.push(Span::styled(
                    &session.formatted_date,
                    Style::default().fg(Color::DarkGray),
                ));

                let style = if is_selected {
                    Style::default().bg(Color::Rgb(40, 40, 40))
                } else {
                    Style::default()
                };

                ListItem::new(Line::from(spans)).style(style)
            })
            .collect();

        let list = List::new(items);
        Widget::render(list, inner, buf);

        // Render scrollbar if needed
        if self.state.filtered_sessions.len() > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state = ScrollbarState::new(self.state.filtered_sessions.len())
                .position(self.state.scroll_offset);

            scrollbar.render(inner, buf, &mut scrollbar_state);
        }
    }

    fn render_preview(&self, area: Rect, buf: &mut Buffer) {
        let preview_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Preview ");

        let inner = preview_block.inner(area);
        preview_block.render(area, buf);

        if let Some(session) = self.state.get_selected_session() {
            let mut lines = vec![];

            // Title
            lines.push(Line::from(vec![Span::styled(
                &session.display_name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )]));

            lines.push(Line::from(""));

            // Metadata
            for preview_line in &session.preview_lines {
                lines.push(Line::from(preview_line.as_str()));
            }

            if !session.metadata.tags.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::raw("Tags: "),
                    Span::styled(
                        session.metadata.tags.join(", "),
                        Style::default().fg(Color::Cyan),
                    ),
                ]));
            }

            let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true });
            paragraph.render(inner, buf);
        } else {
            let no_selection = Paragraph::new("Select a session to preview")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            no_selection.render(inner, buf);
        }
    }

    fn render_help(&self, area: Rect, buf: &mut Buffer) {
        let help_text = match self.state.focus {
            LoadFocus::Search => "Esc: Clear/Cancel • Enter/↓: Focus List • /: Search",
            LoadFocus::List => {
                "↑↓: Navigate • Enter: Load • Del: Delete • Ctrl+F: Favorite • Ctrl+S: Sort • /: Search • Esc: Cancel"
            }
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        help.render(area, buf);
    }
}

impl<'a> WidgetRef for LoadSessionBrowser<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        // Clear background
        Clear.render(area, buf);

        // Calculate dialog size
        let width = area.width.min(100).max(60);
        let height = area.height.min(30).max(15);
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;
        let dialog_area = Rect::new(x, y, width, height);

        // Main dialog block
        let dialog_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Blue))
            .title(" Load Session ");

        let inner = dialog_block.inner(dialog_area);
        dialog_block.render(dialog_area, buf);

        // Layout
        let layout = if self.state.preview_expanded {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),      // Search bar
                    Constraint::Percentage(50), // Session list
                    Constraint::Percentage(30), // Preview
                    Constraint::Length(1),      // Help
                ])
                .split(inner)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Search bar
                    Constraint::Min(5),    // Session list
                    Constraint::Length(1), // Help
                ])
                .split(inner)
        };

        self.render_search_bar(layout[0], buf);
        self.render_session_list(layout[1], buf);

        if self.state.preview_expanded {
            self.render_preview(layout[2], buf);
            self.render_help(layout[3], buf);
        } else {
            self.render_help(layout[2], buf);
        }
    }
}

// Helper functions
fn format_date(date: &DateTime<Local>) -> String {
    let now = Local::now();
    let duration = now.signed_duration_since(*date);

    if duration.num_seconds() < 60 {
        "Just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{} min ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_days() < 7 {
        format!("{} days ago", duration.num_days())
    } else if date.year() == now.year() {
        date.format("%b %d").to_string()
    } else {
        date.format("%b %d, %Y").to_string()
    }
}

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
