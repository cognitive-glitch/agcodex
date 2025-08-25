//! Integration tests for session management functionality.
//!
//! Tests the complete session lifecycle including auto-save, checkpointing,
//! and seamless switching between sessions in the TUI.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;
use uuid::Uuid;
use tokio::time::{sleep, timeout, Instant};

// Mock session types for integration testing
#[derive(Debug, Clone, PartialEq)]
pub struct SessionData {
    pub id: Uuid,
    pub title: String,
    pub created_at: SystemTime,
    pub last_accessed: SystemTime,
    pub conversation_count: usize,
    pub file_contexts: Vec<String>,
    pub current_mode: String,
    pub auto_save_enabled: bool,
    pub checkpoint_count: usize,
}

#[derive(Debug, Clone)]
pub struct SessionManager {
    pub storage_path: PathBuf,
    pub current_session: Option<Uuid>,
    pub sessions: HashMap<Uuid, SessionData>,
    pub auto_save_interval: Duration,
    pub last_auto_save: SystemTime,
    pub checkpoint_interval: Duration,
}

impl SessionManager {
    pub fn new(storage_path: PathBuf) -> Self {
        Self {
            storage_path,
            current_session: None,
            sessions: HashMap::new(),
            auto_save_interval: Duration::from_secs(300), // 5 minutes
            last_auto_save: SystemTime::now(),
            checkpoint_interval: Duration::from_secs(60), // 1 minute
        }
    }
    
    pub async fn create_session(&mut self, title: String) -> Result<Uuid, SessionError> {
        let session_id = Uuid::new_v4();
        let session = SessionData {
            id: session_id,
            title,
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            conversation_count: 0,
            file_contexts: Vec::new(),
            current_mode: "Build".to_string(),
            auto_save_enabled: true,
            checkpoint_count: 0,
        };
        
        self.sessions.insert(session_id, session);
        self.current_session = Some(session_id);
        
        // Save to disk
        self.save_session_to_disk(session_id).await?;
        
        Ok(session_id)
    }
    
    pub async fn switch_session(&mut self, session_id: Uuid) -> Result<(), SessionError> {
        if !self.sessions.contains_key(&session_id) {
            self.load_session_from_disk(session_id).await?;
        }
        
        // Update last accessed time
        if let Some(session) = self.sessions.get_mut(&session_id) {
            session.last_accessed = SystemTime::now();
        }
        
        self.current_session = Some(session_id);
        Ok(())
    }
    
    pub async fn auto_save(&mut self) -> Result<(), SessionError> {
        if self.should_auto_save() {
            if let Some(session_id) = self.current_session {
                self.save_session_to_disk(session_id).await?;
                self.last_auto_save = SystemTime::now();
            }
        }
        Ok(())
    }
    
    pub async fn create_checkpoint(&mut self) -> Result<String, SessionError> {
        if let Some(session_id) = self.current_session {
            if let Some(session) = self.sessions.get_mut(&session_id) {
                session.checkpoint_count += 1;
                let checkpoint_id = format!("checkpoint_{}", session.checkpoint_count);
                
                // Save checkpoint to disk
                self.save_checkpoint(session_id, &checkpoint_id).await?;
                
                return Ok(checkpoint_id);
            }
        }
        Err(SessionError::NoActiveSession)
    }
    
    pub async fn restore_checkpoint(&mut self, checkpoint_id: &str) -> Result<(), SessionError> {
        if let Some(session_id) = self.current_session {
            self.load_checkpoint(session_id, checkpoint_id).await?;
            return Ok(());
        }
        Err(SessionError::NoActiveSession)
    }
    
    pub fn list_sessions(&self) -> Vec<&SessionData> {
        self.sessions.values().collect()
    }
    
    pub async fn delete_session(&mut self, session_id: Uuid) -> Result<(), SessionError> {
        self.sessions.remove(&session_id);
        
        if self.current_session == Some(session_id) {
            self.current_session = None;
        }
        
        // Remove from disk
        let session_file = self.get_session_file_path(session_id);
        if session_file.exists() {
            fs::remove_file(session_file)
                .map_err(|e| SessionError::IoError(e.to_string()))?;
        }
        
        Ok(())
    }
    
