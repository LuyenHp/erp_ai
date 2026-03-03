//! # Auth Handlers
//!
//! API endpoints: register, login, get current user.

use axum::{extract::State, Json};
use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use crate::core::AppState;
use uuid::Uuid;

use crate::core::errors::AppError;
use crate::core::iam::middleware::AuthContext;

use super::models::*;
use super::password;

/// JWT secret key
fn get_jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".into())
}

/// JWT token expiration (24 hours)
const JWT_EXPIRATION_HOURS: i64 = 24;

/// Tạo JWT token từ user info.
fn create_token(user: &User) -> Result<String, AppError> {
    let now = Utc::now();
    let exp = now + chrono::Duration::hours(JWT_EXPIRATION_HOURS);

    let claims = Claims {
        sub: user.id,
        tenant_id: user.tenant_id,
        email: user.email.clone(),
        iat: now.timestamp() as usize,
        exp: exp.timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_jwt_secret().as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Token creation failed: {e}")))
}

// =============================================================================
// POST /auth/register
// =============================================================================

/// Đăng ký user mới + tạo tenant mới.
///
/// Flow:
/// 1. Validate input
/// 2. Hash password bằng argon2
/// 3. Tạo tenant mới
/// 4. Tạo user thuộc tenant đó
/// 5. Trả JWT token
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let pool = &state.pool;
    // Validate
    if req.email.is_empty() || req.password.is_empty() || req.full_name.is_empty() {
        return Err(AppError::BadRequest("All fields are required".into()));
    }
    if req.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".into(),
        ));
    }

    // Hash password
    let password_hash = password::hash_password(&req.password)?;

    // Tạo tenant code từ tên (lowercase, replace spaces with hyphens)
    let tenant_code = req
        .tenant_name
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();

    // Transaction: tạo tenant + user cùng lúc
    let mut tx = pool.begin().await?;

    // Tạo tenant
    let tenant_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO tenants (name, code)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(&req.tenant_name)
    .bind(&tenant_code)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.code().as_deref() == Some("23505") => {
            AppError::Conflict(format!("Tenant '{}' already exists", req.tenant_name))
        }
        _ => AppError::from(e),
    })?;

    // Tạo user (is_superadmin = true cho user đầu tiên của tenant)
    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (tenant_id, email, password_hash, full_name, is_superadmin)
        VALUES ($1, $2, $3, $4, TRUE)
        RETURNING *
        "#,
    )
    .bind(tenant_id)
    .bind(&req.email)
    .bind(&password_hash)
    .bind(&req.full_name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.code().as_deref() == Some("23505") => {
            AppError::Conflict(format!("Email '{}' is already registered", req.email))
        }
        _ => AppError::from(e),
    })?;

    tx.commit().await?;

    tracing::info!(
        user_id = %user.id,
        tenant_id = %tenant_id,
        "New user registered"
    );

    // Tạo JWT token
    let token = create_token(&user)?;

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

// =============================================================================
// POST /auth/login
// =============================================================================

/// Đăng nhập – verify email/password, trả JWT token.
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let pool = &state.pool;
    // Tìm user theo email (bypass RLS vì chưa có tenant context)
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE email = $1 AND is_active = TRUE",
    )
    .bind(&req.email)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("Invalid email or password".into()))?;

    // Verify password
    let valid = password::verify_password(&req.password, &user.password_hash)?;
    if !valid {
        return Err(AppError::Unauthorized("Invalid email or password".into()));
    }

    // Tạo JWT token
    let token = create_token(&user)?;

    tracing::info!(user_id = %user.id, "User logged in");

    Ok(Json(AuthResponse {
        token,
        user: user.into(),
    }))
}

// =============================================================================
// GET /auth/me
// =============================================================================

/// Lấy thông tin user hiện tại (từ JWT token).
pub async fn me(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
) -> Result<Json<UserInfo>, AppError> {
    let pool = &state.pool;
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE id = $1 AND tenant_id = $2",
    )
    .bind(auth.user_id)
    .bind(auth.tenant_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("User not found".into()))?;

    Ok(Json(user.into()))
}
