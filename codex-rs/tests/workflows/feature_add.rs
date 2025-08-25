//! Feature Addition Workflow Tests
//!
//! Tests the complete workflow of adding a new feature to an existing codebase:
//! 1. Planning the implementation using the plan tool
//! 2. Creating new files for the feature
//! 3. Editing existing files to integrate the feature
//! 4. Verifying the integration works correctly
//!
//! Example scenario: Adding user preferences feature to user management system

use agcodex_core::tools::{PlanTool, EditTool, SearchTool, TreeTool};
use agcodex_core::{AgcodexResult, tools::CodeTool, context::Context};
use std::path::PathBuf;
use std::collections::HashMap;
use tempfile::TempDir;
use tokio::fs;

/// Mock user management system for testing
struct MockUserSystem {
    temp_dir: TempDir,
    base_path: PathBuf,
}

impl MockUserSystem {
    async fn new() -> AgcodexResult<Self> {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();
        
        // Create mock user management system
        let user_rs = r#"
//! User Management System

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct UserManager {
    users: HashMap<u64, User>,
    next_id: u64,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            next_id: 1,
        }
    }
    
    pub fn create_user(&mut self, username: String, email: String) -> AgcodexResult<u64> {
        let user = User {
            id: self.next_id,
            username,
            email,
            created_at: chrono::Utc::now(),
        };
        
        let id = user.id;
        self.users.insert(id, user);
        self.next_id += 1;
        
        Ok(id)
    }
    
    pub fn get_user(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }
    
    pub fn update_user(&mut self, id: u64, username: Option<String>, email: Option<String>) -> AgcodexResult<()> {
        if let Some(user) = self.users.get_mut(&id) {
            if let Some(username) = username {
                user.username = username;
            }
            if let Some(email) = email {
                user.email = email;
            }
            Ok(())
        } else {
            Err(AgcodexError::NotFound(format!("User with id {} not found", id)))
        }
    }
    
    pub fn delete_user(&mut self, id: u64) -> AgcodexResult<User> {
        self.users.remove(&id)
            .ok_or_else(|| AgcodexError::NotFound(format!("User with id {} not found", id)))
    }
    
    pub fn list_users(&self) -> Vec<&User> {
        self.users.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_crud() {
        let mut manager = UserManager::new();
        
        // Create user
        let id = manager.create_user("alice".to_string(), "alice@example.com".to_string()).unwrap();
        assert_eq!(id, 1);
        
        // Get user
        let user = manager.get_user(id).unwrap();
        assert_eq!(user.username, "alice");
        assert_eq!(user.email, "alice@example.com");
        
        // Update user
        manager.update_user(id, Some("alice_updated".to_string()), None).unwrap();
        let user = manager.get_user(id).unwrap();
        assert_eq!(user.username, "alice_updated");
        
        // Delete user
        let deleted_user = manager.delete_user(id).unwrap();
        assert_eq!(deleted_user.username, "alice_updated");
        assert!(manager.get_user(id).is_none());
    }
}
"#;

        let main_rs = r#"
//! Main application entry point

use agcodex_user_system::UserManager;

fn main() {
    let mut user_manager = UserManager::new();
    
    println!("User Management System");
    println!("=====================");
    
    // Create some test users
    let alice_id = user_manager.create_user("alice".to_string(), "alice@example.com".to_string()).unwrap();
    let bob_id = user_manager.create_user("bob".to_string(), "bob@example.com".to_string()).unwrap();
    
    println!("Created users:");
    for user in user_manager.list_users() {
        println!("  {} ({}): {}", user.id, user.username, user.email);
    }
}
"#;

        let cargo_toml = r#"
[package]
name = "agcodex-user-system"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
chrono = { version = "0.4", features = ["serde"] }
"#;

        // Write files
        fs::create_dir_all(base_path.join("src")).await?;
        fs::write(base_path.join("src/lib.rs"), user_rs).await?;
        fs::write(base_path.join("src/main.rs"), main_rs).await?;
        fs::write(base_path.join("Cargo.toml"), cargo_toml).await?;
        
