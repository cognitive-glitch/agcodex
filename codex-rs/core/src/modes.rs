//! Operating modes for AGCodex (Plan, Build, Review).
//! Minimal scaffolding to start the refactor without impacting existing flows.

use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingMode {
    Plan,
    Build,
    Review,
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

    pub const fn prompt_suffix(&self) -> &'static str {
        match self.current_mode {
            OperatingMode::Plan => {
                r#"
<mode>PLAN MODE - Read Only</mode>
You are analyzing and planning. You CAN:
✓ Read any file
✓ Search code using ripgrep and fd-find
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
