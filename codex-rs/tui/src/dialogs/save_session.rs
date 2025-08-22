//! Enhanced save session dialog for AGCodex TUI
//! Provides popup dialog with session naming, progress bar, and auto-save integration

use chrono::Local;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
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
use ratatui::widgets::Gauge;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use uuid::Uuid;

/// State for the save session dialog
#[derive(Debug, Clone)]
pub struct SaveSessionState {
    /// Session name input
    pub session_name: String,
    /// Optional description input
    pub description: String,
    /// Currently focused field (0: name, 1: description, 2: save button, 3: cancel button)
    pub focused_field: usize,
    /// Cursor position in the currently focused text field
    pub cursor_pos: usize,
    /// Save location path (~/.agcodex/history/)
    pub save_location: PathBuf,
    /// Whether to show validation errors
    pub show_error: Option<String>,
    /// Save progress (0.0 to 1.0)
    pub save_progress: f64,
    /// Whether save operation is in progress
    pub saving: bool,
    /// Start time of save operation for progress estimation
    pub save_start_time: Option<Instant>,
    /// Session ID being saved
    pub session_id: Option<Uuid>,
    /// Auto-generated timestamp name
    pub default_name: String,
    /// Show success message
    pub show_success: bool,
    /// Estimated time remaining
    pub estimated_time: Option<Duration>,
}

impl SaveSessionState {
    pub fn new() -> Self {
        let save_location = dirs::home_dir()
            .map(|p| p.join(".agcodex/history"))
            .unwrap_or_else(|| PathBuf::from(".agcodex/history"));

        // Generate default timestamp name
        let default_name = Local::now().format("%Y-%m-%d_%H-%M").to_string();

        Self {
            session_name: default_name.clone(),
            description: String::new(),
            focused_field: 0,
            cursor_pos: default_name.len(),
            save_location,
            show_error: None,
            save_progress: 0.0,
            saving: false,
            save_start_time: None,
            session_id: None,
            default_name,
            show_success: false,
            estimated_time: None,
        }
    }

    /// Handle key input for the dialog
    pub fn handle_key_event(&mut self, key: KeyEvent) -> SaveSessionAction {
        if self.saving {
            // Only allow Esc during save
            if key.code == KeyCode::Esc {
                return SaveSessionAction::Cancel;
            }
            return SaveSessionAction::None;
        }

        if self.show_success {
            // Any key closes success dialog
            return SaveSessionAction::Close;
        }

        match key.code {
            KeyCode::Esc => SaveSessionAction::Cancel,
            KeyCode::Enter => {
                match self.focused_field {
                    0 | 1 => {
                        // Enter in text fields moves to next field or saves
                        if self.focused_field == 0 && !self.session_name.trim().is_empty() {
                            self.focused_field = 1;
                            self.cursor_pos = self.description.len();
                        } else if self.focused_field == 1 {
                            self.focused_field = 2; // Move to save button
                        }
                        SaveSessionAction::None
                    }
                    2 => {
                        // Save button
                        if self.validate() {
                            SaveSessionAction::Save
                        } else {
                            SaveSessionAction::None
                        }
                    }
                    3 => SaveSessionAction::Cancel, // Cancel button
                    _ => SaveSessionAction::None,
                }
            }
            KeyCode::Tab => {
                self.next_field();
                SaveSessionAction::None
            }
            KeyCode::BackTab => {
                self.prev_field();
                SaveSessionAction::None
            }
            KeyCode::Char(c) => {
                if self.focused_field <= 1 {
                    // Limit input length
                    let current_text = if self.focused_field == 0 {
                        &self.session_name
                    } else {
                        &self.description
                    };

                    if current_text.len() < 100 {
                        self.insert_char(c);
                    }
                }
                SaveSessionAction::None
            }
            KeyCode::Backspace => {
                if self.focused_field <= 1 {
                    self.delete_char();
                }
                SaveSessionAction::None
            }
            KeyCode::Delete => {
                if self.focused_field <= 1 {
                    self.delete_char_forward();
                }
                SaveSessionAction::None
            }
            KeyCode::Left => {
                if self.focused_field <= 1 {
                    self.move_cursor_left();
                }
                SaveSessionAction::None
            }
            KeyCode::Right => {
                if self.focused_field <= 1 {
                    self.move_cursor_right();
                }
                SaveSessionAction::None
            }
            KeyCode::Home => {
                if self.focused_field <= 1 {
                    self.cursor_pos = 0;
                }
                SaveSessionAction::None
            }
            KeyCode::End => {
                if self.focused_field <= 1 {
                    self.cursor_pos = self.current_field_text().len();
                }
                SaveSessionAction::None
            }
            _ => SaveSessionAction::None,
        }
    }