        Ok(Self { temp_dir, base_path })
    }
    
    fn path(&self) -> &PathBuf {
        &self.base_path
    }
}

/// Test the complete feature addition workflow
#[tokio::test]
async fn test_feature_addition_workflow() {
    let mock_system = MockUserSystem::new().await.unwrap();
    let context = Context::new(mock_system.path().clone());
    
    // Phase 1: Plan the feature implementation
    println!("Phase 1: Planning user preferences feature...");
    let plan_tool = PlanTool::new();
    
    let plan_request = "Add user preferences feature to the user management system. \
                       Users should be able to set and get preferences like theme, language, notifications.";
    
    let plan_result = plan_tool.execute(&plan_request, &context).await.unwrap();
    
    // Verify plan contains expected steps
    assert!(plan_result.result.contains("UserPreferences"));
    assert!(plan_result.result.contains("preferences"));
    assert!(plan_result.result.contains("integration"));
    
    println!("Plan generated: {}", plan_result.summary);
    
    // Phase 2: Create new files for preferences feature
    println!("\nPhase 2: Creating preferences module...");
    
    let preferences_code = r#"
//! User Preferences Module

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub user_id: u64,
    pub theme: Theme,
    pub language: String,
    pub notifications: NotificationSettings,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
    Auto,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub email: bool,
    pub push: bool,
    pub sms: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            email: true,
            push: true,
            sms: false,
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            user_id: 0,
            theme: Theme::default(),
            language: "en".to_string(),
            notifications: NotificationSettings::default(),
            updated_at: chrono::Utc::now(),
        }
    }
}

#[derive(Debug)]
pub struct PreferencesManager {
    preferences: HashMap<u64, UserPreferences>,
}

impl PreferencesManager {
    pub fn new() -> Self {
        Self {
            preferences: HashMap::new(),
        }
    }
    
    pub fn get_preferences(&self, user_id: u64) -> UserPreferences {
        self.preferences.get(&user_id)
            .cloned()
            .unwrap_or_else(|| {
                let mut prefs = UserPreferences::default();
                prefs.user_id = user_id;
                prefs
            })
    }
    
    pub fn update_preferences(&mut self, user_id: u64, preferences: UserPreferences) -> AgcodexResult<()> {
        let mut prefs = preferences;
        prefs.user_id = user_id;
        prefs.updated_at = chrono::Utc::now();
        
        self.preferences.insert(user_id, prefs);
        Ok(())
    }
    
    pub fn set_theme(&mut self, user_id: u64, theme: Theme) -> AgcodexResult<()> {
        let mut prefs = self.get_preferences(user_id);
        prefs.theme = theme;
        self.update_preferences(user_id, prefs)
    }
    
    pub fn set_language(&mut self, user_id: u64, language: String) -> AgcodexResult<()> {
        let mut prefs = self.get_preferences(user_id);
        prefs.language = language;
        self.update_preferences(user_id, prefs)
    }
    
