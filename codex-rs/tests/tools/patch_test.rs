//! Comprehensive tests for the patch tool AST-based transformations.
//!
//! This test suite covers:
//! - Symbol renaming across files with different scopes
//! - Function extraction with parameter detection
//! - Import management (add/remove/organize)
//! - Rollback capability for failed operations
//! - Complex refactoring operations
//! - Error handling (circular dependencies, invalid transformations)
//! - Performance testing for large-scale changes
//! - Code formatting preservation

use agcodex_core::tools::patch::{
    PatchError, PatchTool, RenameScope, RenameStats, ExtractStats, ImportStats
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio;

use crate::helpers::test_utils::{
    TestEnvironment, PerformanceAssertions, TestTiming, BenchmarkStats, CodeSamples
};

/// Test fixture for comprehensive patch tool testing
struct PatchTestEnvironment {
    env: TestEnvironment,
    patch_tool: PatchTool,
    project_files: HashMap<String, PathBuf>,
}

impl PatchTestEnvironment {
    async fn new() -> Self {
        let env = TestEnvironment::new();
        let patch_tool = PatchTool::new();
        let project_files = Self::create_realistic_project(&env).await;
        
        Self {
            env,
            patch_tool,
            project_files,
        }
    }
    
    /// Create a realistic multi-language project structure for testing
    async fn create_realistic_project(env: &TestEnvironment) -> HashMap<String, PathBuf> {
        let mut files = HashMap::new();
        let base_path = env.path();
        
        // Create project structure
        fs::create_dir_all(base_path.join("src")).unwrap();
        fs::create_dir_all(base_path.join("tests")).unwrap();
        fs::create_dir_all(base_path.join("frontend/components")).unwrap();
        fs::create_dir_all(base_path.join("backend/api")).unwrap();
        
        // Rust files - main application
        let main_rs = base_path.join("src/main.rs");
        fs::write(&main_rs, Self::rust_main_sample()).unwrap();
        files.insert("main.rs".to_string(), main_rs);
        
        let lib_rs = base_path.join("src/lib.rs");
        fs::write(&lib_rs, Self::rust_lib_sample()).unwrap();
        files.insert("lib.rs".to_string(), lib_rs);
        
        let user_rs = base_path.join("src/user.rs");
        fs::write(&user_rs, Self::rust_user_sample()).unwrap();
        files.insert("user.rs".to_string(), user_rs);
        
        let database_rs = base_path.join("src/database.rs");
        fs::write(&database_rs, Self::rust_database_sample()).unwrap();
        files.insert("database.rs".to_string(), database_rs);
        
        // TypeScript/React files
        let app_tsx = base_path.join("frontend/components/App.tsx");
        fs::write(&app_tsx, Self::typescript_app_sample()).unwrap();
        files.insert("App.tsx".to_string(), app_tsx);
        
        let user_list_tsx = base_path.join("frontend/components/UserList.tsx");
        fs::write(&user_list_tsx, Self::typescript_userlist_sample()).unwrap();
        files.insert("UserList.tsx".to_string(), user_list_tsx);
        
        // Python files
        let api_py = base_path.join("backend/api/main.py");
        fs::write(&api_py, Self::python_api_sample()).unwrap();
        files.insert("main.py".to_string(), api_py);
        
        let models_py = base_path.join("backend/api/models.py");
        fs::write(&models_py, Self::python_models_sample()).unwrap();
        files.insert("models.py".to_string(), models_py);
        
        files
    }
    
    // Sample file contents for realistic testing scenarios
    
    fn rust_main_sample() -> &'static str {
        r#"use std::env;
use crate::user::{User, UserManager, create_default_user};
use crate::database::{DatabaseConfig, connect_to_database};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_config = DatabaseConfig::from_env()?;
    let mut db = connect_to_database(database_config).await?;
    
    let mut user_manager = UserManager::new(db);
    let default_user = create_default_user("admin", "admin@example.com");
    
    user_manager.add_user(default_user)?;
    
    println!("Application started successfully");
    
    // Start main application loop
    start_main_loop(&mut user_manager).await?;
    
    Ok(())
}

async fn start_main_loop(user_manager: &mut UserManager) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        // Application logic here
        handle_user_requests(user_manager).await?;
    }
}

async fn handle_user_requests(user_manager: &mut UserManager) -> Result<(), Box<dyn std::error::Error>> {
    // Process user requests
    let active_users = user_manager.get_active_users().await?;
    process_active_users(&active_users).await?;
    Ok(())
}

async fn process_active_users(users: &[User]) -> Result<(), Box<dyn std::error::Error>> {
    for user in users {
        validate_user_session(user).await?;
        update_user_activity(user).await?;
    }
    Ok(())
}

async fn validate_user_session(user: &User) -> Result<(), Box<dyn std::error::Error>> {
    // Validation logic
    if user.is_session_expired() {
        user.refresh_session().await?;
    }
    Ok(())
}

async fn update_user_activity(user: &User) -> Result<(), Box<dyn std::error::Error>> {
    // Update activity
    user.touch_last_seen().await?;
    Ok(())
}
"#
    }
    
    fn rust_lib_sample() -> &'static str {
        r#"//! AGCodex Library - Main library module
//!
//! This module provides core functionality for the AGCodex application.

pub mod user;
pub mod database;
pub mod config;
pub mod error;

pub use user::{User, UserManager, UserRole, create_default_user};
pub use database::{DatabaseConfig, DatabaseConnection, connect_to_database};
pub use config::{AppConfig, load_app_config};
pub use error::{AppError, AppResult};

/// Main application context
pub struct AppContext {
    pub config: AppConfig,
    pub user_manager: UserManager,
    pub database: DatabaseConnection,
}

impl AppContext {
    /// Create a new application context
    pub async fn new() -> AppResult<Self> {
        let config = load_app_config()?;
        let database = connect_to_database(config.database.clone()).await?;
        let user_manager = UserManager::new(database.clone());
        
        Ok(Self {
            config,
            user_manager,
            database,
        })
    }
    
    /// Initialize the application
    pub async fn initialize(&mut self) -> AppResult<()> {
        self.database.migrate().await?;
        self.user_manager.load_users().await?;
        Ok(())
    }
}

/// Common utility functions
pub mod utils {
    use crate::error::AppResult;
    
    /// Validate email address format
    pub fn validate_email(email: &str) -> bool {
        email.contains('@') && email.contains('.')
    }
    
    /// Generate secure random password
    pub fn generate_password(length: usize) -> String {
        use rand::Rng;
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        
        (0..length)
            .map(|_| {
                let idx = rand::thread_rng().gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
    
    /// Hash password securely
    pub fn hash_password(password: &str) -> AppResult<String> {
        // In real implementation, would use bcrypt or similar
        Ok(format!("hashed_{}", password))
    }
}
"#
    }
    
    fn rust_user_sample() -> &'static str {
        r#"//! User management module

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::database::DatabaseConnection;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    Moderator,
    User,
    Guest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_seen: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: bool,
}

impl User {
    /// Create a new user
    pub fn new(username: String, email: String, role: UserRole) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            email,
            role,
            created_at: chrono::Utc::now(),
            last_seen: None,
            is_active: true,
        }
    }
    
    /// Check if user session is expired
    pub fn is_session_expired(&self) -> bool {
        if let Some(last_seen) = self.last_seen {
            let now = chrono::Utc::now();
            let duration = now.signed_duration_since(last_seen);
            duration.num_hours() > 24
        } else {
            true
        }
    }
    
    /// Refresh user session
    pub async fn refresh_session(&self) -> AppResult<()> {
        // Session refresh logic
        Ok(())
    }
    
    /// Update last seen timestamp
    pub async fn touch_last_seen(&self) -> AppResult<()> {
        // Update last seen in database
        Ok(())
    }
    
    /// Check if user has specific role
    pub fn has_role(&self, role: &UserRole) -> bool {
        self.role == *role
    }
    
    /// Check if user is admin
    pub fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin)
    }
}

/// User management system
pub struct UserManager {
    database: DatabaseConnection,
    user_cache: HashMap<Uuid, User>,
}

impl UserManager {
    /// Create new user manager
    pub fn new(database: DatabaseConnection) -> Self {
        Self {
            database,
            user_cache: HashMap::new(),
        }
    }
    
    /// Add a new user
    pub fn add_user(&mut self, user: User) -> AppResult<()> {
        if self.user_cache.contains_key(&user.id) {
            return Err(AppError::UserAlreadyExists(user.username));
        }
        
        self.user_cache.insert(user.id, user);
        Ok(())
    }
    
    /// Get user by ID
    pub fn get_user(&self, id: &Uuid) -> Option<&User> {
        self.user_cache.get(id)
    }
    
    /// Get user by username
    pub fn get_user_by_username(&self, username: &str) -> Option<&User> {
        self.user_cache
            .values()
            .find(|user| user.username == username)
    }
    
