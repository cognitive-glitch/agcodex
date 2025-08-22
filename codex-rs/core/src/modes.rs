//! Operating modes for AGCodex (Plan, Build, Review).
//! Minimal scaffolding to start the refactor without impacting existing flows.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;

/// Color representation for mode indicators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModeColor {
    Blue,
    Green,
    Yellow,
}

/// Visual properties for each mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeVisuals {
    pub indicator: &'static str,
    pub color: ModeColor,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OperatingMode {
    Plan,
    Build,
    Review,
}

impl OperatingMode {
    /// Get the visual properties for this mode
    pub const fn visuals(&self) -> ModeVisuals {
        match self {
            Self::Plan => ModeVisuals {
                indicator: "📋 PLAN",
                color: ModeColor::Blue,
                description: "Read-only analysis mode",
            },
            Self::Build => ModeVisuals {
                indicator: "🔨 BUILD",
                color: ModeColor::Green,
                description: "Full access development mode",
            },
            Self::Review => ModeVisuals {
                indicator: "🔍 REVIEW",
                color: ModeColor::Yellow,
                description: "Quality-focused review mode",
            },
        }
    }

    /// Cycle to the next mode
    pub const fn cycle(&self) -> Self {
        match self {
            Self::Plan => Self::Build,
            Self::Build => Self::Review,
            Self::Review => Self::Plan,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModeRestrictions {
    pub allow_file_write: bool,
    pub allow_command_exec: bool,
    pub allow_network_access: bool,
    pub allow_git_operations: bool,
    pub max_file_size: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ModeManager {
    pub current_mode: OperatingMode,
    pub mode_history: Vec<(OperatingMode, SystemTime)>,
    pub restrictions: ModeRestrictions,
}

impl ModeManager {
    pub fn new(initial: OperatingMode) -> Self {
        let mut mgr = Self {
            current_mode: initial,
            mode_history: Vec::new(),
            restrictions: ModeRestrictions::default(),
        };
        mgr.apply_restrictions(initial);
        mgr
    }

    pub fn switch_mode(&mut self, new_mode: OperatingMode) {
        self.mode_history
            .push((self.current_mode, SystemTime::now()));
        self.current_mode = new_mode;
        self.apply_restrictions(new_mode);
    }

    /// Cycle to the next mode (Plan → Build → Review → Plan)
    pub fn cycle(&mut self) -> OperatingMode {
        let new_mode = self.current_mode.cycle();
        self.switch_mode(new_mode);
        new_mode
    }

    /// Get the current mode
    pub const fn current_mode(&self) -> OperatingMode {
        self.current_mode
    }

    /// Get visual properties for the current mode
    pub const fn current_visuals(&self) -> ModeVisuals {
        self.current_mode.visuals()
    }

    /// Create a new thread-safe ModeManager
    pub fn new_shared(initial: OperatingMode) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::new(initial)))
    }

    const fn apply_restrictions(&mut self, mode: OperatingMode) {
        self.restrictions = match mode {
            OperatingMode::Plan => ModeRestrictions {
                allow_file_write: false,
                allow_command_exec: false,
                allow_network_access: true, // Allow research
                allow_git_operations: false,
                max_file_size: None,
            },
            OperatingMode::Build => ModeRestrictions {
                allow_file_write: true,
                allow_command_exec: true,
                allow_network_access: true,
                allow_git_operations: true,
                max_file_size: None,
            },
            OperatingMode::Review => ModeRestrictions {
                allow_file_write: true, // Limited edits allowed
                allow_command_exec: false,
                allow_network_access: true,
                allow_git_operations: false,
                max_file_size: Some(10_000),
            },
        };
    }

    /// Check if the current mode is read-only
    pub const fn is_read_only(&self) -> bool {
        matches!(self.current_mode, OperatingMode::Plan)
    }

    /// Check if file writes are allowed in the current mode
    pub const fn allows_write(&self) -> bool {
        self.restrictions.allow_file_write
    }

    /// Check if command execution is allowed in the current mode
    pub const fn allows_execution(&self) -> bool {
        self.restrictions.allow_command_exec
    }

    /// Check if a file operation is allowed in the current mode
    pub const fn can_write_file(&self, file_size: Option<usize>) -> bool {
        if !self.restrictions.allow_file_write {
            return false;
        }

        if let (Some(max_size), Some(size)) = (self.restrictions.max_file_size, file_size) {
            return size <= max_size;
        }

        true
    }

    /// Check if command execution is allowed in the current mode
    pub const fn can_execute_command(&self) -> bool {
        self.restrictions.allow_command_exec
    }

    /// Check if git operations are allowed in the current mode
    pub const fn can_perform_git_operations(&self) -> bool {
        self.restrictions.allow_git_operations
    }

    /// Check if network access is allowed in the current mode
    pub const fn can_access_network(&self) -> bool {
        self.restrictions.allow_network_access
    }

    /// Get a user-friendly error message for disallowed operations
    pub fn restriction_message(&self, operation: &str) -> String {
        match self.current_mode {
            OperatingMode::Plan => format!(
                "⛔ Operation '{}' not allowed in Plan mode (read-only). Switch to Build mode (Shift+Tab) for full access.",
                operation
            ),
            OperatingMode::Review => {
                // Check if it's a size restriction issue
                if operation.contains("file") || operation.contains("edit") {
                    if let Some(max_size) = self.restrictions.max_file_size {
                        format!(
                            "⚠️ Operation '{}' is limited in Review mode. File edits must be under {} bytes. Switch to Build mode (Shift+Tab) for unlimited access.",
                            operation, max_size
                        )
                    } else {
                        format!(
                            "⚠️ Operation '{}' not allowed in Review mode (quality focus). Switch to Build mode (Shift+Tab) for full access.",
                            operation
                        )
                    }
                } else {
                    format!(
                        "⚠️ Operation '{}' not allowed in Review mode (quality focus). Switch to Build mode (Shift+Tab) for full access.",
                        operation
                    )
                }
            }
            OperatingMode::Build => format!(
                "✓ Operation '{}' should be allowed in Build mode. This may be a bug.",
                operation
            ),
        }
    }

    /// Validate an operation and return a Result with appropriate error message
    pub fn validate_file_write(&self, path: &str, size: Option<usize>) -> Result<(), String> {
        if !self.allows_write() {
            return Err(self.restriction_message(&format!("write to {}", path)));
        }

        if let Some(file_size) = size
            && !self.can_write_file(Some(file_size))
            && let Some(max_size) = self.restrictions.max_file_size
        {
            return Err(format!(
                "⚠️ File '{}' exceeds the {} byte limit in Review mode. File size: {} bytes. Switch to Build mode (Shift+Tab) for unlimited file editing.",
                path, max_size, file_size
            ));
        }

        Ok(())
    }

    /// Validate command execution and return a Result with appropriate error message
    pub fn validate_command_execution(&self, command: &str) -> Result<(), String> {
        if !self.allows_execution() {
            return Err(self.restriction_message(&format!("execute command '{}'", command)));
        }
        Ok(())
    }

    /// Validate git operations and return a Result with appropriate error message
    pub fn validate_git_operation(&self, operation: &str) -> Result<(), String> {
        if !self.can_perform_git_operations() {
            return Err(self.restriction_message(&format!("git operation '{}'", operation)));
        }
        Ok(())
    }

    pub const fn prompt_suffix(&self) -> &'static str {
        match self.current_mode {
            OperatingMode::Plan => {
                r#"
<mode>PLAN MODE - Read Only</mode>
You are analyzing and planning. You CAN:
✓ Read any file
✓ Search code using AST-powered semantic search
✓ Analyze AST structure with tree-sitter
✓ Create detailed implementation plans

You CANNOT:
✗ Edit or write files
✗ Execute commands that modify state
"#
            }
            OperatingMode::Build => {
                r#"
<mode>BUILD MODE - Full Access</mode>
You have complete development capabilities:
✓ Read, write, edit files
✓ Execute commands
✓ Use all tools
✓ Full AST-based editing
"#
            }
            OperatingMode::Review => {
                r#"
<mode>REVIEW MODE - Quality Focus</mode>
You are reviewing code quality. You CAN:
✓ Read and analyze code
✓ Suggest improvements
✓ Make small fixes (< 100 lines)
Focus on: bugs, performance, best practices, security
"#
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mode_cycling() {
        let mut manager = ModeManager::new(OperatingMode::Plan);
        assert_eq!(manager.current_mode(), OperatingMode::Plan);

        // Cycle from Plan to Build
        manager.cycle();
        assert_eq!(manager.current_mode(), OperatingMode::Build);

        // Cycle from Build to Review
        manager.cycle();
        assert_eq!(manager.current_mode(), OperatingMode::Review);

        // Cycle from Review back to Plan
        manager.cycle();
        assert_eq!(manager.current_mode(), OperatingMode::Plan);
    }

    #[test]
    fn test_plan_mode_restrictions() {
        let manager = ModeManager::new(OperatingMode::Plan);

        assert!(manager.is_read_only());
        assert!(!manager.allows_write());
        assert!(!manager.allows_execution());
        assert!(!manager.can_write_file(Some(100)));
        assert!(!manager.can_execute_command());
        assert!(!manager.can_perform_git_operations());
        assert!(manager.can_access_network()); // Research is allowed
    }

    #[test]
    fn test_build_mode_restrictions() {
        let manager = ModeManager::new(OperatingMode::Build);

        assert!(!manager.is_read_only());
        assert!(manager.allows_write());
        assert!(manager.allows_execution());
        assert!(manager.can_write_file(Some(1_000_000))); // Any size allowed
        assert!(manager.can_execute_command());
        assert!(manager.can_perform_git_operations());
        assert!(manager.can_access_network());
    }

    #[test]
    fn test_review_mode_restrictions() {
        let manager = ModeManager::new(OperatingMode::Review);

        assert!(!manager.is_read_only());
        assert!(manager.allows_write()); // Limited writes allowed
        assert!(!manager.allows_execution());

        // Test file size restrictions (10KB limit)
        assert!(manager.can_write_file(Some(5_000))); // Under limit
        assert!(manager.can_write_file(Some(10_000))); // At limit
        assert!(!manager.can_write_file(Some(10_001))); // Over limit

        assert!(!manager.can_execute_command());
        assert!(!manager.can_perform_git_operations());
        assert!(manager.can_access_network());
    }

    #[test]
    fn test_validate_file_write() {
        let mut manager = ModeManager::new(OperatingMode::Plan);

        // Plan mode - no writes allowed
        assert!(manager.validate_file_write("test.txt", Some(100)).is_err());

        // Build mode - all writes allowed
        manager.switch_mode(OperatingMode::Build);
        assert!(
            manager
                .validate_file_write("test.txt", Some(1_000_000))
                .is_ok()
        );

        // Review mode - limited writes
        manager.switch_mode(OperatingMode::Review);
        assert!(manager.validate_file_write("test.txt", Some(5_000)).is_ok());
        assert!(
            manager
                .validate_file_write("test.txt", Some(20_000))
                .is_err()
        );
    }

    #[test]
    fn test_validate_command_execution() {
        let mut manager = ModeManager::new(OperatingMode::Plan);

        // Plan mode - no execution
        assert!(manager.validate_command_execution("ls").is_err());

        // Build mode - execution allowed
        manager.switch_mode(OperatingMode::Build);
        assert!(manager.validate_command_execution("ls").is_ok());

        // Review mode - no execution
        manager.switch_mode(OperatingMode::Review);
        assert!(manager.validate_command_execution("rm -rf /").is_err());
    }

    #[test]
    fn test_mode_history() {
        let mut manager = ModeManager::new(OperatingMode::Plan);
        assert_eq!(manager.mode_history.len(), 0);

        manager.switch_mode(OperatingMode::Build);
        assert_eq!(manager.mode_history.len(), 1);
        assert_eq!(manager.mode_history[0].0, OperatingMode::Plan);

        manager.switch_mode(OperatingMode::Review);
        assert_eq!(manager.mode_history.len(), 2);
        assert_eq!(manager.mode_history[1].0, OperatingMode::Build);
    }

    #[test]
    fn test_restriction_messages() {
        let manager = ModeManager::new(OperatingMode::Plan);
        let msg = manager.restriction_message("write file");
        assert!(msg.contains("Plan mode"));
        assert!(msg.contains("read-only"));
        assert!(msg.contains("Shift+Tab"));

        let manager = ModeManager::new(OperatingMode::Review);
        let msg = manager.restriction_message("edit large file");
        assert!(msg.contains("Review mode"));
        assert!(msg.contains("10000 bytes")); // Should mention the size limit

        let manager = ModeManager::new(OperatingMode::Build);
        let msg = manager.restriction_message("anything");
        assert!(msg.contains("should be allowed"));
        assert!(msg.contains("Build mode"));
    }

    #[test]
    fn test_mode_visuals() {
        assert_eq!(OperatingMode::Plan.visuals().indicator, "📋 PLAN");
        assert_eq!(OperatingMode::Build.visuals().indicator, "🔨 BUILD");
        assert_eq!(OperatingMode::Review.visuals().indicator, "🔍 REVIEW");

        assert_eq!(OperatingMode::Plan.visuals().color, ModeColor::Blue);
        assert_eq!(OperatingMode::Build.visuals().color, ModeColor::Green);
        assert_eq!(OperatingMode::Review.visuals().color, ModeColor::Yellow);
    }
}
