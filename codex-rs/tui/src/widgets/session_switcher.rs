//! Quick session switcher widget for AGCodex TUI
//! Provides Alt+[1-9] quick switching and session tabs at the bottom

use agcodex_persistence::types::OperatingMode;
use agcodex_persistence::types::SessionMetadata;
use chrono::DateTime;
use chrono::Local;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyModifiers;
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
use ratatui::widgets::Tabs;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;
use std::collections::HashMap;
use uuid::Uuid;

/// Session entry for the switcher
#[derive(Debug, Clone)]
pub struct SessionEntry {
    pub id: Uuid,
    pub title: String,
    pub mode: OperatingMode,
    pub is_modified: bool,
    pub last_accessed: DateTime<Local>,
    pub message_count: usize,
    pub shortcut_key: Option<u8>, // 1-9 for Alt+[1-9] shortcuts
}

impl SessionEntry {
    pub fn from_metadata(metadata: SessionMetadata, is_modified: bool) -> Self {
        let title = if metadata.title.is_empty() {
            format!("Session {}", &metadata.id.to_string()[0..8])
        } else {
            metadata.title.clone()
        };

        Self {
            id: metadata.id,
            title,
            mode: metadata.current_mode,
            is_modified,
            last_accessed: metadata.last_accessed.into(),
            message_count: metadata.message_count,
            shortcut_key: None,
        }
    }

    /// Get display string for the tab
    pub fn tab_display(&self) -> String {
        let mode_icon = match self.mode {
            OperatingMode::Plan => "📋",
            OperatingMode::Build => "🔨",
            OperatingMode::Review => "🔍",
        };

        let modified_indicator = if self.is_modified { "*" } else { "" };

        if let Some(key) = self.shortcut_key {
            format!("{} {} {}{}", key, mode_icon, self.title, modified_indicator)
        } else {
            format!("{} {}{}", mode_icon, self.title, modified_indicator)
        }
    }

    /// Get shortened title for compact display
    pub fn short_title(&self, max_len: usize) -> String {
        if self.title.len() <= max_len {
            self.title.clone()
        } else {
            format!("{}…", &self.title[..max_len.saturating_sub(1)])
        }
    }
}

/// State for the session switcher
#[derive(Debug)]
pub struct SessionSwitcherState {
    /// Active sessions (up to 9 for shortcuts)
    pub sessions: Vec<SessionEntry>,
    /// Currently active session ID
    pub active_session_id: Option<Uuid>,
    /// Selected index in the switcher (for Ctrl+Tab navigation)
    pub selected_index: usize,
    /// Whether the switcher is visible
    pub is_visible: bool,
    /// Maximum number of sessions with shortcuts
    pub max_shortcuts: usize,
    /// Modified sessions tracking
    pub modified_sessions: HashMap<Uuid, bool>,
    /// Compact mode for small terminals
    pub compact_mode: bool,
}

