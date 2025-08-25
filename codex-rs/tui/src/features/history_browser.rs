//! History Browser widget for visual timeline navigation of conversation
//!
//! This widget provides a tree-like visualization of the conversation history,
//! showing branches and allowing navigation through different conversation paths.

use std::collections::HashMap;
use std::time::SystemTime;

use agcodex_core::models::ResponseItem;
use ratatui::buffer::Buffer;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::symbols;
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
use uuid::Uuid;

/// Represents a node in the conversation tree
#[derive(Debug, Clone)]
pub struct HistoryNode {
    /// Unique identifier for this node
    pub id: Uuid,
    /// Index in the linear conversation
    pub index: usize,
    /// The actual response item
    pub item: ResponseItem,
    /// Parent node ID (None for root)
    pub parent: Option<Uuid>,
    /// Children node IDs (for branches)
    pub children: Vec<Uuid>,
    /// Timestamp of creation
    pub timestamp: SystemTime,
    /// Branch name if this is a branch point
    pub branch_name: Option<String>,
    /// Whether this node is on the active path
    pub is_active: bool,
}

impl HistoryNode {
    /// Create a new history node
    pub fn new(index: usize, item: ResponseItem, parent: Option<Uuid>) -> Self {
        Self {
            id: Uuid::new_v4(),
            index,
            item,
            parent,
            children: Vec::new(),
            timestamp: SystemTime::now(),
            branch_name: None,
            is_active: true,
        }
    }

    /// Get a preview of the node content
    pub fn preview(&self) -> String {
        match &self.item {
            ResponseItem::Message { role, content, .. } => {
                let text = content
                    .iter()
                    .filter_map(|c| match c {
                        agcodex_core::models::ContentItem::InputText { text }
                        | agcodex_core::models::ContentItem::OutputText { text } => {
                            Some(text.as_str())
                        }
                        agcodex_core::models::ContentItem::InputImage { .. } => Some("[Image]"),
                    })
                    .collect::<Vec<_>>()
                    .join(" ");

                let preview = if text.len() > 80 {
                    format!("{}...", &text[..77])
                } else {
                    text
                };

                format!("[{}] {}", role, preview)
            }
            ResponseItem::Reasoning { .. } => "[Reasoning]".to_string(),
            ResponseItem::FunctionCall { name, .. } => format!("[Function: {}]", name),
            ResponseItem::LocalShellCall { action, .. } => format!("[Shell: {:?}]", action),
            ResponseItem::FunctionCallOutput { .. } => "[Function Output]".to_string(),
            ResponseItem::Other => "[Other]".to_string(),
        }
    }

    /// Get the role for coloring
    pub const fn role(&self) -> &str {
        match &self.item {
            ResponseItem::Message { role, .. } => role.as_str(),
            ResponseItem::Reasoning { .. } => "reasoning",
            ResponseItem::FunctionCall { .. } => "function",
            ResponseItem::LocalShellCall { .. } => "shell",
            ResponseItem::FunctionCallOutput { .. } => "output",
            ResponseItem::Other => "other",
        }
    }
}

/// Conversation tree structure for history browsing
#[derive(Debug, Clone)]
pub struct ConversationTree {
    /// All nodes indexed by ID
    nodes: HashMap<Uuid, HistoryNode>,
    /// Root node ID
    root: Option<Uuid>,
    /// Currently active path (node IDs from root to current)
    active_path: Vec<Uuid>,
    /// Currently selected node for preview
    selected_node: Option<Uuid>,
}

