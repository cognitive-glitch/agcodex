//! Enhanced Agent Notification System for AGCodex TUI
//!
//! Provides visual and audio notifications for agent events with
//! auto-dismissal, stacking, and accessibility features.

use ratatui::buffer::Buffer;
use ratatui::layout::Alignment;
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
use std::collections::VecDeque;
use std::io::Write;
use std::io::{self};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;
use uuid::Uuid;

/// Notification types for different agent events
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationType {
    /// Agent started successfully
    Started,
    /// Agent made progress
    Progress,
    /// Agent completed successfully
    Complete,
    /// Agent failed with error
    Failed,
    /// Agent was cancelled
    Cancelled,
    /// General information
    Info,
    /// Warning message
    Warning,
}

impl NotificationType {
    /// Get icon for this notification type
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Started => "🚀",
            Self::Progress => "⚡",
            Self::Complete => "✓",
            Self::Failed => "✗",
            Self::Cancelled => "⊘",
            Self::Info => "ℹ",
            Self::Warning => "⚠",
        }
    }

    /// Get color for this notification type
    pub const fn color(&self) -> Color {
        match self {
            Self::Started => Color::Blue,
            Self::Progress => Color::Cyan,
            Self::Complete => Color::Green,
            Self::Failed => Color::Red,
            Self::Cancelled => Color::Yellow,
            Self::Info => Color::Gray,
            Self::Warning => Color::Yellow,
        }
    }

    /// Get border style for this notification type
    pub const fn border_color(&self) -> Color {
        match self {
            Self::Complete => Color::Green,
            Self::Failed => Color::Red,
            _ => Color::Cyan,
        }
    }

    /// Should this notification trigger a terminal bell?
    pub const fn should_bell(&self) -> bool {
        matches!(self, Self::Complete | Self::Failed)
    }

    /// Get default duration for auto-dismiss
    pub const fn default_duration(&self) -> Duration {
        match self {
            Self::Complete | Self::Failed => Duration::from_secs(5),
            Self::Progress => Duration::from_secs(3),
            _ => Duration::from_secs(4),
        }
    }
}

/// Individual notification with content and metadata
#[derive(Debug, Clone)]
pub struct Notification {
    /// Unique ID for this notification
    pub id: Uuid,
    /// Type of notification
    pub notification_type: NotificationType,
    /// Agent name or source
    pub agent_name: String,
    /// Main message
    pub message: String,
    /// Additional details (optional)
    pub details: Option<String>,
    /// When the notification was created
    pub created_at: Instant,
    /// How long to display before auto-dismiss
    pub duration: Duration,
    /// Whether this has been acknowledged by user
    pub acknowledged: bool,
    /// Progress percentage (for progress notifications)
    pub progress: Option<f32>,
}

