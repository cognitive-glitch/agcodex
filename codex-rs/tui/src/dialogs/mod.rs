//! Dialog modules for AGCodex TUI
//! Provides popup dialogs for session management and other user interactions

pub mod load_session;
pub mod save_session;

// Re-export main types for convenience
pub use load_session::LoadSessionBrowser;
pub use load_session::LoadSessionState;
pub use save_session::SaveSessionDialog;
pub use save_session::SaveSessionState;
