-- =============================================================================
-- Migration 000: Core Schema – Tenants & Users
-- Prerequisite cho tất cả module khác (IAM, HR, Inventory, etc.)
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- =============================================================================
-- 1. Tenants – Tổ chức / Doanh nghiệp (gốc Multi-tenancy)
-- =============================================================================

CREATE TABLE tenants (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name        VARCHAR(255) NOT NULL,
    code        VARCHAR(100) NOT NULL UNIQUE,
    is_active   BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- =============================================================================
-- 2. Users – Người dùng hệ thống
-- =============================================================================

CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tenant_id       UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email           VARCHAR(255) NOT NULL,
    password_hash   TEXT NOT NULL,
    full_name       VARCHAR(255) NOT NULL,
    is_active       BOOLEAN NOT NULL DEFAULT TRUE,
    is_superadmin   BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Email unique trong toàn hệ thống (cross-tenant)
    CONSTRAINT uq_users_email UNIQUE (email)
);

CREATE INDEX idx_users_tenant ON users (tenant_id);
CREATE INDEX idx_users_email ON users (email);

-- RLS: Multi-tenancy isolation
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
CREATE POLICY users_tenant_isolation ON users
    USING (tenant_id = current_setting('app.current_tenant_id')::UUID);