    fn next_field(&mut self) {
        self.focused_field = (self.focused_field + 1) % 4;
        if self.focused_field <= 1 {
            self.cursor_pos = self.current_field_text().len();
        }
    }

    fn prev_field(&mut self) {
        self.focused_field = if self.focused_field == 0 {
            3
        } else {
            self.focused_field - 1
        };
        if self.focused_field <= 1 {
            self.cursor_pos = self.current_field_text().len();
        }
    }

    fn current_field_text(&self) -> &str {
        match self.focused_field {
            0 => &self.session_name,
            1 => &self.description,
            _ => "",
        }
    }

    fn current_field_text_mut(&mut self) -> &mut String {
        match self.focused_field {
            0 => &mut self.session_name,
            1 => &mut self.description,
            _ => panic!("Invalid field for text mutation"),
        }
    }

    fn insert_char(&mut self, c: char) {
        if self.focused_field <= 1 {
            let cursor_pos = self.cursor_pos;
            let text = self.current_field_text_mut();
            text.insert(cursor_pos, c);
            self.cursor_pos = cursor_pos + c.len_utf8();
            self.show_error = None;
        }
    }

    fn delete_char(&mut self) {
        if self.focused_field <= 1 && self.cursor_pos > 0 {
            let cursor_pos = self.cursor_pos;
            let text = self.current_field_text_mut();
            let char_boundary = text
                .char_indices()
                .rev()
                .find(|(idx, _)| *idx < cursor_pos)
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            text.drain(char_boundary..cursor_pos);
            self.cursor_pos = char_boundary;
            self.show_error = None;
        }
    }

    fn delete_char_forward(&mut self) {
        if self.focused_field <= 1 {
            let cursor_pos = self.cursor_pos;
            let text = self.current_field_text_mut();
            if cursor_pos < text.len() {
                let next_char_boundary = text
                    .char_indices()
                    .find(|(idx, _)| *idx > cursor_pos)
                    .map(|(idx, _)| idx)
                    .unwrap_or(text.len());
                text.drain(cursor_pos..next_char_boundary);
            }
        }
    }

    fn move_cursor_left(&mut self) {
        if self.focused_field <= 1 && self.cursor_pos > 0 {
            let text = self.current_field_text();
            self.cursor_pos = text
                .char_indices()
                .rev()
                .find(|(idx, _)| *idx < self.cursor_pos)
                .map(|(idx, _)| idx)
                .unwrap_or(0);
        }
    }

    fn move_cursor_right(&mut self) {
        if self.focused_field <= 1 {
            let text = self.current_field_text();
            if self.cursor_pos < text.len() {
                self.cursor_pos = text
                    .char_indices()
                    .find(|(idx, _)| *idx > self.cursor_pos)
                    .map(|(idx, _)| idx)
                    .unwrap_or(text.len());
            }
        }
    }

    /// Validate the dialog input
    pub fn validate(&mut self) -> bool {
        let name = self.session_name.trim();
        if name.is_empty() {
            self.show_error = Some("Session name cannot be empty".to_string());
            self.focused_field = 0;
            return false;
        }

        // Check for invalid characters
        if name
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        {
            self.show_error = Some("Session name contains invalid characters".to_string());
            self.focused_field = 0;
            return false;
        }

        // Check length
        if name.len() > 100 {
            self.show_error = Some("Session name too long (max 100 characters)".to_string());
            self.focused_field = 0;
            return false;
        }

        self.show_error = None;
        true
    }

