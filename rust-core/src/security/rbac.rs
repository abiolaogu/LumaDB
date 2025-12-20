use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    All,
    Read(String), // Collection name
    Write(String),
    Execute, // Scripting
}

#[derive(Debug, Clone)]
pub struct Role {
    pub name: String,
    pub permissions: HashSet<Permission>,
}

impl Role {
    pub fn new(name: &str) -> Self {
        Self { 
            name: name.to_string(), 
            permissions: HashSet::new() 
        }
    }
    
    pub fn with_permission(mut self, perm: Permission) -> Self {
        self.permissions.insert(perm);
        self
    }
}

pub struct AccessControl;

impl AccessControl {
    pub fn check(user: &User, role: &Role, required: Permission) -> bool {
        if role.permissions.contains(&Permission::All) { return true; }
        role.permissions.contains(&required)
    }
}