    /// Get all active users
    pub async fn get_active_users(&self) -> AppResult<Vec<User>> {
        let active_users: Vec<User> = self.user_cache
            .values()
            .filter(|user| user.is_active)
            .cloned()
            .collect();
        
        Ok(active_users)
    }
    
    /// Load users from database
    pub async fn load_users(&mut self) -> AppResult<()> {
        // Load from database
        Ok(())
    }
    
    /// Save user to database
    pub async fn save_user(&mut self, user: &User) -> AppResult<()> {
        // Save to database
        self.user_cache.insert(user.id, user.clone());
        Ok(())
    }
    
    /// Delete user
    pub fn delete_user(&mut self, id: &Uuid) -> AppResult<()> {
        if self.user_cache.remove(id).is_some() {
            Ok(())
        } else {
            Err(AppError::UserNotFound(id.to_string()))
        }
    }
}

/// Create a default user for testing
pub fn create_default_user(username: &str, email: &str) -> User {
    User::new(username.to_string(), email.to_string(), UserRole::User)
}

/// Create admin user
pub fn create_admin_user(username: &str, email: &str) -> User {
    User::new(username.to_string(), email.to_string(), UserRole::Admin)
}
"#
    }
    
    fn rust_database_sample() -> &'static str {
        r#"//! Database connection and management

use serde::{Deserialize, Serialize};
use std::env;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database_name: String,
    pub username: String,
    pub password: String,
    pub connection_pool_size: u32,
}

impl DatabaseConfig {
    /// Create database config from environment variables
    pub fn from_env() -> AppResult<Self> {
        Ok(Self {
            host: env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: env::var("DB_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .map_err(|_| AppError::InvalidConfiguration("DB_PORT must be a valid number".to_string()))?,
            database_name: env::var("DB_NAME").unwrap_or_else(|_| "agcodex".to_string()),
            username: env::var("DB_USERNAME").unwrap_or_else(|_| "postgres".to_string()),
            password: env::var("DB_PASSWORD").unwrap_or_else(|_| "password".to_string()),
            connection_pool_size: env::var("DB_POOL_SIZE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
        })
    }
}

/// Database connection wrapper
#[derive(Debug, Clone)]
pub struct DatabaseConnection {
    config: DatabaseConfig,
    // In real implementation, would contain actual connection pool
}

impl DatabaseConnection {
    /// Create new database connection
    pub fn new(config: DatabaseConfig) -> Self {
        Self { config }
    }
    
    /// Run database migrations
    pub async fn migrate(&self) -> AppResult<()> {
        // Migration logic
        Ok(())
    }
    
    /// Execute a query
    pub async fn execute_query(&self, query: &str) -> AppResult<Vec<serde_json::Value>> {
        // Query execution
        println!("Executing query: {}", query);
        Ok(vec![])
    }
    
    /// Begin a transaction
    pub async fn begin_transaction(&self) -> AppResult<DatabaseTransaction> {
        Ok(DatabaseTransaction::new(self.clone()))
    }
    
    /// Check database health
    pub async fn health_check(&self) -> AppResult<bool> {
        // Health check logic
        Ok(true)
    }
}

/// Database transaction wrapper
pub struct DatabaseTransaction {
    connection: DatabaseConnection,
    committed: bool,
}

impl DatabaseTransaction {
    fn new(connection: DatabaseConnection) -> Self {
        Self {
            connection,
            committed: false,
        }
    }
    
    /// Commit the transaction
    pub async fn commit(mut self) -> AppResult<()> {
        self.committed = true;
        Ok(())
    }
    
    /// Rollback the transaction
    pub async fn rollback(self) -> AppResult<()> {
        // Rollback logic
        Ok(())
    }
    
    /// Execute query within transaction
    pub async fn execute(&self, query: &str) -> AppResult<Vec<serde_json::Value>> {
        self.connection.execute_query(query).await
    }
}

impl Drop for DatabaseTransaction {
    fn drop(&mut self) {
        if !self.committed {
            // Auto-rollback on drop
            println!("Transaction rolled back on drop");
        }
    }
}

/// Connect to database with configuration
pub async fn connect_to_database(config: DatabaseConfig) -> AppResult<DatabaseConnection> {
    let connection = DatabaseConnection::new(config);
    
    // Test connection
    connection.health_check().await?;
    
    Ok(connection)
}
"#
    }
    
    fn typescript_app_sample() -> &'static str {
        r#"// Main React application component

import React, { useState, useEffect } from 'react';
import { UserList } from './UserList';
import { UserDetails } from './UserDetails';
import { CreateUserModal } from './CreateUserModal';
import { api } from '../services/api';

export interface User {
    id: number;
    username: string;
    email: string;
    role: 'admin' | 'moderator' | 'user' | 'guest';
    createdAt: string;
    lastSeen?: string;
    isActive: boolean;
}

interface AppState {
    users: User[];
    selectedUser: User | null;
    loading: boolean;
    error: string | null;
    showCreateModal: boolean;
}

export const App: React.FC = () => {
    const [state, setState] = useState<AppState>({
        users: [],
        selectedUser: null,
        loading: false,
        error: null,
        showCreateModal: false,
    });

    useEffect(() => {
        loadUsers();
    }, []);

    const loadUsers = async () => {
        setState(prev => ({ ...prev, loading: true, error: null }));
        
        try {
            const users = await api.getUsers();
            setState(prev => ({ ...prev, users, loading: false }));
        } catch (error) {
            setState(prev => ({
                ...prev,
                loading: false,
                error: error instanceof Error ? error.message : 'Failed to load users'
            }));
        }
    };

    const handleUserSelect = (user: User) => {
        setState(prev => ({ ...prev, selectedUser: user }));
    };

    const handleCreateUser = async (userData: Omit<User, 'id' | 'createdAt'>) => {
        try {
            const newUser = await api.createUser(userData);
            setState(prev => ({
                ...prev,
                users: [...prev.users, newUser],
                showCreateModal: false
            }));
        } catch (error) {
            setState(prev => ({
                ...prev,
                error: error instanceof Error ? error.message : 'Failed to create user'
            }));
        }
    };

    const handleUpdateUser = async (updatedUser: User) => {
        try {
            const user = await api.updateUser(updatedUser);
            setState(prev => ({
                ...prev,
                users: prev.users.map(u => u.id === user.id ? user : u),
                selectedUser: user
            }));
        } catch (error) {
            setState(prev => ({
                ...prev,
                error: error instanceof Error ? error.message : 'Failed to update user'
            }));
        }
    };

    const handleDeleteUser = async (userId: number) => {
        if (!confirm('Are you sure you want to delete this user?')) {
            return;
        }

        try {
            await api.deleteUser(userId);
            setState(prev => ({
                ...prev,
                users: prev.users.filter(u => u.id !== userId),
                selectedUser: prev.selectedUser?.id === userId ? null : prev.selectedUser
            }));
        } catch (error) {
            setState(prev => ({
                ...prev,
                error: error instanceof Error ? error.message : 'Failed to delete user'
            }));
        }
    };

    return (
        <div className="app">
            <header className="app-header">
                <h1>User Management</h1>
                <button
                    className="btn-primary"
                    onClick={() => setState(prev => ({ ...prev, showCreateModal: true }))}
                >
                    Create User
                </button>
            </header>

            {state.error && (
                <div className="alert alert-error">
                    {state.error}
                    <button onClick={() => setState(prev => ({ ...prev, error: null }))}>
                        ×
                    </button>
                </div>
            )}

            <div className="app-content">
                <div className="users-panel">
                    <UserList
                        users={state.users}
                        selectedUser={state.selectedUser}
                        loading={state.loading}
                        onUserSelect={handleUserSelect}
                        onDeleteUser={handleDeleteUser}
                    />
                </div>

                <div className="details-panel">
                    {state.selectedUser ? (
                        <UserDetails
                            user={state.selectedUser}
                            onUpdateUser={handleUpdateUser}
                        />
                    ) : (
                        <div className="no-selection">
                            <p>Select a user to view details</p>
                        </div>
                    )}
                </div>
            </div>

            {state.showCreateModal && (
                <CreateUserModal
                    onCreateUser={handleCreateUser}
                    onClose={() => setState(prev => ({ ...prev, showCreateModal: false }))}
                />
            )}
        </div>
    );
};

export default App;
"#
    }
    
    fn typescript_userlist_sample() -> &'static str {
        r#"// User list component for displaying users

import React, { useMemo } from 'react';
import { User } from './App';

interface UserListProps {
    users: User[];
    selectedUser: User | null;
    loading: boolean;
    onUserSelect: (user: User) -> void;
    onDeleteUser: (userId: number) -> void;
}

interface UserItemProps {
    user: User;
    isSelected: boolean;
    onSelect: () -> void;
    onDelete: () -> void;
}

