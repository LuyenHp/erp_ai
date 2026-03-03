//! # IAM Handlers – CRUD API
//!
//! Handlers cho departments, roles, permissions, user_context_roles, và approvals.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use crate::core::AppState;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::errors::AppError;
use crate::core::iam::middleware::AuthContext;
use crate::core::iam::models::*;
use crate::core::iam::org_tree;

// =============================================================================
// Helper: Set tenant_id trên connection cho RLS
// =============================================================================

/// Set `app.current_tenant_id` trên connection để RLS hoạt động đúng.
/// Phải gọi trước mọi query trên bảng có RLS.
async fn set_tenant_context(pool: &PgPool, tenant_id: Uuid) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, AppError> {
    let mut conn = pool.acquire().await?;
    sqlx::query(&format!(
        "SET LOCAL app.current_tenant_id = '{}'",
        tenant_id
    ))
    .execute(&mut *conn)
    .await?;
    Ok(conn)
}

// =============================================================================
// Departments
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateDepartmentRequest {
    pub name: String,
    pub code: String,
    pub parent_id: Option<Uuid>,
    pub description: Option<String>,
}

/// POST /api/departments – Tạo phòng ban mới
pub async fn create_department(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Json(req): Json<CreateDepartmentRequest>,
) -> Result<Json<Department>, AppError> {
    let pool = &state.pool;
    if req.name.is_empty() || req.code.is_empty() {
        return Err(AppError::BadRequest("name and code are required".into()));
    }

    // Tính toán path và level dựa trên parent
    let (path, level) = match req.parent_id {
        Some(parent_id) => {
            let parent = sqlx::query_as::<_, Department>(
                "SELECT * FROM departments WHERE id = $1 AND tenant_id = $2",
            )
            .bind(parent_id)
            .bind(auth.tenant_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Parent department not found".into()))?;

            let path = format!("{}.{}", parent.path, req.code);
            let level = parent.level + 1;
            (path, level)
        }
        None => {
            // Root department
            (req.code.clone(), 0)
        }
    };

    let dept = sqlx::query_as::<_, Department>(
        r#"
        INSERT INTO departments (tenant_id, name, code, parent_id, path, level, description)
        VALUES ($1, $2, $3, $4, $5::LTREE, $6, $7)
        RETURNING id, tenant_id, name, code, parent_id, path::TEXT as path, level, description, is_active, created_at, updated_at
        "#,
    )
    .bind(auth.tenant_id)
    .bind(&req.name)
    .bind(&req.code)
    .bind(req.parent_id)
    .bind(&path)
    .bind(level)
    .bind(&req.description)
    .fetch_one(pool)
    .await?;

    tracing::info!(dept_id = %dept.id, "Department created: {}", dept.name);
    Ok(Json(dept))
}

/// GET /api/departments/tree – Lấy toàn bộ sơ đồ tổ chức dạng cây
pub async fn get_department_tree(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
) -> Result<Json<Vec<DepartmentTreeNode>>, AppError> {
    let pool = &state.pool;
    let departments = org_tree::get_org_tree_cte(pool, auth.tenant_id, None).await?;
    let tree = org_tree::build_tree_structure(departments);
    Ok(Json(tree))
}

/// GET /api/departments/:id
pub async fn get_department(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Department>, AppError> {
    let pool = &state.pool;
    let dept = sqlx::query_as::<_, Department>(
        "SELECT id, tenant_id, name, code, parent_id, path::TEXT as path, level, description, is_active, created_at, updated_at FROM departments WHERE id = $1 AND tenant_id = $2",
    )
    .bind(id)
    .bind(auth.tenant_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Department not found".into()))?;

    Ok(Json(dept))
}

#[derive(Debug, Deserialize)]
pub struct UpdateDepartmentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

/// PUT /api/departments/:id
pub async fn update_department(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDepartmentRequest>,
) -> Result<Json<Department>, AppError> {
    let pool = &state.pool;
    let dept = sqlx::query_as::<_, Department>(
        r#"
        UPDATE departments
        SET name = COALESCE($3, name),
            description = COALESCE($4, description),
            is_active = COALESCE($5, is_active),
            updated_at = NOW()
        WHERE id = $1 AND tenant_id = $2
        RETURNING id, tenant_id, name, code, parent_id, path::TEXT as path, level, description, is_active, created_at, updated_at
        "#,
    )
    .bind(id)
    .bind(auth.tenant_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(req.is_active)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Department not found".into()))?;

    Ok(Json(dept))
}

/// DELETE /api/departments/:id (soft delete)
pub async fn delete_department(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = &state.pool;
    let result = sqlx::query(
        "UPDATE departments SET is_active = FALSE, updated_at = NOW() WHERE id = $1 AND tenant_id = $2",
    )
    .bind(id)
    .bind(auth.tenant_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Department not found".into()));
    }

    Ok(Json(serde_json::json!({"message": "Department deactivated"})))
}

// =============================================================================
// Roles
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
}

/// POST /api/roles
pub async fn create_role(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<Json<Role>, AppError> {
    let pool = &state.pool;
    if req.name.is_empty() || req.code.is_empty() {
        return Err(AppError::BadRequest("name and code are required".into()));
    }

    let role = sqlx::query_as::<_, Role>(
        r#"
        INSERT INTO roles (tenant_id, name, code, description)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(auth.tenant_id)
    .bind(&req.name)
    .bind(&req.code)
    .bind(&req.description)
    .fetch_one(pool)
    .await?;

    Ok(Json(role))
}

/// GET /api/roles
pub async fn list_roles(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
) -> Result<Json<Vec<Role>>, AppError> {
    let pool = &state.pool;
    let roles = sqlx::query_as::<_, Role>(
        "SELECT * FROM roles WHERE tenant_id = $1 AND is_active = TRUE ORDER BY name",
    )
    .bind(auth.tenant_id)
    .fetch_all(pool)
    .await?;

    Ok(Json(roles))
}

/// GET /api/roles/:id
pub async fn get_role(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Role>, AppError> {
    let pool = &state.pool;
    let role = sqlx::query_as::<_, Role>(
        "SELECT * FROM roles WHERE id = $1 AND tenant_id = $2",
    )
    .bind(id)
    .bind(auth.tenant_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Role not found".into()))?;

    Ok(Json(role))
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

/// PUT /api/roles/:id
pub async fn update_role(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<Role>, AppError> {
    let pool = &state.pool;
    let role = sqlx::query_as::<_, Role>(
        r#"
        UPDATE roles
        SET name = COALESCE($3, name),
            description = COALESCE($4, description),
            is_active = COALESCE($5, is_active),
            updated_at = NOW()
        WHERE id = $1 AND tenant_id = $2 AND is_system = FALSE
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(auth.tenant_id)
    .bind(&req.name)
    .bind(&req.description)
    .bind(req.is_active)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Role not found or is a system role".into()))?;

    Ok(Json(role))
}

/// DELETE /api/roles/:id (soft delete, block system roles)
pub async fn delete_role(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = &state.pool;
    let result = sqlx::query(
        "UPDATE roles SET is_active = FALSE, updated_at = NOW() WHERE id = $1 AND tenant_id = $2 AND is_system = FALSE",
    )
    .bind(id)
    .bind(auth.tenant_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Role not found or is a system role".into()));
    }

    Ok(Json(serde_json::json!({"message": "Role deactivated"})))
}

// =============================================================================
// Permissions
// =============================================================================

#[derive(Debug, Deserialize)]
pub struct CreatePermissionRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub module: Option<String>,
}

/// POST /api/permissions
pub async fn create_permission(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Json(req): Json<CreatePermissionRequest>,
) -> Result<Json<Permission>, AppError> {
    let pool = &state.pool;
    if req.code.is_empty() || req.name.is_empty() {
        return Err(AppError::BadRequest("code and name are required".into()));
    }

    let perm = sqlx::query_as::<_, Permission>(
        r#"
        INSERT INTO permissions (tenant_id, code, name, description, module)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(auth.tenant_id)
    .bind(&req.code)
    .bind(&req.name)
    .bind(&req.description)
    .bind(&req.module)
    .fetch_one(pool)
    .await?;

    Ok(Json(perm))
}

/// GET /api/permissions
pub async fn list_permissions(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
) -> Result<Json<Vec<Permission>>, AppError> {
    let pool = &state.pool;
    let perms = sqlx::query_as::<_, Permission>(
        "SELECT * FROM permissions WHERE tenant_id = $1 ORDER BY module, code",
    )
    .bind(auth.tenant_id)
    .fetch_all(pool)
    .await?;

    Ok(Json(perms))
}

// =============================================================================
// User Context Roles – Phân quyền chéo
// =============================================================================

/// POST /api/user-context-roles – Gán role cho user tại context (→ HITL pending approval)
pub async fn assign_context_role(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Json(req): Json<AssignCrossRoleInput>,
) -> Result<Json<PendingApproval>, AppError> {
    let pool = &state.pool;
    // Tạo pending approval thay vì gán trực tiếp (HITL rule)
    let payload = serde_json::json!({
        "user_id": req.user_id,
        "role_id": req.role_id,
        "context_type": req.context_type,
        "context_id": req.context_id,
    });

    let approval = sqlx::query_as::<_, PendingApproval>(
        r#"
        INSERT INTO pending_approvals (tenant_id, action_type, payload, status, requested_by)
        VALUES ($1, 'assign_cross_role', $2, 'AWAITING_HUMAN_APPROVAL', $3)
        RETURNING *
        "#,
    )
    .bind(auth.tenant_id)
    .bind(&payload)
    .bind(auth.user_id)
    .fetch_one(pool)
    .await?;

    tracing::info!(approval_id = %approval.id, "Cross-role assignment pending approval");
    Ok(Json(approval))
}

/// GET /api/user-context-roles – List tất cả role assignments của tenant
pub async fn list_context_roles(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
) -> Result<Json<Vec<UserContextRole>>, AppError> {
    let pool = &state.pool;
    let roles = sqlx::query_as::<_, UserContextRole>(
        "SELECT * FROM user_context_roles WHERE tenant_id = $1 ORDER BY assigned_at DESC",
    )
    .bind(auth.tenant_id)
    .fetch_all(pool)
    .await?;

    Ok(Json(roles))
}

// =============================================================================
// Pending Approvals – HITL Workflow
// =============================================================================

/// GET /api/approvals – List pending approvals
pub async fn list_approvals(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
) -> Result<Json<Vec<PendingApproval>>, AppError> {
    let pool = &state.pool;
    let approvals = sqlx::query_as::<_, PendingApproval>(
        "SELECT * FROM pending_approvals WHERE tenant_id = $1 ORDER BY created_at DESC",
    )
    .bind(auth.tenant_id)
    .fetch_all(pool)
    .await?;

    Ok(Json(approvals))
}

#[derive(Debug, Deserialize)]
pub struct ApprovalActionRequest {
    pub reason: Option<String>,
}

/// POST /api/approvals/:id/approve – Duyệt yêu cầu
pub async fn approve_request(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<ApprovalActionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = &state.pool;
    let mut tx = pool.begin().await?;

    // Lấy pending approval
    let approval = sqlx::query_as::<_, PendingApproval>(
        "SELECT * FROM pending_approvals WHERE id = $1 AND tenant_id = $2 AND status = 'AWAITING_HUMAN_APPROVAL'",
    )
    .bind(id)
    .bind(auth.tenant_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Pending approval not found".into()))?;

    // Cập nhật trạng thái → APPROVED
    sqlx::query(
        r#"
        UPDATE pending_approvals
        SET status = 'APPROVED', approved_by = $3, reason = $4, resolved_at = NOW()
        WHERE id = $1 AND tenant_id = $2
        "#,
    )
    .bind(id)
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(&req.reason)
    .execute(&mut *tx)
    .await?;

    // Thực thi action dựa trên action_type
    if approval.action_type == "assign_cross_role" {
        let payload = &approval.payload;
        sqlx::query(
            r#"
            INSERT INTO user_context_roles (tenant_id, user_id, role_id, context_type, context_id, assigned_by)
            VALUES ($1, $2::UUID, $3::UUID, $4::context_type_enum, $5::UUID, $6)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(auth.tenant_id)
        .bind(payload["user_id"].as_str().unwrap_or_default())
        .bind(payload["role_id"].as_str().unwrap_or_default())
        .bind(payload["context_type"].as_str().unwrap_or_default())
        .bind(payload["context_id"].as_str().unwrap_or_default())
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    tracing::info!(approval_id = %id, "Approval approved by {}", auth.user_id);
    Ok(Json(serde_json::json!({"message": "Approved successfully"})))
}

/// POST /api/approvals/:id/reject – Từ chối yêu cầu
pub async fn reject_request(
    State(state): State<AppState>,
    auth: axum::Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<ApprovalActionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pool = &state.pool;
    let result = sqlx::query(
        r#"
        UPDATE pending_approvals
        SET status = 'REJECTED', approved_by = $3, reason = $4, resolved_at = NOW()
        WHERE id = $1 AND tenant_id = $2 AND status = 'AWAITING_HUMAN_APPROVAL'
        "#,
    )
    .bind(id)
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(&req.reason)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Pending approval not found".into()));
    }

    tracing::info!(approval_id = %id, "Approval rejected by {}", auth.user_id);
    Ok(Json(serde_json::json!({"message": "Rejected successfully"})))
}
