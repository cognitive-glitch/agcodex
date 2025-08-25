//! Tool adapters to bridge existing tools with the unified registry
//!
//! These adapters wrap existing tool implementations to work with the
//! simple Value -> ToolOutput interface.

use super::registry::ToolError;
use super::registry::ToolOutput;
use crate::modes::ModeManager;
use crate::modes::OperatingMode;
use crate::subagents::IntelligenceLevel;
use serde_json::Value;
use serde_json::json;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::runtime::Runtime;

// Thread-local storage for the current ModeManager
thread_local! {
    static MODE_MANAGER: RefCell<Arc<Mutex<ModeManager>>> = RefCell::new(Arc::new(Mutex::new(ModeManager::new(OperatingMode::Build))));
}

/// Set the ModeManager for the current thread
pub fn set_mode_manager(manager: Arc<Mutex<ModeManager>>) {
    MODE_MANAGER.with(|m| {
        *m.borrow_mut() = manager;
    });
}

/// Initialize the mode manager with a specific operating mode
pub fn init_mode_manager(mode: OperatingMode) {
    let manager = Arc::new(Mutex::new(ModeManager::new(mode)));
    set_mode_manager(manager);
}

/// Update the current mode in the thread-local ModeManager
pub fn update_mode(mode: OperatingMode) {
    MODE_MANAGER.with(|m| {
        let arc = m.borrow();
        if let Ok(mut guard) = arc.lock() {
            guard.switch_mode(mode);
        }
    });
}

/// Get the current mode from thread-local storage
fn get_current_mode() -> OperatingMode {
    MODE_MANAGER.with(|m| {
        m.borrow()
            .lock()
            .map(|guard| guard.current_mode())
            .unwrap_or(OperatingMode::Build)
    })
}

/// Get the ModeManager from thread-local storage
fn with_mode_manager<F, R>(f: F) -> R
where
    F: FnOnce(&ModeManager) -> R,
{
    MODE_MANAGER.with(|m| {
        let arc = m.borrow();
        let guard = arc.lock().unwrap_or_else(|e| {
            // If the lock is poisoned, recover by using the inner value
            e.into_inner()
        });
        f(&guard)
    })
}

// Import existing tools
use super::glob::FileType;
use super::glob::GlobTool;
use super::plan::PlanTool;
use super::think::ThinkTool;
use super::tree::TreeTool;

/// Adapter for the think tool
pub fn adapt_think_tool(input: Value) -> Result<ToolOutput, ToolError> {
    let problem = input["problem"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidInput("missing 'problem' field".into()))?;

    let language = input["language"].as_str();
    let context = input["context"].as_str();

    // Check for explicit intensity override in input
    let _intensity_override = input["intensity"].as_str();

    // Use the code-specific version if language is provided
    let result = if language.is_some() || context.is_some() {
        let code_result = ThinkTool::think_about_code(problem, language, context)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        json!({
            "problem_type": format!("{:?}", code_result.problem_type),
            "complexity": format!("{:?}", code_result.complexity),
            "steps": code_result.steps.iter().map(|s| json!({
                "number": s.step_number,
                "thought": s.thought,
                "reasoning": s.reasoning,
            })).collect::<Vec<_>>(),
            "conclusion": code_result.conclusion,
            "confidence": code_result.confidence,
            "recommended_action": code_result.recommended_action,
            "affected_files": code_result.affected_files,
            "intensity": format!("{}", super::think::ThinkingIntensity::from_prompt(problem)),
        })
    } else {
        // Use basic thinking
        let basic_result =
            ThinkTool::think(problem).map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        json!({
            "steps": basic_result.steps.iter().map(|s| json!({
                "number": s.step_number,
                "thought": s.thought,
                "reasoning": s.reasoning,
            })).collect::<Vec<_>>(),
            "conclusion": basic_result.conclusion,
            "confidence": basic_result.confidence,
            "intensity": basic_result.intensity.map(|i| format!("{}", i)),
            "progress": basic_result.progress.as_ref().map(|p| json!({
                "current_step": p.current_step,
                "total_steps": p.total_steps,
                "strategy": p.strategy,
                "phase": p.phase,
                "intensity": format!("{}", p.intensity),
            })),
        })
    };

    let confidence = result["confidence"].as_f64().unwrap_or(0.0);
    let intensity = result["intensity"].as_str().unwrap_or("Quick");
    let message = if let Some(progress) = result["progress"].as_object() {
        format!(
            "Analyzed with {} thinking ({}/{} steps) - {:.2} confidence",
            intensity,
            progress["current_step"].as_u64().unwrap_or(0),
            progress["total_steps"].as_u64().unwrap_or(0),
            confidence
        )
    } else {
        format!(
            "Analyzed problem with {} thinking - {:.2} confidence",
            intensity, confidence
        )
    };

    Ok(ToolOutput::success(result, message))
}