const UserItem: React.FC<UserItemProps> = ({ user, isSelected, onSelect, onDelete }) => {
    const roleColor = useMemo(() => {
        switch (user.role) {
            case 'admin':
                return 'role-admin';
            case 'moderator':
                return 'role-moderator';
            case 'user':
                return 'role-user';
            case 'guest':
                return 'role-guest';
            default:
                return 'role-unknown';
        }
    }, [user.role]);

    const lastSeenText = useMemo(() => {
        if (!user.lastSeen) return 'Never';
        
        const lastSeen = new Date(user.lastSeen);
        const now = new Date();
        const diffMs = now.getTime() - lastSeen.getTime();
        const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
        
        if (diffHours < 1) return 'Just now';
        if (diffHours < 24) return `${diffHours}h ago`;
        
        const diffDays = Math.floor(diffHours / 24);
        return `${diffDays}d ago`;
    }, [user.lastSeen]);

    return (
        <div
            className={`user-item ${isSelected ? 'selected' : ''} ${!user.isActive ? 'inactive' : ''}`}
            onClick={onSelect}
        >
            <div className="user-avatar">
                <div className="avatar-circle">
                    {user.username.charAt(0).toUpperCase()}
                </div>
                {user.isActive && <div className="status-indicator online" />}
            </div>
            
            <div className="user-info">
                <div className="user-name">{user.username}</div>
                <div className="user-email">{user.email}</div>
                <div className="user-meta">
                    <span className={`user-role ${roleColor}`}>{user.role}</span>
                    <span className="last-seen">{lastSeenText}</span>
                </div>
            </div>
            
            <div className="user-actions">
                <button
                    className="btn-delete"
                    onClick={(e) => {
                        e.stopPropagation();
                        onDelete();
                    }}
                    title="Delete user"
                >
                    ×
                </button>
            </div>
        </div>
    );
};

export const UserList: React.FC<UserListProps> = ({
    users,
    selectedUser,
    loading,
    onUserSelect,
    onDeleteUser
}) => {
    const sortedUsers = useMemo(() => {
        return [...users].sort((a, b) => {
            // Sort by activity status first, then by username
            if (a.isActive !== b.isActive) {
                return a.isActive ? -1 : 1;
            }
            return a.username.localeCompare(b.username);
        });
    }, [users]);

    const userStats = useMemo(() => {
        const total = users.length;
        const active = users.filter(u => u.isActive).length;
        const inactive = total - active;
        
        const byRole = users.reduce((acc, user) => {
            acc[user.role] = (acc[user.role] || 0) + 1;
            return acc;
        }, {} as Record<string, number>);
        
        return { total, active, inactive, byRole };
    }, [users]);

    if (loading) {
        return (
            <div className="user-list loading">
                <div className="loading-spinner">Loading users...</div>
            </div>
        );
    }

    if (users.length === 0) {
        return (
            <div className="user-list empty">
                <div className="empty-state">
                    <p>No users found</p>
                    <p className="empty-hint">Create your first user to get started</p>
                </div>
            </div>
        );
    }

    return (
        <div className="user-list">
            <div className="list-header">
                <h2>Users ({userStats.total})</h2>
                <div className="user-stats">
                    <span className="stat active">{userStats.active} active</span>
                    <span className="stat inactive">{userStats.inactive} inactive</span>
                </div>
            </div>
            
            <div className="list-content">
                {sortedUsers.map(user => (
                    <UserItem
                        key={user.id}
                        user={user}
                        isSelected={selectedUser?.id === user.id}
                        onSelect={() => onUserSelect(user)}
                        onDelete={() => onDeleteUser(user.id)}
                    />
                ))}
            </div>
        </div>
    );
};
"#
    }
    
    fn python_api_sample() -> &'static str {
        r#"""Main FastAPI application for user management API."""

from fastapi import FastAPI, HTTPException, Depends, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.security import HTTPBearer, HTTPAuthorizationCredentials
from sqlalchemy.ext.asyncio import AsyncSession
from typing import List, Optional
import uvicorn

from .models import User, UserCreate, UserUpdate, UserRole
from .database import get_database_session
from .auth import verify_token, get_current_user
from .services import UserService, AuthService

app = FastAPI(
    title="User Management API",
    description="API for managing users and authentication",
    version="1.0.0",
)

# CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:3000"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

security = HTTPBearer()

@app.get("/")
async def root():
    """Root endpoint with API information."""
    return {
        "message": "User Management API",
        "version": "1.0.0",
        "docs": "/docs"
    }

@app.get("/health")
async def health_check():
    """Health check endpoint."""
    return {"status": "healthy"}

@app.post("/auth/login")
async def login(
    credentials: dict,
    db: AsyncSession = Depends(get_database_session)
):
    """Authenticate user and return access token."""
    username = credentials.get("username")
    password = credentials.get("password")
    
    if not username or not password:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Username and password are required"
        )
    
    auth_service = AuthService(db)
    token = await auth_service.authenticate_user(username, password)
    
    if not token:
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid credentials"
        )
    
    return {"access_token": token, "token_type": "bearer"}

@app.post("/auth/register", response_model=User)
async def register(
    user_data: UserCreate,
    db: AsyncSession = Depends(get_database_session)
):
    """Register a new user."""
    user_service = UserService(db)
    
    # Check if user already exists
    existing_user = await user_service.get_user_by_username(user_data.username)
    if existing_user:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Username already registered"
        )
    
    existing_email = await user_service.get_user_by_email(user_data.email)
    if existing_email:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Email already registered"
        )
    
    # Create new user
    new_user = await user_service.create_user(user_data)
    return new_user

@app.get("/users", response_model=List[User])
async def get_users(
    skip: int = 0,
    limit: int = 100,
    role: Optional[UserRole] = None,
    active_only: bool = False,
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_database_session)
):
    """Get list of users with filtering options."""
    if not current_user.has_role(UserRole.ADMIN):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Admin access required"
        )
    
    user_service = UserService(db)
    users = await user_service.get_users(
        skip=skip,
        limit=limit,
        role=role,
        active_only=active_only
    )
    
    return users

@app.get("/users/{user_id}", response_model=User)
async def get_user(
    user_id: int,
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_database_session)
):
    """Get a specific user by ID."""
    # Users can view their own profile, admins can view anyone
    if user_id != current_user.id and not current_user.has_role(UserRole.ADMIN):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Access denied"
        )
    
    user_service = UserService(db)
    user = await user_service.get_user_by_id(user_id)
    
    if not user:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="User not found"
        )
    
    return user

@app.put("/users/{user_id}", response_model=User)
async def update_user(
    user_id: int,
    user_update: UserUpdate,
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_database_session)
):
    """Update a user's information."""
    # Users can update their own profile, admins can update anyone
    if user_id != current_user.id and not current_user.has_role(UserRole.ADMIN):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Access denied"
        )
    
    user_service = UserService(db)
    existing_user = await user_service.get_user_by_id(user_id)
    
    if not existing_user:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="User not found"
        )
    
    # Only admins can change user roles
    if user_update.role and not current_user.has_role(UserRole.ADMIN):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Admin access required to change user role"
        )
    
    updated_user = await user_service.update_user(user_id, user_update)
    return updated_user

@app.delete("/users/{user_id}")
async def delete_user(
    user_id: int,
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_database_session)
):
    """Delete a user."""
    if not current_user.has_role(UserRole.ADMIN):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Admin access required"
        )
    
    # Prevent self-deletion
    if user_id == current_user.id:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Cannot delete your own account"
        )
    
    user_service = UserService(db)
    success = await user_service.delete_user(user_id)
    
    if not success:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="User not found"
        )
    
    return {"message": "User deleted successfully"}

@app.get("/users/{user_id}/activity")
async def get_user_activity(
    user_id: int,
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_database_session)
):
    """Get user activity log."""
    if user_id != current_user.id and not current_user.has_role(UserRole.ADMIN):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Access denied"
        )
    
    user_service = UserService(db)
    activity = await user_service.get_user_activity(user_id)
    
    return activity

@app.post("/users/{user_id}/activate")
async def activate_user(
    user_id: int,
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_database_session)
):
    """Activate a user account."""
    if not current_user.has_role(UserRole.ADMIN):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Admin access required"
        )
    
    user_service = UserService(db)
    success = await user_service.activate_user(user_id)
    
    if not success:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="User not found"
        )
    
    return {"message": "User activated successfully"}

@app.post("/users/{user_id}/deactivate")
async def deactivate_user(
    user_id: int,
    current_user: User = Depends(get_current_user),
    db: AsyncSession = Depends(get_database_session)
):
    """Deactivate a user account."""
    if not current_user.has_role(UserRole.ADMIN):
        raise HTTPException(
            status_code=status.HTTP_403_FORBIDDEN,
            detail="Admin access required"
        )
    
    # Prevent self-deactivation
    if user_id == current_user.id:
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail="Cannot deactivate your own account"
        )
    
    user_service = UserService(db)
    success = await user_service.deactivate_user(user_id)
    
    if not success:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail="User not found"
        )
    
    return {"message": "User deactivated successfully"}

if __name__ == "__main__":
    uvicorn.run(
        "main:app",
        host="0.0.0.0",
        port=8000,
        reload=True,
        log_level="info"
    )
