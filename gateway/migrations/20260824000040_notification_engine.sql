-- 20260824000040: 通知引擎 + Eventide 双向回写支持
-- 1) notification_channels: 通知通道配置（邮件 SMTP / 飞书 webhook / 通用 webhook）
-- 2) notification_rules: 通知规则（事件类型 → 通道 → 接收人）
-- 3) alert_events 新增 external_id 列，存储 Eventide 端告警 ID 用于回写
-- 4) 通知引擎权限点

-- ============================================================
-- notification_channels: 通知通道
-- ============================================================
CREATE TABLE IF NOT EXISTS notification_channels (
    id          CHAR(36)       NOT NULL COMMENT '通道 UUID',
    name        VARCHAR(128)   NOT NULL COMMENT '通道名称',
    channel_type VARCHAR(32)   NOT NULL COMMENT '通道类型：email / feishu / webhook',
    config_json TEXT           NOT NULL COMMENT '通道配置 JSON（SMTP 设置/webhook URL 等）',
    enabled     TINYINT        NOT NULL DEFAULT 1 COMMENT '是否启用',
    created_by  VARCHAR(64)    NOT NULL COMMENT '创建人',
    created_at  VARCHAR(40)    NOT NULL COMMENT '创建时间',
    updated_at  VARCHAR(40)    NOT NULL COMMENT '更新时间',
    PRIMARY KEY (id),
    INDEX idx_channel_type (channel_type),
    INDEX idx_channel_enabled (enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='通知通道配置';

-- ============================================================
-- notification_rules: 通知规则
--   event_type: alert_firing / alert_acknowledged / alert_resolved
--               ticket_assigned / ticket_approved / ticket_rejected / ticket_closed
--               job_failed / system
--   severity_filter: JSON 数组，如 ["disaster","critical"]，为空则不限
--   channel_ids: JSON 数组，关联 notification_channels.id
--   recipient_list: 逗号分隔的接收人（邮箱地址 / 飞书 @user / 通用）
-- ============================================================
CREATE TABLE IF NOT EXISTS notification_rules (
    id              CHAR(36)     NOT NULL COMMENT '规则 UUID',
    name            VARCHAR(128) NOT NULL COMMENT '规则名称',
    event_type      VARCHAR(64)  NOT NULL COMMENT '事件类型',
    severity_filter JSON         NULL COMMENT '严重程度过滤 JSON 数组',
    channel_ids     JSON         NOT NULL COMMENT '通道 ID 列表 JSON 数组',
    recipient_list  TEXT         NULL COMMENT '接收人列表（逗号分隔）',
    enabled         TINYINT      NOT NULL DEFAULT 1 COMMENT '是否启用',
    created_by      VARCHAR(64)  NOT NULL COMMENT '创建人',
    created_at      VARCHAR(40)  NOT NULL COMMENT '创建时间',
    updated_at      VARCHAR(40)  NOT NULL COMMENT '更新时间',
    PRIMARY KEY (id),
    INDEX idx_rule_event (event_type),
    INDEX idx_rule_enabled (enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='通知规则';

-- ============================================================
-- alert_events 新增 external_id 列（存储 Eventide 端告警 ID）
-- ============================================================
-- 注意：MySQL 不支持 ADD COLUMN IF NOT EXISTS，在 main.rs 中幂等处理

-- ============================================================
-- 通知引擎权限点（2 个）
-- ============================================================
INSERT INTO permissions (id, code, name, module, description, created_at) VALUES
('00000000-0000-0000-0000-000000000301', 'notification:read',  '查看通知配置', 'notification', '查看通知通道和规则', NOW()),
('00000000-0000-0000-0000-000000000302', 'notification:manage', '管理通知配置', 'notification', '新增/编辑/删除通知通道和规则', NOW())
ON DUPLICATE KEY UPDATE name = VALUES(name);

-- 给 admin 全部权限
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000301'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000302');

-- 给 operator 管理权限
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000301'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000302');

-- 给 viewer 只读权限
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000301');