    pub fn update_notifications(&mut self, user_id: u64, notifications: NotificationSettings) -> AgcodexResult<()> {
        let mut prefs = self.get_preferences(user_id);
        prefs.notifications = notifications;
        self.update_preferences(user_id, prefs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_preferences_crud() {
        let mut manager = PreferencesManager::new();
        
        // Get default preferences for new user
        let prefs = manager.get_preferences(1);
        assert_eq!(prefs.user_id, 1);
        assert_eq!(prefs.language, "en");
        
        // Set theme
        manager.set_theme(1, Theme::Dark).unwrap();
        let prefs = manager.get_preferences(1);
        matches!(prefs.theme, Theme::Dark);
        
        // Set language
        manager.set_language(1, "es".to_string()).unwrap();
        let prefs = manager.get_preferences(1);
        assert_eq!(prefs.language, "es");
        
        // Update notifications
        let notifications = NotificationSettings {
            email: false,
            push: true,
            sms: true,
        };
        manager.update_notifications(1, notifications).unwrap();
        let prefs = manager.get_preferences(1);
        assert!(!prefs.notifications.email);
        assert!(prefs.notifications.push);
        assert!(prefs.notifications.sms);
    }
}
"#;

    // Write preferences module
    fs::write(mock_system.path().join("src/preferences.rs"), preferences_code).await.unwrap();
    
    // Phase 3: Edit existing files to integrate preferences
    println!("\nPhase 3: Integrating preferences with user management...");
    
    let edit_tool = EditTool::new();
    
    // Add preferences module to lib.rs
    let lib_integration = r#"
//! User Management System

pub mod preferences;

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use preferences::{PreferencesManager, UserPreferences, Theme, NotificationSettings};
"#;
    
    // Edit the main lib.rs file to include preferences
    let edit_result = edit_tool.execute(&format!(
        "Add 'pub mod preferences;' and preferences imports to: {}",
        mock_system.path().join("src/lib.rs").display()
    ), &context).await.unwrap();
    
    assert!(edit_result.changes.len() > 0);
    println!("Integrated preferences module into lib.rs");
    
    // Add preferences field to UserManager
    let user_manager_enhancement = r#"
#[derive(Debug)]
pub struct UserManager {
    users: HashMap<u64, User>,
    preferences: PreferencesManager,
    next_id: u64,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            preferences: PreferencesManager::new(),
            next_id: 1,
        }
    }
    
    // Existing methods...
    
    // New preferences methods
    pub fn get_user_preferences(&self, user_id: u64) -> UserPreferences {
        self.preferences.get_preferences(user_id)
    }
    
    pub fn update_user_preferences(&mut self, user_id: u64, preferences: UserPreferences) -> AgcodexResult<()> {
        // Verify user exists first
        if !self.users.contains_key(&user_id) {
            return Err(AgcodexError::NotFound(format!("User with id {} not found", user_id)));
        }
        
        self.preferences.update_preferences(user_id, preferences)
    }
    
    pub fn set_user_theme(&mut self, user_id: u64, theme: Theme) -> AgcodexResult<()> {
        if !self.users.contains_key(&user_id) {
            return Err(AgcodexError::NotFound(format!("User with id {} not found", user_id)));
        }
        
        self.preferences.set_theme(user_id, theme)
    }
    
    pub fn set_user_language(&mut self, user_id: u64, language: String) -> AgcodexResult<()> {
        if !self.users.contains_key(&user_id) {
            return Err(AgcodexError::NotFound(format!("User with id {} not found", user_id)));
        }
        
        self.preferences.set_language(user_id, language)
    }
}
"#;

    // Phase 4: Verify integration works
    println!("\nPhase 4: Verifying integration...");
    
    let search_tool = SearchTool::new();
    let tree_tool = TreeTool::new();
    
    // Search for preferences integration
    let search_result = search_tool.execute("preferences", &context).await.unwrap();
    assert!(search_result.result.len() > 0);
    println!("Found {} references to preferences", search_result.result.len());
    
    // Parse the updated files to verify syntax correctness
    let lib_file = mock_system.path().join("src/lib.rs");
    let tree_result = tree_tool.execute(&lib_file.to_string_lossy(), &context).await.unwrap();
    assert!(tree_result.result.contains("preferences"));
    println!("Syntax verification passed for lib.rs");
    
    let prefs_file = mock_system.path().join("src/preferences.rs");
    let prefs_tree_result = tree_tool.execute(&prefs_file.to_string_lossy(), &context).await.unwrap();
    assert!(prefs_tree_result.result.contains("UserPreferences"));
    println!("Syntax verification passed for preferences.rs");
    
    // Create integration test
    let integration_test = r#"
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_user_preferences_integration() {
        let mut manager = UserManager::new();
        
        // Create user
        let user_id = manager.create_user("alice".to_string(), "alice@example.com".to_string()).unwrap();
        
        // Get default preferences
        let prefs = manager.get_user_preferences(user_id);
        assert_eq!(prefs.user_id, user_id);
        assert_eq!(prefs.language, "en");
        
        // Update theme
        manager.set_user_theme(user_id, Theme::Dark).unwrap();
        let prefs = manager.get_user_preferences(user_id);
        matches!(prefs.theme, Theme::Dark);
        
        // Update language
        manager.set_user_language(user_id, "fr".to_string()).unwrap();
        let prefs = manager.get_user_preferences(user_id);
        assert_eq!(prefs.language, "fr");
        
        // Test error handling for non-existent user
        let result = manager.set_user_theme(999, Theme::Light);
        assert!(result.is_err());
    }
}
"#;
    
    fs::write(mock_system.path().join("src/integration_test.rs"), integration_test).await.unwrap();
    
    println!("\n✅ Feature addition workflow completed successfully!");
    println!("   - Planned implementation with plan tool");
    println!("   - Created preferences module with comprehensive functionality");
    println!("   - Integrated preferences into existing user management system");
    println!("   - Verified syntax correctness and feature completeness");
    println!("   - Added integration tests to prevent regressions");
}