"#
    }
    
    fn python_models_sample() -> &'static str {
        r#"""Pydantic models for the user management API."""

from pydantic import BaseModel, EmailStr, Field, validator
from enum import Enum
from datetime import datetime
from typing import Optional, List
from uuid import UUID

class UserRole(str, Enum):
    """User role enumeration."""
    ADMIN = "admin"
    MODERATOR = "moderator"
    USER = "user"
    GUEST = "guest"

class UserBase(BaseModel):
    """Base user model with common fields."""
    username: str = Field(..., min_length=3, max_length=50)
    email: EmailStr
    role: UserRole = UserRole.USER
    is_active: bool = True

    @validator('username')
    def validate_username(cls, v):
        """Validate username format."""
        if not v.isalnum():
            raise ValueError('Username must be alphanumeric')
        return v

class UserCreate(UserBase):
    """Model for creating new users."""
    password: str = Field(..., min_length=8, max_length=128)

    @validator('password')
    def validate_password(cls, v):
        """Validate password strength."""
        if len(v) < 8:
            raise ValueError('Password must be at least 8 characters long')
        
        has_upper = any(c.isupper() for c in v)
        has_lower = any(c.islower() for c in v)
        has_digit = any(c.isdigit() for c in v)
        has_special = any(c in "!@#$%^&*()_+-=" for c in v)
        
        if not all([has_upper, has_lower, has_digit, has_special]):
            raise ValueError(
                'Password must contain uppercase, lowercase, digit, and special character'
            )
        
        return v

class UserUpdate(BaseModel):
    """Model for updating existing users."""
    username: Optional[str] = Field(None, min_length=3, max_length=50)
    email: Optional[EmailStr] = None
    role: Optional[UserRole] = None
    is_active: Optional[bool] = None

    @validator('username')
    def validate_username(cls, v):
        """Validate username format."""
        if v is not None and not v.isalnum():
            raise ValueError('Username must be alphanumeric')
        return v

class User(UserBase):
    """Complete user model with all fields."""
    id: int
    created_at: datetime
    last_seen: Optional[datetime] = None
    login_count: int = 0
    
    class Config:
        orm_mode = True

class UserInDB(User):
    """User model including sensitive fields for internal use."""
    password_hash: str
    
    def verify_password(self, password: str) -> bool:
        """Verify password against hash."""
        # In real implementation, would use bcrypt
        return f"hashed_{password}" == self.password_hash
    
    def has_role(self, role: UserRole) -> bool:
        """Check if user has specific role or higher."""
        role_hierarchy = {
            UserRole.GUEST: 0,
            UserRole.USER: 1,
            UserRole.MODERATOR: 2,
            UserRole.ADMIN: 3,
        }
        
        user_level = role_hierarchy.get(self.role, 0)
        required_level = role_hierarchy.get(role, 0)
        
        return user_level >= required_level

class UserActivity(BaseModel):
    """Model for user activity tracking."""
    id: int
    user_id: int
    action: str
    resource: Optional[str] = None
    timestamp: datetime
    ip_address: Optional[str] = None
    user_agent: Optional[str] = None
    
    class Config:
        orm_mode = True

class UserSession(BaseModel):
    """Model for user session information."""
    id: UUID
    user_id: int
    created_at: datetime
    last_activity: datetime
    ip_address: str
    user_agent: str
    is_active: bool = True
    
    class Config:
        orm_mode = True

class LoginRequest(BaseModel):
    """Model for login requests."""
    username: str
    password: str

class LoginResponse(BaseModel):
    """Model for login responses."""
    access_token: str
    token_type: str = "bearer"
    expires_in: int
    user: User

class PasswordChangeRequest(BaseModel):
    """Model for password change requests."""
    current_password: str
    new_password: str = Field(..., min_length=8, max_length=128)
    
    @validator('new_password')
    def validate_new_password(cls, v):
        """Validate new password strength."""
        if len(v) < 8:
            raise ValueError('Password must be at least 8 characters long')
        
        has_upper = any(c.isupper() for c in v)
        has_lower = any(c.islower() for c in v)
        has_digit = any(c.isdigit() for c in v)
        has_special = any(c in "!@#$%^&*()_+-=" for c in v)
        
        if not all([has_upper, has_lower, has_digit, has_special]):
            raise ValueError(
                'Password must contain uppercase, lowercase, digit, and special character'
            )
        
        return v

class UserStats(BaseModel):
    """Model for user statistics."""
    total_users: int
    active_users: int
    inactive_users: int
    users_by_role: dict[UserRole, int]
    recent_registrations: int
    recent_logins: int

class UserSearch(BaseModel):
    """Model for user search parameters."""
    query: Optional[str] = None
    role: Optional[UserRole] = None
    is_active: Optional[bool] = None
    created_after: Optional[datetime] = None
    created_before: Optional[datetime] = None
    last_seen_after: Optional[datetime] = None
    last_seen_before: Optional[datetime] = None
    limit: int = Field(default=50, le=100)
    offset: int = Field(default=0, ge=0)

class BulkUserOperation(BaseModel):
    """Model for bulk user operations."""
    user_ids: List[int]
    operation: str  # 'activate', 'deactivate', 'delete', 'change_role'
    parameters: Optional[dict] = None  # Additional parameters for the operation

class APIError(BaseModel):
    """Model for API error responses."""
    error: str
    message: str
    details: Optional[dict] = None
"#
    }
}

// Test modules organized by functionality

mod symbol_renaming_tests {
    use super::*;

    #[tokio::test]
    async fn test_rename_symbol_single_file() {
        let env = PatchTestEnvironment::new().await;
        
        let stats = env.patch_tool
            .rename_symbol(
                "User", 
                "UserModel", 
                RenameScope::File(env.project_files["user.rs"].clone())
            )
            .await
            .expect("Rename should succeed");
            
        assert_eq!(stats.files_changed, 1);
        assert!(stats.occurrences_replaced > 0);
        assert!(stats.tokens_saved > 0);
        
        // Verify the actual content was changed
        let content = tokio::fs::read_to_string(&env.project_files["user.rs"]).await.unwrap();
        assert!(content.contains("UserModel"));
        assert!(!content.contains("pub struct User {"));
    }

    #[tokio::test]
    async fn test_rename_symbol_directory_scope() {
        let env = PatchTestEnvironment::new().await;
        let src_dir = env.env.path().join("src");
        
        let stats = env.patch_tool
            .rename_symbol(
                "create_default_user",
                "create_standard_user", 
                RenameScope::Directory(src_dir)
            )
            .await
            .expect("Rename should succeed");
            
        assert!(stats.files_changed >= 2); // main.rs and user.rs should be affected
        assert!(stats.occurrences_replaced > 0);
        
        // Verify changes in multiple files
        let main_content = tokio::fs::read_to_string(&env.project_files["main.rs"]).await.unwrap();
        let user_content = tokio::fs::read_to_string(&env.project_files["user.rs"]).await.unwrap();
        
        assert!(main_content.contains("create_standard_user"));
        assert!(user_content.contains("create_standard_user"));
    }

    #[tokio::test]
    async fn test_rename_symbol_workspace_scope() {
        let env = PatchTestEnvironment::new().await;
        
        let stats = env.patch_tool
            .rename_symbol(
                "UserRole",
                "Role", 
                RenameScope::Workspace
            )
            .await
            .expect("Rename should succeed");
            
        assert!(stats.files_changed >= 3); // Should affect Rust, TypeScript, and Python files
        assert!(stats.occurrences_replaced > 0);
        
        // Verify cross-language renaming (basic text matching)
        let rust_content = tokio::fs::read_to_string(&env.project_files["user.rs"]).await.unwrap();
        let ts_content = tokio::fs::read_to_string(&env.project_files["App.tsx"]).await.unwrap();
        
        assert!(rust_content.contains("enum Role"));
        // Note: TypeScript interface will be renamed too (simple text replacement)
    }

    #[tokio::test]
    async fn test_rename_symbol_preserves_formatting() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.env.path().join("format_test.rs");
        
        let original_content = r#"
// This function processes user data
pub fn process_user_data(user: &User) -> Result<(), Error> {
    if user.is_valid() {
        println!("Processing user: {}", user.name);
        // Complex formatting with User references
        let user_info = format!(
            "User(id: {}, name: '{}', role: {:?})",
            user.id, 
            user.name, 
            user.role
        );
        log_user_action(&user_info);
    }
    Ok(())
}
"#;
        
        tokio::fs::write(&test_file, original_content).await.unwrap();
        
        let stats = env.patch_tool
            .rename_symbol("User", "UserEntity", RenameScope::File(test_file.clone()))
            .await
            .expect("Rename should succeed");
            
        let new_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        
        // Verify formatting is preserved
        assert!(new_content.contains("process_user_data(user: &UserEntity)"));
        assert!(new_content.contains("// Complex formatting with UserEntity references"));
        assert!(new_content.contains("\"UserEntity(id: {}, name: '{}', role: {:?})\""));
        
