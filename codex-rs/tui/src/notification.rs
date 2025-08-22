//! Terminal bell notification system for the TUI
//!
//! Provides comprehensive notification system with terminal bells, visual feedback,
//! and accessibility options for task completion, errors, warnings, and user interactions.

use agcodex_core::config_types::TuiNotifications;
use std::collections::HashSet;
use std::io::Write;
use std::io::{self};
use std::thread;
use std::time::Duration;

/// Different notification levels with distinct bell patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NotificationLevel {
    /// Task successfully completed (single bell)
    TaskComplete,
    /// Error occurred (double bell with delay)
    Error,
    /// Warning or non-critical issue (single bell)
    Warning,
    /// Information or status update (no bell by default)
    Info,
}

/// Comprehensive notification system for TUI with multiple feedback modes
pub struct NotificationSystem {
    /// Base configuration from TUI settings
    config: TuiNotifications,
    /// Which notification levels are enabled
    enabled_levels: HashSet<NotificationLevel>,
    /// Whether sound notifications (terminal bell) are enabled
    sound_enabled: bool,
    /// Whether visual flash notifications are enabled
    visual_enabled: bool,
}

/// Legacy alias for backward compatibility
pub type NotificationManager = NotificationSystem;

impl NotificationSystem {
    /// Create a new notification system with the given configuration
    pub fn new(config: TuiNotifications) -> Self {
        let mut enabled_levels = HashSet::new();

        // Configure enabled levels based on config
        if config.agent_complete {
            enabled_levels.insert(NotificationLevel::TaskComplete);
        }
        if config.agent_failed || config.error_occurred {
            enabled_levels.insert(NotificationLevel::Error);
        }
        if config.user_input_needed || config.warnings {
            enabled_levels.insert(NotificationLevel::Warning);
        }
        if config.info_messages {
            enabled_levels.insert(NotificationLevel::Info);
        }

        let sound_enabled = config.terminal_bell;
        let visual_enabled = config.visual_flash;
        Self {
            config,
            enabled_levels,
            sound_enabled,
            visual_enabled,
        }
    }

    /// Update the notification configuration
    pub fn update_config(&mut self, config: TuiNotifications) {
        // Rebuild enabled levels
        self.enabled_levels.clear();
        if config.agent_complete {
            self.enabled_levels.insert(NotificationLevel::TaskComplete);
        }
        if config.agent_failed || config.error_occurred {
            self.enabled_levels.insert(NotificationLevel::Error);
        }
        if config.user_input_needed || config.warnings {
            self.enabled_levels.insert(NotificationLevel::Warning);
        }
        if config.info_messages {
            self.enabled_levels.insert(NotificationLevel::Info);
        }

        self.sound_enabled = config.terminal_bell;
        self.visual_enabled = config.visual_flash;
        self.config = config;
    }

    /// Enable or disable specific notification levels
    pub fn set_level_enabled(&mut self, level: NotificationLevel, enabled: bool) {
        if enabled {
            self.enabled_levels.insert(level);
        } else {
            self.enabled_levels.remove(&level);
        }
    }

    /// Check if a notification level is enabled
    pub fn is_level_enabled(&self, level: NotificationLevel) -> bool {
        self.enabled_levels.contains(&level)
    }

    /// Enable or disable sound notifications
    pub const fn set_sound_enabled(&mut self, enabled: bool) {
        self.sound_enabled = enabled;
    }

    /// Enable or disable visual notifications  
    pub const fn set_visual_enabled(&mut self, enabled: bool) {
        self.visual_enabled = enabled;
    }

    /// Ring the terminal bell with pattern based on notification level
    fn ring_bell(&self, level: NotificationLevel) -> io::Result<()> {
        if !self.sound_enabled || !self.enabled_levels.contains(&level) {
            return Ok(());
        }

        match level {
            NotificationLevel::TaskComplete => {
                // Single bell for successful task completion
                print!("\x07");
                io::stdout().flush()?;
            }
            NotificationLevel::Error => {
                // Double bell with delay for errors
                print!("\x07");
                io::stdout().flush()?;
                thread::sleep(Duration::from_millis(150));
                print!("\x07");
                io::stdout().flush()?;
            }
            NotificationLevel::Warning => {
                // Single bell for warnings
                print!("\x07");
                io::stdout().flush()?;
            }
            NotificationLevel::Info => {
                // No bell for info by default (visual only)
                // This can be overridden if needed
            }
        }

        Ok(())
    }

