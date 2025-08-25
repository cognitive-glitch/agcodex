//! Integration tests for TUI mode switching functionality.
//!
//! Tests the complete mode switching flow including Shift+Tab behavior,
//! visual indicators, and state persistence in the TUI.

use std::time::Duration;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tokio::time::timeout;

// Mock TUI app structures for testing
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    Normal,
    SessionManager,
    HistoryBrowser,
    AgentPanel,
    MessageJump,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperatingMode {
    Plan,
    Build,
    Review,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub current_mode: OperatingMode,
    pub app_mode: AppMode,
    pub mode_history: Vec<OperatingMode>,
    pub visual_indicator: String,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_mode: OperatingMode::Build, // Default mode
            app_mode: AppMode::Normal,
            mode_history: Vec::new(),
            visual_indicator: "🔨 BUILD".to_string(),
            status_message: None,
            should_quit: false,
        }
    }
    
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<(), String> {
        match (key.modifiers, key.code) {
            // Shift+Tab: Mode switching
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.cycle_operating_mode();
                Ok(())
            }
            // Ctrl+S: Session manager
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                self.app_mode = AppMode::SessionManager;
                Ok(())
            }
            // Ctrl+H: History browser
            (KeyModifiers::CONTROL, KeyCode::Char('h')) => {
                self.app_mode = AppMode::HistoryBrowser;
                Ok(())
            }
            // Ctrl+A: Agent panel
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                self.app_mode = AppMode::AgentPanel;
                Ok(())
            }
            // Ctrl+J: Message jump
            (KeyModifiers::CONTROL, KeyCode::Char('j')) => {
                self.app_mode = AppMode::MessageJump;
                Ok(())
            }
            // Escape: Return to normal mode
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.app_mode = AppMode::Normal;
                Ok(())
            }
            // Ctrl+C: Quit
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => {
                self.should_quit = true;
                Ok(())
            }
            _ => Ok(()),
        }
    }
    
    pub fn cycle_operating_mode(&mut self) {
        self.mode_history.push(self.current_mode.clone());
        
        self.current_mode = match self.current_mode {
            OperatingMode::Plan => OperatingMode::Build,
            OperatingMode::Build => OperatingMode::Review,
            OperatingMode::Review => OperatingMode::Plan,
        };
        
        self.update_visual_indicator();
        self.set_mode_switched_message();
    }
    
    fn update_visual_indicator(&mut self) {
        self.visual_indicator = match self.current_mode {
            OperatingMode::Plan => "📋 PLAN".to_string(),
            OperatingMode::Build => "🔨 BUILD".to_string(),
            OperatingMode::Review => "🔍 REVIEW".to_string(),
        };
    }
    
    fn set_mode_switched_message(&mut self) {
        self.status_message = Some(format!("Switched to {} mode", 
            match self.current_mode {
                OperatingMode::Plan => "Plan",
                OperatingMode::Build => "Build",
                OperatingMode::Review => "Review",
            }
        ));
    }
    
    pub fn get_mode_restrictions(&self) -> ModeRestrictions {
        match self.current_mode {
            OperatingMode::Plan => ModeRestrictions {
                allow_file_write: false,
                allow_command_exec: false,
                allow_network_access: true,
                description: "Read-only analysis mode".to_string(),
            },
            OperatingMode::Build => ModeRestrictions {
                allow_file_write: true,
                allow_command_exec: true,
                allow_network_access: true,
                description: "Full development access".to_string(),
            },
            OperatingMode::Review => ModeRestrictions {
                allow_file_write: true,
                allow_command_exec: false,
                allow_network_access: true,
                description: "Quality-focused review mode".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModeRestrictions {
    pub allow_file_write: bool,
    pub allow_command_exec: bool,
    pub allow_network_access: bool,
    pub description: String,
}

// Mock TUI Application
pub struct MockTuiApp {
    pub state: AppState,
    pub terminal: Terminal<TestBackend>,
}

impl MockTuiApp {
    pub fn new() -> Self {
        let backend = TestBackend::new(80, 24);
        let terminal = Terminal::new(backend).unwrap();
        
        Self {
            state: AppState::new(),
            terminal,
        }
    }
    
    pub async fn handle_event(&mut self, event: Event) -> Result<(), String> {
        match event {
            Event::Key(key) => self.state.handle_key_event(key),
            _ => Ok(()),
        }
    }
    
    pub fn render(&mut self) -> Result<String, String> {
        // Simulate rendering the TUI
        let mode_indicator = &self.state.visual_indicator;
        let app_mode = format!("{:?}", self.state.app_mode);
        let restrictions = self.state.get_mode_restrictions();
        
        Ok(format!(
            "Mode: {} | App: {} | Restrictions: {}",
            mode_indicator, app_mode, restrictions.description
        ))
    }
}

#[tokio::test]
async fn test_shift_tab_mode_cycling() {
    let mut app = MockTuiApp::new();
    
    // Start in Build mode
    assert_eq!(app.state.current_mode, OperatingMode::Build);
    assert_eq!(app.state.visual_indicator, "🔨 BUILD");
    
    // First Shift+Tab: Build -> Review
    let shift_tab = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    app.handle_event(shift_tab.clone()).await.unwrap();
    
    assert_eq!(app.state.current_mode, OperatingMode::Review);
    assert_eq!(app.state.visual_indicator, "🔍 REVIEW");
    assert_eq!(app.state.mode_history.len(), 1);
    assert_eq!(app.state.mode_history[0], OperatingMode::Build);
    
    // Second Shift+Tab: Review -> Plan
    app.handle_event(shift_tab.clone()).await.unwrap();
    
    assert_eq!(app.state.current_mode, OperatingMode::Plan);
    assert_eq!(app.state.visual_indicator, "📋 PLAN");
    assert_eq!(app.state.mode_history.len(), 2);
    
    // Third Shift+Tab: Plan -> Build (complete cycle)
    app.handle_event(shift_tab).await.unwrap();
    
    assert_eq!(app.state.current_mode, OperatingMode::Build);
    assert_eq!(app.state.visual_indicator, "🔨 BUILD");
    assert_eq!(app.state.mode_history.len(), 3);
}

#[tokio::test]
async fn test_mode_switching_with_status_messages() {
    let mut app = MockTuiApp::new();
    
    // Switch mode and check status message
    let shift_tab = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    app.handle_event(shift_tab).await.unwrap();
    
    assert!(app.state.status_message.is_some());
    assert!(app.state.status_message.as_ref().unwrap().contains("Review mode"));
}

#[tokio::test]
async fn test_mode_restrictions_enforcement() {
    let mut app = MockTuiApp::new();
    
    // Test Build mode restrictions (full access)
    assert_eq!(app.state.current_mode, OperatingMode::Build);
    let build_restrictions = app.state.get_mode_restrictions();
    assert!(build_restrictions.allow_file_write);
    assert!(build_restrictions.allow_command_exec);
    assert!(build_restrictions.allow_network_access);
    
    // Switch to Plan mode
    let shift_tab = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    app.handle_event(shift_tab.clone()).await.unwrap();
    app.handle_event(shift_tab).await.unwrap(); // Plan mode
    
    let plan_restrictions = app.state.get_mode_restrictions();
    assert!(!plan_restrictions.allow_file_write);
    assert!(!plan_restrictions.allow_command_exec);
    assert!(plan_restrictions.allow_network_access);
}

#[tokio::test]
async fn test_app_mode_switching() {
    let mut app = MockTuiApp::new();
    
    // Test session manager activation
    let ctrl_s = Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
    app.handle_event(ctrl_s).await.unwrap();
    assert_eq!(app.state.app_mode, AppMode::SessionManager);
    
    // Test history browser activation
    let ctrl_h = Event::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
    app.handle_event(ctrl_h).await.unwrap();
    assert_eq!(app.state.app_mode, AppMode::HistoryBrowser);
    
    // Test agent panel activation
    let ctrl_a = Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL));
    app.handle_event(ctrl_a).await.unwrap();
    assert_eq!(app.state.app_mode, AppMode::AgentPanel);
    
    // Test message jump activation
    let ctrl_j = Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
    app.handle_event(ctrl_j).await.unwrap();
    assert_eq!(app.state.app_mode, AppMode::MessageJump);
    
    // Test escape returns to normal
    let escape = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.handle_event(escape).await.unwrap();
    assert_eq!(app.state.app_mode, AppMode::Normal);
}

#[tokio::test]
async fn test_quit_functionality() {
    let mut app = MockTuiApp::new();
    
    assert!(!app.state.should_quit);
    
    let ctrl_c = Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    app.handle_event(ctrl_c).await.unwrap();
    
    assert!(app.state.should_quit);
}

#[tokio::test]
async fn test_rapid_mode_switching() {
    let mut app = MockTuiApp::new();
    let shift_tab = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    
    // Rapidly switch modes multiple times
    for _ in 0..10 {
        app.handle_event(shift_tab.clone()).await.unwrap();
    }
    
    // Should handle rapid switching without issues
    assert_eq!(app.state.mode_history.len(), 10);
    
    // Final mode should be Review (Build -> Review after 10 cycles: 10 % 3 = 1)
    assert_eq!(app.state.current_mode, OperatingMode::Review);
}

#[tokio::test]
async fn test_mode_persistence_across_app_modes() {
    let mut app = MockTuiApp::new();
    
    // Switch operating mode
    let shift_tab = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    app.handle_event(shift_tab).await.unwrap();
    assert_eq!(app.state.current_mode, OperatingMode::Review);
    
    // Switch to session manager app mode
    let ctrl_s = Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
    app.handle_event(ctrl_s).await.unwrap();
    assert_eq!(app.state.app_mode, AppMode::SessionManager);
    
    // Operating mode should persist
    assert_eq!(app.state.current_mode, OperatingMode::Review);
    
    // Return to normal app mode
    let escape = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.handle_event(escape).await.unwrap();
    
    // Operating mode should still be Review
    assert_eq!(app.state.current_mode, OperatingMode::Review);
    assert_eq!(app.state.app_mode, AppMode::Normal);
}

#[tokio::test]
async fn test_visual_indicator_updates() {
    let mut app = MockTuiApp::new();
    
    // Test all mode indicators
    let modes_and_indicators = [
        (OperatingMode::Build, "🔨 BUILD"),
        (OperatingMode::Review, "🔍 REVIEW"),
        (OperatingMode::Plan, "📋 PLAN"),
        (OperatingMode::Build, "🔨 BUILD"), // Full cycle
    ];
    
    let shift_tab = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    
    for (expected_mode, expected_indicator) in &modes_and_indicators {
        if app.state.current_mode != *expected_mode {
            app.handle_event(shift_tab.clone()).await.unwrap();
        }
        
        assert_eq!(app.state.current_mode, *expected_mode);
        assert_eq!(app.state.visual_indicator, *expected_indicator);
    }
}

#[tokio::test]
async fn test_render_output_contains_mode_info() {
    let mut app = MockTuiApp::new();
    
    let output = app.render().unwrap();
    
    // Should contain mode information
    assert!(output.contains("🔨 BUILD"));
    assert!(output.contains("Normal"));
    assert!(output.contains("Full development access"));
    
    // Switch mode and test again
    let shift_tab = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
    app.handle_event(shift_tab).await.unwrap();
    
    let output = app.render().unwrap();
    assert!(output.contains("🔍 REVIEW"));
    assert!(output.contains("Quality-focused review"));
}

#[cfg(test)]
mod mode_switching_edge_cases {
    use super::*;
    
    #[tokio::test]
    async fn test_mode_switching_during_app_mode_change() {
        let mut app = MockTuiApp::new();
        
        // Enter session manager mode
        let ctrl_s = Event::Key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        app.handle_event(ctrl_s).await.unwrap();
        
        // Try to switch operating mode while in session manager
        let shift_tab = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        app.handle_event(shift_tab).await.unwrap();
        
        // Should still work
        assert_eq!(app.state.current_mode, OperatingMode::Review);
        assert_eq!(app.state.app_mode, AppMode::SessionManager);
    }
    
    #[tokio::test]
    async fn test_mode_history_limits() {
        let mut app = MockTuiApp::new();
        let shift_tab = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        
        // Switch modes many times
        for _ in 0..1000 {
            app.handle_event(shift_tab.clone()).await.unwrap();
        }
        
        // History should be maintained (though in real app might be limited)
        assert_eq!(app.state.mode_history.len(), 1000);
    }
    
    #[tokio::test]
    async fn test_unknown_key_combinations() {
        let mut app = MockTuiApp::new();
        
        // Test various key combinations that shouldn't affect mode
        let unknown_keys = [
            Event::Key(KeyEvent::new(KeyCode::F1, KeyModifiers::NONE)),
            Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT)),
            Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL)),
        ];
        
        let initial_mode = app.state.current_mode.clone();
        
        for key in &unknown_keys {
            app.handle_event(key.clone()).await.unwrap();
        }
        
        // Mode should be unchanged
        assert_eq!(app.state.current_mode, initial_mode);
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;
    
    #[tokio::test]
    async fn test_mode_switching_performance() {
        let mut app = MockTuiApp::new();
        let shift_tab = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        
        let start = Instant::now();
        
        // Perform 100 mode switches
        for _ in 0..100 {
            app.handle_event(shift_tab.clone()).await.unwrap();
        }
        
        let duration = start.elapsed();
        
        // Should complete quickly (target: <50ms per switch)
        assert!(duration.as_millis() < 5000, 
               "Mode switching took too long: {:?}", duration);
    }
    
    #[tokio::test]
    async fn test_render_performance() {
        let mut app = MockTuiApp::new();
        
        let start = Instant::now();
        
        // Render 100 times
        for _ in 0..100 {
            let _ = app.render().unwrap();
        }
        
        let duration = start.elapsed();
        
        // Rendering should be fast
        assert!(duration.as_millis() < 1000,
               "Rendering took too long: {:?}", duration);
    }
}

#[cfg(test)]
mod integration_helpers {
    use super::*;
    
    /// Helper to simulate a complete mode switching session
    pub async fn simulate_mode_switching_session(app: &mut MockTuiApp) -> Vec<OperatingMode> {
        let mut visited_modes = Vec::new();
        let shift_tab = Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        
        // Record initial mode
        visited_modes.push(app.state.current_mode.clone());
        
        // Switch through all modes
        for _ in 0..3 {
            app.handle_event(shift_tab.clone()).await.unwrap();
            visited_modes.push(app.state.current_mode.clone());
        }
        
        visited_modes
    }
    
    #[tokio::test]
    async fn test_complete_mode_cycle() {
        let mut app = MockTuiApp::new();
        let visited_modes = simulate_mode_switching_session(&mut app).await;
        
        // Should visit all three modes plus return to start
        assert_eq!(visited_modes.len(), 4);
        assert_eq!(visited_modes[0], OperatingMode::Build);
        assert_eq!(visited_modes[1], OperatingMode::Review);
        assert_eq!(visited_modes[2], OperatingMode::Plan);
        assert_eq!(visited_modes[3], OperatingMode::Build);
    }
}