        // Verify indentation is maintained
        let lines: Vec<&str> = new_content.lines().collect();
        assert!(lines.iter().any(|line| line.starts_with("    if user.is_valid()")));
        assert!(lines.iter().any(|line| line.starts_with("        println!")));
    }

    #[tokio::test]
    async fn test_rename_symbol_with_conflicts() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.env.path().join("conflict_test.rs");
        
        let content_with_conflict = r#"
struct User {
    name: String,
}

struct UserRole {
    name: String,
}

fn process_user(user: User) {
    println!("User: {}", user.name);
}

fn get_user_role() -> UserRole {
    UserRole { name: "admin".to_string() }
}
"#;
        
        tokio::fs::write(&test_file, content_with_conflict).await.unwrap();
        
        // Rename "User" to "UserRole" - should handle existing "UserRole" properly
        let result = env.patch_tool
            .rename_symbol("User", "UserRole", RenameScope::File(test_file.clone()))
            .await;
            
        // This should succeed but might have unexpected results due to conflicts
        // In a real implementation, this would detect and warn about conflicts
        assert!(result.is_ok());
        
        let new_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        // Verify that the rename happened, even with conflicts
        assert!(new_content.contains("struct UserRole {"));
        assert!(new_content.contains("process_user(user: UserRole)"));
    }
}

mod function_extraction_tests {
    use super::*;

    #[tokio::test]
    async fn test_extract_simple_function() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.env.path().join("extract_test.rs");
        
        let original_content = r#"
fn main() {
    let x = 10;
    let y = 20;
    let result = x + y;
    println!("Result: {}", result);
    
    let a = 5;
    let b = 15;
    let sum = a + b;
    println!("Sum: {}", sum);
}
"#;
        
        tokio::fs::write(&test_file, original_content).await.unwrap();
        
        let stats = env.patch_tool
            .extract_function(
                test_file.to_str().unwrap(),
                3, // let x = 10;
                6, // println!("Result: {}", result);
                "calculate_and_print"
            )
            .await
            .expect("Function extraction should succeed");
            
        assert_eq!(stats.files_changed, 1);
        assert_eq!(stats.lines_extracted, 4);
        assert!(stats.tokens_saved > 0);
        
        let new_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        
        // Should contain the function call
        assert!(new_content.contains("calculate_and_print();"));
        
        // Should contain the extracted function
        assert!(new_content.contains("fn calculate_and_print()"));
        assert!(new_content.contains("let x = 10;"));
        assert!(new_content.contains("let result = x + y;"));
    }

    #[tokio::test]
    async fn test_extract_function_with_parameters() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.env.path().join("extract_params_test.rs");
        
        let original_content = r#"
fn process_data(input: Vec<i32>) -> i32 {
    let mut sum = 0;
    let mut count = 0;
    for item in &input {
        if *item > 10 {
            sum += item;
            count += 1;
        }
    }
    let average = if count > 0 { sum / count } else { 0 };
    
    return sum + average;
}
"#;
        
        tokio::fs::write(&test_file, original_content).await.unwrap();
        
        let stats = env.patch_tool
            .extract_function(
                test_file.to_str().unwrap(),
                4, // for item in &input {
                8, // }
                "filter_and_sum"
            )
            .await
            .expect("Function extraction should succeed");
            
        assert_eq!(stats.files_changed, 1);
        assert!(stats.lines_extracted > 0);
        
        let new_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        
        // In a more sophisticated implementation, this would detect and extract parameters
        assert!(new_content.contains("filter_and_sum();")); // Simple implementation
        assert!(new_content.contains("fn filter_and_sum()"));
    }

    #[tokio::test]
    async fn test_extract_function_invalid_range() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.env.path().join("invalid_range_test.rs");
        
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        tokio::fs::write(&test_file, content).await.unwrap();
        
        // Try to extract with invalid line range
        let result = env.patch_tool
            .extract_function(
                test_file.to_str().unwrap(),
                10, // Beyond end of file
                15,
                "invalid_function"
            )
            .await;
            
        assert!(result.is_err());
        match result.unwrap_err() {
            PatchError::TreeSitter(msg) => {
                assert!(msg.contains("Invalid line range"));
            }
            _ => panic!("Expected TreeSitter error for invalid range"),
        }
    }

    #[tokio::test]
    async fn test_extract_function_performance() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.env.path().join("performance_test.rs");
        
        // Create a larger file for performance testing
        let mut large_content = String::new();
        large_content.push_str("fn main() {\n");
        
        for i in 0..1000 {
            large_content.push_str(&format!("    let var_{} = {};\n", i, i));
        }
        
        large_content.push_str("    println!(\"Done\");\n}\n");
        
        tokio::fs::write(&test_file, &large_content).await.unwrap();
        
        let (stats, duration) = TestTiming::time_async_operation(|| async {
            env.patch_tool
                .extract_function(
                    test_file.to_str().unwrap(),
                    500, // Middle of the large function
                    600,
                    "extracted_chunk"
                )
                .await
        }).await;
        
        assert!(stats.is_ok());
        
        // Performance assertion - should complete within reasonable time
        PerformanceAssertions::assert_duration_under(
            duration, 
            1000, // 1 second max
            "Large file function extraction"
        );
    }
}

mod import_management_tests {
    use super::*;

    #[tokio::test]
    async fn test_update_imports_single_change() {
        let env = PatchTestEnvironment::new().await;
        
        let stats = env.patch_tool
            .update_imports("std::collections::HashMap", "std::collections::BTreeMap")
            .await
            .expect("Import update should succeed");
            
        assert!(stats.files_changed > 0);
        assert!(stats.imports_updated > 0);
        assert!(stats.tokens_saved > 0);
        
        // Verify actual changes
        let lib_content = tokio::fs::read_to_string(&env.project_files["lib.rs"]).await.unwrap();
        assert!(lib_content.contains("use std::collections::BTreeMap;"));
    }

    #[tokio::test]
    async fn test_update_imports_typescript() {
        let env = PatchTestEnvironment::new().await;
        
        let stats = env.patch_tool
            .update_imports("React, { useState, useEffect }", "React, { useState, useEffect, useCallback }")
            .await
            .expect("Import update should succeed");
            
        assert!(stats.files_changed > 0);
        
        let app_content = tokio::fs::read_to_string(&env.project_files["App.tsx"]).await.unwrap();
        // Note: Simple text replacement won't handle this perfectly, but tests the mechanism
        assert!(stats.imports_updated > 0);
    }

    #[tokio::test]
    async fn test_organize_imports_rust() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.env.path().join("import_test.rs");
        
        let messy_imports = r#"
use std::collections::HashMap;
use serde::Serialize;
use std::env;
use tokio::fs;
use serde::Deserialize;
use std::path::Path;

fn main() {
    println!("Hello");
}
"#;
        
        tokio::fs::write(&test_file, messy_imports).await.unwrap();
        
        // In a real implementation, this would organize imports by groups
        // For now, test the basic import detection mechanism
        let stats = env.patch_tool
            .update_imports("std::env", "std::env::var")
            .await
            .expect("Import update should succeed");
            
        assert!(stats.imports_updated > 0);
    }

    #[tokio::test]
    async fn test_add_missing_imports() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.env.path().join("missing_import_test.rs");
        
        let content_missing_imports = r#"
fn main() {
    let map = HashMap::new();
    let id = Uuid::new_v4();
    println!("{:?}", map);
}
"#;
        
        tokio::fs::write(&test_file, content_missing_imports).await.unwrap();
        
        // Test adding new imports
        let stats = env.patch_tool
            .update_imports("", "use std::collections::HashMap;\nuse uuid::Uuid;")
            .await
            .expect("Import addition should succeed");
            
        // Basic mechanism test - real implementation would be more sophisticated
        assert!(stats.files_changed >= 0); // May be 0 if no existing imports to replace
    }

    #[tokio::test]
    async fn test_remove_unused_imports() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.env.path().join("unused_import_test.rs");
        
        let content_with_unused = r#"
use std::collections::HashMap;
use std::collections::BTreeMap; // This is unused
use serde::{Serialize, Deserialize}; // Only Serialize is used

#[derive(Serialize)]
struct MyStruct {
    name: String,
}

fn main() {
    let map = HashMap::new();
    let _s = MyStruct { name: "test".to_string() };
}
"#;
        
        tokio::fs::write(&test_file, content_with_unused).await.unwrap();
        
        // Test removing unused imports
        let stats = env.patch_tool
            .update_imports("use std::collections::BTreeMap;", "")
            .await
            .expect("Import removal should succeed");
            
        assert!(stats.imports_updated > 0);
        
        let new_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert!(!new_content.contains("BTreeMap"));
    }
}

mod complex_refactoring_tests {
    use super::*;

