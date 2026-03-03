//! # IAM Skills – AI Function Calling (MCP Standard)
//!
//! Các skill cho phép AI thao tác với hệ thống phân quyền và sơ đồ tổ chức.
//! Mỗi skill có metadata mô tả bằng tiếng Việt để AI hiểu và gọi đúng hàm.
//!
//! ## Quy tắc:
//! - Metadata phải mô tả rõ mục đích, input, output
//! - Các thao tác nhạy cảm (gán quyền) PHẢI trả về `AWAITING_HUMAN_APPROVAL`
//! - AI KHÔNG ĐƯỢC tự ý cấp quyền (HITL – Human-in-the-loop)

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::iam::models::{ApprovalStatus, ContextType, EffectivePermission};

// =============================================================================
// Skill Metadata – Chuẩn MCP cho AI Function Calling
// =============================================================================

/// Metadata mô tả skill cho AI. Chuẩn MCP (Model Context Protocol).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Tên hàm duy nhất
    pub name: String,
    /// Mô tả ngắn gọn bằng tiếng Việt cho AI hiểu mục đích
    pub description: String,
    /// Mô tả chi tiết các tình huống sử dụng
    pub usage_examples: Vec<String>,
    /// Schema của input parameters (JSON Schema format)
    pub parameters: Value,
    /// Schema của output
    pub returns: Value,
}

/// Kết quả trả về từ skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub data: Option<Value>,
    pub message: String,
    /// Trạng thái đặc biệt cho HITL workflow
    pub approval_status: Option<ApprovalStatus>,
}

// =============================================================================
// Skill 1: get_user_effective_permissions
// =============================================================================

/// Trả về metadata chuẩn MCP cho skill `get_user_effective_permissions`.
/// AI sử dụng metadata này để biết khi nào cần gọi hàm.
pub fn get_user_effective_permissions_metadata() -> SkillMetadata {
    SkillMetadata {
        name: "get_user_effective_permissions".into(),
        description: "Lấy danh sách tất cả quyền hạn có hiệu lực của một nhân viên \
            tại một ngữ cảnh cụ thể (phòng ban, dự án, chi nhánh). \
            Sử dụng để trả lời câu hỏi: 'Nhân viên A có được phép làm gì ở chi nhánh B?'"
            .into(),
        usage_examples: vec![
            "Nhân viên Nguyễn Văn A có quyền duyệt đơn ở chi nhánh Hà Nội không?".into(),
            "Liệt kê tất cả quyền của user ID abc-123 tại phòng Kỹ thuật".into(),
            "Kiểm tra ai có quyền approve_order tại dự án Alpha".into(),
        ],
        parameters: json!({
            "type": "object",
            "required": ["user_id", "context_id"],
            "properties": {
                "user_id": {
                    "type": "string",
                    "format": "uuid",
                    "description": "UUID của nhân viên cần kiểm tra quyền"
                },
                "context_id": {
                    "type": "string",
                    "format": "uuid",
                    "description": "UUID của ngữ cảnh (phòng ban/dự án/chi nhánh)"
                }
            }
        }),
        returns: json!({
            "type": "object",
            "properties": {
                "permissions": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "permission_code": { "type": "string" },
                            "permission_name": { "type": "string" },
                            "granted_by_role": { "type": "string" },
                            "context_type": { "type": "string" },
                            "context_id": { "type": "string", "format": "uuid" }
                        }
                    }
                }
            }
        }),
    }
}

