-- ============================================================
-- 2026-09-09: 告警组（独立实体，替代之前用部门作为「选择组」的做法）
-- 短信策略的「选择组」改为引用告警组（alert_group_id），
-- 候选人员从告警组成员中筛选，实际接收人仍以策略的 recipient_user_ids 为准。
-- ============================================================

-- ---- 1. 告警组表 ----
CREATE TABLE IF NOT EXISTS alert_groups (
    id           CHAR(36)      NOT NULL COMMENT '告警组 UUID',
    code         VARCHAR(64)   NOT NULL COMMENT '组编码（业务可读，唯一）',
    name         VARCHAR(128)  NOT NULL COMMENT '组名称',
    description  VARCHAR(512)  NULL     COMMENT '描述',
    enabled      TINYINT(1)    NOT NULL DEFAULT 1 COMMENT '1=启用 0=停用',
    created_by   VARCHAR(64)   NOT NULL COMMENT '创建人',
    created_at   VARCHAR(40)   NOT NULL COMMENT '创建时间',
    updated_at   VARCHAR(40)   NOT NULL COMMENT '更新时间',
    PRIMARY KEY (id),
    UNIQUE KEY uk_alert_group_code (code),
    INDEX idx_alert_group_enabled (enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='告警组';

-- ---- 2. 告警组成员表（组 ↔ 用户多对多） ----
CREATE TABLE IF NOT EXISTS alert_group_members (
    group_id   CHAR(36)    NOT NULL COMMENT '告警组 id',
    user_id    CHAR(36)    NOT NULL COMMENT '用户 id',
    created_at VARCHAR(40) NOT NULL COMMENT '加入时间',
    PRIMARY KEY (group_id, user_id),
    INDEX idx_alert_group_member_user (user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='告警组成员';

-- ---- 3. 权限点：告警组 ----
INSERT INTO permissions (id, code, name, module, description, created_at) VALUES
('00000000-0000-0000-0000-000000000306', 'alert_group:read',   '查看告警组', 'notification', '查看告警组及其成员', NOW()),
('00000000-0000-0000-0000-000000000307', 'alert_group:manage', '管理告警组', 'notification', '新增/编辑/删除/启停告警组及成员维护', NOW())
ON DUPLICATE KEY UPDATE name = VALUES(name);

INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000306'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000307'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000306'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000307'),
('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000306');

-- ---- 4. 把短信策略的 department_id 改为 alert_group_id ----
-- 兼容写法：先加新列，再迁移（这里老列无数据），最后删除旧列与旧索引
ALTER TABLE alert_sms_strategies
    ADD COLUMN alert_group_id CHAR(36) NULL COMMENT '选择组：告警组 id' AFTER name_keyword;

-- 把旧 department_id 上的数据（若有）并入 alert_group_id 并按 code 复用/创建对应告警组；当前环境该列为空。
UPDATE alert_sms_strategies s
LEFT JOIN departments d ON d.id = s.department_id
SET s.alert_group_id = (
    SELECT g.id FROM alert_groups g
    WHERE g.code = CONCAT('dept-', d.id)
    LIMIT 1
)
WHERE s.department_id IS NOT NULL;

-- 若上面没匹配到（部门被删等），按 code 自动创建占位组
INSERT IGNORE INTO alert_groups (id, code, name, description, enabled, created_by, created_at, updated_at)
SELECT UUID(),
       CONCAT('dept-', d.id),
       CONCAT('从部门迁移：', d.name),
       '由短信策略历史 department_id 自动创建的告警组',
       1,
       'system',
       UTC_TIMESTAMP(),
       UTC_TIMESTAMP()
FROM alert_sms_strategies s
JOIN departments d ON d.id = s.department_id
LEFT JOIN alert_groups g ON g.code = CONCAT('dept-', d.id)
WHERE s.alert_group_id IS NULL AND g.id IS NULL;

-- 再补一次映射
UPDATE alert_sms_strategies s
JOIN departments d ON d.id = s.department_id
LEFT JOIN alert_groups g ON g.code = CONCAT('dept-', d.id)
SET s.alert_group_id = g.id
WHERE s.alert_group_id IS NULL AND g.id IS NOT NULL;

-- 删除老列与索引
ALTER TABLE alert_sms_strategies DROP INDEX idx_sms_dept;
ALTER TABLE alert_sms_strategies DROP COLUMN department_id;
ALTER TABLE alert_sms_strategies ADD INDEX idx_sms_group (alert_group_id);