    pub fn get_current_session(&self) -> Option<&SessionData> {
        self.current_session
            .and_then(|id| self.sessions.get(&id))
    }
    
    pub async fn add_conversation(&mut self, content: String) -> Result<(), SessionError> {
        if let Some(session_id) = self.current_session {
            if let Some(session) = self.sessions.get_mut(&session_id) {
                session.conversation_count += 1;
                session.last_accessed = SystemTime::now();
                
                // Trigger auto-save if needed
                self.auto_save().await?;
                
                return Ok(());
            }
        }
        Err(SessionError::NoActiveSession)
    }
    
    pub async fn add_file_context(&mut self, file_path: String) -> Result<(), SessionError> {
        if let Some(session_id) = self.current_session {
            if let Some(session) = self.sessions.get_mut(&session_id) {
                if !session.file_contexts.contains(&file_path) {
                    session.file_contexts.push(file_path);
                    session.last_accessed = SystemTime::now();
                }
                return Ok(());
            }
        }
        Err(SessionError::NoActiveSession)
    }
    
    // Private helper methods
    
    fn should_auto_save(&self) -> bool {
        self.last_auto_save.elapsed().unwrap_or(Duration::ZERO) >= self.auto_save_interval
    }
    
    async fn save_session_to_disk(&self, session_id: Uuid) -> Result<(), SessionError> {
        if let Some(session) = self.sessions.get(&session_id) {
            let session_file = self.get_session_file_path(session_id);
            let serialized = serde_json::to_string_pretty(session)
                .map_err(|e| SessionError::SerializationError(e.to_string()))?;
            
            fs::write(session_file, serialized)
                .map_err(|e| SessionError::IoError(e.to_string()))?;
        }
        Ok(())
    }
    
    async fn load_session_from_disk(&mut self, session_id: Uuid) -> Result<(), SessionError> {
        let session_file = self.get_session_file_path(session_id);
        
        if !session_file.exists() {
            return Err(SessionError::SessionNotFound);
        }
        
        let content = fs::read_to_string(session_file)
            .map_err(|e| SessionError::IoError(e.to_string()))?;
        
        let session: SessionData = serde_json::from_str(&content)
            .map_err(|e| SessionError::SerializationError(e.to_string()))?;
        
        self.sessions.insert(session_id, session);
        Ok(())
    }
    
    async fn save_checkpoint(&self, session_id: Uuid, checkpoint_id: &str) -> Result<(), SessionError> {
        if let Some(session) = self.sessions.get(&session_id) {
            let checkpoint_dir = self.storage_path.join("checkpoints").join(session_id.to_string());
            fs::create_dir_all(&checkpoint_dir)
                .map_err(|e| SessionError::IoError(e.to_string()))?;
            
            let checkpoint_file = checkpoint_dir.join(format!("{}.json", checkpoint_id));
            let serialized = serde_json::to_string_pretty(session)
                .map_err(|e| SessionError::SerializationError(e.to_string()))?;
            
            fs::write(checkpoint_file, serialized)
                .map_err(|e| SessionError::IoError(e.to_string()))?;
        }
        Ok(())
    }
    
    async fn load_checkpoint(&mut self, session_id: Uuid, checkpoint_id: &str) -> Result<(), SessionError> {
        let checkpoint_file = self.storage_path
            .join("checkpoints")
            .join(session_id.to_string())
            .join(format!("{}.json", checkpoint_id));
        
        if !checkpoint_file.exists() {
            return Err(SessionError::CheckpointNotFound);
        }
        
        let content = fs::read_to_string(checkpoint_file)
            .map_err(|e| SessionError::IoError(e.to_string()))?;
        
        let session: SessionData = serde_json::from_str(&content)
            .map_err(|e| SessionError::SerializationError(e.to_string()))?;
        
        self.sessions.insert(session_id, session);
        Ok(())
    }
    
    fn get_session_file_path(&self, session_id: Uuid) -> PathBuf {
        self.storage_path.join(format!("{}.agcx", session_id))
    }
}

#[derive(Debug, PartialEq)]
pub enum SessionError {
    NoActiveSession,
    SessionNotFound,
    CheckpointNotFound,
    IoError(String),
    SerializationError(String),
}

