//! Mode indicator widget for displaying the current operating mode with smooth transitions

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
use std::time::Duration;
use std::time::Instant;

/// Widget that displays the current operating mode with color-coded styling and animations
#[derive(Debug, Clone)]
pub struct ModeIndicator {
    mode: OperatingMode,
    focused: bool,
    /// Time when the mode was last changed (for transition animations)
    transition_start: Option<Instant>,
    /// Previous mode for transition effect
    previous_mode: Option<OperatingMode>,
}

impl ModeIndicator {
    /// Create a new mode indicator
    pub const fn new(mode: OperatingMode) -> Self {
        Self {
            mode,
            focused: false,
            transition_start: None,
            previous_mode: None,
        }
    }

    /// Create a new mode indicator with transition animation from previous mode
    pub fn with_transition(mode: OperatingMode, previous_mode: OperatingMode) -> Self {
        Self {
            mode,
            focused: false,
            transition_start: Some(Instant::now()),
            previous_mode: Some(previous_mode),
        }
    }

    /// Set whether the indicator is focused (for showing description)
    pub const fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Calculate transition progress (0.0 to 1.0)
    fn transition_progress(&self) -> f32 {
        if let Some(start) = self.transition_start {
            let elapsed = start.elapsed();
            let duration = Duration::from_millis(300); // 300ms transition
            let progress = elapsed.as_secs_f32() / duration.as_secs_f32();
            progress.min(1.0)
        } else {
            1.0
        }
    }

    /// Get interpolated color during transition
    fn get_transition_color(&self) -> Color {
        let progress = self.transition_progress();

        if progress >= 1.0 {
            // Transition complete
            let visuals = self.mode.visuals();
            Self::mode_color_to_ratatui(visuals.color)
        } else if let Some(prev_mode) = self.previous_mode {
            // Interpolate between colors
            let prev_visuals = prev_mode.visuals();
            let curr_visuals = self.mode.visuals();

            // For simplicity, we'll use a step function at 50% progress
            // In a real implementation, you could do proper color interpolation
            if progress < 0.5 {
                Self::mode_color_to_ratatui(prev_visuals.color)
            } else {
                Self::mode_color_to_ratatui(curr_visuals.color)
            }
        } else {
            let visuals = self.mode.visuals();
            Self::mode_color_to_ratatui(visuals.color)
        }
    }

    /// Convert ModeColor to ratatui Color
    const fn mode_color_to_ratatui(mode_color: ModeColor) -> Color {
        match mode_color {
            ModeColor::Blue => Color::Blue,
            ModeColor::Green => Color::Green,
            ModeColor::Yellow => Color::Yellow,
        }
    }

    /// Get the style for the current mode with transition effects
    fn get_style(&self) -> Style {
        let color = self.get_transition_color();
        let progress = self.transition_progress();

        // Add pulsing effect during transition
        let mut style = Style::default()
            .fg(Color::Black)
            .bg(color)
            .add_modifier(Modifier::BOLD);

        // Add italic modifier during transition for visual feedback
        if progress < 1.0 {
            style = style.add_modifier(Modifier::ITALIC);
        }

        style
    }

    /// Get the border style for the widget with transition effects
    fn get_border_style(&self) -> Style {
        let color = self.get_transition_color();
        let progress = self.transition_progress();

        let mut style = Style::default().fg(color);

        // Make border bold during transition
        if progress < 1.0 {
            style = style.add_modifier(Modifier::BOLD);
        }

        style
    }

    /// Get animated indicator text based on transition progress
    fn get_indicator_text(&self) -> String {
        let visuals = self.mode.visuals();
        let progress = self.transition_progress();

        if progress < 1.0 {
            // Show animation during transition
            let anim_chars = ["⬤", "◉", "◎", "○"];
            let index = ((1.0 - progress) * anim_chars.len() as f32) as usize;
            let anim = anim_chars.get(index).unwrap_or(&anim_chars[0]);
            format!("{} {} MODE", anim, visuals.indicator)
        } else {
            format!("{} MODE", visuals.indicator)
        }
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
        let indicator_text = self.get_indicator_text();

        // Create the main indicator content with animation
        let indicator_span = Span::styled(indicator_text, style);
        let indicator_line = Line::from(vec![indicator_span]);

        // Create block with thick borders and title
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .border_type(ratatui::widgets::BorderType::Thick)
            .title(" Mode ");

        // If focused, show description below
        if self.focused && area.height > 3 {
            let lines = [
                indicator_line,
                Line::from(Span::styled(
                    if self.transition_progress() < 1.0 {
                        format!("➜ {}", visuals.description)
                    } else {
                        visuals.description.to_string()
                    },
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
