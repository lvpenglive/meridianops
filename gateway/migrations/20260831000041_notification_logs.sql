-- 20260831000041: 通知发送日志
-- 每次通过外部通道（email/feishu/webhook）发送的结果都写一条记录，用于问题排查。

CREATE TABLE IF NOT EXISTS notification_logs (
    id              CHAR(36)     NOT NULL COMMENT '日志 UUID',
    rule_id         CHAR(36)     NULL COMMENT '匹配到的规则 ID（可为空：手动测试时）',
    rule_name       VARCHAR(128) NULL COMMENT '规则名快照（便于 rule 删除后仍能审计）',
    channel_id      CHAR(36)     NOT NULL COMMENT '通道 ID',
    channel_name    VARCHAR(128) NOT NULL COMMENT '通道名快照',
    channel_type    VARCHAR(32)  NOT NULL COMMENT '通道类型：email / feishu / webhook',
    event_type      VARCHAR(64)  NOT NULL COMMENT '触发事件类型：alert_firing 等',
    severity        VARCHAR(32)  NULL COMMENT '严重级别快照：disaster / critical / warning / info',
    recipients      TEXT         NULL COMMENT '实际接收人（逗号分隔/多邮箱）',
    title           VARCHAR(512) NOT NULL COMMENT '通知标题',
    content         MEDIUMTEXT   NULL COMMENT '通知正文摘要或完整内容',
    link            VARCHAR(512) NULL COMMENT '站内跳转链接',
    status          VARCHAR(16)  NOT NULL COMMENT '发送结果：success / failed',
    error_msg       TEXT         NULL COMMENT '失败时的错误详情（最多 4KB 截断）',
    response_snippet VARCHAR(512) NULL COMMENT '第三方响应摘要（HTTP 状态/邮件响应等，成功时可用）',
    duration_ms     INT UNSIGNED NULL COMMENT '发送耗时（毫秒）',
    triggered_by    VARCHAR(64)  NULL COMMENT '触发来源：rule / manual_test / 其它',
    sent_at         VARCHAR(40)  NOT NULL COMMENT '发送时间（RFC3339）',
    PRIMARY KEY (id),
    INDEX idx_log_sent_at (sent_at),
    INDEX idx_log_status (status),
    INDEX idx_log_event (event_type),
    INDEX idx_log_channel (channel_id),
    INDEX idx_log_rule (rule_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='通知发送日志（外部通道结果审计）';

-- 通知发送日志权限点（复用 notification:read 做前端菜单 gate，再加 1 个读权限点用于管理端）
INSERT INTO permissions (id, code, name, module, description, created_at) VALUES
('00000000-0000-0000-0000-000000000303', 'notification_log:read', '查看通知发送日志', 'notification', '查看通知引擎的每次发送结果和错误详情', NOW())
ON DUPLICATE KEY UPDATE name = VALUES(name);

INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000303'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000303'),
('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000303');