impl Notification {
    /// Create a new notification
    pub fn new(notification_type: NotificationType, agent_name: String, message: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            notification_type: notification_type.clone(),
            agent_name,
            message,
            details: None,
            created_at: Instant::now(),
            duration: notification_type.default_duration(),
            acknowledged: false,
            progress: None,
        }
    }

    /// Set additional details
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    /// Set custom duration
    pub const fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Set progress value
    pub const fn with_progress(mut self, progress: f32) -> Self {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self
    }

    /// Check if notification should be dismissed
    pub fn should_dismiss(&self) -> bool {
        self.acknowledged || self.created_at.elapsed() > self.duration
    }

    /// Get remaining time before auto-dismiss
    pub fn remaining_time(&self) -> Duration {
        self.duration
            .checked_sub(self.created_at.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    /// Format the notification for display
    pub fn format(&self) -> Vec<Line> {
        let mut lines = Vec::new();

        // Header line with icon and agent name
        let header = vec![
            Span::styled(
                self.notification_type.icon(),
                Style::default().fg(self.notification_type.color()),
            ),
            Span::raw(" "),
            Span::styled(
                format!("@{}", self.agent_name),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ];
        lines.push(Line::from(header));

        // Message line
        lines.push(Line::from(Span::styled(
            &self.message,
            Style::default().fg(Color::White),
        )));

        // Progress bar if applicable
        if let Some(progress) = self.progress {
            let bar = self.format_progress_bar(progress, 15);
            lines.push(Line::from(Span::styled(
                bar,
                Style::default().fg(Color::Cyan),
            )));
        }

        // Details if present
        if let Some(ref details) = self.details {
            lines.push(Line::from(Span::styled(
                details,
                Style::default().fg(Color::Gray),
            )));
        }

        lines
    }

    /// Format a simple progress bar
    fn format_progress_bar(&self, progress: f32, width: usize) -> String {
        let filled = (width as f32 * progress) as usize;
        let mut bar = String::with_capacity(width + 7);
        bar.push('[');
        for i in 0..width {
            if i < filled {
                bar.push('█');
            } else {
                bar.push('░');
            }
        }
        bar.push_str(&format!("] {:3.0}%", progress * 100.0));
        bar
    }
}

/// Notification manager for handling multiple notifications
#[derive(Debug, Clone)]
pub struct NotificationManager {
    /// Active notifications queue
    notifications: Arc<Mutex<VecDeque<Notification>>>,
    /// Maximum notifications to display
    max_visible: usize,
    /// Whether to play terminal bell
    bell_enabled: bool,
    /// Whether to show visual flash
    visual_flash_enabled: bool,
    /// Position for rendering
    position: NotificationPosition,
    /// Animation frame for visual effects
    animation_frame: usize,
    /// Last animation update
    last_update: Instant,
}

/// Position for notification display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new() -> Self {
        Self {
            notifications: Arc::new(Mutex::new(VecDeque::new())),
            max_visible: 3,
            bell_enabled: true,
            visual_flash_enabled: true,
            position: NotificationPosition::BottomRight,
            animation_frame: 0,
            last_update: Instant::now(),
        }
    }

    /// Set maximum visible notifications
    pub const fn with_max_visible(mut self, max: usize) -> Self {
        self.max_visible = max;
        self
    }

    /// Set notification position
    pub const fn with_position(mut self, position: NotificationPosition) -> Self {
        self.position = position;
        self
    }

    /// Enable or disable terminal bell
    pub const fn with_bell(mut self, enabled: bool) -> Self {
        self.bell_enabled = enabled;
        self
    }

    /// Enable or disable visual flash
    pub const fn with_visual_flash(mut self, enabled: bool) -> Self {
        self.visual_flash_enabled = enabled;
        self
    }

    /// Add a new notification
    pub fn notify(&self, notification: Notification) {
        // Play terminal bell if appropriate
        if self.bell_enabled && notification.notification_type.should_bell() {
            self.ring_bell();
        }

        // Add to queue
        let mut notifications = self.notifications.lock().unwrap();
        notifications.push_back(notification);

        // Limit queue size (keep last N * 2 for history)
        while notifications.len() > self.max_visible * 2 {
            notifications.pop_front();
        }
    }

    /// Quick helper to notify agent started
    pub fn agent_started(&self, agent_name: String) {
        self.notify(Notification::new(
            NotificationType::Started,
            agent_name.clone(),
            format!("Agent {} started", agent_name),
        ));
    }

    /// Quick helper to notify agent progress
    pub fn agent_progress(&self, agent_name: String, progress: f32, message: String) {
        self.notify(
            Notification::new(NotificationType::Progress, agent_name, message)
                .with_progress(progress)
                .with_duration(Duration::from_secs(2)),
        );
    }

    /// Quick helper to notify agent completion
    pub fn agent_completed(&self, agent_name: String, message: String) {
        self.notify(Notification::new(
            NotificationType::Complete,
            agent_name.clone(),
            message,
        ));
    }

    /// Quick helper to notify agent failure
    pub fn agent_failed(&self, agent_name: String, error: String) {
        self.notify(
            Notification::new(
                NotificationType::Failed,
                agent_name.clone(),
                "Agent failed".to_string(),
            )
            .with_details(error),
        );
    }

    /// Ring the terminal bell
    fn ring_bell(&self) {
        print!("\x07");
        let _ = io::stdout().flush();
    }

    /// Visual flash effect (for accessibility)
    pub fn visual_flash(&self) {
        if !self.visual_flash_enabled {
            return;
        }

        // This would typically be handled by the terminal emulator
        // or by temporarily inverting colors in the TUI
        print!("\x1B[?5h"); // Reverse video on
        let _ = io::stdout().flush();

        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(100));
            print!("\x1B[?5l"); // Reverse video off
            let _ = io::stdout().flush();
        });
    }

    /// Update animation and remove expired notifications
    pub fn tick(&mut self) {
        // Update animation frame
        if self.last_update.elapsed() > Duration::from_millis(100) {
            self.animation_frame = (self.animation_frame + 1) % 8;
            self.last_update = Instant::now();
        }

        // Remove expired notifications
        let mut notifications = self.notifications.lock().unwrap();
        notifications.retain(|n| !n.should_dismiss());
    }

    /// Acknowledge a notification by ID
    pub fn acknowledge(&self, id: Uuid) {
        let mut notifications = self.notifications.lock().unwrap();
        if let Some(notification) = notifications.iter_mut().find(|n| n.id == id) {
            notification.acknowledged = true;
        }
    }

    /// Clear all notifications
    pub fn clear_all(&self) {
        let mut notifications = self.notifications.lock().unwrap();
        notifications.clear();
    }

    /// Get visible notifications
    fn get_visible_notifications(&self) -> Vec<Notification> {
        let notifications = self.notifications.lock().unwrap();
        notifications
            .iter()
            .filter(|n| !n.should_dismiss())
            .take(self.max_visible)
            .cloned()
            .collect()
    }
}