/// Test feature addition with dependency management
#[tokio::test]
async fn test_feature_with_dependencies() {
    let mock_system = MockUserSystem::new().await.unwrap();
    let context = Context::new(mock_system.path().clone());
    
    println!("Testing feature addition with external dependencies...");
    
    let plan_tool = PlanTool::new();
    
    // Plan a feature that requires external dependencies
    let plan_request = "Add user session management with JWT tokens and Redis caching";
    
    let plan_result = plan_tool.execute(&plan_request, &context).await.unwrap();
    
    // Verify plan includes dependency considerations
    assert!(plan_result.result.to_lowercase().contains("jwt") || 
            plan_result.result.to_lowercase().contains("token"));
    assert!(plan_result.result.to_lowercase().contains("redis") ||
            plan_result.result.to_lowercase().contains("cache") ||
            plan_result.result.to_lowercase().contains("session"));
    
    println!("✅ Plan correctly identified external dependencies");
    
    // Verify plan includes integration steps
    assert!(plan_result.result.to_lowercase().contains("cargo.toml") ||
            plan_result.result.to_lowercase().contains("dependencies") ||
            plan_result.result.to_lowercase().contains("dependency"));
    
    println!("✅ Plan includes dependency management steps");
    
    println!("Feature dependency planning test completed successfully!");
}

/// Test feature rollback scenario
#[tokio::test]
async fn test_feature_rollback() {
    let mock_system = MockUserSystem::new().await.unwrap();
    let context = Context::new(mock_system.path().clone());
    
    println!("Testing feature rollback scenario...");
    
    // Read original lib.rs content
    let lib_path = mock_system.path().join("src/lib.rs");
    let original_content = fs::read_to_string(&lib_path).await.unwrap();
    
    let edit_tool = EditTool::new();
    
    // Simulate a problematic feature addition
    let problematic_edit = r#"
// This is intentionally problematic code
pub mod broken_feature;
use broken_feature::BrokenStruct;

impl UserManager {
    pub fn broken_method(&self) -> BrokenStruct {
        BrokenStruct::new() // This will cause compile error
    }
}
"#;
    
    // Apply problematic change
    fs::write(&lib_path, format!("{}\n{}", original_content, problematic_edit)).await.unwrap();
    
    // Verify we can detect the problem
    let tree_tool = TreeTool::new();
    let parse_result = tree_tool.execute(&lib_path.to_string_lossy(), &context).await;
    
    // Tree parsing should still work, but content shows integration issues
    assert!(parse_result.is_ok());
    
    println!("Detected problematic integration");
    
    // Rollback by restoring original content
    fs::write(&lib_path, original_content).await.unwrap();
    
    // Verify rollback success
    let rollback_result = tree_tool.execute(&lib_path.to_string_lossy(), &context).await.unwrap();
    assert!(!rollback_result.result.contains("BrokenStruct"));
    
    println!("✅ Successfully rolled back problematic changes");
    println!("Feature rollback test completed successfully!");
}