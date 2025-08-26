//! Feature modules for the AGCodex TUI
//!
//! This module contains advanced UI features for navigation, history management,
//! agent orchestration, and conversation control.

pub mod agent_notifications;
pub mod agent_panel;
pub mod history_browser;

// Agent panel exports

// Agent notifications exports

// History browser exports
pub use history_browser::HistoryBrowser;

// Re-export the message jump widget from widgets module
pub use crate::widgets::message_jump::MessageJump;
pub use crate::widgets::message_jump::RoleFilter;
