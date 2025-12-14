
use serde::{Serialize, Deserialize};

pub type TenantId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Editor,
    Viewer,
    ServiceAccount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub tenant_id: TenantId,
    pub user_id: String,
    pub role: Role,
}

impl UserContext {
    pub fn can_write(&self) -> bool {
        matches!(self.role, Role::Admin | Role::Editor | Role::ServiceAccount)
    }

    pub fn can_read(&self) -> bool {
        true
    }
}