impl SessionSwitcherState {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            active_session_id: None,
            selected_index: 0,
            is_visible: true,
            max_shortcuts: 9,
            modified_sessions: HashMap::new(),
            compact_mode: false,
        }
    }

    /// Add or update a session
    pub fn add_session(&mut self, metadata: SessionMetadata, make_active: bool) {
        let is_modified = self
            .modified_sessions
            .get(&metadata.id)
            .copied()
            .unwrap_or(false);
        let mut entry = SessionEntry::from_metadata(metadata, is_modified);
        let entry_id = entry.id;

        // Check if session already exists
        if let Some(pos) = self.sessions.iter().position(|s| s.id == entry.id) {
            // Update existing session
            self.sessions[pos] = entry;
        } else {
            // Add new session
            if self.sessions.len() < self.max_shortcuts {
                entry.shortcut_key = Some((self.sessions.len() + 1) as u8);
            }
            self.sessions.push(entry);
        }

        if make_active {
            self.active_session_id = Some(entry_id);
            self.selected_index = self
                .sessions
                .iter()
                .position(|s| s.id == entry_id)
                .unwrap_or(0);
        }

        // Sort by last accessed time (most recent first)
        self.sessions
            .sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));

        // Reassign shortcut keys
        self.reassign_shortcuts();
    }

    /// Remove a session
    pub fn remove_session(&mut self, id: Uuid) {
        self.sessions.retain(|s| s.id != id);
        self.modified_sessions.remove(&id);

        // If removed session was active, switch to first available
        if self.active_session_id == Some(id) {
            self.active_session_id = self.sessions.first().map(|s| s.id);
            self.selected_index = 0;
        }

        self.reassign_shortcuts();
    }

    /// Mark session as modified
    pub fn mark_modified(&mut self, id: Uuid, modified: bool) {
        self.modified_sessions.insert(id, modified);

        if let Some(session) = self.sessions.iter_mut().find(|s| s.id == id) {
            session.is_modified = modified;
        }
    }

    /// Switch to session by shortcut key (1-9)
    pub fn switch_by_shortcut(&mut self, key: u8) -> Option<Uuid> {
        if !(1..=9).contains(&key) {
            return None;
        }

        self.sessions
            .iter()
            .find(|s| s.shortcut_key == Some(key))
            .map(|s| {
                self.active_session_id = Some(s.id);
                self.selected_index = self
                    .sessions
                    .iter()
                    .position(|sess| sess.id == s.id)
                    .unwrap_or(0);
                s.id
            })
    }

    /// Switch to next session (Ctrl+Tab)
    pub fn switch_next(&mut self) -> Option<Uuid> {
        if self.sessions.is_empty() {
            return None;
        }

        self.selected_index = (self.selected_index + 1) % self.sessions.len();
        let session = &self.sessions[self.selected_index];
        self.active_session_id = Some(session.id);
        Some(session.id)
    }

    /// Switch to previous session (Ctrl+Shift+Tab)
    pub fn switch_previous(&mut self) -> Option<Uuid> {
        if self.sessions.is_empty() {
            return None;
        }

        self.selected_index = if self.selected_index == 0 {
            self.sessions.len() - 1
        } else {
            self.selected_index - 1
        };

        let session = &self.sessions[self.selected_index];
        self.active_session_id = Some(session.id);
        Some(session.id)
    }

    /// Reassign shortcut keys after changes
    fn reassign_shortcuts(&mut self) {
        for (i, session) in self.sessions.iter_mut().enumerate() {
            if i < self.max_shortcuts {
                session.shortcut_key = Some((i + 1) as u8);
            } else {
                session.shortcut_key = None;
            }
        }
    }

    /// Get the active session
    pub fn active_session(&self) -> Option<&SessionEntry> {
        self.active_session_id
            .and_then(|id| self.sessions.iter().find(|s| s.id == id))
    }

    /// Handle key events
    pub fn handle_key(&mut self, key: KeyEvent) -> SessionSwitcherAction {
        // Alt+[1-9] for quick switching
        if key.modifiers == KeyModifiers::ALT
            && let KeyCode::Char(c) = key.code
            && let Some(digit) = c.to_digit(10)
            && (1..=9).contains(&digit)
            && let Some(id) = self.switch_by_shortcut(digit as u8)
        {
            return SessionSwitcherAction::Switch(id);
        }

        // Ctrl+Tab / Ctrl+Shift+Tab for cycling
        if key.modifiers == KeyModifiers::CONTROL
            && key.code == KeyCode::Tab
            && let Some(id) = self.switch_next()
        {
            return SessionSwitcherAction::Switch(id);
        }

        if key.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT) {
            match key.code {
                KeyCode::Tab | KeyCode::BackTab => {
                    if let Some(id) = self.switch_previous() {
                        return SessionSwitcherAction::Switch(id);
                    }
                }
                _ => {}
            }
        }

        SessionSwitcherAction::None
    }

    /// Toggle visibility
    pub const fn toggle_visibility(&mut self) {
        self.is_visible = !self.is_visible;
    }

    /// Set compact mode based on terminal width
    pub const fn set_compact_mode(&mut self, terminal_width: u16) {
        self.compact_mode = terminal_width < 100;
    }
}