impl ConversationTree {
    /// Create a new conversation tree
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root: None,
            active_path: Vec::new(),
            selected_node: None,
        }
    }

    /// Add a node to the tree
    pub fn add_node(&mut self, node: HistoryNode) -> Uuid {
        let id = node.id;

        // Update parent's children list
        if let Some(parent_id) = node.parent {
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                parent.children.push(id);
            }
        } else {
            // This is a root node
            self.root = Some(id);
        }

        // Mark as active and add to active path
        if node.is_active {
            self.active_path.push(id);
        }

        self.nodes.insert(id, node);
        id
    }

    /// Create a branch from a specific node
    pub fn create_branch(
        &mut self,
        from_node: Uuid,
        branch_name: String,
        item: ResponseItem,
    ) -> Option<Uuid> {
        let parent_node = self.nodes.get(&from_node)?;
        let new_index = parent_node.index + 1;

        let mut new_node = HistoryNode::new(new_index, item, Some(from_node));
        new_node.branch_name = Some(branch_name);
        new_node.is_active = false; // New branches start inactive

        Some(self.add_node(new_node))
    }

    /// Switch to a different branch
    pub fn switch_to_branch(&mut self, target_node: Uuid) -> bool {
        if !self.nodes.contains_key(&target_node) {
            return false;
        }

        // Mark old path as inactive
        for id in &self.active_path {
            if let Some(node) = self.nodes.get_mut(id) {
                node.is_active = false;
            }
        }

        // Build new active path from root to target
        let mut new_path = Vec::new();
        let mut current = Some(target_node);

        while let Some(node_id) = current {
            new_path.push(node_id);
            current = self.nodes.get(&node_id).and_then(|n| n.parent);
        }

        new_path.reverse();

        // Mark new path as active
        for id in &new_path {
            if let Some(node) = self.nodes.get_mut(id) {
                node.is_active = true;
            }
        }

        self.active_path = new_path;
        true
    }

    /// Get nodes for rendering (in tree order)
    pub fn get_render_nodes(&self) -> Vec<(usize, Uuid, bool)> {
        let mut result = Vec::new();
        if let Some(root_id) = self.root {
            self.collect_nodes_recursive(root_id, 0, &mut result);
        }
        result
    }

    /// Recursively collect nodes for rendering
    fn collect_nodes_recursive(
        &self,
        node_id: Uuid,
        depth: usize,
        result: &mut Vec<(usize, Uuid, bool)>,
    ) {
        if let Some(node) = self.nodes.get(&node_id) {
            let is_selected = self.selected_node == Some(node_id);
            result.push((depth, node_id, is_selected));

            // Add children
            for child_id in &node.children {
                self.collect_nodes_recursive(*child_id, depth + 1, result);
            }
        }
    }
}

impl Default for ConversationTree {
    fn default() -> Self {
        Self::new()
    }
}

/// History Browser widget for visual timeline navigation
#[allow(dead_code)]
pub struct HistoryBrowser {
    /// The conversation tree structure
    tree: ConversationTree,
    /// Whether the browser is visible
    visible: bool,
    /// Scroll offset for the tree view
    scroll_offset: usize,
    /// Maximum visible items
    max_visible: usize,
    /// Show preview pane
    show_preview: bool,
    /// Preview context lines
    preview_context: usize,
}

impl HistoryBrowser {
    /// Create a new history browser
    pub fn new() -> Self {
        Self {
            tree: ConversationTree::new(),
            visible: false,
            scroll_offset: 0,
            max_visible: 20,
            show_preview: true,
            preview_context: 3,
        }
    }

    /// Show the history browser with conversation items
    pub fn show(&mut self, items: Vec<ResponseItem>) {
        self.tree = ConversationTree::new();

        let mut parent: Option<Uuid> = None;
        for (index, item) in items.into_iter().enumerate() {
            let node = HistoryNode::new(index, item, parent);
            parent = Some(self.tree.add_node(node));
        }

        self.visible = true;

        // Select the last node by default
        if let Some(last_id) = self.tree.active_path.last() {
            self.tree.selected_node = Some(*last_id);
        }
    }

    /// Hide the history browser
    pub const fn hide(&mut self) {
        self.visible = false;
        self.scroll_offset = 0;
    }

