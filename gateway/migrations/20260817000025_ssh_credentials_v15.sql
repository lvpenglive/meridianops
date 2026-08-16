-- 20260817000025: V1.5 SSH 执行模块
-- 1) ssh_credentials 表：独立凭据存储（密码/私钥 AES-256-GCM 加密）
-- 2) job_definitions 加字段：executor_type / credential_id

-- ===== SSH 凭据表 =====
CREATE TABLE IF NOT EXISTS ssh_credentials (
    id              BIGINT AUTO_INCREMENT PRIMARY KEY,
    name            VARCHAR(128) NOT NULL COMMENT '凭据名称（如：生产root密钥）',
    auth_type       VARCHAR(16)  NOT NULL DEFAULT 'password' COMMENT '认证方式: password / key',
    username        VARCHAR(128) NOT NULL COMMENT 'SSH 用户名',
    -- 加密存储：AES-256-GCM，nonce(12B) + ciphertext+tag，hex 编码
    password_enc    VARCHAR(512) NOT NULL DEFAULT '' COMMENT '加密后的密码（auth_type=password 时有值）',
    private_key_enc TEXT         NULL COMMENT '加密后的私钥 PEM（auth_type=key 时有值）',
    passphrase_enc  VARCHAR(512) NOT NULL DEFAULT '' COMMENT '加密后的私钥口令（可选）',
    -- 主机密钥指纹（可选，用于 host key 验证；空=接受所有）
    host_key_fingerprint VARCHAR(128) NOT NULL DEFAULT '' COMMENT '预期主机密钥指纹(SHA256)，空=跳过验证',
    description     VARCHAR(512) NOT NULL DEFAULT '' COMMENT '描述',
    created_by      VARCHAR(128) NOT NULL COMMENT '创建人',
    created_at      DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_cred_name (name),
    INDEX idx_cred_creator (created_by)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='SSH 凭据';

-- ===== job_definitions 加字段 =====
ALTER TABLE job_definitions
    ADD COLUMN executor_type VARCHAR(16) NOT NULL DEFAULT 'ssh' COMMENT '执行器类型: mock / ssh' AFTER enabled,
    ADD COLUMN credential_id BIGINT NULL COMMENT '关联 ssh_credentials.id（executor_type=ssh 时必填）' AFTER executor_type,
    ADD INDEX idx_job_executor (executor_type);

-- ===== 凭据管理权限种子 =====
INSERT IGNORE INTO permissions (id, code, name, module, description, created_at) VALUES
('20000000-0000-0000-0000-000000000031', 'credential:read',   '查看凭据', '凭据管理', '查看SSH凭据列表',     UTC_TIMESTAMP()),
('20000000-0000-0000-0000-000000000032', 'credential:create', '创建凭据', '凭据管理', '新建/编辑SSH凭据',    UTC_TIMESTAMP()),
('20000000-0000-0000-0000-000000000033', 'credential:delete', '删除凭据', '凭据管理', '删除SSH凭据',         UTC_TIMESTAMP());

-- admin：全部 3 个凭据权限
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000031'),
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000032'),
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000033');

-- operator：读 + 创建编辑（不可删除）
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000031'),
('00000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000032');

-- viewer：只读
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000031');