/// Widget implementation for rendering notifications
impl Widget for &NotificationManager {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let notifications = self.get_visible_notifications();
        if notifications.is_empty() {
            return;
        }

        // Calculate notification area based on position
        let notification_width = 25u16;
        let notification_height = 4u16;
        let padding = 1u16;

        let (base_x, base_y, y_direction) = match self.position {
            NotificationPosition::TopLeft => (area.left() + padding, area.top() + padding, 1i16),
            NotificationPosition::TopRight => (
                area.right().saturating_sub(notification_width + padding),
                area.top() + padding,
                1i16,
            ),
            NotificationPosition::BottomLeft => (
                area.left() + padding,
                area.bottom().saturating_sub(notification_height + padding),
                -1i16,
            ),
            NotificationPosition::BottomRight => (
                area.right().saturating_sub(notification_width + padding),
                area.bottom().saturating_sub(notification_height + padding),
                -1i16,
            ),
        };

        // Render each notification
        for (idx, notification) in notifications.iter().enumerate() {
            let y_offset =
                (idx as i16 * (notification_height as i16 + 1) * y_direction).unsigned_abs();
            let notification_area = Rect {
                x: base_x,
                y: if y_direction > 0 {
                    base_y + y_offset
                } else {
                    base_y.saturating_sub(y_offset)
                },
                width: notification_width,
                height: notification_height,
            };

            // Skip if out of bounds
            if notification_area.bottom() > area.bottom() || notification_area.top() < area.top() {
                continue;
            }

            self.render_notification(notification_area, buf, notification, idx == 0);
        }
    }
}

