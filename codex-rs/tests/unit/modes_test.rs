//! Unit tests for AGCodex operating modes (Plan/Build/Review).
//!
//! Tests mode switching, restrictions enforcement, and state transitions
//! as defined in CLAUDE.md Phase 6 requirements.

use agcodex_core::modes::{ModeManager, OperatingMode, ModeRestrictions};
use std::time::{Duration, SystemTime};

#[test]
fn test_initial_mode_setup() {
    let manager = ModeManager::new(OperatingMode::Plan);
    
    assert_eq!(manager.current_mode, OperatingMode::Plan);
    assert!(manager.mode_history.is_empty());
    
    // Plan mode restrictions
    assert!(!manager.restrictions.allow_file_write);
    assert!(!manager.restrictions.allow_command_exec);
    assert!(manager.restrictions.allow_network_access);
    assert!(!manager.restrictions.allow_git_operations);
    assert_eq!(manager.restrictions.max_file_size, None);
}

#[test]
fn test_build_mode_restrictions() {
    let manager = ModeManager::new(OperatingMode::Build);
    
    assert_eq!(manager.current_mode, OperatingMode::Build);
    
    // Build mode has full access
    assert!(manager.restrictions.allow_file_write);
    assert!(manager.restrictions.allow_command_exec);
    assert!(manager.restrictions.allow_network_access);
    assert!(manager.restrictions.allow_git_operations);
    assert_eq!(manager.restrictions.max_file_size, None);
}

#[test]
fn test_review_mode_restrictions() {
    let manager = ModeManager::new(OperatingMode::Review);
    
    assert_eq!(manager.current_mode, OperatingMode::Review);
    
    // Review mode restrictions
    assert!(manager.restrictions.allow_file_write); // Limited edits allowed
    assert!(!manager.restrictions.allow_command_exec);
    assert!(manager.restrictions.allow_network_access);
    assert!(!manager.restrictions.allow_git_operations);
    assert_eq!(manager.restrictions.max_file_size, Some(10_000));
}

#[test]
fn test_mode_switching_with_history() {
    let mut manager = ModeManager::new(OperatingMode::Plan);
    let start_time = SystemTime::now();
    
    // Switch to Build mode
    manager.switch_mode(OperatingMode::Build);
    
    assert_eq!(manager.current_mode, OperatingMode::Build);
    assert_eq!(manager.mode_history.len(), 1);
    
    let (prev_mode, switch_time) = &manager.mode_history[0];
    assert_eq!(*prev_mode, OperatingMode::Plan);
    assert!(switch_time >= &start_time);
    
    // Switch to Review mode
    manager.switch_mode(OperatingMode::Review);
    
    assert_eq!(manager.current_mode, OperatingMode::Review);
    assert_eq!(manager.mode_history.len(), 2);
    
    let (prev_mode, _) = &manager.mode_history[1];
    assert_eq!(*prev_mode, OperatingMode::Build);
}

#[test]
fn test_mode_cycling_shift_tab_simulation() {
    let mut manager = ModeManager::new(OperatingMode::Plan);
    
    // Simulate Shift+Tab cycling: Plan -> Build -> Review -> Plan
    manager.switch_mode(OperatingMode::Build);
    assert_eq!(manager.current_mode, OperatingMode::Build);
    
    manager.switch_mode(OperatingMode::Review);
    assert_eq!(manager.current_mode, OperatingMode::Review);
    
    manager.switch_mode(OperatingMode::Plan);
    assert_eq!(manager.current_mode, OperatingMode::Plan);
    
    // Verify complete cycle recorded in history
    assert_eq!(manager.mode_history.len(), 3);
    let modes: Vec<OperatingMode> = manager.mode_history
        .iter()
        .map(|(mode, _)| *mode)
        .collect();
    assert_eq!(modes, vec![
        OperatingMode::Plan,
        OperatingMode::Build,
        OperatingMode::Review,
    ]);
}