    /// Start save operation
    pub fn start_save(&mut self, session_id: Uuid) {
        self.saving = true;
        self.save_progress = 0.0;
        self.save_start_time = Some(Instant::now());
        self.session_id = Some(session_id);
        self.show_error = None;
    }

    /// Update save progress
    pub fn update_progress(&mut self, progress: f64) {
        self.save_progress = progress.clamp(0.0, 1.0);

        // Estimate time remaining based on elapsed time
        if let Some(start_time) = self.save_start_time {
            let elapsed = start_time.elapsed();
            if progress > 0.0 && progress < 1.0 {
                let total_estimated = elapsed.as_secs_f64() / progress;
                let remaining = total_estimated - elapsed.as_secs_f64();
                self.estimated_time = Some(Duration::from_secs_f64(remaining));
            }
        }
    }

    /// Complete save operation
    pub const fn complete_save(&mut self) {
        self.saving = false;
        self.save_progress = 1.0;
        self.show_success = true;
        self.estimated_time = None;
    }

    /// Set error message
    pub fn set_error(&mut self, error: String) {
        self.show_error = Some(error);
        self.saving = false;
        self.save_progress = 0.0;
        self.estimated_time = None;
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for SaveSessionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions that can be triggered by the save dialog
#[derive(Debug, Clone, PartialEq)]
pub enum SaveSessionAction {
    None,
    Save,
    Cancel,
    Close,
}

/// Enhanced Save Session Dialog widget with progress bar
pub struct SaveSessionDialog<'a> {
    state: &'a SaveSessionState,
}

impl<'a> SaveSessionDialog<'a> {
    pub const fn new(state: &'a SaveSessionState) -> Self {
        Self { state }
    }

    fn render_input_field(
        &self,
        area: Rect,
        buf: &mut Buffer,
        title: &str,
        text: &str,
        placeholder: &str,
        focused: bool,
        cursor_pos: usize,
    ) {
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if focused {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            });

        let inner = block.inner(area);
        block.render(area, buf);

        // Render text or placeholder
        let content = if text.is_empty() && !focused {
            Span::styled(
                placeholder,
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )
        } else {
            Span::raw(text)
        };

        let paragraph = Paragraph::new(Line::from(content));
        paragraph.render(inner, buf);

        // Render cursor if focused
        if focused && inner.width > 0 {
            let visible_cursor = cursor_pos.min(text.len());
            let cursor_x = inner.x + (visible_cursor as u16).min(inner.width - 1);
            if cursor_x < inner.right()
                && let Some(cell) = buf.cell_mut((cursor_x, inner.y))
            {
                cell.set_style(Style::default().bg(Color::White).fg(Color::Black));
            }
        }
    }

    fn render_button(
        &self,
        area: Rect,
        buf: &mut Buffer,
        text: &str,
        focused: bool,
        disabled: bool,
    ) {
        let style = if disabled {
            Style::default().fg(Color::DarkGray)
        } else if focused {
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if focused && !disabled {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(style);

        let inner_area = block.inner(area);
        block.render(area, buf);
        paragraph.render(inner_area, buf);
    }

    fn render_progress_bar(&self, area: Rect, buf: &mut Buffer) {
        let progress_text = if let Some(duration) = self.state.estimated_time {
            format!(
                "Saving... {:.0}% - {}s remaining",
                self.state.save_progress * 100.0,
                duration.as_secs()
            )
        } else {
            format!("Saving... {:.0}%", self.state.save_progress * 100.0)
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" Progress "),
            )
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
            .percent((self.state.save_progress * 100.0) as u16)
            .label(progress_text);

        gauge.render(area, buf);
    }

    fn render_success_message(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Green))
            .title(" Success ");

        let inner = block.inner(area);
        block.render(area, buf);