    /// Provide visual flash feedback for accessibility
    fn visual_flash(&self, _level: NotificationLevel) -> io::Result<()> {
        if !self.visual_enabled {
            return Ok(());
        }

        // Visual bell using reverse video flash
        // This is handled by the terminal - send the visual bell sequence
        print!("\x1B[?5h"); // Turn on reverse video
        io::stdout().flush()?;
        thread::sleep(Duration::from_millis(50));
        print!("\x1B[?5l"); // Turn off reverse video  
        io::stdout().flush()?;

        Ok(())
    }

    /// Main notification method that handles both sound and visual feedback
    pub fn notify(&self, level: NotificationLevel) -> io::Result<()> {
        if !self.enabled_levels.contains(&level) {
            return Ok(());
        }

        // Ring bell if sound is enabled
        if self.sound_enabled {
            self.ring_bell(level)?;
        }

        // Visual flash for accessibility if sound is disabled or as supplement
        if self.visual_enabled && (!self.sound_enabled || level == NotificationLevel::Error) {
            self.visual_flash(level)?;
        }

        Ok(())
    }

    /// Convenience method to notify with a message for debugging
    pub fn notify_with_message(&self, level: NotificationLevel, message: &str) -> io::Result<()> {
        // Log the notification for debugging (in debug builds)
        #[cfg(debug_assertions)]
        eprintln!("[NOTIFICATION {:?}] {}", level, message);

        self.notify(level)
    }

    // ===== High-level notification methods =====

    /// Notify when an agent completes a task successfully
    pub fn agent_completed(&self) -> io::Result<()> {
        if self.config.agent_complete {
            self.notify(NotificationLevel::TaskComplete)?;
        }
        Ok(())
    }

    /// Notify when an agent completes a task successfully with message
    pub fn agent_completed_with_message(&self, agent_name: &str) -> io::Result<()> {
        if self.config.agent_complete {
            self.notify_with_message(
                NotificationLevel::TaskComplete,
                &format!("Agent '{}' completed successfully", agent_name),
            )?;
        }
        Ok(())
    }

    /// Notify when an agent fails
    pub fn agent_failed(&self) -> io::Result<()> {
        if self.config.agent_failed {
            self.notify(NotificationLevel::Error)?;
        }
        Ok(())
    }

    /// Notify when an agent fails with error message
    pub fn agent_failed_with_message(&self, agent_name: &str, error: &str) -> io::Result<()> {
        if self.config.agent_failed {
            self.notify_with_message(
                NotificationLevel::Error,
                &format!("Agent '{}' failed: {}", agent_name, error),
            )?;
        }
        Ok(())
    }

    /// Notify when a general error occurs
    pub fn error_occurred(&self) -> io::Result<()> {
        if self.config.error_occurred {
            self.notify(NotificationLevel::Error)?;
        }
        Ok(())
    }

    /// Notify when a general error occurs with message
    pub fn error_occurred_with_message(&self, error: &str) -> io::Result<()> {
        if self.config.error_occurred {
            self.notify_with_message(NotificationLevel::Error, error)?;
        }
        Ok(())
    }

    /// Notify when user input is needed (e.g., approval requests)
    pub fn user_input_needed(&self) -> io::Result<()> {
        if self.config.user_input_needed {
            self.notify(NotificationLevel::Warning)?;
        }
        Ok(())
    }

    /// Notify when user input is needed with message
    pub fn user_input_needed_with_message(&self, message: &str) -> io::Result<()> {
        if self.config.user_input_needed {
            self.notify_with_message(NotificationLevel::Warning, message)?;
        }
        Ok(())
    }

    /// Notify with an info message (typically visual only)
    pub fn info(&self, message: &str) -> io::Result<()> {
        self.notify_with_message(NotificationLevel::Info, message)
    }

    /// Notify with a warning (single bell)
    pub fn warning(&self, message: &str) -> io::Result<()> {
        self.notify_with_message(NotificationLevel::Warning, message)
    }

    /// Get current configuration
    pub const fn config(&self) -> &TuiNotifications {
        &self.config
    }

