//! Git worktree management for parallel agent execution
//!
//! This module provides git worktree isolation for agents to work in parallel
//! without conflicts, and handles merging their changes back together.

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tracing::debug;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

use crate::subagents::SubagentError;
use crate::subagents::SubagentResult;

/// Git worktree for agent isolation
#[derive(Debug, Clone)]
pub struct AgentWorktree {
    /// Unique identifier for this worktree
    pub id: Uuid,
    /// Agent name using this worktree
    pub agent_name: String,
    /// Path to the worktree
    pub path: PathBuf,
    /// Branch name for this worktree
    pub branch: String,
    /// Base branch this was created from
    pub base_branch: String,
    /// Whether this worktree is currently active
    pub active: bool,
    /// Creation timestamp
    pub created_at: std::time::SystemTime,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ConflictStrategy {
    /// Fail on any conflict
    Fail,
    /// Keep changes from the incoming branch
    KeepTheirs,
    /// Keep changes from the current branch
    KeepOurs,
    /// Attempt automatic merge, fail if conflict
    AutoMerge,
    /// Create conflict markers for manual resolution
    Manual,
}

/// Merge result from combining agent work
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// Whether the merge was successful
    pub success: bool,
    /// Files that were modified
    pub modified_files: Vec<PathBuf>,
    /// Files with conflicts (if any)
    pub conflicts: Vec<PathBuf>,
    /// Merge commit hash (if successful)
    pub commit_hash: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Manager for git worktrees used by agents
pub struct WorktreeManager {
    /// Base repository path
    base_repo: PathBuf,
    /// Directory for worktrees
    worktree_dir: PathBuf,
    /// Active worktrees
    worktrees: Arc<RwLock<HashMap<Uuid, AgentWorktree>>>,
    /// Lock for git operations
    git_lock: Arc<Mutex<()>>,
}

impl WorktreeManager {
    /// Create a new worktree manager
    pub fn new(base_repo: PathBuf) -> SubagentResult<Self> {
        let worktree_dir = base_repo.join(".agcodex").join("worktrees");

        // Create worktree directory if it doesn't exist
        std::fs::create_dir_all(&worktree_dir).map_err(SubagentError::Io)?;

        Ok(Self {
            base_repo,
            worktree_dir,
            worktrees: Arc::new(RwLock::new(HashMap::new())),
            git_lock: Arc::new(Mutex::new(())),
        })
    }

    /// Create a new worktree for an agent
    pub async fn create_worktree(
        &self,
        agent_name: &str,
        base_branch: Option<&str>,
    ) -> SubagentResult<AgentWorktree> {
        let _lock = self.git_lock.lock().await;

        let id = Uuid::new_v4();
        let branch_name = format!("agent/{}/{}", agent_name, id);
        let worktree_path = self.worktree_dir.join(&branch_name);
        let base_branch = base_branch.unwrap_or("main").to_string();

        info!(
            "Creating worktree for agent '{}' from branch '{}'",
            agent_name, base_branch
        );

        // Create the worktree
        let output = Command::new("git")
            .args([
                "worktree",
                "add",
                "-b",
                &branch_name,
                worktree_path.to_str().unwrap(),
                &base_branch,
            ])
            .current_dir(&self.base_repo)
            .output()
            .await
            .map_err(|e| {
                SubagentError::ExecutionFailed(format!("Failed to create worktree: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SubagentError::ExecutionFailed(format!(
                "Git worktree creation failed: {}",
                stderr
            )));
        }

        let worktree = AgentWorktree {
            id,
            agent_name: agent_name.to_string(),
            path: worktree_path,
            branch: branch_name,
            base_branch,
            active: true,
            created_at: std::time::SystemTime::now(),
        };

        // Store the worktree
        let mut worktrees = self.worktrees.write().await;
        worktrees.insert(id, worktree.clone());

        debug!("Created worktree {} for agent {}", id, agent_name);
        Ok(worktree)
    }

