-- ============================================================
-- 2026-08-13: API 令牌管理（用于外部 CMDB/系统对接，可控制有效期）
-- ============================================================

CREATE TABLE IF NOT EXISTS api_tokens (
    id              CHAR(36)      NOT NULL COMMENT '令牌 ID (UUID)',
    name            VARCHAR(128)  NOT NULL COMMENT '令牌名称，如「蓝鲸 CMDB 对接」',
    token           VARCHAR(64)   NOT NULL COMMENT '令牌明文（mk-前缀 + 随机 48 字符），唯一索引',
    token_hash      VARCHAR(128)  NOT NULL COMMENT '令牌 SHA256 哈希（用于快速查表，避免明文泄露风险）',
    owner_user_id   CHAR(36)      NOT NULL COMMENT '创建者 user.id',
    scopes          JSON          NOT NULL COMMENT '权限范围：["asset:create","asset:read"] 等数组',
    role            VARCHAR(16)   NOT NULL DEFAULT 'operator' COMMENT '角色：admin / operator / viewer，决定 permissions 上限',
    expires_at      VARCHAR(64)   NULL COMMENT '过期时间（RFC3339），NULL 表示永不过期',
    revoked         TINYINT       NOT NULL DEFAULT 0 COMMENT '是否吊销：0=有效 1=吊销',
    revoked_at      VARCHAR(64)   NULL COMMENT '吊销时间',
    last_used_at    VARCHAR(64)   NULL COMMENT '最近一次使用时间',
    created_at      VARCHAR(64)   NOT NULL,
    updated_at      VARCHAR(64)   NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uk_api_tokens_token (token),
    UNIQUE KEY uk_api_tokens_token_hash (token_hash),
    INDEX idx_api_tokens_owner (owner_user_id),
    INDEX idx_api_tokens_expires (expires_at),
    INDEX idx_api_tokens_revoked (revoked)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='外部系统对接 API 令牌';