impl Default for SessionSwitcherState {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions from the session switcher
#[derive(Debug, Clone, PartialEq)]
pub enum SessionSwitcherAction {
    None,
    Switch(Uuid),
    Close(Uuid),
    New,
}

/// Session switcher widget (tab bar)
pub struct SessionSwitcher<'a> {
    state: &'a SessionSwitcherState,
}

impl<'a> SessionSwitcher<'a> {
    pub const fn new(state: &'a SessionSwitcherState) -> Self {
        Self { state }
    }

    fn get_tab_titles(&self, available_width: u16) -> Vec<String> {
        if self.state.sessions.is_empty() {
            return vec!["No sessions".to_string()];
        }

        let mut titles: Vec<String> = Vec::new();
        let mut total_width = 0u16;

        // Calculate how much space each tab can use
        let _max_tab_width = if self.state.compact_mode { 15 } else { 25 };

        for session in &self.state.sessions {
            let full_title = session.tab_display();
            let title = if self.state.compact_mode {
                // In compact mode, show shortened titles
                let short = session.short_title(12);
                let modified = if session.is_modified { "*" } else { "" };
                if let Some(key) = session.shortcut_key {
                    format!("{}:{}{}", key, short, modified)
                } else {
                    format!("{}{}", short, modified)
                }
            } else {
                full_title.clone()
            };

            let title_width = title.len() as u16 + 3; // Add padding

            if total_width + title_width > available_width && !titles.is_empty() {
                // Add indicator that there are more sessions
                if let Some(last) = titles.last_mut() {
                    *last = format!("{}…", last.trim_end());
                }
                break;
            }

            titles.push(title);
            total_width += title_width;
        }

        titles
    }

    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = self.get_tab_titles(area.width);

        let active_index = self
            .state
            .active_session_id
            .and_then(|id| self.state.sessions.iter().position(|s| s.id == id))
            .unwrap_or(0);

        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .select(active_index.min(self.state.sessions.len().saturating_sub(1)))
            .style(Style::default().fg(Color::Gray))
            .highlight_style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Rgb(40, 40, 40))
                    .add_modifier(Modifier::BOLD),
            )
            .divider(" │ ");

        tabs.render(area, buf);
    }

    fn render_help(&self, area: Rect, buf: &mut Buffer) {
        let help_text = if self.state.compact_mode {
            "Alt+[1-9] • Ctrl+Tab"
        } else {
            "Alt+[1-9]: Quick Switch • Ctrl+Tab: Cycle • Ctrl+N: New"
        };

        let help = Line::from(vec![
            Span::raw(" "),
            Span::styled(help_text, Style::default().fg(Color::DarkGray)),
        ]);

        buf.set_line(area.x, area.y, &help, area.width);
    }
}

impl<'a> WidgetRef for SessionSwitcher<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        if !self.state.is_visible || area.height < 1 {
            return;
        }

        // Use single line for tabs
        if area.height == 1 {
            self.render_tabs(area, buf);
            return;
        }

        // If we have 2 lines, show tabs and help
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        self.render_tabs(layout[0], buf);

        if area.height >= 2 {
            self.render_help(layout[1], buf);
        }
    }
}

/// Detailed session switcher popup (for when there are many sessions)
#[allow(dead_code)]
pub struct SessionSwitcherPopup<'a> {
    state: &'a SessionSwitcherState,
}

impl<'a> SessionSwitcherPopup<'a> {
    pub const fn new(state: &'a SessionSwitcherState) -> Self {
        Self { state }
    }
}

