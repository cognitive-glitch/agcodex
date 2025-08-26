//! Message Jump widget for navigating to any previous message in the conversation

use std::time::SystemTime;

use agcodex_core::models::ContentItem;
use agcodex_core::models::ResponseItem;
use nucleo_matcher::Matcher;
use nucleo_matcher::Utf32Str;
use ratatui::buffer::Buffer;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;

use crate::bottom_pane::popup_consts::MAX_POPUP_ROWS;
use crate::bottom_pane::scroll_state::ScrollState;
use crate::bottom_pane::selection_popup_common::GenericDisplayRow;
use crate::bottom_pane::selection_popup_common::render_rows;

/// Represents a single message entry in the jump list
#[derive(Debug, Clone)]
pub struct MessageEntry {
    /// Index in the conversation history
    pub index: usize,
    /// Role of the message (user, assistant, system)
    pub role: String,
    /// Content preview (first 100 chars)
    pub preview: String,
    /// Full content for search matching
    pub full_content: String,
    /// Timestamp if available
    pub timestamp: Option<SystemTime>,
    /// Original ResponseItem reference for context restoration
    pub item: ResponseItem,
}

impl MessageEntry {
    /// Create a new message entry from a ResponseItem
    pub fn new(index: usize, item: ResponseItem) -> Self {
        let (role, content) = match &item {
            ResponseItem::Message { role, content, .. } => {
                let text = extract_text_content(content);
                (role.clone(), text)
            }
            ResponseItem::Reasoning { .. } => {
                ("reasoning".to_string(), "Reasoning content".to_string())
            }
            ResponseItem::FunctionCall { name, .. } => {
                ("function".to_string(), format!("Function call: {}", name))
            }
            ResponseItem::LocalShellCall { action, .. } => {
                ("shell".to_string(), format!("Shell: {:?}", action))
            }
            ResponseItem::FunctionCallOutput { .. } => {
                ("function_output".to_string(), "Function output".to_string())
            }
            ResponseItem::Other => ("other".to_string(), "Other content".to_string()),
        };

        let preview = if content.len() > 100 {
            format!("{}...", &content[..97])
        } else {
            content.clone()
        };

        Self {
            index,
            role,
            preview,
            full_content: content,
            timestamp: None, // TODO: Extract timestamp from ResponseItem if available
            item,
        }
    }

    /// Get formatted display text for this message
    pub fn display_text(&self) -> String {
        format!("#{}: [{}] {}", self.index + 1, self.role, self.preview)
    }
}

/// Filter options for message roles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleFilter {
    All,
    User,
    Assistant,
    System,
    Function,
    Other,
}

impl RoleFilter {
    pub fn matches(&self, role: &str) -> bool {
        match self {
            RoleFilter::All => true,
            RoleFilter::User => role == "user",
            RoleFilter::Assistant => role == "assistant",
            RoleFilter::System => role == "system",
            RoleFilter::Function => role == "function" || role == "function_output",
            RoleFilter::Other => !matches!(
                role,
                "user" | "assistant" | "system" | "function" | "function_output"
            ),
        }
    }

    pub const fn display_name(&self) -> &'static str {
        match self {
            RoleFilter::All => "All",
            RoleFilter::User => "User",
            RoleFilter::Assistant => "Assistant",
            RoleFilter::System => "System",
            RoleFilter::Function => "Function",
            RoleFilter::Other => "Other",
        }
    }

    pub const fn cycle_next(&self) -> Self {
        match self {
            RoleFilter::All => RoleFilter::User,
            RoleFilter::User => RoleFilter::Assistant,
            RoleFilter::Assistant => RoleFilter::System,
            RoleFilter::System => RoleFilter::Function,
            RoleFilter::Function => RoleFilter::Other,
            RoleFilter::Other => RoleFilter::All,
        }
    }
}

/// Visual state for the message jump popup
pub struct MessageJump {
    /// All available messages
    all_messages: Vec<MessageEntry>,
    /// Currently filtered and displayed messages
    filtered_messages: Vec<MessageEntry>,
    /// Current search query
    search_query: String,
    /// Current role filter
    role_filter: RoleFilter,
    /// Fuzzy matcher for search
    matcher: Matcher,
    /// Selection and scroll state
    state: ScrollState,
    /// Whether the popup is currently visible
    visible: bool,
    /// Context preview lines (messages before/after selected)
    context_lines: usize,
}

impl MessageJump {
    /// Create a new message jump widget
    pub fn new() -> Self {
        Self {
            all_messages: Vec::new(),
            filtered_messages: Vec::new(),
            search_query: String::new(),
            role_filter: RoleFilter::All,
            matcher: Matcher::new(nucleo_matcher::Config::DEFAULT),
            state: ScrollState::new(),
            visible: false,
            context_lines: 2,
        }
    }

    /// Show the popup and load messages from conversation history
    pub fn show(&mut self, messages: Vec<ResponseItem>) {
        self.all_messages = messages
            .into_iter()
            .enumerate()
            .map(|(i, item)| MessageEntry::new(i, item))
            .collect();

        self.visible = true;
        self.apply_filters();

        // Select the last message by default
        if !self.filtered_messages.is_empty() {
            self.state.selected_idx = Some(self.filtered_messages.len() - 1);
        }
    }

