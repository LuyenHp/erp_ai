//! # Auth Models
//!
//! Struct cho authentication: User (DB), JWT Claims, request/response types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// User – Từ database
// =============================================================================

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    #[serde(skip_serializing)] // Không bao giờ trả password hash ra API
    pub password_hash: String,
    pub full_name: String,
    pub is_active: bool,
    pub is_superadmin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// JWT Claims
// =============================================================================

/// Payload bên trong JWT token.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject = user_id
    pub sub: Uuid,
    /// Tenant ID – cô lập Multi-tenancy
    pub tenant_id: Uuid,
    pub email: String,
    /// Expiration time (UNIX timestamp)
    pub exp: usize,
    /// Issued at (UNIX timestamp)
    pub iat: usize,
}

// =============================================================================
// Request / Response Types
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
    /// Tên tenant mới (tạo tenant + user cùng lúc khi đăng ký lần đầu)
    pub tenant_name: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

/// Thông tin user trả về (không chứa password_hash)
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub full_name: String,
    pub is_superadmin: bool,
}

impl From<User> for UserInfo {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            tenant_id: u.tenant_id,
            email: u.email,
            full_name: u.full_name,
            is_superadmin: u.is_superadmin,
        }
    }
}