    /// Check if any notifications are enabled
    pub fn has_enabled_notifications(&self) -> bool {
        !self.enabled_levels.is_empty() && (self.sound_enabled || self.visual_enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agcodex_core::config_types::TuiNotifications;

    #[test]
    fn test_notification_system_creation() {
        let config = TuiNotifications::default();
        let system = NotificationSystem::new(config);

        // All notifications should be enabled by default
        assert!(system.config.terminal_bell);
        assert!(system.config.agent_complete);
        assert!(system.config.agent_failed);
        assert!(system.config.error_occurred);
        assert!(system.config.user_input_needed);
        assert!(system.sound_enabled);
        assert!(system.visual_enabled);
    }

    #[test]
    fn test_disabled_notifications() {
        let config = TuiNotifications {
            terminal_bell: false,
            visual_flash: false,
            agent_complete: false,
            agent_failed: false,
            error_occurred: false,
            user_input_needed: false,
            warnings: false,
            info_messages: false,
        };
        let system = NotificationSystem::new(config);

        assert!(!system.sound_enabled);
        assert!(!system.is_level_enabled(NotificationLevel::TaskComplete));
        assert!(!system.is_level_enabled(NotificationLevel::Error));
        assert!(!system.is_level_enabled(NotificationLevel::Warning));

        // These should not ring the bell (though we can't easily test the actual bell)
        assert!(system.agent_completed().is_ok());
        assert!(system.agent_failed().is_ok());
        assert!(system.error_occurred().is_ok());
        assert!(system.user_input_needed().is_ok());
    }

    #[test]
    fn test_config_update() {
        let initial_config = TuiNotifications::default();
        let mut system = NotificationSystem::new(initial_config);

        let new_config = TuiNotifications {
            terminal_bell: false,
            visual_flash: false,
            agent_complete: false,
            agent_failed: true,
            error_occurred: true,
            user_input_needed: false,
            warnings: false,
            info_messages: false,
        };

        system.update_config(new_config.clone());
        assert_eq!(system.config, new_config);
        assert!(!system.sound_enabled);
        assert!(system.is_level_enabled(NotificationLevel::Error));
        assert!(!system.is_level_enabled(NotificationLevel::TaskComplete));
        assert!(!system.is_level_enabled(NotificationLevel::Warning));
    }

    #[test]
    fn test_level_management() {
        let config = TuiNotifications::default();
        let mut system = NotificationSystem::new(config);

        // Test enabling/disabling levels
        system.set_level_enabled(NotificationLevel::TaskComplete, false);
        assert!(!system.is_level_enabled(NotificationLevel::TaskComplete));

        system.set_level_enabled(NotificationLevel::TaskComplete, true);
        assert!(system.is_level_enabled(NotificationLevel::TaskComplete));
    }

    #[test]
    fn test_sound_and_visual_controls() {
        let config = TuiNotifications::default();
        let mut system = NotificationSystem::new(config);

        system.set_sound_enabled(false);
        assert!(!system.sound_enabled);

        system.set_visual_enabled(false);
        assert!(!system.visual_enabled);

        // Even with everything disabled, has_enabled_notifications should be false
        assert!(!system.has_enabled_notifications());
    }

    #[test]
    fn test_notification_levels() {
        // Test that our enum variants work as expected
        let levels = vec![
            NotificationLevel::TaskComplete,
            NotificationLevel::Error,
            NotificationLevel::Warning,
            NotificationLevel::Info,
        ];

        for level in levels {
            // Test that levels can be created and used
            let mut set = HashSet::new();
            set.insert(level);
            assert!(set.contains(&level));
        }
    }

    // Note: We can't easily test actual bell output without mocking stdout
    // These tests verify the control logic works correctly
    #[test]
    fn test_notification_methods_dont_panic() {
        let config = TuiNotifications::default();
        let system = NotificationSystem::new(config);

        // These should all complete without error
        assert!(system.notify(NotificationLevel::TaskComplete).is_ok());
        assert!(system.notify(NotificationLevel::Error).is_ok());
        assert!(system.notify(NotificationLevel::Warning).is_ok());
        assert!(system.notify(NotificationLevel::Info).is_ok());

        assert!(system.agent_completed_with_message("test-agent").is_ok());
        assert!(
            system
                .agent_failed_with_message("test-agent", "test error")
                .is_ok()
        );
        assert!(system.info("test info").is_ok());
        assert!(system.warning("test warning").is_ok());
    }
}
