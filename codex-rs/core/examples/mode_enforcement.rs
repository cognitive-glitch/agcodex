//! Example demonstrating how to use mode enforcement with the tool registry
//!
//! Run with: cargo run --example mode_enforcement

use agcodex_core::modes::OperatingMode;
use agcodex_core::tools::create_default_registry;
use agcodex_core::tools::init_mode_manager;
use agcodex_core::tools::update_mode;
use serde_json::json;

fn main() {
    println!("=== AGCodex Mode Enforcement Example ===\n");

    // Create the default tool registry
    let registry = create_default_registry();

    // Example 1: Plan Mode (Read-only)
    println!("📋 Testing PLAN mode (read-only):");
    init_mode_manager(OperatingMode::Plan);

    // Try to edit a file in Plan mode - should fail
    let edit_input = json!({
        "file": "test.txt",
        "old_text": "hello",
        "new_text": "world"
    });

    match registry.execute("edit", edit_input) {
        Ok(output) => println!("  ✓ Edit succeeded: {}", output.summary),
        Err(err) => println!("  ✗ Edit blocked: {}", err),
    }

    // Try to run a command in Plan mode - should fail for non-read-only commands
    let bash_input = json!({
        "command": "touch newfile.txt"
    });

    match registry.execute("bash", bash_input) {
        Ok(output) => println!("  ✓ Command succeeded: {}", output.summary),
        Err(err) => println!("  ✗ Command blocked: {}", err),
    }

    // But read-only commands should work
    let read_command = json!({
        "command": "echo 'Testing read-only command'"
    });

    match registry.execute("bash", read_command) {
        Ok(output) => println!("  ✓ Read command succeeded: {}", output.summary),
        Err(err) => println!("  ✗ Read command failed: {}", err),
    }

    // Example 2: Build Mode (Full access)
    println!("\n🔨 Testing BUILD mode (full access):");
    update_mode(OperatingMode::Build);

    // Now the same edit should work
    let edit_input = json!({
        "file": "Cargo.toml",  // Using a file that exists
        "old_text": "version",
        "new_text": "version"  // Not actually changing anything for safety
    });

    match registry.execute("edit", edit_input) {
        Ok(output) => println!("  ✓ Edit allowed: {}", output.summary),
        Err(err) => println!("  ✗ Edit failed: {}", err),
    }

    // Commands should work
    let bash_input = json!({
        "command": "echo 'Build mode allows commands'"
    });

    match registry.execute("bash", bash_input) {
        Ok(output) => println!("  ✓ Command allowed: {}", output.summary),
        Err(err) => println!("  ✗ Command failed: {}", err),
    }

    // Example 3: Review Mode (Limited edits)
    println!("\n🔍 Testing REVIEW mode (quality focus, limited edits):");
    update_mode(OperatingMode::Review);

    // Small edits should work (under 10KB)
    let small_edit = json!({
        "file": "small_file.txt",
        "old_text": "test",
        "new_text": "review"
    });

    match registry.execute("edit", small_edit) {
        Ok(output) => println!("  ✓ Small edit allowed: {}", output.summary),
        Err(err) => {
            // Expected to fail if file doesn't exist, but would work for small real files
            println!("  ℹ️  Small edit result: {}", err);
        }
    }

    // Commands should be blocked in Review mode
    let command = json!({
        "command": "cargo build"
    });

    match registry.execute("bash", command) {
        Ok(output) => println!("  ✓ Command succeeded: {}", output.summary),
        Err(err) => println!("  ✗ Command blocked: {}", err),
    }

    // Read-only operations should always work
    println!("\n📖 Testing read-only operations (work in all modes):");

    // Search tools work in all modes
    let glob_input = json!({
        "pattern": "*.rs",
        "path": "src"
    });

    match registry.execute("glob", glob_input) {
        Ok(output) => println!("  ✓ Glob search allowed: {}", output.summary),
        Err(err) => println!("  ✗ Glob search failed: {}", err),
    }

    // Analysis tools work in all modes
    let think_input = json!({
        "problem": "How to implement mode enforcement?"
    });

    match registry.execute("think", think_input) {
        Ok(output) => println!("  ✓ Think tool allowed: {}", output.summary),
        Err(err) => println!("  ✗ Think tool failed: {}", err),
    }

    println!("\n=== Mode Enforcement Example Complete ===");
    println!("\nKey takeaways:");
    println!("• Plan mode: Read-only access, no writes or command execution");
    println!("• Build mode: Full access to all operations");
    println!("• Review mode: Limited file edits (< 10KB), no command execution");
    println!("• Search and analysis tools work in all modes");
    println!("• Use Shift+Tab in the TUI to switch between modes");
}
