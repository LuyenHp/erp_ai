//! # IAM Models
//!
//! Định nghĩa các struct cho module Identity & Access Management.
//! Tất cả struct đều derive `serde::Serialize/Deserialize` để tương tác JSON
//! và `sqlx::FromRow` để map trực tiếp từ PostgreSQL rows.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// Context Type Enum – Mapping với PostgreSQL ENUM `context_type_enum`
// =============================================================================

/// Loại ngữ cảnh cho phân quyền chéo.
/// Map 1:1 với PostgreSQL ENUM `context_type_enum`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "context_type_enum", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContextType {
    Department,
    Project,
    Branch,
}

/// Trạng thái duyệt cho workflow HITL.
/// Map 1:1 với PostgreSQL ENUM `approval_status_enum`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "approval_status_enum", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovalStatus {
    AwaitingHumanApproval,
    Approved,
    Rejected,
}

// =============================================================================
// Department – Sơ đồ tổ chức phân cấp
// =============================================================================

/// Phòng ban/chi nhánh trong sơ đồ tổ chức.
/// Sử dụng `path` (ltree) cho truy vấn phân cấp nhanh,
/// `parent_id` cho referential integrity.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Department {
    pub id: Uuid,
    /// Tenant sở hữu – cô lập dữ liệu đa doanh nghiệp (Multi-tenancy)
    pub tenant_id: Uuid,
    pub name: String,
    pub code: String,
    pub parent_id: Option<Uuid>,
    /// Đường dẫn ltree phân cấp, VD: "company.hanoi.engineering"
    pub path: String,
    /// Độ sâu trong cây (0 = root/gốc)
    pub level: i32,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Node trong cây tổ chức – dạng nested cho API response.
/// Chuyển đổi từ flat `Department` list sang cấu trúc cây JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepartmentTreeNode {
    #[serde(flatten)]
    pub department: Department,
    pub children: Vec<DepartmentTreeNode>,
}

// =============================================================================
// Role – Vai trò trong hệ thống
// =============================================================================

/// Vai trò được định nghĩa trong tenant.
/// VD: "Trưởng phòng", "Nhân viên", "Giám đốc chi nhánh".
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Role {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    /// Role hệ thống (admin, superadmin) không được phép xóa
    pub is_system: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// Permission – Quyền hạn nguyên thủy
// =============================================================================

/// Quyền hạn nguyên thủy trong hệ thống.
/// VD: "view_inventory", "approve_order", "manage_employees".
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Permission {
    pub id: Uuid,
    pub tenant_id: Uuid,
    /// Code định danh duy nhất, VD: "view_inventory"
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    /// Module sở hữu permission, VD: "inventory", "hr"
    pub module: Option<String>,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// Role-Permission Mapping
// =============================================================================

/// Mapping N:N giữa Role và Permission.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RolePermission {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub role_id: Uuid,
    pub permission_id: Uuid,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// User Context Role – BẢNG CỐT LÕI: Phân quyền chéo
// =============================================================================

/// Gán quyền chéo theo ngữ cảnh.
/// Giải quyết bài toán: "User A có Role B tại Ngữ cảnh C".
///
/// VD:
/// - Nguyễn Văn A (user_id) là Trưởng phòng (role_id) tại Phòng Kỹ thuật (DEPARTMENT, context_id)
/// - Nguyễn Văn A (user_id) là Thành viên (role_id) tại Dự án Alpha (PROJECT, context_id)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserContextRole {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub context_type: ContextType,
    pub context_id: Uuid,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: Option<Uuid>,
    /// Thời hạn quyền – NULL nghĩa là vĩnh viễn
    pub expires_at: Option<DateTime<Utc>>,
}

// =============================================================================
// Effective Permission – Kết quả truy vấn quyền thực tế
// =============================================================================

/// Quyền thực tế (effective) của user tại một context cụ thể.
/// Kết quả từ JOIN: user_context_roles → role_permissions → permissions.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EffectivePermission {
    pub permission_id: Uuid,
    pub permission_code: String,
    pub permission_name: String,
    /// Role nào cấp quyền này
    pub granted_by_role_id: Uuid,
    pub granted_by_role_name: String,
    /// Context nơi quyền có hiệu lực
    pub context_type: ContextType,
    pub context_id: Uuid,
}

// =============================================================================
// Pending Approval – HITL Workflow
// =============================================================================

/// Bản ghi chờ duyệt cho các thao tác nhạy cảm.
/// AI hoặc hệ thống tạo bản ghi ở đây, Giám đốc/Admin duyệt trên UI.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PendingApproval {
    pub id: Uuid,
    pub tenant_id: Uuid,
    /// Loại hành động, VD: "assign_cross_role"
    pub action_type: String,
    /// Dữ liệu chi tiết dạng JSON
    pub payload: serde_json::Value,
    pub status: ApprovalStatus,
    pub requested_by: Uuid,
    pub approved_by: Option<Uuid>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

// =============================================================================
// Request / Input Types – Cho API handlers
// =============================================================================

/// Input cho việc kiểm tra quyền trong middleware.
#[derive(Debug, Clone, Deserialize)]
pub struct PermissionCheckInput {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub permission_code: String,
    pub context_type: ContextType,
    pub context_id: Uuid,
}

/// Input cho AI Skill: gán quyền chéo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignCrossRoleInput {
    pub user_id: Uuid,
    pub role_id: Uuid,
    pub context_type: ContextType,
    pub context_id: Uuid,
}