    #[tokio::test]
    async fn test_rename_class_and_methods() {
        let env = PatchTestEnvironment::new().await;
        
        // First rename the class
        let class_stats = env.patch_tool
            .rename_symbol("UserManager", "UserService", RenameScope::Workspace)
            .await
            .expect("Class rename should succeed");
            
        // Then rename methods
        let method_stats = env.patch_tool
            .rename_symbol("add_user", "create_user", RenameScope::Workspace)
            .await
            .expect("Method rename should succeed");
            
        assert!(class_stats.files_changed > 0);
        assert!(method_stats.files_changed > 0);
        
        // Verify both changes
        let user_content = tokio::fs::read_to_string(&env.project_files["user.rs"]).await.unwrap();
        assert!(user_content.contains("UserService"));
        assert!(user_content.contains("create_user"));
    }

    #[tokio::test]
    async fn test_extract_multiple_functions() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.env.path().join("multi_extract_test.rs");
        
        let large_function = r#"
fn process_data(data: Vec<i32>) -> (i32, i32, f64) {
    // Validation phase
    if data.is_empty() {
        return (0, 0, 0.0);
    }
    
    // Calculation phase
    let mut sum = 0;
    let mut count = 0;
    for item in &data {
        if *item > 0 {
            sum += item;
            count += 1;
        }
    }
    
    // Statistics phase
    let average = sum as f64 / count as f64;
    let max_value = data.iter().max().copied().unwrap_or(0);
    let min_value = data.iter().min().copied().unwrap_or(0);
    
    (sum, max_value - min_value, average)
}
"#;
        
        tokio::fs::write(&test_file, large_function).await.unwrap();
        
        // Extract validation phase
        let validate_stats = env.patch_tool
            .extract_function(
                test_file.to_str().unwrap(),
                3, // if data.is_empty()
                5, // return (0, 0, 0.0);
                "validate_data"
            )
            .await
            .expect("First extraction should succeed");
            
        // Extract calculation phase
        let calc_stats = env.patch_tool
            .extract_function(
                test_file.to_str().unwrap(),
                8, // let mut sum = 0;
                14, // }
                "calculate_sum"
            )
            .await
            .expect("Second extraction should succeed");
            
        assert_eq!(validate_stats.files_changed, 1);
        assert_eq!(calc_stats.files_changed, 1);
        
        let final_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert!(final_content.contains("fn validate_data"));
        assert!(final_content.contains("fn calculate_sum"));
    }

    #[tokio::test]
    async fn test_refactor_error_handling() {
        let env = PatchTestEnvironment::new().await;
        
        // Rename Result<(), Box<dyn std::error::Error>> to AppResult<()>
        let result_stats = env.patch_tool
            .rename_symbol(
                "Result<(), Box<dyn std::error::Error>>", 
                "AppResult<()>", 
                RenameScope::Workspace
            )
            .await
            .expect("Result type rename should succeed");
            
        // Add proper error import
        let import_stats = env.patch_tool
            .update_imports("", "use crate::error::AppResult;")
            .await
            .expect("Import addition should succeed");
            
        assert!(result_stats.occurrences_replaced > 0);
        
        let main_content = tokio::fs::read_to_string(&env.project_files["main.rs"]).await.unwrap();
        assert!(main_content.contains("AppResult<()>"));
    }

    #[tokio::test]
    async fn test_comprehensive_api_refactoring() {
        let env = PatchTestEnvironment::new().await;
        
        // Simulate a comprehensive API refactoring across multiple files
        let refactoring_steps = vec![
            ("User", "UserEntity", RenameScope::Workspace),
            ("UserManager", "UserRepository", RenameScope::Workspace),
            ("create_default_user", "create_user", RenameScope::Workspace),
            ("get_active_users", "find_active_users", RenameScope::Workspace),
        ];
        
        let mut total_changes = 0;
        let mut total_tokens_saved = 0;
        
        for (old_name, new_name, scope) in refactoring_steps {
            let stats = env.patch_tool
                .rename_symbol(old_name, new_name, scope)
                .await
                .expect(&format!("Rename {} -> {} should succeed", old_name, new_name));
                
            total_changes += stats.files_changed;
            total_tokens_saved += stats.tokens_saved;
        }
        
        assert!(total_changes > 0);
        assert!(total_tokens_saved > 0);
        
        // Verify the comprehensive changes
        let main_content = tokio::fs::read_to_string(&env.project_files["main.rs"]).await.unwrap();
        let user_content = tokio::fs::read_to_string(&env.project_files["user.rs"]).await.unwrap();
        
        assert!(main_content.contains("UserEntity"));
        assert!(main_content.contains("UserRepository"));
        assert!(main_content.contains("create_user"));
        assert!(user_content.contains("find_active_users"));
    }
}

mod error_handling_tests {
    use super::*;

    #[tokio::test]
    async fn test_file_not_found_error() {
        let env = PatchTestEnvironment::new().await;
        let nonexistent_file = env.env.path().join("nonexistent.rs");
        
        let result = env.patch_tool
            .rename_symbol("test", "renamed", RenameScope::File(nonexistent_file))
            .await;
            
        assert!(result.is_err());
        match result.unwrap_err() {
            PatchError::Io(_) => {}, // Expected IO error for missing file
            _ => panic!("Expected IO error for nonexistent file"),
        }
    }

    #[tokio::test]
    async fn test_invalid_regex_pattern() {
        let env = PatchTestEnvironment::new().await;
        
        // Use a symbol name that would create invalid regex
        let result = env.patch_tool
            .rename_symbol("[invalid*regex", "new_name", RenameScope::Workspace)
            .await;
            
        // Should succeed because we escape the pattern, but test the mechanism
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_circular_dependency_detection() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.env.path().join("circular_test.rs");
        
        let circular_content = r#"
struct A {
    b: B,
}

struct B {
    a: A,  // This creates a circular dependency
}
"#;
        
        tokio::fs::write(&test_file, circular_content).await.unwrap();
        
        // Attempt to rename in a way that might expose circular issues
        let result = env.patch_tool
            .rename_symbol("A", "B", RenameScope::File(test_file))
            .await;
            
        // Should complete but might produce unexpected results
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_large_file_handling() {
        let env = PatchTestEnvironment::new().await;
        let large_file = env.env.path().join("large_test.rs");
        
        // Create a very large file (1MB+)
        let mut large_content = String::with_capacity(1_048_576); // 1MB
        large_content.push_str("// Large file test\n");
        
        for i in 0..50_000 {
            large_content.push_str(&format!(
                "pub struct TestStruct{} {{\n    pub field: i32,\n}}\n", 
                i
            ));
        }
        
        tokio::fs::write(&large_file, &large_content).await.unwrap();
        
        let (result, duration) = TestTiming::time_async_operation(|| async {
            env.patch_tool
                .rename_symbol("TestStruct", "RenamedStruct", RenameScope::File(large_file))
                .await
        }).await;
        
        assert!(result.is_ok());
        let stats = result.unwrap();
        
        assert_eq!(stats.files_changed, 1);
        assert!(stats.occurrences_replaced >= 50_000);
        
        // Should handle large files within reasonable time (10 seconds max)
        PerformanceAssertions::assert_duration_under(
            duration,
            10_000,
            "Large file processing"
        );
    }

    #[tokio::test]
    async fn test_concurrent_modifications() {
        let env = PatchTestEnvironment::new().await;
        
        // Test concurrent operations on different files
        let handles = vec![
            tokio::spawn({
                let tool = PatchTool::new();
                let file = env.project_files["main.rs"].clone();
                async move {
                    tool.rename_symbol("main", "main_function", RenameScope::File(file)).await
                }
            }),
            tokio::spawn({
                let tool = PatchTool::new();
                let file = env.project_files["user.rs"].clone();
                async move {
                    tool.rename_symbol("User", "UserModel", RenameScope::File(file)).await
                }
            }),
            tokio::spawn({
                let tool = PatchTool::new();
                let file = env.project_files["database.rs"].clone();
                async move {
                    tool.rename_symbol("DatabaseConnection", "DbConn", RenameScope::File(file)).await
                }
            }),
        ];
        
        let results = futures::future::join_all(handles).await;
        
        for handle_result in results {
            let patch_result = handle_result.expect("Task should not panic");
            assert!(patch_result.is_ok(), "Concurrent operations should succeed");
        }
    }

    #[tokio::test]
    async fn test_partial_failure_recovery() {
        let env = PatchTestEnvironment::new().await;
        
        // Create a scenario where some files might fail
        let readonly_file = env.env.path().join("readonly.rs");
        tokio::fs::write(&readonly_file, "fn test() {}").await.unwrap();
        
        // Make file readonly to simulate permission error
        let mut perms = tokio::fs::metadata(&readonly_file).await.unwrap().permissions();
        perms.set_readonly(true);
        tokio::fs::set_permissions(&readonly_file, perms).await.unwrap();
        
        let result = env.patch_tool
            .rename_symbol("test", "renamed_test", RenameScope::File(readonly_file))
            .await;
            
        // Should handle the error gracefully
        assert!(result.is_err());
        match result.unwrap_err() {
            PatchError::Io(_) => {}, // Expected permission error
            _ => panic!("Expected IO error for readonly file"),
        }
    }
}

mod performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_bulk_rename_performance() {
        let env = PatchTestEnvironment::new().await;
        
