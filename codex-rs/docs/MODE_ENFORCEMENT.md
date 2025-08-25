# Mode Enforcement Implementation Guide

## Overview

The ModeManager now enforces operating mode restrictions at the tool adapter level, ensuring that Plan, Build, and Review modes properly restrict operations throughout the AGCodex system.

## Architecture

### 1. ModeManager (core/src/modes.rs)
- Defines three operating modes: Plan, Build, Review
- Provides validation methods for file writes, command execution, and git operations
- Generates helpful error messages with mode-switching hints

### 2. Tool Adapters (core/src/tools/adapters.rs)
- Thread-local storage for ModeManager instance
- Each adapter checks mode restrictions before executing operations
- Provides helpful error messages when operations are blocked

### 3. Mode Restrictions

#### Plan Mode (📋 Read-only)
- ❌ File writes blocked
- ❌ Command execution blocked
- ❌ Git operations blocked
- ✅ File reading allowed
- ✅ Search operations allowed
- ✅ Analysis tools allowed

#### Build Mode (🔨 Full access)
- ✅ All file operations allowed
- ✅ All commands allowed (with safety checks)
- ✅ Git operations allowed
- ✅ No size restrictions

#### Review Mode (🔍 Quality focus)
- ✅ Limited file edits (< 10KB)
- ❌ Command execution blocked
- ❌ Git operations blocked
- ✅ Reading and analysis allowed

## Integration with TUI

### Setting the Mode

```rust
use agcodex_core::tools::{init_mode_manager, update_mode};
use agcodex_core::modes::OperatingMode;

// Initialize with starting mode
init_mode_manager(OperatingMode::Build);

// Update mode when user presses Shift+Tab
update_mode(new_mode);
```

### Example TUI Integration

```rust
// In tui/src/app.rs

impl App {
    pub fn handle_mode_switch(&mut self) {
        // Cycle to next mode
        let new_mode = self.mode_manager.cycle();
        
        // Update the tool adapters
        agcodex_core::tools::update_mode(new_mode);
        
        // Update UI to show new mode
        self.update_mode_indicator();
    }
}
```

### Handling Tool Execution

```rust
// When executing a tool from the TUI
let registry = create_default_registry();
let input = json!({
    "file": "main.rs",
    "old_text": "foo",
    "new_text": "bar"
});

match registry.execute("edit", input) {
    Ok(output) => {
        // Show success in TUI
        self.show_notification(&output.summary);
    }
    Err(err) => {
        // Show error with mode hint
        self.show_error(&err.to_string());
        // Error will include "Press Shift+Tab to switch to Build mode"
    }
}
```

## Error Messages

The system provides context-aware error messages:

### Plan Mode
```
⛔ Operation 'write to main.rs' not allowed in Plan mode (read-only). 
Switch to Build mode (Shift+Tab) for full access.
```

### Review Mode
```
⚠️ File 'large.rs' exceeds the 10000 byte limit in Review mode. 
File size: 25000 bytes. Switch to Build mode (Shift+Tab) for unlimited file editing.
```

### Dangerous Commands (All Modes)
```
⛔ Dangerous command 'rm -rf /' is blocked for safety. 
This command could cause system damage.
```

## Testing Mode Enforcement

### Unit Tests
```bash
cargo test -p agcodex-core tools::adapters::tests
```

### Integration Example
```bash
cargo run --example mode_enforcement
```

### Manual Testing in TUI
1. Launch AGCodex: `cargo run --bin agcodex`
2. Press Shift+Tab to cycle modes
3. Try operations in each mode:
   - Plan: Try editing a file (should fail)
   - Build: Try any operation (should succeed)
   - Review: Try large edits (should fail)

## Implementation Checklist

- [x] ModeManager with validation methods
- [x] Thread-local storage in adapters
- [x] Mode validation in edit tool
- [x] Mode validation in bash tool
- [x] Mode validation in patch tool
- [x] Helper functions for mode management
- [x] Comprehensive error messages
- [x] Unit tests for mode enforcement
- [x] Integration example
- [ ] TUI integration (app.rs)
- [ ] Mode indicator in status bar
- [ ] Keyboard shortcut (Shift+Tab)

## Safety Features

### Command Execution Safety
Even in Build mode, certain dangerous commands are blocked:
- `rm -rf /`
- `dd if=/dev/zero of=/dev/sda`
- Other system-damaging commands

### Read-Only Command Allowlist
In Plan and Review modes, only these commands are allowed:
- `ls`, `pwd`, `echo`, `date`, `whoami`
- `cat`, `grep`, `find`, `which`
- `head`, `tail`, `uname`

### File Size Validation
Review mode enforces a 10KB limit on file edits to ensure only small, focused changes.

## Future Enhancements

1. **Configurable Limits**: Allow users to configure Review mode file size limits
2. **Mode Profiles**: Save custom mode configurations
3. **Audit Logging**: Track operations blocked by mode restrictions
4. **Override Mechanism**: Allow temporary mode override with confirmation
5. **Git Integration**: Special handling for git operations in Review mode

## API Reference

### Public Functions

```rust
/// Initialize mode manager with a specific mode
pub fn init_mode_manager(mode: OperatingMode)

/// Update the current mode
pub fn update_mode(mode: OperatingMode)

/// Set a custom ModeManager instance
pub fn set_mode_manager(manager: Arc<Mutex<ModeManager>>)
```

### ModeManager Methods

```rust
impl ModeManager {
    /// Validate file write operation
    pub fn validate_file_write(&self, path: &str, size: Option<usize>) -> Result<(), String>
    
    /// Validate command execution
    pub fn validate_command_execution(&self, command: &str) -> Result<(), String>
    
    /// Validate git operation
    pub fn validate_git_operation(&self, operation: &str) -> Result<(), String>
    
    /// Get user-friendly error message
    pub fn restriction_message(&self, operation: &str) -> String
}
```

## Conclusion

The mode enforcement system ensures that AGCodex operates safely and predictably in different contexts:
- **Plan mode** for safe exploration and analysis
- **Build mode** for active development
- **Review mode** for focused quality improvements

The implementation is non-intrusive, thread-safe, and provides clear feedback to users about why operations are blocked and how to enable them.