/// Thực thi skill: Lấy danh sách quyền effective của user tại context.
///
/// ## Logic:
/// 1. Tìm tất cả roles của user tại context (qua `user_context_roles`)
/// 2. JOIN với `role_permissions` → `permissions` để lấy danh sách quyền
/// 3. Lọc quyền đã hết hạn (`expires_at`)
/// 4. Trả về danh sách quyền kèm thông tin role cấp quyền
pub async fn get_user_effective_permissions(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    context_id: Uuid,
) -> Result<SkillResult, sqlx::Error> {
    // Query lấy tất cả quyền effective, JOIN 3 bảng
    // RLS ở DB layer đã filter tenant_id, application layer filter thêm
    let permissions = sqlx::query_as::<_, EffectivePermission>(
        r#"
        SELECT
            p.id AS permission_id,
            p.code AS permission_code,
            p.name AS permission_name,
            r.id AS granted_by_role_id,
            r.name AS granted_by_role_name,
            ucr.context_type,
            ucr.context_id
        FROM user_context_roles ucr
        INNER JOIN roles r ON r.id = ucr.role_id AND r.tenant_id = ucr.tenant_id
        INNER JOIN role_permissions rp ON rp.role_id = ucr.role_id AND rp.tenant_id = ucr.tenant_id
        INNER JOIN permissions p ON p.id = rp.permission_id AND p.tenant_id = rp.tenant_id
        WHERE ucr.tenant_id = $1
          AND ucr.user_id = $2
          AND ucr.context_id = $3
          -- Loại bỏ quyền đã hết hạn
          AND (ucr.expires_at IS NULL OR ucr.expires_at > NOW())
        ORDER BY p.code ASC
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(context_id)
    .fetch_all(pool)
    .await?;

    let count = permissions.len();
    let data = serde_json::to_value(&permissions).unwrap_or(Value::Null);

    Ok(SkillResult {
        success: true,
        data: Some(json!({
            "user_id": user_id,
            "context_id": context_id,
            "total_permissions": count,
            "permissions": data
        })),
        message: format!(
            "Tìm thấy {} quyền có hiệu lực của user {} tại context {}",
            count, user_id, context_id
        ),
        approval_status: None,
    })
}

// =============================================================================
// Skill 2: assign_cross_role
// =============================================================================

/// Trả về metadata chuẩn MCP cho skill `assign_cross_role`.
pub fn assign_cross_role_metadata() -> SkillMetadata {
    SkillMetadata {
        name: "assign_cross_role".into(),
        description: "Gán vai trò (role) cho nhân viên tại một ngữ cảnh cụ thể \
            (phòng ban/dự án/chi nhánh). ĐÂY LÀ THAO TÁC NHẠY CẢM: hệ thống sẽ \
            tạo yêu cầu chờ duyệt (AWAITING_HUMAN_APPROVAL) và Giám đốc/Admin phải \
            duyệt trên giao diện trước khi quyền được cấp. AI KHÔNG được tự ý cấp quyền."
            .into(),
        usage_examples: vec![
            "Gán Nguyễn Văn A làm Trưởng phòng tại Phòng Kỹ thuật".into(),
            "Thêm user xyz vào dự án Alpha với vai trò Thành viên".into(),
            "Cấp quyền Quản lý chi nhánh cho nhân viên mới tại chi nhánh HCM".into(),
        ],
        parameters: json!({
            "type": "object",
            "required": ["user_id", "role_id", "context_type", "context_id"],
            "properties": {
                "user_id": {
                    "type": "string",
                    "format": "uuid",
                    "description": "UUID của nhân viên cần gán quyền"
                },
                "role_id": {
                    "type": "string",
                    "format": "uuid",
                    "description": "UUID của vai trò cần gán"
                },
                "context_type": {
                    "type": "string",
                    "enum": ["DEPARTMENT", "PROJECT", "BRANCH"],
                    "description": "Loại ngữ cảnh: DEPARTMENT (phòng ban), PROJECT (dự án), BRANCH (chi nhánh)"
                },
                "context_id": {
                    "type": "string",
                    "format": "uuid",
                    "description": "UUID của ngữ cảnh (phòng ban/dự án/chi nhánh)"
                }
            }
        }),
        returns: json!({
            "type": "object",
            "properties": {
                "approval_id": {
                    "type": "string",
                    "format": "uuid",
                    "description": "UUID của yêu cầu chờ duyệt"
                },
                "status": {
                    "type": "string",
                    "enum": ["AWAITING_HUMAN_APPROVAL"],
                    "description": "Trạng thái: luôn là AWAITING_HUMAN_APPROVAL"
                }
            }
        }),
    }
}

/// Thực thi skill: Tạo yêu cầu gán quyền chéo – LUÔN YÊU CẦU DUYỆT.
///
/// ## HITL – Human-in-the-loop:
/// - AI KHÔNG ĐƯỢC trực tiếp insert vào `user_context_roles`
/// - Thay vào đó, tạo bản ghi trong `pending_approvals` với status `AWAITING_HUMAN_APPROVAL`
/// - Giám đốc/Admin duyệt trên UI, sau đó hệ thống mới thực sự gán quyền
///
/// ## Logic:
/// 1. Validate input (user, role, context tồn tại)
/// 2. Kiểm tra quyền đã tồn tại chưa (tránh duplicate)
/// 3. Tạo `PendingApproval` record
/// 4. Trả về `AWAITING_HUMAN_APPROVAL` – **KHÔNG BAO GIỜ tự gán quyền**
pub async fn assign_cross_role(
    pool: &PgPool,
    tenant_id: Uuid,
    requested_by: Uuid,
    user_id: Uuid,
    role_id: Uuid,
    context_type: ContextType,
    context_id: Uuid,
) -> Result<SkillResult, sqlx::Error> {
    // -------------------------------------------------------------------------
    // 1. Kiểm tra role tồn tại và thuộc đúng tenant
    // -------------------------------------------------------------------------
    let role_exists = sqlx::query_scalar::<_, bool>(
        r#"SELECT EXISTS (SELECT 1 FROM roles WHERE id = $1 AND tenant_id = $2 AND is_active = TRUE)"#,
    )
    .bind(role_id)
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;

    if !role_exists {
        return Ok(SkillResult {
            success: false,
            data: None,
            message: format!("Role {} không tồn tại hoặc đã bị vô hiệu hóa", role_id),
            approval_status: None,
        });
    }

    // -------------------------------------------------------------------------
    // 2. Kiểm tra quyền đã được gán chưa (tránh tạo approval trùng lặp)
    // -------------------------------------------------------------------------
    let already_assigned = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM user_context_roles
            WHERE tenant_id = $1
              AND user_id = $2
              AND role_id = $3
              AND context_type = $4
              AND context_id = $5
              AND (expires_at IS NULL OR expires_at > NOW())
        )
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(role_id)
    .bind(&context_type)
    .bind(context_id)
    .fetch_one(pool)
    .await?;

    if already_assigned {
        return Ok(SkillResult {
            success: false,
            data: None,
            message: format!(
                "User {} đã có role {} tại context {} (type: {:?}). Không cần gán lại.",
                user_id, role_id, context_id, context_type
            ),
            approval_status: None,
        });
    }

    // -------------------------------------------------------------------------
    // 3. Kiểm tra đã có pending approval chưa (tránh spam)
    // -------------------------------------------------------------------------
    let pending_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM pending_approvals
            WHERE tenant_id = $1
              AND action_type = 'assign_cross_role'
              AND status = 'AWAITING_HUMAN_APPROVAL'
              AND payload->>'user_id' = $2
              AND payload->>'role_id' = $3
              AND payload->>'context_id' = $4
        )
        "#,
    )
    .bind(tenant_id)
    .bind(user_id.to_string())
    .bind(role_id.to_string())
    .bind(context_id.to_string())
    .fetch_one(pool)
    .await?;

    if pending_exists {
        return Ok(SkillResult {
            success: false,
            data: None,
            message: "Đã có yêu cầu gán quyền tương tự đang chờ duyệt. Vui lòng chờ Admin xử lý."
                .into(),
            approval_status: Some(ApprovalStatus::AwaitingHumanApproval),
        });
    }

    // -------------------------------------------------------------------------
    // 4. TẠO PENDING APPROVAL – ** KHÔNG TỰ GÁN QUYỀN **
    //    Đây là quy tắc HITL bắt buộc: AI phải chờ con người duyệt.
    // -------------------------------------------------------------------------
    let payload = json!({
        "user_id": user_id,
        "role_id": role_id,
        "context_type": format!("{:?}", context_type),
        "context_id": context_id,
        "description": format!(
            "Gán role {} cho user {} tại {:?} {}",
            role_id, user_id, context_type, context_id
        )
    });

    let approval_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO pending_approvals (tenant_id, action_type, payload, status, requested_by)
        VALUES ($1, 'assign_cross_role', $2, 'AWAITING_HUMAN_APPROVAL', $3)
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(&payload)
    .bind(requested_by)
    .fetch_one(pool)
    .await?;

    // -------------------------------------------------------------------------
    // 5. Trả kết quả: AWAITING_HUMAN_APPROVAL
    // -------------------------------------------------------------------------
    Ok(SkillResult {
        success: true,
        data: Some(json!({
            "approval_id": approval_id,
            "action_type": "assign_cross_role",
            "payload": payload,
            "status": "AWAITING_HUMAN_APPROVAL",
            "message_for_user": "Yêu cầu gán quyền đã được tạo. Giám đốc/Admin cần duyệt trên giao diện quản lý."
        })),
        message: format!(
            "Đã tạo yêu cầu gán quyền (ID: {}). Trạng thái: CHỜ DUYỆT. \
            Giám đốc/Admin cần phê duyệt trước khi quyền có hiệu lực.",
            approval_id
        ),
        approval_status: Some(ApprovalStatus::AwaitingHumanApproval),
    })
}

// =============================================================================
// Registry – Đăng ký tất cả IAM skills cho MCP
// =============================================================================

/// Trả về metadata của tất cả IAM skills.
/// Dùng khi AI cần biết nó có thể gọi những hàm nào liên quan đến phân quyền.
pub fn register_iam_skills() -> Vec<SkillMetadata> {
    vec![
        get_user_effective_permissions_metadata(),
        assign_cross_role_metadata(),
    ]
}