        let success_text = vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("✓ "),
                Span::styled(
                    "Session saved successfully!",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("Name: "),
                Span::styled(&self.state.session_name, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::raw("Location: "),
                Span::styled(
                    self.state.save_location.display().to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Press any key to continue",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
        ];

        let paragraph = Paragraph::new(success_text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        paragraph.render(inner, buf);
    }
}

impl<'a> WidgetRef for SaveSessionDialog<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        // Clear the background
        Clear.render(area, buf);

        // Calculate dialog dimensions
        let dialog_width = 70;
        let dialog_height = if self.state.show_success {
            12
        } else if self.state.saving {
            10
        } else {
            18
        };

        let x = area.width.saturating_sub(dialog_width) / 2;
        let y = area.height.saturating_sub(dialog_height) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        // Show success message if save completed
        if self.state.show_success {
            self.render_success_message(dialog_area, buf);
            return;
        }

        // Main dialog block
        let block = Block::default()
            .title(" Save Session ")
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Blue));

        let inner = block.inner(dialog_area);
        block.render(dialog_area, buf);

        if self.state.saving {
            // Show progress bar during save
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Info text
                    Constraint::Length(1), // Spacing
                    Constraint::Length(3), // Progress bar
                    Constraint::Length(1), // Spacing
                    Constraint::Length(1), // Cancel hint
                ])
                .split(inner);

            let info_text = format!("Saving session: {}", self.state.session_name);
            let info = Paragraph::new(info_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::White));
            info.render(layout[0], buf);

            self.render_progress_bar(layout[2], buf);

            let cancel_hint = Paragraph::new("Press Esc to cancel")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));
            cancel_hint.render(layout[4], buf);
        } else {
            // Normal input mode
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Save location
                    Constraint::Length(1), // Spacing
                    Constraint::Length(3), // Session name
                    Constraint::Length(3), // Description
                    Constraint::Length(1), // Spacing
                    Constraint::Length(2), // Error message
                    Constraint::Length(1), // Spacing
                    Constraint::Length(3), // Buttons
                ])
                .split(inner);

            // Save location info
            let location_text = format!("📁 {}", self.state.save_location.display());
            let location = Paragraph::new(location_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            location.render(layout[0], buf);

            // Session name input
            self.render_input_field(
                layout[2],
                buf,
                "Session Name",
                &self.state.session_name,
                &format!("e.g., {}", self.state.default_name),
                self.state.focused_field == 0,
                if self.state.focused_field == 0 {
                    self.state.cursor_pos
                } else {
                    0
                },
            );

            // Description input
            self.render_input_field(
                layout[3],
                buf,
                "Description (optional)",
                &self.state.description,
                "Brief description of this session...",
                self.state.focused_field == 1,
                if self.state.focused_field == 1 {
                    self.state.cursor_pos
                } else {
                    0
                },
            );

            // Error message
            if let Some(ref error) = self.state.show_error {
                let error_text = format!("⚠ {}", error);
                let error_paragraph = Paragraph::new(error_text)
                    .style(Style::default().fg(Color::Red))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true });
                error_paragraph.render(layout[5], buf);
            }

            // Buttons
            let button_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(20),
                    Constraint::Percentage(25),
                    Constraint::Percentage(10),
                    Constraint::Percentage(25),
                    Constraint::Percentage(20),
                ])
                .split(layout[7]);

            self.render_button(
                button_layout[1],
                buf,
                "💾 Save",
                self.state.focused_field == 2,
                self.state.session_name.trim().is_empty(),
            );

            self.render_button(
                button_layout[3],
                buf,
                "Cancel",
                self.state.focused_field == 3,
                false,
            );
        }

        // Help text at the bottom
        let help_text = if self.state.saving {
            "Saving session... Please wait"
        } else if self.state.show_success {
            "Session saved! Press any key to continue"
        } else {
            "Tab: Navigate • Enter: Confirm • Esc: Cancel • Ctrl+S: Quick Save"
        };

        let help_area = Rect::new(
            dialog_area.x,
            dialog_area.y + dialog_area.height,
            dialog_area.width,
            1,
        );

        if help_area.y < area.height {
            let help = Paragraph::new(help_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            help.render(help_area, buf);
        }
    }
}