    /// Hide the popup
    pub fn hide(&mut self) {
        self.visible = false;
        self.search_query.clear();
        self.role_filter = RoleFilter::All;
        self.state.reset();
    }

    /// Check if the popup is currently visible
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    /// Update the search query and refresh filtering
    pub fn set_search_query(&mut self, query: String) {
        self.search_query = query;
        self.apply_filters();
    }

    /// Get the current search query
    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    /// Cycle to the next role filter
    pub fn cycle_role_filter(&mut self) {
        self.role_filter = self.role_filter.cycle_next();
        self.apply_filters();
    }

    /// Get the current role filter
    pub const fn role_filter(&self) -> RoleFilter {
        self.role_filter
    }

    /// Move selection up
    pub fn move_up(&mut self) {
        let len = self.filtered_messages.len();
        self.state.move_up_wrap(len);
        self.state.ensure_visible(len, len.min(MAX_POPUP_ROWS));
    }

    /// Move selection down  
    pub fn move_down(&mut self) {
        let len = self.filtered_messages.len();
        self.state.move_down_wrap(len);
        self.state.ensure_visible(len, len.min(MAX_POPUP_ROWS));
    }

    /// Get the currently selected message entry
    pub fn selected_message(&self) -> Option<&MessageEntry> {
        self.state
            .selected_idx
            .and_then(|idx| self.filtered_messages.get(idx))
    }

    /// Get context messages around the selected message
    pub fn get_context_messages(&self) -> Option<Vec<&MessageEntry>> {
        let selected = self.selected_message()?;
        let selected_index = selected.index;

        let start = selected_index.saturating_sub(self.context_lines);
        let end = (selected_index + self.context_lines + 1).min(self.all_messages.len());

        Some(self.all_messages[start..end].iter().collect())
    }

    /// Apply current search and role filters
    fn apply_filters(&mut self) {
        self.filtered_messages.clear();

        for message in &self.all_messages {
            // Apply role filter
            if !self.role_filter.matches(&message.role) {
                continue;
            }

            // Apply search filter
            if !self.search_query.is_empty() {
                let mut haystack_buf = Vec::new();
                let mut needle_buf = Vec::new();
                let haystack = Utf32Str::new(&message.full_content, &mut haystack_buf);
                let needle = Utf32Str::new(&self.search_query, &mut needle_buf);

                if self.matcher.fuzzy_match(haystack, needle).is_none() {
                    // Also try matching against the display text
                    let display_text = message.display_text();
                    let mut display_buf = Vec::new();
                    let display_haystack = Utf32Str::new(&display_text, &mut display_buf);
                    if self.matcher.fuzzy_match(display_haystack, needle).is_none() {
                        continue;
                    }
                }
            }

            self.filtered_messages.push(message.clone());
        }

        // Update selection state
        let len = self.filtered_messages.len();
        self.state.clamp_selection(len);
        self.state.ensure_visible(len, len.min(MAX_POPUP_ROWS));
    }

    /// Calculate the required height for the popup
    pub fn calculate_required_height(&self) -> u16 {
        if !self.visible {
            return 0;
        }

        // Base height for the message list
        let list_height = self.filtered_messages.len().clamp(1, MAX_POPUP_ROWS) as u16;

        // Add space for search bar and filter indicator
        let ui_height = 4; // Search bar (1) + filter line (1) + borders (2)

        // Add space for context preview if a message is selected
        let context_height = if self.selected_message().is_some() {
            (self.context_lines * 2 + 1) as u16 // Before + selected + after
        } else {
            0
        };

        list_height + ui_height + context_height
    }
}

impl Default for MessageJump {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for MessageJump {
    fn render(self, area: Rect, buf: &mut Buffer) {
        WidgetRef::render_ref(&self, area, buf);
    }
}

impl WidgetRef for MessageJump {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Clear the background
        Clear.render(area, buf);

        // Create the main block
        let block = Block::default()
            .title("Message Jump (Ctrl+J)")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);
        block.render(area, buf);

        // Layout: Search bar, filter indicator, message list, context preview
        let chunks = Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Search bar
                Constraint::Length(1), // Filter indicator
                Constraint::Min(3),    // Message list
                Constraint::Length(if self.selected_message().is_some() {
                    (self.context_lines * 2 + 3) as u16
                } else {
                    0
                }), // Context preview
            ])
            .split(inner);

        // Render search bar
        let search_text = if self.search_query.is_empty() {
            Text::from("Type to search messages...").style(Style::default().fg(Color::DarkGray))
        } else {
            Text::from(self.search_query.clone())
        };

        let search_paragraph =
            Paragraph::new(search_text).block(Block::default().borders(Borders::BOTTOM));
        search_paragraph.render(chunks[0], buf);

        // Render filter indicator
        let filter_text = format!(
            "Filter: {} | {} messages | Tab: cycle filters | ↑/k ↓/j: navigate | Enter: jump | Esc: cancel",
            self.role_filter.display_name(),
            self.filtered_messages.len()
        );
        let filter_paragraph = Paragraph::new(filter_text)
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        filter_paragraph.render(chunks[1], buf);

        // Render message list
        if chunks.len() > 2 {
            self.render_message_list(chunks[2], buf);
        }

        // Render context preview
        if chunks.len() > 3 && chunks[3].height > 0 {
            self.render_context_preview(chunks[3], buf);
        }
    }
}