#[test]
fn test_prompt_suffix_content() {
    let plan_manager = ModeManager::new(OperatingMode::Plan);
    let build_manager = ModeManager::new(OperatingMode::Build);
    let review_manager = ModeManager::new(OperatingMode::Review);
    
    // Verify plan mode prompt contains correct indicators
    let plan_prompt = plan_manager.prompt_suffix();
    assert!(plan_prompt.contains("PLAN MODE - Read Only"));
    assert!(plan_prompt.contains("✓ Read any file"));
    assert!(plan_prompt.contains("✗ Edit or write files"));
    
    // Verify build mode prompt contains correct indicators
    let build_prompt = build_manager.prompt_suffix();
    assert!(build_prompt.contains("BUILD MODE - Full Access"));
    assert!(build_prompt.contains("✓ Read, write, edit files"));
    assert!(build_prompt.contains("✓ Execute commands"));
    
    // Verify review mode prompt contains correct indicators
    let review_prompt = review_manager.prompt_suffix();
    assert!(review_prompt.contains("REVIEW MODE - Quality Focus"));
    assert!(review_prompt.contains("✓ Make small fixes"));
    assert!(review_prompt.contains("bugs, performance, best practices"));
}

#[test]
fn test_mode_restrictions_enforcement() {
    // This test simulates how restrictions would be checked
    // by other components in the system
    
    let plan_mgr = ModeManager::new(OperatingMode::Plan);
    let build_mgr = ModeManager::new(OperatingMode::Build);
    let review_mgr = ModeManager::new(OperatingMode::Review);
    
    // File write permissions
    assert!(!plan_mgr.restrictions.allow_file_write);
    assert!(build_mgr.restrictions.allow_file_write);
    assert!(review_mgr.restrictions.allow_file_write);
    
    // Command execution permissions
    assert!(!plan_mgr.restrictions.allow_command_exec);
    assert!(build_mgr.restrictions.allow_command_exec);
    assert!(!review_mgr.restrictions.allow_command_exec);
    
    // File size limits (Review mode only)
    assert!(plan_mgr.restrictions.max_file_size.is_none());
    assert!(build_mgr.restrictions.max_file_size.is_none());
    assert_eq!(review_mgr.restrictions.max_file_size, Some(10_000));
}

#[test]
fn test_mode_history_timing() {
    let mut manager = ModeManager::new(OperatingMode::Plan);
    let start = SystemTime::now();
    
    // Wait a small amount to ensure time difference
    std::thread::sleep(Duration::from_millis(1));
    
    manager.switch_mode(OperatingMode::Build);
    
    let (_, switch_time) = &manager.mode_history[0];
    assert!(switch_time > &start);
    assert!(switch_time.elapsed().unwrap() < Duration::from_secs(1));
}

#[cfg(test)]
mod mode_defaults {
    use super::*;
    
    #[test]
    fn test_default_mode_restrictions() {
        let default_restrictions = ModeRestrictions::default();
        
        // Default should be most restrictive
        assert!(!default_restrictions.allow_file_write);
        assert!(!default_restrictions.allow_command_exec);
        assert!(!default_restrictions.allow_network_access);
        assert!(!default_restrictions.allow_git_operations);
        assert_eq!(default_restrictions.max_file_size, None);
    }
    
    #[test]
    fn test_mode_enum_equality() {
        assert_eq!(OperatingMode::Plan, OperatingMode::Plan);
        assert_ne!(OperatingMode::Plan, OperatingMode::Build);
        assert_ne!(OperatingMode::Build, OperatingMode::Review);
    }
}

#[cfg(test)]
mod integration_helpers {
    use super::*;
    
    /// Helper to simulate TUI mode switching behavior
    pub fn simulate_shift_tab_press(manager: &mut ModeManager) {
        let next_mode = match manager.current_mode {
            OperatingMode::Plan => OperatingMode::Build,
            OperatingMode::Build => OperatingMode::Review,
            OperatingMode::Review => OperatingMode::Plan,
        };
        manager.switch_mode(next_mode);
    }
    
    #[test]
    fn test_shift_tab_helper() {
        let mut manager = ModeManager::new(OperatingMode::Plan);
        
        simulate_shift_tab_press(&mut manager);
        assert_eq!(manager.current_mode, OperatingMode::Build);
        
        simulate_shift_tab_press(&mut manager);
        assert_eq!(manager.current_mode, OperatingMode::Review);
        
        simulate_shift_tab_press(&mut manager);
        assert_eq!(manager.current_mode, OperatingMode::Plan);
    }
}