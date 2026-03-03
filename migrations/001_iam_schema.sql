-- =============================================================================
-- Migration 001: IAM Schema – Identity & Access Management
-- Sơ đồ tổ chức phân cấp + Phân quyền chéo theo ngữ cảnh
-- =============================================================================

-- Enable required extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "ltree";

-- =============================================================================
-- 1. ENUM types
-- =============================================================================

-- Context types for cross-context role assignment
CREATE TYPE context_type_enum AS ENUM ('DEPARTMENT', 'PROJECT', 'BRANCH');

-- Status for pending approval workflow (HITL)
CREATE TYPE approval_status_enum AS ENUM (
    'AWAITING_HUMAN_APPROVAL',
    'APPROVED',
    'REJECTED'
);

-- =============================================================================
-- 2. Departments – Hierarchical Org Chart (ltree + parent_id)
-- =============================================================================
-- Sử dụng ltree cho truy vấn phân cấp siêu nhanh (GiST index),
-- giữ parent_id cho referential integrity.

CREATE TABLE departments (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id   UUID NOT NULL,
    name        VARCHAR(255) NOT NULL,
    code        VARCHAR(100) NOT NULL,
    parent_id   UUID REFERENCES departments(id) ON DELETE SET NULL,
    path        LTREE NOT NULL,               -- VD: 'company.hanoi.engineering'
    level       INT NOT NULL DEFAULT 0,        -- Depth in tree (0 = root)
    description TEXT,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Đảm bảo code unique trong cùng tenant
    CONSTRAINT uq_departments_tenant_code UNIQUE (tenant_id, code)
);

-- GiST index cho ltree – truy vấn ancestor/descendant O(1)
CREATE INDEX idx_departments_path_gist ON departments USING GIST (path);
-- B-tree index cho lookup theo tenant
CREATE INDEX idx_departments_tenant ON departments (tenant_id);
-- B-tree index cho parent lookup
CREATE INDEX idx_departments_parent ON departments (tenant_id, parent_id);

-- RLS: Multi-tenancy isolation
ALTER TABLE departments ENABLE ROW LEVEL SECURITY;
CREATE POLICY departments_tenant_isolation ON departments
    USING (tenant_id = current_setting('app.current_tenant_id')::UUID);

-- =============================================================================
-- 3. Roles – Vai trò trong hệ thống
-- =============================================================================

CREATE TABLE roles (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id   UUID NOT NULL,
    name        VARCHAR(255) NOT NULL,
    code        VARCHAR(100) NOT NULL,
    description TEXT,
    is_system   BOOLEAN NOT NULL DEFAULT FALSE, -- Role hệ thống không được xóa
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_roles_tenant_code UNIQUE (tenant_id, code)
);

CREATE INDEX idx_roles_tenant ON roles (tenant_id);

-- RLS: Multi-tenancy isolation
ALTER TABLE roles ENABLE ROW LEVEL SECURITY;
CREATE POLICY roles_tenant_isolation ON roles
    USING (tenant_id = current_setting('app.current_tenant_id')::UUID);

-- =============================================================================
-- 4. Permissions – Quyền hạn nguyên thủy
-- =============================================================================

CREATE TABLE permissions (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id   UUID NOT NULL,
    code        VARCHAR(255) NOT NULL,         -- VD: 'view_inventory', 'approve_order'
    name        VARCHAR(255) NOT NULL,
    description TEXT,
    module      VARCHAR(100),                  -- Module sở hữu permission
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT uq_permissions_tenant_code UNIQUE (tenant_id, code)
);

CREATE INDEX idx_permissions_tenant ON permissions (tenant_id);
CREATE INDEX idx_permissions_module ON permissions (tenant_id, module);

-- RLS: Multi-tenancy isolation
ALTER TABLE permissions ENABLE ROW LEVEL SECURITY;
CREATE POLICY permissions_tenant_isolation ON permissions
    USING (tenant_id = current_setting('app.current_tenant_id')::UUID);

-- =============================================================================
-- 5. Role-Permissions mapping (N:N)
-- =============================================================================

CREATE TABLE role_permissions (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id     UUID NOT NULL,
    role_id       UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Mỗi cặp role-permission chỉ tồn tại 1 lần trong cùng tenant
    CONSTRAINT uq_role_permissions UNIQUE (tenant_id, role_id, permission_id)
);

-- Index tối ưu cho JOIN khi check permission
CREATE INDEX idx_role_permissions_role ON role_permissions (tenant_id, role_id);
CREATE INDEX idx_role_permissions_perm ON role_permissions (tenant_id, permission_id);