    /// Check if the browser is visible
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    /// Move selection up in the tree
    pub fn move_up(&mut self) {
        let nodes = self.tree.get_render_nodes();
        if let Some(current_idx) = nodes.iter().position(|(_, _, selected)| *selected)
            && current_idx > 0
        {
            let (_, prev_node, _) = nodes[current_idx - 1];
            self.tree.selected_node = Some(prev_node);
            self.ensure_visible(current_idx - 1, nodes.len());
        }
    }

    /// Move selection down in the tree
    pub fn move_down(&mut self) {
        let nodes = self.tree.get_render_nodes();
        if let Some(current_idx) = nodes.iter().position(|(_, _, selected)| *selected)
            && current_idx < nodes.len() - 1
        {
            let (_, next_node, _) = nodes[current_idx + 1];
            self.tree.selected_node = Some(next_node);
            self.ensure_visible(current_idx + 1, nodes.len());
        }
    }

    /// Ensure selected item is visible
    fn ensure_visible(&mut self, index: usize, total: usize) {
        if index < self.scroll_offset {
            self.scroll_offset = index;
        } else if index >= self.scroll_offset + self.max_visible {
            self.scroll_offset = index.saturating_sub(self.max_visible - 1);
        }

        // Clamp scroll offset
        self.scroll_offset = self
            .scroll_offset
            .min(total.saturating_sub(self.max_visible));
    }

    /// Get the currently selected node
    pub fn selected_node(&self) -> Option<&HistoryNode> {
        self.tree
            .selected_node
            .and_then(|id| self.tree.nodes.get(&id))
    }

    /// Create a branch from the selected node
    pub fn create_branch_from_selected(&mut self, name: String, item: ResponseItem) -> bool {
        if let Some(selected_id) = self.tree.selected_node {
            self.tree.create_branch(selected_id, name, item).is_some()
        } else {
            false
        }
    }

    /// Switch to the selected branch
    pub fn switch_to_selected_branch(&mut self) -> bool {
        if let Some(selected_id) = self.tree.selected_node {
            self.tree.switch_to_branch(selected_id)
        } else {
            false
        }
    }

    /// Toggle preview pane
    pub const fn toggle_preview(&mut self) {
        self.show_preview = !self.show_preview;
    }
}

impl Default for HistoryBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for HistoryBrowser {
    fn render(self, area: Rect, buf: &mut Buffer) {
        WidgetRef::render_ref(&self, area, buf);
    }
}

impl WidgetRef for HistoryBrowser {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        // Clear the background
        Clear.render(area, buf);

        // Create the main block
        let block = Block::default()
            .title("History Browser (Ctrl+H)")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Magenta));

        let inner = block.inner(area);
        block.render(area, buf);

        // Split into tree view and preview if enabled
        let chunks = if self.show_preview {
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(inner)
                .to_vec()
        } else {
            vec![inner]
        };

        // Render the tree view
        self.render_tree_view(chunks[0], buf);

        // Render preview if enabled
        if self.show_preview && chunks.len() > 1 {
            self.render_preview(chunks[1], buf);
        }
    }
}

