//! # Org Tree – Sơ đồ tổ chức (Organization Chart)
//!
//! Cung cấp các hàm truy vấn sơ đồ tổ chức phân cấp với hiệu suất cao.
//! Hỗ trợ 2 phương thức:
//! 1. **ltree query** – O(1) với GiST index (ưu tiên)
//! 2. **Recursive CTE** – fallback cho các hệ thống không có ltree
//!
//! Kết quả flat list được chuyển thành nested tree structure cho API response.

use sqlx::PgPool;
use uuid::Uuid;

use super::models::{Department, DepartmentTreeNode};

// =============================================================================
// 1. ltree-based Query – Phương thức ưu tiên (fastest)
// =============================================================================

/// Lấy toàn bộ sơ đồ tổ chức của một tenant sử dụng ltree.
///
/// Sử dụng toán tử `<@` (is descendant of) trên cột `path` với GiST index
/// để truy vấn tất cả phòng ban con/cháu của root path.
///
/// ## Arguments
/// * `pool` - Connection pool PostgreSQL
/// * `tenant_id` - UUID của tenant (Multi-tenancy isolation)
/// * `root_path` - Đường dẫn ltree gốc (VD: "company" cho toàn bộ, "company.hanoi" cho chi nhánh HN)
///
/// ## Performance
/// - GiST index trên `path` cho O(1) lookup
/// - ORDER BY `path` để đảm bảo thứ tự parent → children
pub async fn get_org_tree_ltree(
    pool: &PgPool,
    tenant_id: Uuid,
    root_path: &str,
) -> Result<Vec<Department>, sqlx::Error> {
    // RLS filter tenant_id ở DB layer, application layer filter thêm
    // để defense-in-depth (bảo mật nhiều lớp)
    let departments = sqlx::query_as::<_, Department>(
        r#"
        SELECT id, tenant_id, name, code, parent_id, path::TEXT as path,
               level, description, is_active, created_at, updated_at
        FROM departments
        WHERE tenant_id = $1
          AND path <@ $2::LTREE
          AND is_active = TRUE
        ORDER BY path ASC
        "#,
    )
    .bind(tenant_id)
    .bind(root_path)
    .fetch_all(pool)
    .await?;

    Ok(departments)
}

// =============================================================================
// 2. Recursive CTE – Fallback method
// =============================================================================

/// Lấy toàn bộ sơ đồ tổ chức sử dụng Recursive CTE (Common Table Expression).
///
/// Phương thức này không phụ thuộc vào extension ltree, dùng `parent_id`
/// để duyệt cây từ gốc xuống lá.
///
/// ## Arguments
/// * `pool` - Connection pool PostgreSQL
/// * `tenant_id` - UUID của tenant
/// * `root_id` - UUID của phòng ban gốc (None = lấy từ root, tức parent_id IS NULL)
///
/// ## Performance
/// - Sử dụng recursive CTE: hiệu suất phụ thuộc vào độ sâu cây
/// - Phù hợp cho cây có depth <= 10 levels
pub async fn get_org_tree_cte(
    pool: &PgPool,
    tenant_id: Uuid,
    root_id: Option<Uuid>,
) -> Result<Vec<Department>, sqlx::Error> {
    let departments = sqlx::query_as::<_, Department>(
        r#"
        WITH RECURSIVE org_tree AS (
            -- Base case: lấy node gốc
            SELECT id, tenant_id, name, code, parent_id, path::TEXT as path,
                   level, description, is_active, created_at, updated_at
            FROM departments
            WHERE tenant_id = $1
              AND is_active = TRUE
              AND (
                  -- Nếu root_id = NULL → lấy tất cả root nodes (parent_id IS NULL)
                  ($2::UUID IS NULL AND parent_id IS NULL)
                  OR
                  -- Nếu root_id != NULL → lấy node cụ thể
                  id = $2
              )

            UNION ALL

            -- Recursive case: lấy các node con
            SELECT d.id, d.tenant_id, d.name, d.code, d.parent_id, d.path::TEXT as path,
                   d.level, d.description, d.is_active, d.created_at, d.updated_at
            FROM departments d
            INNER JOIN org_tree ot ON d.parent_id = ot.id
            WHERE d.tenant_id = $1
              AND d.is_active = TRUE
        )
        SELECT * FROM org_tree
        ORDER BY level ASC, name ASC
        "#,
    )
    .bind(tenant_id)
    .bind(root_id)
    .fetch_all(pool)
    .await?;

    Ok(departments)
}

// =============================================================================
// 3. Build Tree Structure – Chuyển flat list → nested JSON tree
// =============================================================================

/// Chuyển danh sách phẳng các `Department` thành cấu trúc cây nested.
///
/// Algorithm: O(n) sử dụng HashMap để index theo ID, sau đó duyệt 1 lần
/// để gắn children vào parent tương ứng.
///
/// ## Arguments
/// * `departments` - Danh sách phẳng các Department (đã sorted theo level/path)
///
/// ## Returns
/// Vec các root nodes, mỗi node chứa children đệ quy
pub fn build_tree_structure(departments: Vec<Department>) -> Vec<DepartmentTreeNode> {
    use std::collections::HashMap;

    if departments.is_empty() {
        return Vec::new();
    }

    // Step 1: Tạo tất cả nodes và index theo ID
    let mut node_map: HashMap<Uuid, DepartmentTreeNode> = HashMap::new();
    let mut order: Vec<Uuid> = Vec::new();

    for dept in &departments {
        node_map.insert(
            dept.id,
            DepartmentTreeNode {
                department: dept.clone(),
                children: Vec::new(),
            },
        );
        order.push(dept.id);
    }

    // Step 2: Duyệt ngược (từ lá lên gốc) để gắn children
    // Duyệt ngược để đảm bảo children đã hoàn chỉnh trước khi gắn vào parent
    let mut roots: Vec<DepartmentTreeNode> = Vec::new();

    for id in order.into_iter().rev() {
        let node = match node_map.remove(&id) {
            Some(n) => n,
            None => continue,
        };

        match node.department.parent_id {
            Some(parent_id) => {
                // Gắn node vào parent
                if let Some(parent) = node_map.get_mut(&parent_id) {
                    parent.children.insert(0, node); // insert(0) để giữ thứ tự
                } else {
                    // Parent không tìm thấy trong dataset → coi như root
                    roots.insert(0, node);
                }
            }
            None => {
                // Node gốc (không có parent)
                roots.insert(0, node);
            }
        }
    }

    roots
}
