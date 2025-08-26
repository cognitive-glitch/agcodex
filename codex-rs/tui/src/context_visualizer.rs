//! Context window visualization for AGCodex TUI
//!
//! Provides real-time visualization of context window usage, token consumption,
//! AST compression metrics, and intelligent breakdown of context utilization.
//! Features color-coded warnings and progressive disclosure of technical details.

use std::time::Instant;

use agcodex_core::context_engine::ast_compactor::CompactResult;
use agcodex_core::context_engine::ast_compactor::CompressionLevel;
use agcodex_core::protocol::TokenUsage;
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
use ratatui::widgets::Borders;
use ratatui::widgets::Gauge;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;

/// Configuration for context visualization thresholds
#[derive(Debug, Clone)]
pub struct ContextVisualizerConfig {
    /// Green threshold (usage < this percentage)
    pub green_threshold: f32,
    /// Yellow threshold (usage < this percentage but >= green)
    pub yellow_threshold: f32,
    /// Red threshold (usage >= yellow threshold)
    /// Auto-truncation warning starts here
    pub warning_threshold: f32,
    /// Critical threshold for urgent warnings
    pub critical_threshold: f32,
}

impl Default for ContextVisualizerConfig {
    fn default() -> Self {
        Self {
            green_threshold: 0.50,    // < 50% = green
            yellow_threshold: 0.80,   // 50-80% = yellow
            warning_threshold: 0.80,  // >= 80% = red with warnings
            critical_threshold: 0.95, // >= 95% = critical
        }
    }
}

/// Comprehensive context usage breakdown
#[derive(Debug, Clone, Default)]
pub struct ContextBreakdown {
    /// System prompt token count
    pub system_prompt_tokens: u64,
    /// Message history token count
    pub message_history_tokens: u64,
    /// Tool output/descriptions token count
    pub tool_output_tokens: u64,
    /// Cached tokens (from previous requests)
    pub cached_tokens: Option<u64>,
    /// Reasoning tokens (o1/o3 models)
    pub reasoning_tokens: Option<u64>,
    /// Total available context window
    pub context_window_size: Option<u64>,
}

impl ContextBreakdown {
    /// Calculate total used tokens
    pub const fn total_used(&self) -> u64 {
        self.system_prompt_tokens + self.message_history_tokens + self.tool_output_tokens
    }

    /// Calculate remaining tokens
    pub fn remaining_tokens(&self) -> Option<u64> {
        self.context_window_size
            .map(|total| total.saturating_sub(self.total_used()))
    }

