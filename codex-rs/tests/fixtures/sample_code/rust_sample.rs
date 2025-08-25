// Sample Rust code for testing AST parsing and compression

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub email: String,
    pub age: Option<u8>,
    pub roles: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    Admin,
    Moderator,
    User,
    Guest,
}

impl User {
    pub fn new(id: u64, name: String, email: String) -> Self {
        Self {
            id,
            name,
            email,
            age: None,
            roles: vec![Role::User],
        }
    }
    
    pub fn with_age(mut self, age: u8) -> Self {
        self.age = Some(age);
        self
    }
    
    pub fn add_role(&mut self, role: Role) {
        if !self.roles.contains(&role) {
            self.roles.push(role);
        }
    }
    
    pub fn has_role(&self, role: &Role) -> bool {
        self.roles.contains(role)
    }
    
    pub fn is_admin(&self) -> bool {
        self.has_role(&Role::Admin)
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name, self.email)
    }
}

pub struct UserRegistry {
    users: HashMap<u64, User>,
    next_id: u64,
}

impl UserRegistry {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            next_id: 1,
        }
    }
    
    pub fn register_user(&mut self, name: String, email: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        let user = User::new(id, name, email);
        self.users.insert(id, user);
        
        id
    }
    
    pub fn get_user(&self, id: u64) -> Option<&User> {
        self.users.get(&id)
    }
    
    pub fn get_user_mut(&mut self, id: u64) -> Option<&mut User> {
        self.users.get_mut(&id)
    }
    
    pub fn remove_user(&mut self, id: u64) -> Option<User> {
        self.users.remove(&id)
    }
    
    pub fn list_users(&self) -> Vec<&User> {
        self.users.values().collect()
    }
    
    pub fn find_by_email(&self, email: &str) -> Option<&User> {
        self.users.values().find(|user| user.email == email)
    }
    
    pub fn count(&self) -> usize {
        self.users.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_user_creation() {
        let user = User::new(1, "Alice".to_string(), "alice@example.com".to_string());
        
        assert_eq!(user.id, 1);
        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.age, None);
        assert_eq!(user.roles, vec![Role::User]);
    }
    
    #[test]
    fn test_user_with_age() {
        let user = User::new(1, "Bob".to_string(), "bob@example.com".to_string())
            .with_age(25);
        
        assert_eq!(user.age, Some(25));
    }
    
    #[test]
    fn test_role_management() {
        let mut user = User::new(1, "Carol".to_string(), "carol@example.com".to_string());
        
        assert!(!user.is_admin());
        
        user.add_role(Role::Admin);
        assert!(user.is_admin());
        assert!(user.has_role(&Role::Admin));
        
        // Adding the same role should not duplicate
        user.add_role(Role::Admin);
        assert_eq!(user.roles.len(), 2); // User + Admin
    }
    
    #[test]
    fn test_user_registry() {
        let mut registry = UserRegistry::new();
        
        let id1 = registry.register_user("Alice".to_string(), "alice@example.com".to_string());
        let id2 = registry.register_user("Bob".to_string(), "bob@example.com".to_string());
        
        assert_eq!(registry.count(), 2);
        assert_ne!(id1, id2);
        
        let alice = registry.get_user(id1).unwrap();
        assert_eq!(alice.name, "Alice");
        
        let found = registry.find_by_email("bob@example.com").unwrap();
        assert_eq!(found.name, "Bob");
        
        registry.remove_user(id1);
        assert_eq!(registry.count(), 1);
        assert!(registry.get_user(id1).is_none());
    }
}