impl NotificationManager {
    /// Render a single notification
    fn render_notification(
        &self,
        area: Rect,
        buf: &mut Buffer,
        notification: &Notification,
        is_latest: bool,
    ) {
        // Clear the area
        Clear.render(area, buf);

        // Create notification block with border
        let border_style = if is_latest && self.animation_frame % 2 == 0 {
            Style::default()
                .fg(notification.notification_type.border_color())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(notification.notification_type.border_color())
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(border_style);

        let inner = block.inner(area);
        block.render(area, buf);

        // Render notification content
        let lines = notification.format();
        let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
        paragraph.render(inner, buf);

        // Add fade effect for older notifications
        if !is_latest {
            // Apply dimming to non-latest notifications
            for y in area.top()..area.bottom() {
                for x in area.left()..area.right() {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        let style = cell.style().add_modifier(Modifier::DIM);
                        cell.set_style(style);
                    }
                }
            }
        }

        // Show remaining time indicator for latest notification
        if is_latest {
            let remaining = notification.remaining_time();
            if remaining < Duration::from_secs(2) {
                let fade_indicator = format!("{}s", remaining.as_secs());
                buf.set_string(
                    area.right().saturating_sub(fade_indicator.len() as u16 + 1),
                    area.top(),
                    &fade_indicator,
                    Style::default().fg(Color::DarkGray),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let notification = Notification::new(
            NotificationType::Complete,
            "test-agent".to_string(),
            "Task completed successfully".to_string(),
        );

        assert_eq!(notification.agent_name, "test-agent");
        assert_eq!(notification.message, "Task completed successfully");
        assert!(!notification.acknowledged);
        assert_eq!(notification.notification_type, NotificationType::Complete);
    }

    #[test]
    fn test_notification_with_details() {
        let notification = Notification::new(
            NotificationType::Failed,
            "test-agent".to_string(),
            "Task failed".to_string(),
        )
        .with_details("Connection timeout".to_string());

        assert_eq!(notification.details, Some("Connection timeout".to_string()));
    }

    #[test]
    fn test_notification_with_progress() {
        let notification = Notification::new(
            NotificationType::Progress,
            "test-agent".to_string(),
            "Processing...".to_string(),
        )
        .with_progress(0.75);

        assert_eq!(notification.progress, Some(0.75));
    }

    #[test]
    fn test_notification_dismissal() {
        let mut notification = Notification::new(
            NotificationType::Info,
            "test".to_string(),
            "Info".to_string(),
        )
        .with_duration(Duration::from_millis(100));

        assert!(!notification.should_dismiss());

        std::thread::sleep(Duration::from_millis(150));
        assert!(notification.should_dismiss());

        notification.acknowledged = true;
        assert!(notification.should_dismiss());
    }

    #[test]
    fn test_notification_manager() {
        let manager = NotificationManager::new()
            .with_max_visible(2)
            .with_position(NotificationPosition::TopRight)
            .with_bell(false);

        assert_eq!(manager.max_visible, 2);
        assert_eq!(manager.position, NotificationPosition::TopRight);
        assert!(!manager.bell_enabled);

        // Add notifications
        manager.agent_started("agent1".to_string());
        manager.agent_progress("agent1".to_string(), 0.5, "Halfway done".to_string());
        manager.agent_completed("agent1".to_string(), "Success!".to_string());

        let notifications = manager.notifications.lock().unwrap();
        assert_eq!(notifications.len(), 3);
    }

    #[test]
    fn test_notification_queue_limit() {
        let manager = NotificationManager::new().with_max_visible(2);

        // Add more than max_visible * 2 notifications
        for i in 0..10 {
            manager.agent_started(format!("agent{}", i));
        }

        let notifications = manager.notifications.lock().unwrap();
        assert!(notifications.len() <= manager.max_visible * 2);
    }

    #[test]
    fn test_notification_types() {
        let types = vec![
            NotificationType::Started,
            NotificationType::Progress,
            NotificationType::Complete,
            NotificationType::Failed,
            NotificationType::Cancelled,
            NotificationType::Info,
            NotificationType::Warning,
        ];

        for notification_type in types {
            assert!(!notification_type.icon().is_empty());
            let _ = notification_type.color(); // Just verify it doesn't panic
            let _ = notification_type.border_color();
            let _ = notification_type.default_duration();
        }

        // Test bell conditions
        assert!(NotificationType::Complete.should_bell());
        assert!(NotificationType::Failed.should_bell());
        assert!(!NotificationType::Info.should_bell());
    }

    #[test]
    fn test_progress_bar_formatting() {
        let notification = Notification::new(
            NotificationType::Progress,
            "test".to_string(),
            "Loading".to_string(),
        );

        let bar = notification.format_progress_bar(0.0, 10);
        assert!(bar.contains("░░░░░░░░░░"));
        assert!(bar.contains("0%"));

        let bar = notification.format_progress_bar(0.5, 10);
        assert!(bar.contains("█████"));
        assert!(bar.contains("50%"));

        let bar = notification.format_progress_bar(1.0, 10);
        assert!(bar.contains("██████████"));
        assert!(bar.contains("100%"));
    }
}