    /// Remove a worktree
    pub async fn remove_worktree(&self, id: Uuid) -> SubagentResult<()> {
        let _lock = self.git_lock.lock().await;

        let mut worktrees = self.worktrees.write().await;
        let worktree = worktrees
            .remove(&id)
            .ok_or_else(|| SubagentError::ExecutionFailed(format!("Worktree {} not found", id)))?;

        info!("Removing worktree {} for agent {}", id, worktree.agent_name);

        // Remove the worktree
        let output = Command::new("git")
            .args(["worktree", "remove", worktree.path.to_str().unwrap()])
            .current_dir(&self.base_repo)
            .output()
            .await
            .map_err(|e| {
                SubagentError::ExecutionFailed(format!("Failed to remove worktree: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Git worktree removal failed: {}", stderr);
            // Try force removal
            let _ = Command::new("git")
                .args([
                    "worktree",
                    "remove",
                    "--force",
                    worktree.path.to_str().unwrap(),
                ])
                .current_dir(&self.base_repo)
                .output()
                .await;
        }

        // Delete the branch
        let _ = Command::new("git")
            .args(["branch", "-D", &worktree.branch])
            .current_dir(&self.base_repo)
            .output()
            .await;

        Ok(())
    }

    /// Commit changes in a worktree
    pub async fn commit_changes(&self, worktree_id: Uuid, message: &str) -> SubagentResult<String> {
        let worktrees = self.worktrees.read().await;
        let worktree = worktrees.get(&worktree_id).ok_or_else(|| {
            SubagentError::ExecutionFailed(format!("Worktree {} not found", worktree_id))
        })?;

        let worktree_path = worktree.path.clone();
        drop(worktrees); // Release the lock

        info!("Committing changes in worktree {}", worktree_id);

        // Stage all changes
        let output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&worktree_path)
            .output()
            .await
            .map_err(|e| {
                SubagentError::ExecutionFailed(format!("Failed to stage changes: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SubagentError::ExecutionFailed(format!(
                "Git add failed: {}",
                stderr
            )));
        }

        // Commit changes
        let output = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&worktree_path)
            .output()
            .await
            .map_err(|e| SubagentError::ExecutionFailed(format!("Failed to commit: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("nothing to commit") {
                return Ok("No changes to commit".to_string());
            }
            return Err(SubagentError::ExecutionFailed(format!(
                "Git commit failed: {}",
                stderr
            )));
        }

        // Get the commit hash
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&worktree_path)
            .output()
            .await
            .map_err(|e| {
                SubagentError::ExecutionFailed(format!("Failed to get commit hash: {}", e))
            })?;

        let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        debug!(
            "Committed changes in worktree {}: {}",
            worktree_id, commit_hash
        );

        Ok(commit_hash)
    }