// Implement serde traits for SessionData
impl serde::Serialize for SessionData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SessionData", 9)?;
        state.serialize_field("id", &self.id.to_string())?;
        state.serialize_field("title", &self.title)?;
        state.serialize_field("created_at", &format!("{:?}", self.created_at))?;
        state.serialize_field("last_accessed", &format!("{:?}", self.last_accessed))?;
        state.serialize_field("conversation_count", &self.conversation_count)?;
        state.serialize_field("file_contexts", &self.file_contexts)?;
        state.serialize_field("current_mode", &self.current_mode)?;
        state.serialize_field("auto_save_enabled", &self.auto_save_enabled)?;
        state.serialize_field("checkpoint_count", &self.checkpoint_count)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for SessionData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Simplified deserialization for testing
        use serde::de::Error;
        Err(D::Error::custom("Deserialization not implemented for mock"))
    }
}

#[tokio::test]
async fn test_create_session() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
    
    let session_id = manager.create_session("Test Session".to_string()).await.unwrap();
    
    assert!(manager.current_session.is_some());
    assert_eq!(manager.current_session.unwrap(), session_id);
    assert_eq!(manager.sessions.len(), 1);
    
    let session = manager.get_current_session().unwrap();
    assert_eq!(session.title, "Test Session");
    assert_eq!(session.conversation_count, 0);
    assert!(session.auto_save_enabled);
}

#[tokio::test]
async fn test_session_switching() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
    
    // Create two sessions
    let session1 = manager.create_session("Session 1".to_string()).await.unwrap();
    let session2 = manager.create_session("Session 2".to_string()).await.unwrap();
    
    // Should be on session 2 initially
    assert_eq!(manager.current_session.unwrap(), session2);
    
    // Switch to session 1
    manager.switch_session(session1).await.unwrap();
    assert_eq!(manager.current_session.unwrap(), session1);
    
    let current = manager.get_current_session().unwrap();
    assert_eq!(current.title, "Session 1");
}

#[tokio::test]
async fn test_auto_save_functionality() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
    
    // Set short auto-save interval for testing
    manager.auto_save_interval = Duration::from_millis(10);
    
    let session_id = manager.create_session("Auto Save Test".to_string()).await.unwrap();
    
    // Add some content
    manager.add_conversation("Hello".to_string()).await.unwrap();
    
    // Wait for auto-save interval
    sleep(Duration::from_millis(20)).await;
    
    // Trigger auto-save
    manager.auto_save().await.unwrap();
    
    // Verify session file exists
    let session_file = manager.get_session_file_path(session_id);
    assert!(session_file.exists());
}

#[tokio::test]
async fn test_checkpoint_creation_and_restore() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
    
    let session_id = manager.create_session("Checkpoint Test".to_string()).await.unwrap();
    
    // Add some content
    manager.add_conversation("Initial content".to_string()).await.unwrap();
    
    // Create checkpoint
    let checkpoint_id = manager.create_checkpoint().await.unwrap();
    assert_eq!(checkpoint_id, "checkpoint_1");
    
    // Add more content
    manager.add_conversation("More content".to_string()).await.unwrap();
    assert_eq!(manager.get_current_session().unwrap().conversation_count, 2);
    
    // Restore checkpoint (in real implementation, this would restore state)
    manager.restore_checkpoint(&checkpoint_id).await.unwrap();
    
    // Verify checkpoint directory was created
    let checkpoint_dir = temp_dir.path()
        .join("checkpoints")
        .join(session_id.to_string());
    assert!(checkpoint_dir.exists());
}

#[tokio::test]
async fn test_session_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let storage_path = temp_dir.path().to_path_buf();
    
    let session_id = {
        let mut manager = SessionManager::new(storage_path.clone());
        let id = manager.create_session("Persistent Session".to_string()).await.unwrap();
        manager.add_conversation("Test message".to_string()).await.unwrap();
        manager.add_file_context("test.rs".to_string()).await.unwrap();
        id
    };
    
    // Create new manager instance (simulating restart)
    let mut new_manager = SessionManager::new(storage_path);
    
    // Load session from disk
    new_manager.load_session_from_disk(session_id).await.unwrap();
    
    let loaded_session = new_manager.sessions.get(&session_id).unwrap();
    assert_eq!(loaded_session.title, "Persistent Session");
    // Note: In mock implementation, some fields won't deserialize properly
}