impl HistoryBrowser {
    /// Render the tree view of conversation history
    fn render_tree_view(&self, area: Rect, buf: &mut Buffer) {
        let nodes = self.tree.get_render_nodes();

        // Create lines for the tree visualization
        let mut lines = Vec::new();

        // Add header
        lines.push(Line::from(vec![
            Span::styled("↑↓", Style::default().fg(Color::Gray)),
            Span::raw(" Navigate | "),
            Span::styled("Enter", Style::default().fg(Color::Gray)),
            Span::raw(" Jump | "),
            Span::styled("b", Style::default().fg(Color::Gray)),
            Span::raw(" Branch | "),
            Span::styled("p", Style::default().fg(Color::Gray)),
            Span::raw(" Preview"),
        ]));
        lines.push(Line::from(""));

        // Render visible nodes
        let end = (self.scroll_offset + self.max_visible).min(nodes.len());
        for (depth, node_id, is_selected) in &nodes[self.scroll_offset..end] {
            let node = match self.tree.nodes.get(node_id) {
                Some(n) => n,
                None => continue,
            };

            let indent = "  ".repeat(*depth);

            // Choose tree symbols
            let symbol = if node.children.is_empty() {
                symbols::line::VERTICAL_RIGHT
            } else if node.children.len() > 1 {
                "├┬"
            } else {
                symbols::line::VERTICAL_RIGHT
            };

            // Style based on role and selection
            let (role_style, preview_style) = if *is_selected {
                (
                    get_role_style(node.role()).add_modifier(Modifier::BOLD | Modifier::REVERSED),
                    Style::default().add_modifier(Modifier::REVERSED),
                )
            } else if node.is_active {
                (
                    get_role_style(node.role()).add_modifier(Modifier::BOLD),
                    Style::default(),
                )
            } else {
                (
                    get_role_style(node.role()).add_modifier(Modifier::DIM),
                    Style::default().fg(Color::DarkGray),
                )
            };

            // Build the line
            let mut spans = vec![
                Span::raw(indent),
                Span::styled(symbol, Style::default().fg(Color::Gray)),
                Span::raw(" "),
                Span::styled(format!("#{} ", node.index + 1), role_style),
            ];

            // Add branch name if present
            if let Some(branch_name) = &node.branch_name {
                spans.push(Span::styled(
                    format!("[{}] ", branch_name),
                    Style::default().fg(Color::Yellow),
                ));
            }

            spans.push(Span::styled(node.preview(), preview_style));

            lines.push(Line::from(spans));
        }

        // Add scroll indicators if needed
        if self.scroll_offset > 0 {
            lines[1] = Line::from(vec![
                Span::styled("▲ ", Style::default().fg(Color::Yellow)),
                Span::raw("More above..."),
            ]);
        }

        if end < nodes.len() {
            lines.push(Line::from(vec![
                Span::styled("▼ ", Style::default().fg(Color::Yellow)),
                Span::raw("More below..."),
            ]));
        }

        let tree_text = Text::from(lines);
        let tree_paragraph = Paragraph::new(tree_text);
        tree_paragraph.render(area, buf);
    }

    /// Render the preview pane
    fn render_preview(&self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .title("Preview")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));

        let inner = block.inner(area);
        block.render(area, buf);

        if let Some(node) = self.selected_node() {
            // Get full message content
            let content = match &node.item {
                ResponseItem::Message { role, content, .. } => {
                    let text = content
                        .iter()
                        .filter_map(|c| match c {
                            agcodex_core::models::ContentItem::InputText { text }
                            | agcodex_core::models::ContentItem::OutputText { text } => {
                                Some(text.as_str())
                            }
                            agcodex_core::models::ContentItem::InputImage { .. } => Some("[Image]"),
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    format!("Role: {}\n\n{}", role, text)
                }
                ResponseItem::Reasoning { content, .. } => match content {
                    Some(items) => {
                        let reasoning_text = items
                            .iter()
                            .map(|item| format!("{:?}", item))
                            .collect::<Vec<_>>()
                            .join("\n");
                        format!("Reasoning:\n\n{}", reasoning_text)
                    }
                    None => "Reasoning: (empty)".to_string(),
                },
                ResponseItem::FunctionCall {
                    name, arguments, ..
                } => {
                    format!("Function Call: {}\nArguments: {}", name, arguments)
                }
                ResponseItem::LocalShellCall { action, .. } => {
                    format!("Shell Command:\n{:?}", action)
                }
                ResponseItem::FunctionCallOutput {
                    call_id, output, ..
                } => {
                    format!("Function Output: {}\n\n{}", call_id, output)
                }
                ResponseItem::Other => "Other content".to_string(),
            };

            // Create preview text with word wrapping
            let wrapped_lines: Vec<String> = content
                .lines()
                .flat_map(|line| {
                    if line.len() <= inner.width as usize {
                        vec![line.to_string()]
                    } else {
                        // Simple word wrap
                        let mut wrapped = Vec::new();
                        let mut current = String::new();
                        for word in line.split_whitespace() {
                            if current.len() + word.len() + 1 > inner.width as usize
                                && !current.is_empty()
                            {
                                wrapped.push(current.clone());
                                current.clear();
                            }
                            if !current.is_empty() {
                                current.push(' ');
                            }
                            current.push_str(word);
                        }
                        if !current.is_empty() {
                            wrapped.push(current);
                        }
                        wrapped
                    }
                })
                .collect();

            // Take only what fits in the preview area
            let preview_lines: Vec<Line> = wrapped_lines
                .iter()
                .take(inner.height as usize)
                .map(|s| Line::from(s.as_str()))
                .collect();

            let preview_text = Text::from(preview_lines);
            let preview_paragraph = Paragraph::new(preview_text).alignment(Alignment::Left);
            preview_paragraph.render(inner, buf);
        } else {
            let no_selection = Paragraph::new("No message selected")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center);
            no_selection.render(inner, buf);
        }
    }
}