impl MessageJump {
    /// Render the message list with fuzzy matching highlights
    fn render_message_list(&self, area: Rect, buf: &mut Buffer) {
        let rows: Vec<GenericDisplayRow> = if self.filtered_messages.is_empty() {
            Vec::new()
        } else {
            self.filtered_messages
                .iter()
                .map(|msg| {
                    let display_text = msg.display_text();

                    // Calculate fuzzy match indices for highlighting
                    // For now, just do simple substring highlighting
                    let match_indices = if !self.search_query.is_empty() {
                        let query_lower = self.search_query.to_lowercase();
                        let text_lower = display_text.to_lowercase();
                        if let Some(start) = text_lower.find(&query_lower) {
                            let indices: Vec<usize> = (start..start + query_lower.len()).collect();
                            Some(indices)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    GenericDisplayRow {
                        name: display_text,
                        match_indices,
                        is_current: false, // TODO: Mark current message if available
                        description: msg.timestamp.map(|_| "timestamp".to_string()),
                    }
                })
                .collect()
        };

        render_rows(area, buf, &rows, &self.state, MAX_POPUP_ROWS, false);
    }

    /// Render context preview showing messages before/after selected
    fn render_context_preview(&self, area: Rect, buf: &mut Buffer) {
        let Some(context_messages) = self.get_context_messages() else {
            return;
        };

        let block = Block::default()
            .title("Context Preview")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));

        let inner = block.inner(area);
        block.render(area, buf);

        let selected_index = self.selected_message().map(|m| m.index);

        let mut lines = Vec::new();
        for (i, msg) in context_messages.iter().enumerate() {
            let is_selected = selected_index == Some(msg.index);
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };

            let prefix = if is_selected { "► " } else { "  " };
            let line = Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("#{}: ", msg.index + 1), style),
                Span::styled(format!("[{}] ", msg.role), style),
                Span::styled(&msg.preview, style),
            ]);
            lines.push(line);

            // Don't exceed the available height
            if i >= inner.height as usize {
                break;
            }
        }

        let context_text = Text::from(lines);
        let context_paragraph = Paragraph::new(context_text);
        context_paragraph.render(inner, buf);
    }
}

/// Extract text content from ContentItem vector
fn extract_text_content(content: &[ContentItem]) -> String {
    content
        .iter()
        .map(|item| match item {
            ContentItem::InputText { text } | ContentItem::OutputText { text } => text.as_str(),
            ContentItem::InputImage { .. } => "[Image]",
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use agcodex_core::models::ResponseItem;

    fn create_test_message(role: &str, content: &str) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: role.to_string(),
            content: vec![ContentItem::OutputText {
                text: content.to_string(),
            }],
        }
    }

    #[test]
    fn test_message_entry_creation() {
        let item = create_test_message("user", "Hello, world!");
        let entry = MessageEntry::new(0, item);

        assert_eq!(entry.index, 0);
        assert_eq!(entry.role, "user");
        assert_eq!(entry.preview, "Hello, world!");
        assert_eq!(entry.full_content, "Hello, world!");
    }

    #[test]
    fn test_message_entry_preview_truncation() {
        let long_content = "a".repeat(150);
        let item = create_test_message("user", &long_content);
        let entry = MessageEntry::new(0, item);

        assert_eq!(entry.preview.len(), 100); // 97 chars + "..."
        assert!(entry.preview.ends_with("..."));
    }

    #[test]
    fn test_role_filter_matching() {
        assert!(RoleFilter::All.matches("user"));
        assert!(RoleFilter::All.matches("assistant"));
        assert!(RoleFilter::User.matches("user"));
        assert!(!RoleFilter::User.matches("assistant"));
        assert!(RoleFilter::Assistant.matches("assistant"));
        assert!(!RoleFilter::Assistant.matches("user"));
    }

    #[test]
    fn test_role_filter_cycling() {
        let filter = RoleFilter::All;
        assert_eq!(filter.cycle_next(), RoleFilter::User);

        let filter = RoleFilter::Other;
        assert_eq!(filter.cycle_next(), RoleFilter::All);
    }

    #[test]
    fn test_message_jump_filtering() {
        let mut jump = MessageJump::new();
        let messages = vec![
            create_test_message("user", "Hello"),
            create_test_message("assistant", "Hi there"),
            create_test_message("user", "How are you?"),
        ];

        jump.show(messages);
        assert_eq!(jump.filtered_messages.len(), 3);

        jump.role_filter = RoleFilter::User;
        jump.apply_filters();
        assert_eq!(jump.filtered_messages.len(), 2);

        jump.set_search_query("Hello".to_string());
        assert_eq!(jump.filtered_messages.len(), 1);
    }
}