    /// Merge changes from multiple agents
    pub async fn merge_agent_changes(
        &self,
        worktree_ids: Vec<Uuid>,
        target_branch: &str,
        strategy: ConflictStrategy,
    ) -> SubagentResult<MergeResult> {
        let _lock = self.git_lock.lock().await;

        info!(
            "Merging {} agent worktrees into branch '{}'",
            worktree_ids.len(),
            target_branch
        );

        let mut result = MergeResult {
            success: true,
            modified_files: Vec::new(),
            conflicts: Vec::new(),
            commit_hash: None,
            error: None,
        };

        // Switch to target branch in main repo
        let output = Command::new("git")
            .args(["checkout", target_branch])
            .current_dir(&self.base_repo)
            .output()
            .await
            .map_err(|e| {
                SubagentError::ExecutionFailed(format!("Failed to checkout target branch: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            result.success = false;
            result.error = Some(format!("Failed to checkout {}: {}", target_branch, stderr));
            return Ok(result);
        }

        // Merge each worktree branch
        for worktree_id in worktree_ids {
            let worktrees = self.worktrees.read().await;
            let worktree = match worktrees.get(&worktree_id) {
                Some(w) => w.clone(),
                None => {
                    warn!("Worktree {} not found, skipping", worktree_id);
                    continue;
                }
            };
            drop(worktrees);

            let merge_result = self.merge_single_branch(&worktree.branch, strategy).await?;

            if !merge_result.success {
                result.success = false;
                result.conflicts.extend(merge_result.conflicts);
                result.error = merge_result.error;
                break;
            }

            result.modified_files.extend(merge_result.modified_files);
        }

        if result.success {
            // Get the final commit hash
            let output = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&self.base_repo)
                .output()
                .await
                .map_err(|e| {
                    SubagentError::ExecutionFailed(format!("Failed to get commit hash: {}", e))
                })?;

            result.commit_hash = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }

        Ok(result)
    }

    /// Merge a single branch
    async fn merge_single_branch(
        &self,
        branch: &str,
        strategy: ConflictStrategy,
    ) -> SubagentResult<MergeResult> {
        info!("Merging branch '{}' with strategy {:?}", branch, strategy);

        let merge_args = match strategy {
            ConflictStrategy::Fail => vec!["merge", "--no-ff", branch],
            ConflictStrategy::KeepTheirs => vec!["merge", "--no-ff", "-X", "theirs", branch],
            ConflictStrategy::KeepOurs => vec!["merge", "--no-ff", "-X", "ours", branch],
            ConflictStrategy::AutoMerge => vec!["merge", "--no-ff", branch],
            ConflictStrategy::Manual => vec!["merge", "--no-ff", "--no-commit", branch],
        };

        let output = Command::new("git")
            .args(&merge_args)
            .current_dir(&self.base_repo)
            .output()
            .await
            .map_err(|e| SubagentError::ExecutionFailed(format!("Failed to merge: {}", e)))?;

        let mut result = MergeResult {
            success: output.status.success(),
            modified_files: Vec::new(),
            conflicts: Vec::new(),
            commit_hash: None,
            error: None,
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Check for conflicts
            if stderr.contains("CONFLICT") || stderr.contains("Automatic merge failed") {
                result.conflicts = self.get_conflicted_files().await?;

                if strategy == ConflictStrategy::Fail {
                    // Abort the merge
                    let _ = Command::new("git")
                        .args(["merge", "--abort"])
                        .current_dir(&self.base_repo)
                        .output()
                        .await;

                    result.error =
                        Some(format!("Merge conflicts detected: {:?}", result.conflicts));
                }
            } else {
                result.error = Some(format!("Merge failed: {}", stderr));
            }
        } else {
            // Get modified files
            result.modified_files = self.get_modified_files(branch).await?;
        }

        Ok(result)
    }

    /// Get list of conflicted files
    async fn get_conflicted_files(&self) -> SubagentResult<Vec<PathBuf>> {
        let output = Command::new("git")
            .args(["diff", "--name-only", "--diff-filter=U"])
            .current_dir(&self.base_repo)
            .output()
            .await
            .map_err(|e| {
                SubagentError::ExecutionFailed(format!("Failed to get conflicts: {}", e))
            })?;

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| PathBuf::from(line.trim()))
            .collect();

        Ok(files)
    }

    /// Get list of modified files in a merge
    async fn get_modified_files(&self, branch: &str) -> SubagentResult<Vec<PathBuf>> {
        let output = Command::new("git")
            .args(["diff", "--name-only", &format!("{}^", branch), branch])
            .current_dir(&self.base_repo)
            .output()
            .await
            .map_err(|e| {
                SubagentError::ExecutionFailed(format!("Failed to get modified files: {}", e))
            })?;

        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| PathBuf::from(line.trim()))
            .collect();

        Ok(files)
    }

    /// Clean up old worktrees
    pub async fn cleanup_old_worktrees(&self, max_age: std::time::Duration) -> SubagentResult<()> {
        let now = std::time::SystemTime::now();
        let worktrees = self.worktrees.read().await;

        let mut to_remove = Vec::new();
        for (id, worktree) in worktrees.iter() {
            if let Ok(age) = now.duration_since(worktree.created_at)
                && age > max_age
                && !worktree.active
            {
                to_remove.push(*id);
            }
        }
        drop(worktrees);

        for id in to_remove {
            info!("Cleaning up old worktree {}", id);
            if let Err(e) = self.remove_worktree(id).await {
                warn!("Failed to clean up worktree {}: {}", id, e);
            }
        }

        Ok(())
    }

    /// List all active worktrees
    pub async fn list_worktrees(&self) -> Vec<AgentWorktree> {
        let worktrees = self.worktrees.read().await;
        worktrees.values().cloned().collect()
    }

    /// Get worktree by ID
    pub async fn get_worktree(&self, id: Uuid) -> Option<AgentWorktree> {
        let worktrees = self.worktrees.read().await;
        worktrees.get(&id).cloned()
    }