/// Get style for a role
fn get_role_style(role: &str) -> Style {
    match role {
        "user" => Style::default().fg(Color::Green),
        "assistant" => Style::default().fg(Color::Blue),
        "system" => Style::default().fg(Color::Yellow),
        "reasoning" => Style::default().fg(Color::Magenta),
        "function" | "shell" => Style::default().fg(Color::Cyan),
        "output" => Style::default().fg(Color::Gray),
        _ => Style::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agcodex_core::models::ContentItem;

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
    fn test_history_node_creation() {
        let item = create_test_message("user", "Test message");
        let node = HistoryNode::new(0, item, None);

        assert_eq!(node.index, 0);
        assert!(node.parent.is_none());
        assert!(node.children.is_empty());
        assert!(node.is_active);
    }

    #[test]
    fn test_conversation_tree_building() {
        let mut tree = ConversationTree::new();

        let root_item = create_test_message("user", "First message");
        let root_node = HistoryNode::new(0, root_item, None);
        let root_id = tree.add_node(root_node);

        assert_eq!(tree.root, Some(root_id));
        assert_eq!(tree.active_path, vec![root_id]);

        let child_item = create_test_message("assistant", "Response");
        let child_node = HistoryNode::new(1, child_item, Some(root_id));
        let child_id = tree.add_node(child_node);

        assert_eq!(tree.active_path, vec![root_id, child_id]);
        assert_eq!(tree.nodes.get(&root_id).unwrap().children, vec![child_id]);
    }

    #[test]
    fn test_branching() {
        let mut tree = ConversationTree::new();

        let root_item = create_test_message("user", "Root");
        let root_node = HistoryNode::new(0, root_item, None);
        let root_id = tree.add_node(root_node);

        let branch_item = create_test_message("assistant", "Branch");
        let branch_id = tree.create_branch(root_id, "Alternative".to_string(), branch_item);

        assert!(branch_id.is_some());
        let branch_id = branch_id.unwrap();

        assert_eq!(tree.nodes.get(&root_id).unwrap().children.len(), 1);
        assert_eq!(
            tree.nodes.get(&branch_id).unwrap().branch_name,
            Some("Alternative".to_string())
        );
        assert!(!tree.nodes.get(&branch_id).unwrap().is_active);
    }

    #[test]
    fn test_history_browser_visibility() {
        let mut browser = HistoryBrowser::new();
        assert!(!browser.is_visible());

        browser.show(vec![
            create_test_message("user", "Hello"),
            create_test_message("assistant", "Hi"),
        ]);
        assert!(browser.is_visible());

        browser.hide();
        assert!(!browser.is_visible());
    }
}