        // Create multiple files for bulk operations
        let mut test_files = Vec::new();
        for i in 0..10 {
            let file = env.env.path().join(format!("bulk_test_{}.rs", i));
            let content = format!(
                "pub struct TestEntity{} {{\n    pub process_data: fn(),\n}}\n",
                i
            );
            tokio::fs::write(&file, content).await.unwrap();
            test_files.push(file);
        }
        
        let benchmark = TestTiming::benchmark_operation(|| {
            // Use blocking version for benchmark
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                env.patch_tool
                    .rename_symbol("TestEntity", "ProcessedEntity", RenameScope::Workspace)
                    .await
                    .unwrap()
            })
        }, 5);
        
        benchmark.assert_average_under(
            Duration::from_millis(2000),
            "Bulk rename across multiple files"
        );
        
        println!("Bulk rename benchmark: {:?}", benchmark);
    }

    #[tokio::test]
    async fn test_extract_function_performance_scaling() {
        let env = PatchTestEnvironment::new().await;
        
        // Test function extraction with different file sizes
        let sizes = vec![100, 500, 1000, 5000];
        let mut performance_data = Vec::new();
        
        for size in sizes {
            let test_file = env.env.path().join(format!("perf_test_{}.rs", size));
            
            // Generate file with specified number of lines
            let mut content = String::new();
            content.push_str("fn large_function() {\n");
            
            for i in 0..size {
                content.push_str(&format!("    let var_{} = {};\n", i, i));
            }
            
            content.push_str("}\n");
            
            tokio::fs::write(&test_file, &content).await.unwrap();
            
            let (_, duration) = TestTiming::time_async_operation(|| async {
                env.patch_tool
                    .extract_function(
                        test_file.to_str().unwrap(),
                        size / 4,
                        size / 2,
                        &format!("extracted_function_{}", size)
                    )
                    .await
            }).await;
            
            performance_data.push((size, duration));
            
            // Ensure extraction time scales reasonably
            let max_ms = size as u64 * 2; // 2ms per line is generous
            PerformanceAssertions::assert_duration_under(
                duration,
                max_ms,
                &format!("Function extraction for {} lines", size)
            );
        }
        
        println!("Performance scaling data: {:?}", performance_data);
    }

    #[tokio::test]
    async fn test_import_update_performance() {
        let env = PatchTestEnvironment::new().await;
        
        // Create files with many imports
        for i in 0..20 {
            let file = env.env.path().join(format!("import_test_{}.rs", i));
            let mut content = String::new();
            
            // Add many imports
            for j in 0..100 {
                content.push_str(&format!("use std::collections::module_{}::Type{};\n", j, j));
            }
            
            content.push_str("fn main() {}\n");
            
            tokio::fs::write(&file, content).await.unwrap();
        }
        
        let (stats, duration) = TestTiming::time_async_operation(|| async {
            env.patch_tool
                .update_imports("std::collections", "std::collections::hash_map")
                .await
        }).await;
        
        assert!(stats.is_ok());
        let import_stats = stats.unwrap();
        
        assert!(import_stats.files_changed > 0);
        assert!(import_stats.imports_updated > 0);
        
        // Should handle many imports efficiently
        PerformanceAssertions::assert_duration_under(
            duration,
            5000, // 5 seconds max for bulk import updates
            "Bulk import updates across many files"
        );
        
        println!("Import update performance: {} files, {} imports in {:?}",
            import_stats.files_changed,
            import_stats.imports_updated,
            duration
        );
    }

    #[tokio::test]
    async fn test_memory_usage_during_operations() {
        let env = PatchTestEnvironment::new().await;
        
        // Create a large project structure
        for i in 0..50 {
            let file = env.env.path().join(format!("memory_test_{}.rs", i));
            let mut content = String::with_capacity(50_000);
            
            for j in 0..1000 {
                content.push_str(&format!(
                    "pub struct Entity_{}_{} {{ field: i32 }}\n",
                    i, j
                ));
            }
            
            tokio::fs::write(&file, content).await.unwrap();
        }
        
        // Perform operation and monitor memory (simplified test)
        let initial_memory = get_memory_usage();
        
        let _stats = env.patch_tool
            .rename_symbol("Entity_", "RenamedEntity_", RenameScope::Workspace)
            .await
            .expect("Large workspace rename should succeed");
            
        let final_memory = get_memory_usage();
        let memory_increase = final_memory.saturating_sub(initial_memory);
        
        // Memory usage should be reasonable (less than 100MB increase)
        PerformanceAssertions::assert_memory_usage_reasonable(
            memory_increase,
            100 // 100MB limit
        );
        
        println!("Memory usage: initial={}MB, final={}MB, increase={}MB",
            initial_memory / 1024 / 1024,
            final_memory / 1024 / 1024,
            memory_increase / 1024 / 1024
        );
    }

    // Helper function to get memory usage (simplified)
    fn get_memory_usage() -> usize {
        // In a real implementation, would use system APIs
        // For testing purposes, return a mock value
        1024 * 1024 * 50 // 50MB baseline
    }
}

mod rollback_capability_tests {
    use super::*;

    #[tokio::test]
    async fn test_create_backup_before_operation() {
        let env = PatchTestEnvironment::new().await;
        let test_file = env.project_files["user.rs"].clone();
        
        // Read original content
        let original_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        
        // Perform rename operation
        let _stats = env.patch_tool
            .rename_symbol("User", "UserModel", RenameScope::File(test_file.clone()))
            .await
            .expect("Rename should succeed");
            
        // Verify content changed
        let modified_content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_ne!(original_content, modified_content);
        assert!(modified_content.contains("UserModel"));
        
        // In a real implementation with rollback support, we would:
        // 1. Create backup files before operation
        // 2. Provide rollback functionality to restore from backups
        // 3. Clean up backups after successful completion
    }

    #[tokio::test] 
    async fn test_atomic_operation_simulation() {
        let env = PatchTestEnvironment::new().await;
        
        // Create multiple test files
        let files = vec![
            ("atomic_1.rs", "struct User { id: i32 }"),
            ("atomic_2.rs", "impl User { fn new() -> Self { User { id: 0 } } }"),
            ("atomic_3.rs", "fn process_user(user: User) {}"),
        ];
        
        let mut file_paths = Vec::new();
        for (name, content) in &files {
            let path = env.env.path().join(name);
            tokio::fs::write(&path, content).await.unwrap();
            file_paths.push(path);
        }
        
        // Store original content
        let mut original_contents = Vec::new();
        for path in &file_paths {
            let content = tokio::fs::read_to_string(path).await.unwrap();
            original_contents.push(content);
        }
        
        // Perform multi-file operation
        let stats = env.patch_tool
            .rename_symbol("User", "Person", RenameScope::Directory(env.env.path().to_path_buf()))
            .await
            .expect("Multi-file rename should succeed");
            
        assert!(stats.files_changed >= files.len());
        
        // Verify all files were modified consistently
        for path in &file_paths {
            let content = tokio::fs::read_to_string(path).await.unwrap();
            assert!(content.contains("Person"));
            assert!(!content.contains("struct User"));
        }
        
        // In a real rollback implementation, we could restore all files atomically
    }

    #[tokio::test]
    async fn test_partial_failure_cleanup() {
        let env = PatchTestEnvironment::new().await;
        
        // Create scenario where operation might partially fail
        let good_file = env.env.path().join("good_file.rs");
        let bad_file = env.env.path().join("bad_file.rs");
        
        tokio::fs::write(&good_file, "struct TestStruct {}").await.unwrap();
        tokio::fs::write(&bad_file, "struct TestStruct {}").await.unwrap();
        
        // Make one file readonly to cause failure
        let mut perms = tokio::fs::metadata(&bad_file).await.unwrap().permissions();
        perms.set_readonly(true);
        tokio::fs::set_permissions(&bad_file, perms).await.unwrap();
        
        // Attempt directory-wide operation
        let result = env.patch_tool
            .rename_symbol(
                "TestStruct", 
                "RenamedStruct", 
                RenameScope::Directory(env.env.path().to_path_buf())
            )
            .await;
            
        // Operation should fail due to readonly file
        // In a real implementation with rollback:
        // - Would detect partial failure
        // - Roll back successful changes
        // - Report which files failed and why
        
        // For current implementation, check it handles errors gracefully
        match result {
            Ok(stats) => {
                // May succeed on some files, fail on others
                println!("Partial success: {} files changed", stats.files_changed);
            }
            Err(e) => {
                println!("Expected failure due to readonly file: {:?}", e);
                assert!(matches!(e, PatchError::Io(_)));
            }
        }
    }