    /// Calculate usage ratio (0.0 to 1.0)
    pub fn usage_ratio(&self) -> f32 {
        if let Some(total) = self.context_window_size {
            if total > 0 {
                self.total_used() as f32 / total as f32
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

/// AST compression metrics for display
#[derive(Debug, Clone)]
pub struct CompressionMetrics {
    /// Current compression level
    pub level: CompressionLevel,
    /// Achieved compression ratio (0.0 to 1.0)
    pub compression_ratio: f32,
    /// Original token count before compression
    pub original_tokens: usize,
    /// Compressed token count after compression
    pub compressed_tokens: usize,
    /// Is compression currently active?
    pub is_active: bool,
}

impl Default for CompressionMetrics {
    fn default() -> Self {
        Self {
            level: CompressionLevel::Medium,
            compression_ratio: 0.0,
            original_tokens: 0,
            compressed_tokens: 0,
            is_active: false,
        }
    }
}

impl From<CompactResult> for CompressionMetrics {
    fn from(result: CompactResult) -> Self {
        Self {
            level: CompressionLevel::Medium, // Default, should be passed separately
            compression_ratio: result.compression_ratio,
            original_tokens: result.original_tokens,
            compressed_tokens: result.compressed_tokens,
            is_active: true,
        }
    }
}

/// Main context window visualizer widget
#[derive(Debug, Clone)]
pub struct ContextVisualizer {
    /// Current token usage from conversation
    pub token_usage: TokenUsage,
    /// Detailed context breakdown
    pub context_breakdown: ContextBreakdown,
    /// AST compression metrics
    pub compression_metrics: CompressionMetrics,
    /// Configuration for thresholds and colors
    pub config: ContextVisualizerConfig,
    /// Whether the visualizer is expanded (shows detailed breakdown)
    pub expanded: bool,
    /// Whether the widget is focused/active
    pub focused: bool,
    /// Animation state for pulsing warnings
    animation_start: Option<Instant>,
    /// Last update timestamp for change detection
    last_update: Instant,
}

impl Default for ContextVisualizer {
    fn default() -> Self {
        Self {
            token_usage: TokenUsage::default(),
            context_breakdown: ContextBreakdown::default(),
            compression_metrics: CompressionMetrics::default(),
            config: ContextVisualizerConfig::default(),
            expanded: false,
            focused: false,
            animation_start: None,
            last_update: Instant::now(),
        }
    }
}

impl ContextVisualizer {
    /// Create a new context visualizer with default configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Update token usage information
    pub fn update_token_usage(&mut self, token_usage: TokenUsage) {
        self.token_usage = token_usage;
        self.last_update = Instant::now();

        // Start animation if we're in warning territory
        if self.is_warning_level() && self.animation_start.is_none() {
            self.animation_start = Some(Instant::now());
        }
    }

    /// Update context breakdown information
    pub fn update_context_breakdown(&mut self, breakdown: ContextBreakdown) {
        self.context_breakdown = breakdown;
        self.last_update = Instant::now();
    }

    /// Update compression metrics
    pub fn update_compression_metrics(&mut self, metrics: CompressionMetrics) {
        self.compression_metrics = metrics;
        self.last_update = Instant::now();
    }

    /// Toggle expanded view
    pub const fn toggle_expanded(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Set focused state
    pub const fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Check if usage is at warning level
    fn is_warning_level(&self) -> bool {
        self.context_breakdown.usage_ratio() >= self.config.warning_threshold
    }

    /// Check if usage is at critical level
    fn is_critical_level(&self) -> bool {
        self.context_breakdown.usage_ratio() >= self.config.critical_threshold
    }

    /// Get usage level color based on current ratio
    fn get_usage_color(&self) -> Color {
        let ratio = self.context_breakdown.usage_ratio();

        if ratio >= self.config.warning_threshold {
            if self.is_critical_level() {
                Color::Rgb(220, 20, 60) // Crimson for critical
            } else {
                Color::Red
            }
        } else if ratio >= self.config.green_threshold {
            Color::Yellow
        } else {
            Color::Green
        }
    }

    /// Get animated warning indicator
    fn get_warning_indicator(&self) -> &'static str {
        if !self.is_warning_level() {
            return "";
        }

        if let Some(start) = self.animation_start {
            let elapsed = start.elapsed().as_millis() % 1000;
            if elapsed < 500 {
                if self.is_critical_level() {
                    "⚠️ CRITICAL"
                } else {
                    "⚠️ WARNING"
                }
            } else {
                " " // Blinking effect
            }
        } else if self.is_critical_level() {
            "⚠️ CRITICAL"
        } else {
            "⚠️ WARNING"
        }
    }

    /// Format token count with K/M suffixes
    fn format_tokens(tokens: u64) -> String {
        if tokens >= 1_000_000 {
            format!("{:.1}M", tokens as f32 / 1_000_000.0)
        } else if tokens >= 1_000 {
            format!("{:.1}K", tokens as f32 / 1_000.0)
        } else {
            tokens.to_string()
        }
    }

    /// Generate main progress bar line
    fn render_progress_bar(&self, area: Rect, buf: &mut Buffer) {
        let usage_ratio = self.context_breakdown.usage_ratio();
        let usage_percentage = (usage_ratio * 100.0) as u16;
        let color = self.get_usage_color();

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Context Window Usage ")
                    .border_style(Style::default().fg(color)),
            )
            .gauge_style(Style::default().fg(color).bg(Color::Black))
            .percent(usage_percentage.min(100))
            .label(format!(
                "{}% ({} / {})",
                usage_percentage,
                Self::format_tokens(self.context_breakdown.total_used()),
                self.context_breakdown
                    .context_window_size
                    .map(Self::format_tokens)
                    .unwrap_or_else(|| "Unknown".to_string())
            ));

        gauge.render(area, buf);
    }

    /// Generate compression status line
    fn render_compression_status(&self, area: Rect, buf: &mut Buffer) {
        if !self.compression_metrics.is_active {
            let line = Line::from(vec![
                Span::styled("AST Compression: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Inactive",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]);
            Paragraph::new(line).render(area, buf);
            return;
        }

        let compression_percent = (self.compression_metrics.compression_ratio * 100.0) as u16;
        let savings = self
            .compression_metrics
            .original_tokens
            .saturating_sub(self.compression_metrics.compressed_tokens);

        let spans = vec![
            Span::styled("AST Compression: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{:?}", self.compression_metrics.level),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" ("),
            Span::styled(
                format!("{}%", compression_percent),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(") • Saved "),
            Span::styled(
                Self::format_tokens(savings as u64),
                Style::default().fg(Color::Green),
            ),
            Span::raw(" tokens"),
        ];

        let line = Line::from(spans);
        Paragraph::new(line).render(area, buf);
    }

    /// Generate detailed breakdown lines
    fn render_breakdown(&self, area: Rect, buf: &mut Buffer) {
        let total_used = self.context_breakdown.total_used();
        let total_window = self
            .context_breakdown
            .context_window_size
            .unwrap_or(100_000);

        let breakdown_lines = vec![
            self.create_breakdown_line(
                "System Prompt:",
                self.context_breakdown.system_prompt_tokens,
                total_used,
                Color::Blue,
            ),
            self.create_breakdown_line(
                "Message History:",
                self.context_breakdown.message_history_tokens,
                total_used,
                Color::Magenta,
            ),
            self.create_breakdown_line(
                "Tool Outputs:",
                self.context_breakdown.tool_output_tokens,
                total_used,
                Color::Cyan,
            ),
            if let Some(cached) = self.context_breakdown.cached_tokens {
                self.create_breakdown_line("Cached Tokens:", cached, total_used, Color::Green)
            } else {
                Line::default()
            },
            if let Some(reasoning) = self.context_breakdown.reasoning_tokens {
                self.create_breakdown_line("Reasoning:", reasoning, total_used, Color::Yellow)
            } else {
                Line::default()
            },
            // Remaining space
            if let Some(remaining) = self.context_breakdown.remaining_tokens() {
                self.create_breakdown_line(
                    "Available:",
                    remaining,
                    total_window,
                    if remaining < total_window / 10 {
                        Color::Red
                    } else {
                        Color::Gray
                    },
                )
            } else {
                Line::default()
            },
        ];

        let paragraph = Paragraph::new(breakdown_lines);
        paragraph.render(area, buf);
    }

    /// Create a single breakdown line with percentage
    fn create_breakdown_line(&self, label: &str, tokens: u64, total: u64, color: Color) -> Line {
        if tokens == 0 {
            return Line::default();
        }

        let percentage = if total > 0 {
            ((tokens as f64 / total as f64) * 100.0) as u16
        } else {
            0
        };

        let bar_width = (percentage / 5).min(20); // Max 20 chars for bar
        let bar = "█".repeat(bar_width as usize);

        Line::from(vec![
            Span::styled(format!("{:<15}", label), Style::default().fg(Color::Gray)),
            Span::styled(
                format!("{:>8}", Self::format_tokens(tokens)),
                Style::default().fg(color),
            ),
            Span::raw(" ("),
            Span::styled(
                format!("{:>3}%", percentage),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::raw(") "),
            Span::styled(bar, Style::default().fg(color)),
        ])
    }

    /// Calculate required widget height based on expanded state
    pub const fn required_height(&self) -> u16 {
        if self.expanded {
            10 // Expanded: progress bar + compression + detailed breakdown
        } else {
            4 // Compact: progress bar + compression + warning
        }
    }
}

impl Widget for ContextVisualizer {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.render_ref(area, buf);
    }
}

impl WidgetRef for ContextVisualizer {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        <&ContextVisualizer as WidgetRef>::render_ref(&self, area, buf);
    }
}

impl WidgetRef for &ContextVisualizer {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        if area.height < 3 {
            return; // Not enough space to render anything useful
        }

        let warning_indicator = self.get_warning_indicator();

        // Create layout based on expansion state
        let constraints = if self.expanded {
            vec![
                Constraint::Length(3), // Progress bar
                Constraint::Length(1), // Compression status
                Constraint::Min(4),    // Detailed breakdown
                if !warning_indicator.is_empty() {
                    Constraint::Length(1)
                } else {
                    Constraint::Length(0)
                }, // Warning
            ]
        } else {
            vec![
                Constraint::Length(3), // Progress bar
                Constraint::Length(1), // Compression status
                if !warning_indicator.is_empty() {
                    Constraint::Length(1)
                } else {
                    Constraint::Length(0)
                }, // Warning
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        let mut chunk_idx = 0;

        // Render progress bar
        self.render_progress_bar(chunks[chunk_idx], buf);
        chunk_idx += 1;

        // Render compression status
        if chunk_idx < chunks.len() {
            self.render_compression_status(chunks[chunk_idx], buf);
            chunk_idx += 1;
        }

        // Render detailed breakdown if expanded
        if self.expanded && chunk_idx < chunks.len() {
            self.render_breakdown(chunks[chunk_idx], buf);
            chunk_idx += 1;
        }

        // Render warning if present
        if !warning_indicator.is_empty() && chunk_idx < chunks.len() {
            let warning_color = if self.is_critical_level() {
                Color::Rgb(220, 20, 60) // Crimson
            } else {
                Color::Red
            };

            let warning_text = if self.is_critical_level() {
                format!(
                    "{} Context critically full! Auto-truncation imminent.",
                    warning_indicator
                )
            } else {
                format!(
                    "{} Approaching context limit. Consider compression.",
                    warning_indicator
                )
            };

            let warning_line = Line::from(Span::styled(
                warning_text,
                Style::default()
                    .fg(warning_color)
                    .add_modifier(Modifier::BOLD | Modifier::ITALIC),
            ));

            Paragraph::new(warning_line)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(warning_color)),
                )
                .render(chunks[chunk_idx], buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_visualizer_creation() {
        let visualizer = ContextVisualizer::new();
        assert!(!visualizer.expanded);
        assert!(!visualizer.focused);
    }

    #[test]
    fn test_usage_ratio_calculation() {
        let mut breakdown = ContextBreakdown::default();
        breakdown.system_prompt_tokens = 1000;
        breakdown.message_history_tokens = 4000;
        breakdown.context_window_size = Some(10_000);

        assert_eq!(breakdown.total_used(), 5000);
        assert_eq!(breakdown.usage_ratio(), 0.5);
        assert_eq!(breakdown.remaining_tokens(), Some(5000));
    }

    #[test]
    fn test_warning_thresholds() {
        let mut visualizer = ContextVisualizer::new();
        visualizer.context_breakdown.system_prompt_tokens = 8500;
        visualizer.context_breakdown.context_window_size = Some(10_000);

        assert!(visualizer.is_warning_level());
        assert_eq!(visualizer.get_usage_color(), Color::Red);
    }

    #[test]
    fn test_token_formatting() {
        assert_eq!(ContextVisualizer::format_tokens(500), "500");
        assert_eq!(ContextVisualizer::format_tokens(1500), "1.5K");
        assert_eq!(ContextVisualizer::format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn test_compression_metrics_conversion() {
        let compact_result = CompactResult {
            compacted: "compressed code".to_string(),
            compression_ratio: 0.85,
            original_tokens: 1000,
            compressed_tokens: 150,
            semantic_weights: None,
        };

        let metrics: CompressionMetrics = compact_result.into();
        assert_eq!(metrics.compression_ratio, 0.85);
        assert_eq!(metrics.original_tokens, 1000);
        assert_eq!(metrics.compressed_tokens, 150);
        assert!(metrics.is_active);
    }
}
