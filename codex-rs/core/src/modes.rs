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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