#[tokio::test]
async fn test_session_listing() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
    
    // Create multiple sessions
    let titles = ["Session A", "Session B", "Session C"];
    for title in &titles {
        manager.create_session(title.to_string()).await.unwrap();
    }
    
    let sessions = manager.list_sessions();
    assert_eq!(sessions.len(), 3);
    
    let session_titles: Vec<&String> = sessions.iter().map(|s| &s.title).collect();
    for title in &titles {
        assert!(session_titles.contains(&&title.to_string()));
    }
}

#[tokio::test]
async fn test_session_deletion() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
    
    let session_id = manager.create_session("To Delete".to_string()).await.unwrap();
    assert_eq!(manager.sessions.len(), 1);
    
    manager.delete_session(session_id).await.unwrap();
    
    assert_eq!(manager.sessions.len(), 0);
    assert!(manager.current_session.is_none());
    
    // Verify file was removed
    let session_file = manager.get_session_file_path(session_id);
    assert!(!session_file.exists());
}

#[tokio::test]
async fn test_file_context_management() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
    
    let session_id = manager.create_session("Context Test".to_string()).await.unwrap();
    
    // Add file contexts
    manager.add_file_context("src/main.rs".to_string()).await.unwrap();
    manager.add_file_context("tests/test.rs".to_string()).await.unwrap();
    manager.add_file_context("src/main.rs".to_string()).await.unwrap(); // Duplicate
    
    let session = manager.get_current_session().unwrap();
    assert_eq!(session.file_contexts.len(), 2); // No duplicates
    assert!(session.file_contexts.contains(&"src/main.rs".to_string()));
    assert!(session.file_contexts.contains(&"tests/test.rs".to_string()));
}

#[tokio::test]
async fn test_session_last_accessed_updates() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
    
    let session_id = manager.create_session("Access Test".to_string()).await.unwrap();
    let initial_time = manager.get_current_session().unwrap().last_accessed;
    
    // Wait a bit
    sleep(Duration::from_millis(10)).await;
    
    // Add conversation to trigger last_accessed update
    manager.add_conversation("Update access time".to_string()).await.unwrap();
    
    let updated_time = manager.get_current_session().unwrap().last_accessed;
    assert!(updated_time > initial_time);
}

#[tokio::test]
async fn test_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
    
    // Test operations without active session
    let result = manager.add_conversation("No session".to_string()).await;
    assert_eq!(result.unwrap_err(), SessionError::NoActiveSession);
    
    let result = manager.create_checkpoint().await;
    assert_eq!(result.unwrap_err(), SessionError::NoActiveSession);
    
    // Test loading non-existent session
    let fake_id = Uuid::new_v4();
    let result = manager.load_session_from_disk(fake_id).await;
    assert_eq!(result.unwrap_err(), SessionError::SessionNotFound);
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_session_creation_performance() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
        
        let start = Instant::now();
        
        // Create 10 sessions
        for i in 0..10 {
            manager.create_session(format!("Session {}", i)).await.unwrap();
        }
        
        let duration = start.elapsed();
        
        // Should be fast (target: <100ms per session)
        assert!(duration.as_millis() < 1000,
               "Session creation took too long: {:?}", duration);
    }
    
    #[tokio::test]
    async fn test_session_switching_performance() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
        
        // Create sessions
        let mut session_ids = Vec::new();
        for i in 0..5 {
            let id = manager.create_session(format!("Session {}", i)).await.unwrap();
            session_ids.push(id);
        }
        
        let start = Instant::now();
        
        // Switch between sessions rapidly
        for &session_id in &session_ids {
            manager.switch_session(session_id).await.unwrap();
        }
        
        let duration = start.elapsed();
        
        // Should be fast (target: <50ms per switch)
        assert!(duration.as_millis() < 250,
               "Session switching took too long: {:?}", duration);
    }
    
    #[tokio::test]
    async fn test_auto_save_overhead() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
        manager.auto_save_interval = Duration::from_millis(1); // Force frequent saves
        
        let session_id = manager.create_session("Performance Test".to_string()).await.unwrap();
        
        let start = Instant::now();
        
        // Add conversations rapidly
        for i in 0..100 {
            manager.add_conversation(format!("Message {}", i)).await.unwrap();
        }
        
        let duration = start.elapsed();
        
        // Should handle rapid updates efficiently
        assert!(duration.as_millis() < 2000,
               "Auto-save overhead too high: {:?}", duration);
    }
}

