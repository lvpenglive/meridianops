-- 审计日志表：记录所有 API 写操作，用于追溯和安全审计
-- actor 为操作用户 username，target_type/target_id 指向业务实体
-- detail 存 JSON 快照（操作前后差异或关键参数）
CREATE TABLE audit_logs (
    id              BIGINT AUTO_INCREMENT PRIMARY KEY,
    actor_username  VARCHAR(128) NOT NULL,
    action          VARCHAR(64)  NOT NULL,
    target_type     VARCHAR(64)  NOT NULL,
    target_id       VARCHAR(128) NOT NULL,
    detail          JSON         NULL,
    ip              VARCHAR(64)  NOT NULL DEFAULT '',
    status          VARCHAR(16)  NOT NULL DEFAULT 'success',
    created_at      VARCHAR(64)  NOT NULL,
    INDEX idx_audit_actor (actor_username),
    INDEX idx_audit_action (action),
    INDEX idx_audit_target (target_type, target_id),
    INDEX idx_audit_created (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
