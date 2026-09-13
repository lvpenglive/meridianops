-- 资产多人负责人 + 同步用户身份字段
-- 1) users 增加来源/外部 ID，便于 CMDB 同步对上或创建用户
-- 2) ci_instance_owners 存多人；owner_id 仍保留为主责（第一人）兼容旧查询

ALTER TABLE users
    ADD COLUMN source VARCHAR(64) NULL COMMENT '用户来源：空=手工，同步写入 blueking 等' AFTER remark,
    ADD COLUMN external_id VARCHAR(128) NULL COMMENT '外部用户标识' AFTER source,
    ADD INDEX idx_users_source_ext (source, external_id);

CREATE TABLE IF NOT EXISTS ci_instance_owners (
    instance_id CHAR(36)     NOT NULL COMMENT 'ci_instances.id',
    user_id     CHAR(36)     NOT NULL COMMENT 'users.id',
    sort_order  INT          NOT NULL DEFAULT 0,
    created_at  VARCHAR(64)  NOT NULL,
    PRIMARY KEY (instance_id, user_id),
    KEY idx_cio_user (user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='资产负责人（多人）';

INSERT IGNORE INTO ci_instance_owners (instance_id, user_id, sort_order, created_at)
SELECT id, owner_id, 0, UTC_TIMESTAMP()
FROM ci_instances
WHERE owner_id IS NOT NULL AND owner_id <> '';