-- RLS: Multi-tenancy isolation
ALTER TABLE role_permissions ENABLE ROW LEVEL SECURITY;
CREATE POLICY role_permissions_tenant_isolation ON role_permissions
    USING (tenant_id = current_setting('app.current_tenant_id')::UUID);

-- =============================================================================
-- 6. User Context Roles – BẢNG CỐT LÕI: Phân quyền chéo
-- =============================================================================
-- Giải quyết bài toán: "User A có Role B tại Ngữ cảnh C"
-- VD: Nguyễn Văn A là Trưởng phòng (role) tại Phòng Kỹ thuật (department)
-- VD: Nguyễn Văn A là Thành viên (role) tại Dự án Alpha (project)

CREATE TABLE user_context_roles (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id     UUID NOT NULL,
    user_id       UUID NOT NULL,               -- FK tới bảng users (sẽ tạo ở migration khác)
    role_id       UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    context_type  context_type_enum NOT NULL,   -- DEPARTMENT | PROJECT | BRANCH
    context_id    UUID NOT NULL,               -- ID của department/project/branch tương ứng
    assigned_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    assigned_by   UUID,                        -- Ai đã gán quyền này
    expires_at    TIMESTAMPTZ,                 -- Quyền có thời hạn (NULL = vĩnh viễn)

    -- Mỗi user chỉ giữ 1 role tại 1 context cụ thể
    CONSTRAINT uq_user_context_role UNIQUE (tenant_id, user_id, role_id, context_type, context_id)
);

-- *** INDEX TỐI ƯU CHO MIDDLEWARE ***
-- Composite index chính – được query liên tục bởi RequirePermission middleware
-- Thứ tự cột: tenant_id → user_id → context_type → context_id (selectivity cao → thấp)
CREATE INDEX idx_ucr_permission_check
    ON user_context_roles (tenant_id, user_id, context_type, context_id);

-- Index phụ cho query "tất cả roles của 1 user trong tenant"
CREATE INDEX idx_ucr_user ON user_context_roles (tenant_id, user_id);

-- Index cho query "tất cả users tại 1 context"
CREATE INDEX idx_ucr_context ON user_context_roles (tenant_id, context_type, context_id);

-- RLS: Multi-tenancy isolation
ALTER TABLE user_context_roles ENABLE ROW LEVEL SECURITY;
CREATE POLICY ucr_tenant_isolation ON user_context_roles
    USING (tenant_id = current_setting('app.current_tenant_id')::UUID);

-- =============================================================================
-- 7. Pending Approvals – HITL workflow cho các thao tác nhạy cảm
-- =============================================================================
-- Khi AI hoặc hệ thống yêu cầu gán quyền, tạo bản ghi chờ duyệt ở đây.
-- Giám đốc/Admin duyệt trên UI trước khi thực thi.

CREATE TABLE pending_approvals (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id       UUID NOT NULL,
    action_type     VARCHAR(100) NOT NULL,     -- VD: 'assign_cross_role', 'delete_data'
    payload         JSONB NOT NULL,            -- Dữ liệu chi tiết của hành động chờ duyệt
    status          approval_status_enum NOT NULL DEFAULT 'AWAITING_HUMAN_APPROVAL',
    requested_by    UUID NOT NULL,             -- User/AI đã yêu cầu
    approved_by     UUID,                      -- Admin đã duyệt (NULL nếu chưa)
    reason          TEXT,                      -- Lý do approve/reject
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at     TIMESTAMPTZ,               -- Thời điểm được duyệt/từ chối

    CONSTRAINT chk_resolved CHECK (
        (status = 'AWAITING_HUMAN_APPROVAL' AND resolved_at IS NULL AND approved_by IS NULL)
        OR (status != 'AWAITING_HUMAN_APPROVAL' AND resolved_at IS NOT NULL AND approved_by IS NOT NULL)
    )
);

CREATE INDEX idx_pending_approvals_tenant ON pending_approvals (tenant_id, status);
CREATE INDEX idx_pending_approvals_action ON pending_approvals (tenant_id, action_type, status);

-- RLS: Multi-tenancy isolation
ALTER TABLE pending_approvals ENABLE ROW LEVEL SECURITY;
CREATE POLICY pending_approvals_tenant_isolation ON pending_approvals
    USING (tenant_id = current_setting('app.current_tenant_id')::UUID);
