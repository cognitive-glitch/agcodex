//! Save Session Dialog widget for AGCodex TUI

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
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;
use std::path::PathBuf;

/// State for the save dialog input fields
#[derive(Debug, Clone)]
pub struct SaveDialogState {
    /// Session name input
    pub session_name: String,
    /// Optional description input
    pub description: String,
    /// Currently focused field (0: name, 1: description, 2: save button, 3: cancel button)
    pub focused_field: usize,
    /// Cursor position in the currently focused text field
    pub cursor_pos: usize,
    /// Save location path (~/.agcodex/history/sessions/)
    pub save_location: PathBuf,
    /// Whether to show validation errors
    pub show_error: Option<String>,
    /// Whether save operation is in progress
    pub saving: bool,
}

impl SaveDialogState {
    pub fn new() -> Self {
        let save_location = dirs::home_dir()
            .map(|p| p.join(".agcodex/history/sessions"))
            .unwrap_or_else(|| PathBuf::from(".agcodex/history/sessions"));

        Self {
            session_name: String::new(),
            description: String::new(),
            focused_field: 0,
            cursor_pos: 0,
            save_location,
            show_error: None,
            saving: false,
        }
    }

    /// Handle key input for the dialog
    pub fn handle_key_event(&mut self, key: KeyEvent) -> SaveDialogAction {
        if self.saving {
            return SaveDialogAction::None; // Ignore input while saving
        }

        match key.code {
            KeyCode::Esc => SaveDialogAction::Cancel,
            KeyCode::Enter => {
                match self.focused_field {
                    0 | 1 => {
                        // Enter in text fields moves to next field or saves
                        if self.focused_field == 1 || self.session_name.trim().is_empty() {
                            self.move_to_save_button()
                        } else {
                            self.focused_field = 1;
                            self.cursor_pos = self.description.len();
                        }
                        SaveDialogAction::None
                    }
                    2 => SaveDialogAction::Save,   // Save button
                    3 => SaveDialogAction::Cancel, // Cancel button
                    _ => SaveDialogAction::None,
                }
            }
            KeyCode::Tab => {
                self.next_field();
                SaveDialogAction::None
            }
            KeyCode::BackTab => {
                self.prev_field();
                SaveDialogAction::None
            }
            KeyCode::Char(c) => {
                if self.focused_field <= 1 {
                    self.insert_char(c);
                }
                SaveDialogAction::None
            }
            KeyCode::Backspace => {
                if self.focused_field <= 1 {
                    self.delete_char();
                }
                SaveDialogAction::None
            }
            KeyCode::Left => {
                if self.focused_field <= 1 {
                    self.move_cursor_left();
                }
                SaveDialogAction::None
            }
            KeyCode::Right => {
                if self.focused_field <= 1 {
                    self.move_cursor_right();
                }
                SaveDialogAction::None
            }
            KeyCode::Home => {
                if self.focused_field <= 1 {
                    self.cursor_pos = 0;
                }
                SaveDialogAction::None
            }
            KeyCode::End => {
                if self.focused_field <= 1 {
                    self.cursor_pos = self.current_field_text().len();
                }
                SaveDialogAction::None
            }
            _ => SaveDialogAction::None,
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

    fn move_to_save_button(&mut self) {
        self.focused_field = 2; // Save button
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
            self.cursor_pos += c.len_utf8();
            self.show_error = None;
        }
    }

    fn delete_char(&mut self) {
        if self.focused_field <= 1 && self.cursor_pos > 0 {
            let cursor_pos = self.cursor_pos;
            let text = self.current_field_text_mut();
            let mut chars: Vec<char> = text.chars().collect();
            if !chars.is_empty() && cursor_pos > 0 {
                let char_pos = text
                    .char_indices()
                    .enumerate()
                    .find(|(_, (byte_pos, _))| *byte_pos >= cursor_pos)
                    .map(|(char_idx, _)| char_idx)
                    .unwrap_or(chars.len());
                if char_pos > 0 {
                    let removed_char = chars.remove(char_pos - 1);
                    *text = chars.into_iter().collect();
                    self.cursor_pos -= removed_char.len_utf8();
                }
            }
            self.show_error = None;
        }
    }

    fn move_cursor_left(&mut self) {
        if self.focused_field <= 1 && self.cursor_pos > 0 {
            let text = self.current_field_text();
            if let Some((byte_pos, _)) = text
                .char_indices()
                .rev()
                .find(|(byte_pos, _)| *byte_pos < self.cursor_pos)
            {
                self.cursor_pos = byte_pos;
            } else {
                self.cursor_pos = 0;
            }
        }
    }

    fn move_cursor_right(&mut self) {
        if self.focused_field <= 1 {
            let text = self.current_field_text();
            if let Some((byte_pos, _)) = text
                .char_indices()
                .find(|(byte_pos, _)| *byte_pos > self.cursor_pos)
            {
                self.cursor_pos = byte_pos;
            } else {
                self.cursor_pos = text.len();
            }
        }
    }

    /// Validate the dialog input
    pub fn validate(&mut self) -> bool {
        let name = self.session_name.trim();
        if name.is_empty() {
            self.show_error = Some("Session name cannot be empty".to_string());
            self.focused_field = 0; // Focus name field
            return false;
        }

        // Check for invalid characters in session name
        if name
            .chars()
            .any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        {
            self.show_error = Some("Session name contains invalid characters".to_string());
            self.focused_field = 0;
            return false;
        }

        self.show_error = None;
        true
    }

    /// Set saving state
    pub fn set_saving(&mut self, saving: bool) {
        self.saving = saving;
    }

    /// Set error message
    pub fn set_error(&mut self, error: String) {
        self.show_error = Some(error);
    }
}

impl Default for SaveDialogState {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions that can be triggered by the save dialog
#[derive(Debug, Clone, PartialEq)]
pub enum SaveDialogAction {
    None,
    Save,
    Cancel,
}

/// Save Session Dialog widget
pub struct SaveDialog<'a> {
    state: &'a SaveDialogState,
}

impl<'a> SaveDialog<'a> {
    pub fn new(state: &'a SaveDialogState) -> Self {
        Self { state }
    }

