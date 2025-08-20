//! Mode indicator widget for displaying the current operating mode

use agcodex_core::modes::ModeColor;
use agcodex_core::modes::OperatingMode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Widget;
use ratatui::widgets::WidgetRef;

/// Widget that displays the current operating mode with color-coded styling
#[derive(Debug, Clone)]
pub struct ModeIndicator {
    mode: OperatingMode,
    focused: bool,
}

impl ModeIndicator {
    /// Create a new mode indicator
    pub const fn new(mode: OperatingMode) -> Self {
        Self {
            mode,
            focused: false,
        }
    }

    /// Set whether the indicator is focused (for showing description)
    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Convert ModeColor to ratatui Color
    const fn mode_color_to_ratatui(mode_color: ModeColor) -> Color {
        match mode_color {
            ModeColor::Blue => Color::Blue,
            ModeColor::Green => Color::Green,
            ModeColor::Yellow => Color::Yellow,
        }
    }

    /// Get the style for the current mode
    fn get_style(&self) -> Style {
        let visuals = self.mode.visuals();
        let color = Self::mode_color_to_ratatui(visuals.color);

        Style::default()
            .fg(Color::Black)
            .bg(color)
            .add_modifier(Modifier::BOLD)
    }

    /// Get the border style for the widget
    fn get_border_style(&self) -> Style {
        let visuals = self.mode.visuals();
        let color = Self::mode_color_to_ratatui(visuals.color);

        Style::default().fg(color)
    }
}

impl Widget for ModeIndicator {
    fn render(self, area: Rect, buf: &mut Buffer) {
        WidgetRef::render_ref(&self, area, buf);
    }
}

impl WidgetRef for ModeIndicator {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let visuals = self.mode.visuals();
        let style = self.get_style();
        let border_style = self.get_border_style();

        // Create the main indicator content
        let indicator_span = Span::styled(visuals.indicator, style);
        let indicator_line = Line::from(vec![indicator_span]);

        // Create block with borders
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        // If focused, show description below
        if self.focused && area.height > 3 {
            let lines = [
                indicator_line,
                Line::from(Span::styled(
                    visuals.description,
                    Style::default().fg(border_style.fg.unwrap_or(Color::White)),
                )),
            ];

            // Render with multiple lines
            let inner_area = block.inner(area);
            block.render(area, buf);

            for (i, line) in lines.iter().enumerate() {
                if i < inner_area.height as usize {
                    line.render(
                        Rect {
                            x: inner_area.x,
                            y: inner_area.y + i as u16,
                            width: inner_area.width,
                            height: 1,
                        },
                        buf,
                    );
                }
            }
        } else {
            // Render single line indicator
            let inner_area = block.inner(area);
            block.render(area, buf);
            indicator_line.render(inner_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    #[test]
    fn test_mode_indicator_creation() {
        let indicator = ModeIndicator::new(OperatingMode::Plan);
        assert_eq!(indicator.mode, OperatingMode::Plan);
        assert!(!indicator.focused);
    }

    #[test]
    fn test_mode_indicator_focused() {
        let indicator = ModeIndicator::new(OperatingMode::Build).focused(true);
        assert!(indicator.focused);
    }

    #[test]
    fn test_mode_color_conversion() {
        assert_eq!(
            ModeIndicator::mode_color_to_ratatui(ModeColor::Blue),
            Color::Blue
        );
        assert_eq!(
            ModeIndicator::mode_color_to_ratatui(ModeColor::Green),
            Color::Green
        );
        assert_eq!(
            ModeIndicator::mode_color_to_ratatui(ModeColor::Yellow),
            Color::Yellow
        );
    }

    #[test]
    fn test_widget_renders() {
        let indicator = ModeIndicator::new(OperatingMode::Plan);
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);

        indicator.render(area, &mut buf);

        // Basic test that it doesn't crash
        assert!(!buf.content().is_empty());
    }
}