impl<'a> WidgetRef for SessionSwitcherPopup<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        // Calculate popup dimensions
        let popup_width = 60.min(area.width - 4);
        let popup_height = (self.state.sessions.len() as u16 + 4).min(area.height - 4);

        let x = (area.width.saturating_sub(popup_width)) / 2;
        let y = (area.height.saturating_sub(popup_height)) / 2;
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        // Clear background
        for row in popup_area.top()..popup_area.bottom() {
            for col in popup_area.left()..popup_area.right() {
                if let Some(cell) = buf.cell_mut((col, row)) {
                    cell.set_char(' ');
                    cell.set_style(Style::default().bg(Color::Black));
                }
            }
        }

        // Draw popup border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Session Switcher ");

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        // Render session list
        for (i, session) in self.state.sessions.iter().enumerate() {
            if i >= inner.height as usize {
                break;
            }

            let y = inner.y + i as u16;
            let is_active = Some(session.id) == self.state.active_session_id;
            let is_selected = i == self.state.selected_index;

            let mode_icon = match session.mode {
                OperatingMode::Plan => "📋",
                OperatingMode::Build => "🔨",
                OperatingMode::Review => "🔍",
            };

            let modified = if session.is_modified { "*" } else { " " };
            let shortcut = if let Some(key) = session.shortcut_key {
                format!("Alt+{}", key)
            } else {
                "     ".to_string()
            };

            let line_text = format!(
                " {} {} {} {}{}",
                shortcut,
                mode_icon,
                session.title,
                modified,
                if is_active { " (active)" } else { "" }
            );

            let style = if is_selected {
                Style::default().bg(Color::Rgb(40, 40, 40)).fg(Color::White)
            } else if is_active {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Gray)
            };

            let line = Line::from(line_text).style(style);
            buf.set_line(inner.x, y, &line, inner.width);
        }

        // Help text at bottom
        if popup_area.bottom() < area.height {
            let help = Line::from(vec![Span::styled(
                "↑↓: Navigate • Enter: Switch • Esc: Cancel",
                Style::default().fg(Color::DarkGray),
            )]);

            let help_y = popup_area.bottom();
            let help_x = popup_area.x + (popup_area.width.saturating_sub(help.width() as u16)) / 2;
            buf.set_line(help_x, help_y, &help, popup_area.width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_metadata(id: Uuid, title: &str) -> SessionMetadata {
        use agcodex_persistence::types::SessionMetadata;

        SessionMetadata {
            id,
            title: title.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed: Utc::now(),
            message_count: 5,
            turn_count: 3,
            current_mode: OperatingMode::Build,
            model: "gpt-4".to_string(),
            tags: vec![],
            is_favorite: false,
            file_size: 1024,
            compression_ratio: 0.7,
            format_version: 1,
            checkpoints: vec![],
        }
    }

    #[test]
    fn test_session_switcher_shortcuts() {
        let mut state = SessionSwitcherState::new();

        // Add some sessions
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        state.add_session(create_test_metadata(id1, "Session 1"), true);
        state.add_session(create_test_metadata(id2, "Session 2"), false);
        state.add_session(create_test_metadata(id3, "Session 3"), false);

        // Check shortcut keys are assigned
        assert_eq!(state.sessions[0].shortcut_key, Some(1));
        assert_eq!(state.sessions[1].shortcut_key, Some(2));
        assert_eq!(state.sessions[2].shortcut_key, Some(3));

        // Test switching by shortcut
        let switched = state.switch_by_shortcut(2);
        assert_eq!(switched, Some(state.sessions[1].id));
        assert_eq!(state.active_session_id, Some(state.sessions[1].id));
    }

    #[test]
    fn test_session_cycling() {
        let mut state = SessionSwitcherState::new();

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        state.add_session(create_test_metadata(id1, "Session 1"), true);
        state.add_session(create_test_metadata(id2, "Session 2"), false);

        // Test next cycling
        let next = state.switch_next();
        assert!(next.is_some());
        assert_eq!(state.selected_index, 1);

        // Should wrap around
        let next = state.switch_next();
        assert!(next.is_some());
        assert_eq!(state.selected_index, 0);

        // Test previous cycling
        let prev = state.switch_previous();
        assert!(prev.is_some());
        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn test_modified_tracking() {
        let mut state = SessionSwitcherState::new();

        let id = Uuid::new_v4();
        state.add_session(create_test_metadata(id, "Test"), true);

        // Mark as modified
        state.mark_modified(id, true);
        assert!(state.sessions[0].is_modified);

        // Unmark
        state.mark_modified(id, false);
        assert!(!state.sessions[0].is_modified);
    }
}