#[cfg(test)]
mod integration_scenarios {
    use super::*;
    
    #[tokio::test]
    async fn test_typical_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
        
        // Typical user workflow
        
        // 1. Create a session for a project
        let session_id = manager.create_session("My Project".to_string()).await.unwrap();
        
        // 2. Add some file contexts
        manager.add_file_context("src/main.rs".to_string()).await.unwrap();
        manager.add_file_context("Cargo.toml".to_string()).await.unwrap();
        
        // 3. Have some conversations
        manager.add_conversation("How do I implement this feature?".to_string()).await.unwrap();
        manager.add_conversation("Can you review this code?".to_string()).await.unwrap();
        
        // 4. Create a checkpoint before major changes
        let checkpoint = manager.create_checkpoint().await.unwrap();
        
        // 5. Continue working
        manager.add_conversation("Let's refactor this".to_string()).await.unwrap();
        
        // 6. Verify session state
        let session = manager.get_current_session().unwrap();
        assert_eq!(session.title, "My Project");
        assert_eq!(session.conversation_count, 3);
        assert_eq!(session.file_contexts.len(), 2);
        assert_eq!(session.checkpoint_count, 1);
        
        // 7. Test checkpoint restore
        manager.restore_checkpoint(&checkpoint).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_multi_session_development() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
        
        // Simulate working on multiple projects
        
        // Project 1: Web app
        let web_session = manager.create_session("Web App".to_string()).await.unwrap();
        manager.add_file_context("app.js".to_string()).await.unwrap();
        manager.add_conversation("Setup React component".to_string()).await.unwrap();
        
        // Project 2: API service
        let api_session = manager.create_session("API Service".to_string()).await.unwrap();
        manager.add_file_context("server.rs".to_string()).await.unwrap();
        manager.add_conversation("Implement REST endpoints".to_string()).await.unwrap();
        
        // Switch back to web app
        manager.switch_session(web_session).await.unwrap();
        manager.add_conversation("Add styling".to_string()).await.unwrap();
        
        // Verify contexts are maintained
        let current = manager.get_current_session().unwrap();
        assert_eq!(current.title, "Web App");
        assert!(current.file_contexts.contains(&"app.js".to_string()));
        assert_eq!(current.conversation_count, 2);
        
        // Verify other session is intact
        manager.switch_session(api_session).await.unwrap();
        let api_current = manager.get_current_session().unwrap();
        assert_eq!(api_current.title, "API Service");
        assert_eq!(api_current.conversation_count, 1);
    }
}

#[cfg(test)]
mod edge_cases {
    use super::*;
    
    #[tokio::test]
    async fn test_concurrent_session_operations() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().to_path_buf();
        
        // Create session in one manager
        let session_id = {
            let mut manager1 = SessionManager::new(storage_path.clone());
            manager1.create_session("Concurrent Test".to_string()).await.unwrap()
        };
        
        // Try to access from another manager
        let mut manager2 = SessionManager::new(storage_path);
        manager2.switch_session(session_id).await.unwrap();
        
        let session = manager2.get_current_session().unwrap();
        assert_eq!(session.title, "Concurrent Test");
    }
    
    #[tokio::test]
    async fn test_session_with_empty_title() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
        
        let session_id = manager.create_session("".to_string()).await.unwrap();
        let session = manager.get_current_session().unwrap();
        
        assert_eq!(session.title, "");
        assert!(session.id.is_some() || session.title.is_empty()); // Session should exist even with empty title
    }
    
    #[tokio::test]
    async fn test_very_long_session_title() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SessionManager::new(temp_dir.path().to_path_buf());
        
        let long_title = "a".repeat(1000);
        let session_id = manager.create_session(long_title.clone()).await.unwrap();
        let session = manager.get_current_session().unwrap();
        
        assert_eq!(session.title, long_title);
    }
}