    /// Mark a worktree as inactive
    pub async fn deactivate_worktree(&self, id: Uuid) -> SubagentResult<()> {
        let mut worktrees = self.worktrees.write().await;
        if let Some(worktree) = worktrees.get_mut(&id) {
            worktree.active = false;
            Ok(())
        } else {
            Err(SubagentError::ExecutionFailed(format!(
                "Worktree {} not found",
                id
            )))
        }
    }
}

/// Worktree pool for managing multiple worktrees efficiently
pub struct WorktreePool {
    manager: Arc<WorktreeManager>,
    /// Maximum number of concurrent worktrees
    max_worktrees: usize,
    /// Worktrees available for reuse
    available: Arc<Mutex<Vec<AgentWorktree>>>,
    /// Worktrees currently in use
    in_use: Arc<Mutex<HashMap<Uuid, AgentWorktree>>>,
}

impl WorktreePool {
    pub fn new(manager: Arc<WorktreeManager>, max_worktrees: usize) -> Self {
        Self {
            manager,
            max_worktrees,
            available: Arc::new(Mutex::new(Vec::new())),
            in_use: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Acquire a worktree from the pool
    pub async fn acquire(&self, agent_name: &str) -> SubagentResult<AgentWorktree> {
        // Try to get an available worktree
        let mut available = self.available.lock().await;
        if let Some(mut worktree) = available.pop() {
            worktree.agent_name = agent_name.to_string();
            worktree.active = true;

            let mut in_use = self.in_use.lock().await;
            in_use.insert(worktree.id, worktree.clone());

            debug!("Reusing worktree {} for agent {}", worktree.id, agent_name);
            return Ok(worktree);
        }
        drop(available);

        // Check if we've reached the limit
        let in_use = self.in_use.lock().await;
        if in_use.len() >= self.max_worktrees {
            return Err(SubagentError::ExecutionFailed(format!(
                "Worktree pool limit ({}) reached",
                self.max_worktrees
            )));
        }
        drop(in_use);

        // Create a new worktree
        let worktree = self.manager.create_worktree(agent_name, None).await?;

        let mut in_use = self.in_use.lock().await;
        in_use.insert(worktree.id, worktree.clone());

        Ok(worktree)
    }

    /// Release a worktree back to the pool
    pub async fn release(&self, id: Uuid) -> SubagentResult<()> {
        let mut in_use = self.in_use.lock().await;
        if let Some(mut worktree) = in_use.remove(&id) {
            // Reset the worktree
            let output = Command::new("git")
                .args(["clean", "-fd"])
                .current_dir(&worktree.path)
                .output()
                .await?;

            if output.status.success() {
                let _ = Command::new("git")
                    .args(["checkout", "."])
                    .current_dir(&worktree.path)
                    .output()
                    .await;

                worktree.active = false;

                let mut available = self.available.lock().await;
                available.push(worktree);

                debug!("Released worktree {} back to pool", id);
            } else {
                // If reset fails, remove the worktree
                warn!("Failed to reset worktree {}, removing", id);
                self.manager.remove_worktree(id).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_test_repo() -> (TempDir, WorktreeManager) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .await
            .unwrap();

        // Configure git
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()
            .await
            .unwrap();

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()
            .await
            .unwrap();

        // Create initial commit
        std::fs::write(repo_path.join("README.md"), "# Test Repo").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&repo_path)
            .output()
            .await
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .await
            .unwrap();

        let manager = WorktreeManager::new(repo_path).unwrap();
        (temp_dir, manager)
    }

    #[tokio::test]
    async fn test_create_and_remove_worktree() {
        let (_temp_dir, manager) = setup_test_repo().await;

        // Create worktree
        let worktree = manager.create_worktree("test-agent", None).await.unwrap();
        assert_eq!(worktree.agent_name, "test-agent");
        assert!(worktree.path.exists());

        // Remove worktree
        manager.remove_worktree(worktree.id).await.unwrap();
        assert!(!worktree.path.exists());
    }

    #[tokio::test]
    async fn test_worktree_pool() {
        let (_temp_dir, manager) = setup_test_repo().await;
        let pool = WorktreePool::new(Arc::new(manager), 2);

        // Acquire worktrees
        let wt1 = pool.acquire("agent1").await.unwrap();
        let wt2 = pool.acquire("agent2").await.unwrap();

        // Should fail when pool is full
        assert!(pool.acquire("agent3").await.is_err());

        // Release and reacquire
        pool.release(wt1.id).await.unwrap();
        let wt3 = pool.acquire("agent3").await.unwrap();
        assert_eq!(wt3.id, wt1.id); // Should reuse the same worktree
    }
}