    fn render_input_field(
        &self,
        area: Rect,
        buf: &mut Buffer,
        title: &str,
        text: &str,
        focused: bool,
        cursor_pos: usize,
    ) {
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Gray)
            });

        let inner = block.inner(area);

        // Render the block
        block.render(area, buf);

        // Render the text
        let content = if text.is_empty() && !focused {
            Span::styled(
                match title {
                    "Session Name" => "Enter session name...",
                    "Description (optional)" => "Enter description...",
                    _ => "",
                },
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
        if focused && !text.is_empty() {
            let cursor_x = inner.x + (cursor_pos.min(text.len()) as u16);
            if cursor_x < inner.right() {
                buf.cell_mut((cursor_x, inner.y))
                    .set_style(Style::default().bg(Color::White).fg(Color::Black));
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
            .border_style(style);

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(style);

        let inner = block.inner(area);
        block.render(area, buf);
        paragraph.render(inner, buf);
    }
}

impl<'a> WidgetRef for SaveDialog<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        // Clear the background
        Clear.render(area, buf);

        // Create the main dialog box
        let dialog_width = 60;
        let dialog_height = 15;
        let x = (area.width.saturating_sub(dialog_width)) / 2;
        let y = (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        // Main dialog block
        let block = Block::default()
            .title(" Save Session ")
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Blue));

        let inner = block.inner(dialog_area);
        block.render(dialog_area, buf);
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Save location info
                Constraint::Length(1), // Spacing
                Constraint::Length(3), // Session name input
                Constraint::Length(3), // Description input
                Constraint::Length(1), // Spacing
                Constraint::Length(2), // Error message area
                Constraint::Length(3), // Buttons
            ])
            .split(inner);

        // Save location info
        let save_location_text = format!("Save to: {}", self.state.save_location.display());
        let location_paragraph = Paragraph::new(save_location_text)
            .style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )
            .alignment(Alignment::Center);
        location_paragraph.render(layout[0], buf);

        // Session name input
        self.render_input_field(
            layout[2],
            buf,
            "Session Name",
            &self.state.session_name,
            self.state.focused_field == 0,
            self.state.cursor_pos,
        );

        // Description input
        self.render_input_field(
            layout[3],
            buf,
            "Description (optional)",
            &self.state.description,
            self.state.focused_field == 1,
            if self.state.focused_field == 1 {
                self.state.cursor_pos
            } else {
                0
            },
        );

        // Error message
        if let Some(ref error) = self.state.show_error {
            let error_paragraph = Paragraph::new(error.as_str())
                .style(Style::default().fg(Color::Red))
                .alignment(Alignment::Center);
            error_paragraph.render(layout[5], buf);
        }

        // Buttons
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(layout[6]);

        let save_text = if self.state.saving {
            "Saving..."
        } else {
            "Save"
        };
        let save_disabled = self.state.saving || self.state.session_name.trim().is_empty();

        self.render_button(
            button_layout[1],
            buf,
            save_text,
            self.state.focused_field == 2,
            save_disabled,
        );

        self.render_button(
            button_layout[2],
            buf,
            "Cancel",
            self.state.focused_field == 3,
            self.state.saving,
        );

        // Help text at the bottom
        let help_text = if self.state.saving {
            "Please wait..."
        } else {
            "Tab/Shift+Tab: Navigate • Enter: Save/Next • Esc: Cancel"
        };

        let help_area = Rect::new(
            dialog_area.x,
            dialog_area.y + dialog_area.height,
            dialog_area.width,
            1,
        );

        if help_area.y < area.height {
            let help_paragraph = Paragraph::new(help_text)
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            help_paragraph.render(help_area, buf);
        }
    }
}
