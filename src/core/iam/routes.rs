//! # IAM Routes
//!
//! Axum Router setup cho tất cả IAM endpoints.
//! Được mount dưới `/api` trong main router.

use axum::{
    routing::{delete, get, post, put},
    Router,
};
use sqlx::PgPool;

use super::handlers;

/// Tạo IAM router với tất cả CRUD routes.
///
/// Routes:
/// - `/departments` – CRUD + tree view
/// - `/roles` – CRUD
/// - `/permissions` – Create + List
/// - `/user-context-roles` – Assign (HITL) + List
/// - `/approvals` – List + Approve + Reject
pub fn iam_routes() -> Router<PgPool> {
    Router::new()
        // Departments
        .route("/departments", post(handlers::create_department))
        .route("/departments/tree", get(handlers::get_department_tree))
        .route("/departments/{id}", get(handlers::get_department))
        .route("/departments/{id}", put(handlers::update_department))
        .route("/departments/{id}", delete(handlers::delete_department))
        // Roles
        .route("/roles", post(handlers::create_role))
        .route("/roles", get(handlers::list_roles))
        .route("/roles/{id}", get(handlers::get_role))
        .route("/roles/{id}", put(handlers::update_role))
        .route("/roles/{id}", delete(handlers::delete_role))
        // Permissions
        .route("/permissions", post(handlers::create_permission))
        .route("/permissions", get(handlers::list_permissions))
        // User Context Roles
        .route("/user-context-roles", post(handlers::assign_context_role))
        .route("/user-context-roles", get(handlers::list_context_roles))
        // Approvals (HITL workflow)
        .route("/approvals", get(handlers::list_approvals))
        .route("/approvals/{id}/approve", post(handlers::approve_request))
        .route("/approvals/{id}/reject", post(handlers::reject_request))
}