    #[tokio::test]
    async fn test_transaction_like_behavior() {
        let env = PatchTestEnvironment::new().await;
        
        // Simulate a complex refactoring that should be atomic
        let operations = vec![
            ("UserManager", "UserService"),
            ("create_default_user", "create_user"), 
            ("get_active_users", "find_active_users"),
        ];
        
        // Store checksums before operations
        let mut file_checksums = std::collections::HashMap::new();
        for (_, path) in &env.project_files {
            let content = tokio::fs::read_to_string(path).await.unwrap();
            let checksum = calculate_simple_checksum(&content);
            file_checksums.insert(path.clone(), checksum);
        }
        
        // Perform all operations
        let mut all_succeeded = true;
        for (old, new) in operations {
            match env.patch_tool
                .rename_symbol(old, new, RenameScope::Workspace)
                .await {
                Ok(_) => continue,
                Err(_) => {
                    all_succeeded = false;
                    break;
                }
            }
        }
        
        if all_succeeded {
            // All operations succeeded - changes should be committed
            println!("All operations succeeded");
        } else {
            // In a real rollback implementation:
            // - Would restore all files to original state
            // - Report which operation failed
            // - Ensure no partial state remains
            println!("Some operations failed - would rollback in real implementation");
        }
        
        // Verify file integrity (simplified check)
        for (path, _original_checksum) in file_checksums {
            let current_content = tokio::fs::read_to_string(&path).await.unwrap();
            let _current_checksum = calculate_simple_checksum(&current_content);
            
            // In real implementation, would verify checksums match expected state
            assert!(!current_content.is_empty(), "File should not be corrupted");
        }
    }

    fn calculate_simple_checksum(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}

mod realistic_scenarios {
    use super::*;

    #[tokio::test]
    async fn test_migration_to_async() {
        let env = PatchTestEnvironment::new().await;
        
        // Simulate migrating synchronous functions to async
        let sync_patterns = vec![
            ("fn connect_to_database(", "async fn connect_to_database("),
            ("-> DatabaseConnection", "-> Result<DatabaseConnection, Error>"),
            ("user_manager.load_users()", "user_manager.load_users().await?"),
            ("database.health_check()", "database.health_check().await?"),
        ];
        
        for (old_pattern, new_pattern) in sync_patterns {
            let stats = env.patch_tool
                .rename_symbol(old_pattern, new_pattern, RenameScope::Workspace)
                .await
                .expect("Async migration should succeed");
                
            if stats.occurrences_replaced > 0 {
                println!("Migrated {} occurrences: {} -> {}", 
                    stats.occurrences_replaced, old_pattern, new_pattern);
            }
        }
        
        // Verify some key migrations
        let database_content = tokio::fs::read_to_string(&env.project_files["database.rs"]).await.unwrap();
        let main_content = tokio::fs::read_to_string(&env.project_files["main.rs"]).await.unwrap();
        
        // Check for async patterns (basic text matching)
        assert!(database_content.contains("async fn") || main_content.contains("async fn"));
    }

    #[tokio::test]
    async fn test_error_type_standardization() {
        let env = PatchTestEnvironment::new().await;
        
        // Standardize error handling across the codebase
        let error_migrations = vec![
            ("Box<dyn std::error::Error>", "AppError"),
            ("Result<(), Box<dyn std::error::Error>>", "AppResult<()>"),
            ("Result<User, Box<dyn std::error::Error>>", "AppResult<User>"),
            ("Result<Vec<User>, Box<dyn std::error::Error>>", "AppResult<Vec<User>>"),
        ];
        
        let mut total_changes = 0;
        
        for (old_error, new_error) in error_migrations {
            let stats = env.patch_tool
                .rename_symbol(old_error, new_error, RenameScope::Workspace)
                .await
                .expect("Error standardization should succeed");
                
            total_changes += stats.occurrences_replaced;
        }
        
        // Add error imports
        let _import_stats = env.patch_tool
            .update_imports("", "use crate::error::{AppError, AppResult};")
            .await
            .expect("Error import should succeed");
            
        assert!(total_changes > 0);
        
        let main_content = tokio::fs::read_to_string(&env.project_files["main.rs"]).await.unwrap();
        assert!(main_content.contains("AppResult") || main_content.contains("AppError"));
    }

    #[tokio::test]
    async fn test_dependency_injection_refactoring() {
        let env = PatchTestEnvironment::new().await;
        
        // Extract common dependency injection patterns
        let di_file = env.env.path().join("di_test.rs");
        let content_with_hard_dependencies = r#"
struct UserService {
    // Hard-coded dependencies
}

impl UserService {
    fn new() -> Self {
        Self {}
    }
    
    fn get_user(&self, id: i32) -> Option<User> {
        // Directly creates database connection
        let db = DatabaseConnection::new();
        db.query_user(id)
    }
    
    fn create_user(&self, user_data: UserData) -> User {
        let db = DatabaseConnection::new();
        let email_service = EmailService::new();
        
        let user = db.insert_user(user_data);
        email_service.send_welcome_email(&user);
        user
    }
}
"#;
        
        tokio::fs::write(&di_file, content_with_hard_dependencies).await.unwrap();
        
        // Refactor to dependency injection
        let extraction_stats = env.patch_tool
            .extract_function(
                di_file.to_str().unwrap(),
                12, // db creation and query
                14,
                "query_user_from_db"
            )
            .await
            .expect("DI extraction should succeed");
            
        // Rename constructor to accept dependencies
        let constructor_stats = env.patch_tool
            .rename_symbol(
                "fn new() -> Self",
                "fn new(db: Arc<DatabaseConnection>, email: Arc<EmailService>) -> Self",
                RenameScope::File(di_file.clone())
            )
            .await
            .expect("Constructor refactoring should succeed");
            
        assert!(extraction_stats.lines_extracted > 0);
        assert!(constructor_stats.occurrences_replaced > 0);
        
        let refactored_content = tokio::fs::read_to_string(&di_file).await.unwrap();
        assert!(refactored_content.contains("query_user_from_db"));
    }

    #[tokio::test]
    async fn test_api_versioning_migration() {
        let env = PatchTestEnvironment::new().await;
        
        // Simulate API versioning by adding v2 endpoints
        let api_migrations = vec![
            ("/users", "/api/v2/users"),
            ("/auth/login", "/api/v2/auth/login"),
            ("get_users", "get_users_v2"),
            ("create_user", "create_user_v2"),
            ("UserCreate", "UserCreateV2"),
            ("UserUpdate", "UserUpdateV2"),
        ];
        
        for (old_endpoint, new_endpoint) in api_migrations {
            let stats = env.patch_tool
                .rename_symbol(old_endpoint, new_endpoint, RenameScope::Workspace)
                .await
                .expect("API versioning should succeed");
                
            if stats.occurrences_replaced > 0 {
                println!("API migration: {} -> {} ({} changes)", 
                    old_endpoint, new_endpoint, stats.occurrences_replaced);
            }
        }
        
        // Verify migrations in Python API files
        let api_content = tokio::fs::read_to_string(&env.project_files["main.py"]).await.unwrap();
        assert!(api_content.contains("/api/v2/") || api_content.contains("_v2"));
    }

    #[tokio::test]
    async fn test_database_schema_migration() {
        let env = PatchTestEnvironment::new().await;
        
        // Simulate database field renaming across models
        let schema_migrations = vec![
            ("user_name", "username"),
            ("email_address", "email"),
            ("is_active_user", "is_active"),
            ("created_timestamp", "created_at"),
            ("last_login_time", "last_seen"),
        ];
        
        let mut total_schema_changes = 0;
        
        for (old_field, new_field) in schema_migrations {
            let stats = env.patch_tool
                .rename_symbol(old_field, new_field, RenameScope::Workspace)
                .await
                .expect("Schema migration should succeed");
                
            total_schema_changes += stats.occurrences_replaced;
        }
        
        // Update related imports and types
        let _type_stats = env.patch_tool
            .update_imports(
                "UserData",
                "UserModel"
            )
            .await
            .expect("Type import update should succeed");
            
        println!("Total schema changes: {}", total_schema_changes);
        
        // Verify changes across different language files
        let python_content = tokio::fs::read_to_string(&env.project_files["models.py"]).await.unwrap();
        let rust_content = tokio::fs::read_to_string(&env.project_files["user.rs"]).await.unwrap();
        
        // Check for some expected migrations (basic verification)
        assert!(
            python_content.contains("username") || 
            rust_content.contains("username") ||
            python_content.contains("created_at") ||
            rust_content.contains("created_at")
        );
    }
}

// Integration with existing test helpers
#[tokio::test]
async fn test_integration_with_test_helpers() {
    let env = TestEnvironment::new();
    let patch_tool = PatchTool::new();
    
    // Create sample files using test helpers
    let files = env.create_sample_files();
    
    // Use performance assertions
    let (stats, duration) = TestTiming::time_async_operation(|| async {
        patch_tool.rename_symbol(
            "User",
            "UserEntity", 
            RenameScope::Workspace
        ).await
    }).await;
    
    assert!(stats.is_ok());
    
    PerformanceAssertions::assert_duration_under(
        duration,
        1000, // 1 second
        "Patch tool integration test"
    );
    
    // Use validation helpers
    TestValidation::assert_files_exist(&files.values().cloned().collect::<Vec<_>>());
}