/// Adapter for the plan tool
pub fn adapt_plan_tool(input: Value) -> Result<ToolOutput, ToolError> {
    // The plan tool expects a goal string, which can be either 'goal' or 'description'
    let goal = input["goal"]
        .as_str()
        .or_else(|| input["description"].as_str())
        .ok_or_else(|| ToolError::InvalidInput("missing 'goal' or 'description' field".into()))?;

    // Optionally append constraints to the goal if provided
    let constraints = input["constraints"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // If constraints are provided, append them to the goal description
    let full_goal = if constraints.is_empty() {
        goal.to_string()
    } else {
        format!("{} with constraints: {}", goal, constraints.join(", "))
    };

    let tool = PlanTool::new();
    let result = tool
        .plan(&full_goal)
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

    // Convert tasks to JSON format
    let tasks_json: Vec<Value> = result
        .tasks
        .iter()
        .map(|task| {
            json!({
                "id": task.id.to_string(),
                "description": task.description,
                "depends_on": task.depends_on.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                "can_parallelize": task.can_parallelize,
            })
        })
        .collect();

    // Convert parallel groups to JSON format
    let parallel_groups_json: Vec<Value> = result
        .parallel_groups
        .iter()
        .map(|group| json!(group.iter().map(|id| id.to_string()).collect::<Vec<_>>()))
        .collect();

    // Convert dependency graph to JSON object
    let mut dependency_graph_json = serde_json::Map::new();
    for (task_id, deps) in result.dependency_graph.iter() {
        dependency_graph_json.insert(
            task_id.to_string(),
            json!(deps.iter().map(|id| id.to_string()).collect::<Vec<_>>()),
        );
    }

    Ok(ToolOutput::success(
        json!({
            "tasks": tasks_json,
            "dependency_graph": dependency_graph_json,
            "parallel_groups": parallel_groups_json,
            "estimated_complexity": format!("{:?}", result.estimated_complexity),
        }),
        format!("Created plan with {} tasks", result.tasks.len()),
    ))
}

/// Adapter for the glob tool
pub fn adapt_glob_tool(input: Value) -> Result<ToolOutput, ToolError> {
    let pattern = input["pattern"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidInput("missing 'pattern' field".into()))?;

    let path = input["path"].as_str().unwrap_or(".");

    // Create GlobTool with base directory
    let base_path = PathBuf::from(path);
    let tool = GlobTool::new(base_path);

    // Use the glob method to search for files
    let result = tool
        .glob(pattern)
        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

    // Map the FileMatch results to JSON
    let files: Vec<Value> = result
        .result
        .iter()
        .map(|m| {
            json!({
                "path": m.path.display().to_string(),
                "relative_path": m.relative_path.display().to_string(),
                "size": m.size,
                "extension": m.extension,
                "file_type": match m.file_type {
                    FileType::File => "file",
                    FileType::Directory => "directory",
                    FileType::Symlink => "symlink",
                    FileType::Other => "other",
                },
                "content_category": format!("{:?}", m.content_category),
                "executable": m.executable,
                "estimated_lines": m.estimated_lines,
            })
        })
        .collect();

    // Calculate search duration from timestamps
    let duration_ms = result
        .metadata
        .completed_at
        .duration_since(result.metadata.started_at)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Ok(ToolOutput::success(
        json!({
            "matches": files,
            "total_files": result.result.len(),
            "search_duration_ms": duration_ms,
            "summary": result.summary,
        }),
        format!(
            "Found {} files matching pattern '{}'",
            result.result.len(),
            pattern
        ),
    ))
}

/// Adapter for the search/index tool
pub fn adapt_search_tool(input: Value) -> Result<ToolOutput, ToolError> {
    let _query_text = input["query"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidInput("missing 'query' field".into()))?;

    let _limit = input["limit"].as_u64().unwrap_or(10) as usize;
    let _path = input["path"].as_str();

    // For now, return a placeholder since IndexTool requires setup
    // In production, this would use the actual IndexTool
    Ok(ToolOutput::success(
        json!({
            "results": [],
            "message": "Search functionality requires index initialization"
        }),
        "Search requires index setup",
    ))
}

/// Adapter for the tree tool
pub fn adapt_tree_tool(input: Value) -> Result<ToolOutput, ToolError> {
    let file_path = input["file"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidInput("missing 'file' field".into()))?;

    // Since this is a synchronous adapter, we need to use a blocking approach
    // In production, this should be called from an async context
    let rt = Runtime::new()
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create runtime: {}", e)))?;

    let result = rt.block_on(async {
        // Create the tool with default intelligence level
        let tool = TreeTool::new(IntelligenceLevel::Medium)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Parse the file (language will be auto-detected)
        let parse_result = tool
            .parse_file(PathBuf::from(file_path))
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // Extract information from ParsedAst
        let language_str = parse_result.language.as_str();
        let node_count = parse_result.node_count;
        let has_errors = parse_result.has_errors();
        let parse_time_ms = parse_result.parse_time.as_millis() as u64;

        Ok::<_, ToolError>(json!({
            "language": language_str,
            "node_count": node_count,
            "has_errors": has_errors,
            "parse_time_ms": parse_time_ms,
            "source_length": parse_result.source_code.len(),
            "file": file_path,
        }))
    })?;

    // Extract values before moving result
    let node_count = result["node_count"].as_u64().unwrap_or(0);
    let language = result["language"].as_str().unwrap_or("unknown").to_string();

    Ok(ToolOutput::success(
        result,
        format!("Parsed {} file with {} nodes", language, node_count),
    ))
}

/// Adapter for the edit tool (simple text replacement)
pub fn adapt_edit_tool(input: Value) -> Result<ToolOutput, ToolError> {
    let file = input["file"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidInput("missing 'file' field".into()))?;

    let old_text = input["old_text"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidInput("missing 'old_text' field".into()))?;

    let new_text = input["new_text"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidInput("missing 'new_text' field".into()))?;

    // Read file
    let content = std::fs::read_to_string(file).map_err(ToolError::Io)?;

    // Check if old_text exists
    if !content.contains(old_text) {
        return Err(ToolError::InvalidInput("old_text not found in file".into()));
    }

    // Calculate the size of the new content
    let new_content = content.replace(old_text, new_text);
    let file_size = new_content.len();

    // Validate file write operation with ModeManager
    with_mode_manager(|manager| manager.validate_file_write(file, Some(file_size)))
        .map_err(ToolError::InvalidInput)?;

    // Write file
    std::fs::write(file, &new_content).map_err(ToolError::Io)?;

    Ok(ToolOutput::success(
        json!({
            "file": file,
            "changes": 1,
            "old_text": old_text,
            "new_text": new_text,
        }),
        format!("Edited {} successfully", file),
    ))
}

/// Adapter for the patch tool (bulk operations)
pub fn adapt_patch_tool(input: Value) -> Result<ToolOutput, ToolError> {
    let operation = input["operation"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidInput("missing 'operation' field".into()))?;

    // Validate that patch operations (which modify files) are allowed
    with_mode_manager(|manager| {
        manager.validate_file_write(&format!("patch operation: {}", operation), None)
    })
    .map_err(ToolError::InvalidInput)?;

    match operation {
        "rename_symbol" => {
            let old_name = input["old_name"]
                .as_str()
                .ok_or_else(|| ToolError::InvalidInput("missing 'old_name'".into()))?;
            let new_name = input["new_name"]
                .as_str()
                .ok_or_else(|| ToolError::InvalidInput("missing 'new_name'".into()))?;

            // Placeholder for actual implementation
            Ok(ToolOutput::success(
                json!({
                    "operation": "rename_symbol",
                    "old_name": old_name,
                    "new_name": new_name,
                    "message": "Symbol rename requires async runtime"
                }),
                "Symbol rename placeholder",
            ))
        }
        "extract_function" => {
            let file = input["file"]
                .as_str()
                .ok_or_else(|| ToolError::InvalidInput("missing 'file'".into()))?;
            let function_name = input["function_name"]
                .as_str()
                .ok_or_else(|| ToolError::InvalidInput("missing 'function_name'".into()))?;

            // Additional validation for file operations in Review mode
            // In a real implementation, we'd check the actual file size
            if let OperatingMode::Review = get_current_mode() {
                // For Review mode, we should check the actual operation size
                // This is a placeholder that assumes extract_function creates small changes
                with_mode_manager(|manager| {
                    manager.validate_file_write(file, Some(5000)) // Assume 5KB for extraction
                })
                .map_err(ToolError::InvalidInput)?;
            }

            Ok(ToolOutput::success(
                json!({
                    "operation": "extract_function",
                    "file": file,
                    "function_name": function_name,
                    "message": "Function extraction requires async runtime"
                }),
                "Function extraction placeholder",
            ))
        }
        _ => Err(ToolError::InvalidInput(format!(
            "unknown operation: {}",
            operation
        ))),
    }
}

/// Adapter for the grep tool
pub fn adapt_grep_tool(input: Value) -> Result<ToolOutput, ToolError> {
    let pattern = input["pattern"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidInput("missing 'pattern' field".into()))?;

    let path = input["path"].as_str().unwrap_or(".");
    let language = input["language"].as_str();

    // Create default grep config - builder methods not yet implemented
    let _config = super::grep_simple::GrepConfig::default();

    // For now, return placeholder since GrepTool requires more setup
    Ok(ToolOutput::success(
        json!({
            "pattern": pattern,
            "path": path,
            "language": language,
            "message": "Grep functionality requires further implementation"
        }),
        "Grep search placeholder",
    ))
}

/// Adapter for bash tool (command execution)
pub fn adapt_bash_tool(input: Value) -> Result<ToolOutput, ToolError> {
    let command = input["command"]
        .as_str()
        .ok_or_else(|| ToolError::InvalidInput("missing 'command' field".into()))?;

    // Validate command execution with ModeManager
    with_mode_manager(|manager| manager.validate_command_execution(command))
        .map_err(ToolError::InvalidInput)?;

    // Additional safety check - even in Build mode, we maintain some restrictions
    let read_only_commands = [
        "ls", "pwd", "echo", "date", "whoami", "uname", "cat", "grep", "find", "which", "head",
        "tail",
    ];
    let dangerous_commands = ["rm", "dd", "mkfs", "format"];
    let first_word = command.split_whitespace().next().unwrap_or("");

    // Check if it's a dangerous command that should always be blocked
    if dangerous_commands.contains(&first_word) {
        // Check for special dangerous patterns
        if command.contains("rm -rf /") || command.contains("dd if=/dev/zero") {
            return Err(ToolError::InvalidInput(format!(
                "⛔ Dangerous command '{}' is blocked for safety. This command could cause system damage.",
                command
            )));
        }
    }

    // In Plan and Review modes, only allow read-only commands
    let current_mode = get_current_mode();
    if matches!(current_mode, OperatingMode::Plan | OperatingMode::Review)
        && !read_only_commands.contains(&first_word)
    {
        return Err(ToolError::InvalidInput(format!(
            "⛔ Command '{}' not allowed in {} mode. Only read-only commands are permitted. Press Shift+Tab to switch to Build mode.",
            first_word,
            match current_mode {
                OperatingMode::Plan => "Plan",
                OperatingMode::Review => "Review",
                _ => "current",
            }
        )));
    }

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(ToolError::Io)?;

    Ok(ToolOutput::success(
        json!({
            "command": command,
            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
            "success": output.status.success(),
        }),
        format!("Executed command: {}", command),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to set mode for testing
    fn set_test_mode(mode: OperatingMode) {
        let manager = Arc::new(Mutex::new(ModeManager::new(mode)));
        set_mode_manager(manager);
    }

    #[test]
    fn test_adapt_think_basic() {
        let input = json!({
            "problem": "How to implement a cache?"
        });

        let result = adapt_think_tool(input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.success);
        assert!(output.result["steps"].is_array());
    }

    #[test]
    fn test_adapt_think_with_language() {
        let input = json!({
            "problem": "Fix null pointer exception",
            "language": "java",
            "context": "UserService.java"
        });

        let result = adapt_think_tool(input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.success);
        assert!(output.result["problem_type"].is_string());
    }

    #[test]
    fn test_adapt_glob() {
        let input = json!({
            "pattern": "*.rs",
            "path": ".",
            "file_type": "file"
        });

        let result = adapt_glob_tool(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_adapt_bash_safe() {
        // Build mode allows commands
        set_test_mode(OperatingMode::Build);

        let input = json!({
            "command": "echo hello"
        });

        let result = adapt_bash_tool(input);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.success);
        assert_eq!(output.result["stdout"].as_str().unwrap().trim(), "hello");
    }

    #[test]
    fn test_adapt_bash_unsafe() {
        let input = json!({
            "command": "rm -rf /"
        });

        let result = adapt_bash_tool(input);
        assert!(result.is_err());

        if let Err(ToolError::InvalidInput(msg)) = result {
            assert!(msg.contains("Dangerous command"));
        } else {
            panic!("Expected InvalidInput error");
        }
    }

    #[test]
    fn test_edit_tool_plan_mode_blocked() {
        // Set Plan mode - should block writes
        set_test_mode(OperatingMode::Plan);

        let input = json!({
            "file": "test.txt",
            "old_text": "foo",
            "new_text": "bar"
        });

        let result = adapt_edit_tool(input);
        assert!(result.is_err());

        if let Err(ToolError::InvalidInput(msg)) = result {
            assert!(msg.contains("Plan mode"));
            assert!(msg.contains("read-only"));
            assert!(msg.contains("Shift+Tab"));
        } else {
            panic!("Expected InvalidInput error with mode message");
        }
    }

    #[test]
    fn test_bash_tool_plan_mode_blocked() {
        // Set Plan mode - should block non-read-only commands
        set_test_mode(OperatingMode::Plan);

        let input = json!({
            "command": "touch newfile.txt"
        });

        let result = adapt_bash_tool(input);
        assert!(result.is_err());

        if let Err(ToolError::InvalidInput(msg)) = result {
            assert!(msg.contains("not allowed"));
            assert!(msg.contains("Plan mode"));
        } else {
            panic!("Expected InvalidInput error");
        }
    }

    #[test]
    fn test_bash_tool_plan_mode_allows_read() {
        // Set Plan mode - should allow read-only commands
        set_test_mode(OperatingMode::Plan);

        let input = json!({
            "command": "pwd"
        });

        let result = adapt_bash_tool(input);
        // pwd should work in Plan mode
        assert!(result.is_ok());
    }

    #[test]
    fn test_patch_tool_review_mode() {
        // Set Review mode - should allow limited edits
        set_test_mode(OperatingMode::Review);

        let input = json!({
            "operation": "extract_function",
            "file": "small_file.rs",
            "function_name": "helper"
        });

        // This should succeed as it's under the size limit (assumed 5KB)
        let result = adapt_patch_tool(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mode_switching() {
        // Test that mode switching works correctly
        set_test_mode(OperatingMode::Plan);
        assert_eq!(get_current_mode(), OperatingMode::Plan);

        set_test_mode(OperatingMode::Build);
        assert_eq!(get_current_mode(), OperatingMode::Build);

        set_test_mode(OperatingMode::Review);
        assert_eq!(get_current_mode(), OperatingMode::Review);
    }
}
