//! # RequirePermission Middleware
//!
//! Axum middleware kiểm tra quyền truy cập dựa trên ngữ cảnh (context-based).
//! Thay vì chỉ kiểm tra quyền global, middleware này kiểm tra:
//! "User có quyền X tại Context Y hay không?"
//!
//! Flow:
//! 1. Lấy `user_id`, `tenant_id` từ request extensions (đã set bởi auth middleware)
//! 2. Lấy `context_type`, `context_id` từ request headers hoặc path
//! 3. Query `user_context_roles` JOIN `role_permissions` JOIN `permissions`
//! 4. Trả 403 nếu không có quyền, pass-through nếu OK

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use crate::core::AppState;
use sqlx::PgPool;
use uuid::Uuid;

use super::models::ContextType;

// =============================================================================
// Auth Context – Thông tin xác thực được inject bởi auth middleware trước đó
// =============================================================================

/// Thông tin xác thực của user, được set vào request extensions
/// bởi authentication middleware (JWT/Session) trước khi đến đây.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
}

// =============================================================================
// Permission Requirement – Cấu hình cho middleware
// =============================================================================

/// Yêu cầu về quyền mà một route cần.
/// Bao gồm permission code và context mà quyền phải có hiệu lực.
#[derive(Debug, Clone)]
pub struct PermissionRequirement {
    /// Code của permission cần kiểm tra, VD: "approve_order"
    pub permission_code: String,
    /// Loại context cần kiểm tra
    pub context_type: ContextType,
}

impl PermissionRequirement {
    pub fn new(permission_code: impl Into<String>, context_type: ContextType) -> Self {
        Self {
            permission_code: permission_code.into(),
            context_type,
        }
    }
}

// =============================================================================
// RequirePermission – Core middleware function
// =============================================================================

/// Middleware kiểm tra quyền theo ngữ cảnh.
///
/// ## Cách sử dụng với Axum Router:
/// ```rust,ignore
/// use axum::{Router, middleware};
///
/// let app = Router::new()
///     .route("/orders/:id/approve", post(approve_order))
///     .layer(middleware::from_fn_with_state(
///         app_state.clone(),
///         require_permission,
///     ));
/// ```
///
/// ## Headers cần thiết:
/// - `X-Context-Id`: UUID của context (department/project/branch)
///
/// ## Extensions cần thiết (set bởi auth middleware trước):
/// - `AuthContext`: chứa user_id và tenant_id
pub async fn require_permission(
    State(state): State<AppState>,
    requirement: State<PermissionRequirement>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let pool = &state.pool;
    // -------------------------------------------------------------------------
    // 1. Lấy AuthContext từ extensions (đã set bởi auth middleware)
    // -------------------------------------------------------------------------
    let auth_ctx = match request.extensions().get::<AuthContext>() {
        Some(ctx) => ctx.clone(),
        None => {
            tracing::warn!("RequirePermission: Missing AuthContext in request extensions");
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "error": "unauthorized",
                    "message": "Authentication required"
                })),
            )
                .into_response();
        }
    };

    // -------------------------------------------------------------------------
    // 2. Lấy context_id từ request header
    // -------------------------------------------------------------------------
    let context_id = match request
        .headers()
        .get("X-Context-Id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => {
            tracing::warn!(
                user_id = %auth_ctx.user_id,
                "RequirePermission: Missing or invalid X-Context-Id header"
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "bad_request",
                    "message": "X-Context-Id header is required and must be a valid UUID"
                })),
            )
                .into_response();
        }
    };

    // -------------------------------------------------------------------------
    // 3. Query kiểm tra quyền – JOIN qua 3 bảng
    //    user_context_roles → role_permissions → permissions
    //    Filter theo tenant_id (Multi-tenancy) + context cụ thể
    // -------------------------------------------------------------------------
    let has_permission = match check_context_permission(
        pool,
        auth_ctx.tenant_id,
        auth_ctx.user_id,
        &requirement.permission_code,
        &requirement.context_type,
        context_id,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!(
                error = %e,
                user_id = %auth_ctx.user_id,
                permission = %requirement.permission_code,
                "RequirePermission: Database error during permission check"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "internal_error",
                    "message": "Failed to verify permissions"
                })),
            )
                .into_response();
        }
    };

    // -------------------------------------------------------------------------
    // 4. Trả 403 nếu không có quyền
    // -------------------------------------------------------------------------
    if !has_permission {
        tracing::info!(
            user_id = %auth_ctx.user_id,
            permission = %requirement.permission_code,
            context_id = %context_id,
            "RequirePermission: Access denied"
        );
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "forbidden",
                "message": "You do not have the required permission in this context",
                "required_permission": requirement.permission_code,
                "context_type": format!("{:?}", requirement.context_type),
                "context_id": context_id
            })),
        )
            .into_response();
    }

    // -------------------------------------------------------------------------
    // 5. Pass-through – User có quyền, tiếp tục xử lý request
    // -------------------------------------------------------------------------
    tracing::debug!(
        user_id = %auth_ctx.user_id,
        permission = %requirement.permission_code,
        context_id = %context_id,
        "RequirePermission: Access granted"
    );

    next.run(request).await
}

// =============================================================================
// Permission Check Query – Tách riêng để dễ test
// =============================================================================

/// Kiểm tra user có permission cụ thể tại context cụ thể hay không.
///
/// Query sử dụng composite index `idx_ucr_permission_check` trên
/// `(tenant_id, user_id, context_type, context_id)` để đảm bảo hiệu suất
/// khi middleware gọi liên tục trên mỗi request.
///
/// Cũng kiểm tra `expires_at` để loại bỏ quyền đã hết hạn.
async fn check_context_permission(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    permission_code: &str,
    context_type: &ContextType,
    context_id: Uuid,
) -> Result<bool, sqlx::Error> {
    // Query tối ưu: EXISTS + 3-way JOIN + composite index hit
    // RLS trên PostgreSQL đã filter tenant_id, nhưng ta vẫn filter ở application
    // level để defense-in-depth (bảo mật nhiều lớp)
    let result = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM user_context_roles ucr
            INNER JOIN role_permissions rp ON rp.role_id = ucr.role_id
                AND rp.tenant_id = ucr.tenant_id
            INNER JOIN permissions p ON p.id = rp.permission_id
                AND p.tenant_id = rp.tenant_id
            WHERE ucr.tenant_id = $1
              AND ucr.user_id = $2
              AND p.code = $3
              AND ucr.context_type = $4
              AND ucr.context_id = $5
              -- Loại bỏ quyền đã hết hạn
              AND (ucr.expires_at IS NULL OR ucr.expires_at > NOW())
        )
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(permission_code)
    .bind(context_type)
    .bind(context_id)
    .fetch_one(pool)
    .await?;

    Ok(result)
}
