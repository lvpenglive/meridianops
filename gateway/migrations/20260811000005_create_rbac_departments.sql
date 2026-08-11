-- ============================================================
-- RBAC + 树形部门：roles / permissions / role_permissions / departments
-- 并改造 users 表：新增 role_id（外键）、department_id（外键）
-- 保留 users.role 字符串字段作为冗余，便于平滑过渡（后续可删）
-- ============================================================

-- ---- 部门表（树形，parent_id 自引用）----
CREATE TABLE departments (
    id           CHAR(36)     NOT NULL,
    name         VARCHAR(128) NOT NULL,
    parent_id    CHAR(36)     NULL,
    sort_order   INT          NOT NULL DEFAULT 0,
    enabled      TINYINT      NOT NULL DEFAULT 1,
    created_at   VARCHAR(64)  NOT NULL,
    updated_at   VARCHAR(64)  NOT NULL,
    PRIMARY KEY (id),
    KEY idx_dept_parent (parent_id),
    KEY idx_dept_sort (sort_order)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- ---- 角色表 ----
CREATE TABLE roles (
    id           CHAR(36)     NOT NULL,
    name         VARCHAR(64)  NOT NULL,
    display_name VARCHAR(128) NOT NULL DEFAULT '',
    description  VARCHAR(255) NOT NULL DEFAULT '',
    enabled      TINYINT      NOT NULL DEFAULT 1,
    built_in     TINYINT      NOT NULL DEFAULT 0,
    created_at   VARCHAR(64)  NOT NULL,
    updated_at   VARCHAR(64)  NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uk_roles_name (name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- ---- 权限点表 ----
CREATE TABLE permissions (
    id           CHAR(36)     NOT NULL,
    code         VARCHAR(128) NOT NULL,
    name         VARCHAR(128) NOT NULL DEFAULT '',
    module       VARCHAR(64)  NOT NULL DEFAULT '',
    description  VARCHAR(255) NOT NULL DEFAULT '',
    created_at   VARCHAR(64)  NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uk_perm_code (code),
    KEY idx_perm_module (module)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- ---- 角色-权限关联 ----
CREATE TABLE role_permissions (
    role_id       CHAR(36) NOT NULL,
    permission_id CHAR(36) NOT NULL,
    PRIMARY KEY (role_id, permission_id),
    KEY idx_rp_perm (permission_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- ---- users 表加列：role_id / department_id ----
ALTER TABLE users ADD COLUMN role_id CHAR(36) NULL AFTER role;
ALTER TABLE users ADD COLUMN department_id CHAR(36) NULL AFTER role_id;
ALTER TABLE users ADD KEY idx_users_role_id (role_id);
ALTER TABLE users ADD KEY idx_users_dept_id (department_id);