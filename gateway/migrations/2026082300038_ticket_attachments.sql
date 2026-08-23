-- 工单附件表
CREATE TABLE IF NOT EXISTS ticket_attachments (
    id          VARCHAR(36)  PRIMARY KEY,
    ticket_id   VARCHAR(36)  NOT NULL,
    filename    VARCHAR(255) NOT NULL COMMENT '原始文件名',
    file_size   BIGINT       NOT NULL DEFAULT 0 COMMENT '文件大小(字节)',
    file_type   VARCHAR(64)  DEFAULT NULL COMMENT 'MIME 类型',
    storage_key VARCHAR(512) NOT NULL COMMENT '存储路径/key',
    uploader_id VARCHAR(64)  NOT NULL,
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at  DATETIME     NULL,
    INDEX idx_attach_ticket (ticket_id),
    INDEX idx_attach_uploader (uploader_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='工单附件';

-- 工单关注表
CREATE TABLE IF NOT EXISTS ticket_watchers (
    id          VARCHAR(36)  PRIMARY KEY,
    ticket_id   VARCHAR(36)  NOT NULL,
    user_id     VARCHAR(64)  NOT NULL,
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY uk_ticket_watcher (ticket_id, user_id),
    INDEX idx_watcher_user (user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='工单关注人';

-- 用户通知设置表
CREATE TABLE IF NOT EXISTS user_notification_settings (
    user_id      VARCHAR(64) PRIMARY KEY,
    settings_json JSON       NOT NULL COMMENT '各类型通知开关',
    updated_at   DATETIME    NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='用户通知设置';

-- 工单动态字段表（支持模板自定义表单）
CREATE TABLE IF NOT EXISTS ticket_custom_fields (
    id          VARCHAR(36)  PRIMARY KEY,
    ticket_id   VARCHAR(36)  NOT NULL,
    field_key   VARCHAR(64)  NOT NULL COMMENT '字段key',
    field_label VARCHAR(128) NOT NULL COMMENT '字段显示名',
    field_type  VARCHAR(32)  NOT NULL DEFAULT 'text' COMMENT 'text/textarea/select/date/number',
    field_value TEXT,
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_custom_ticket (ticket_id),
    UNIQUE KEY uk_ticket_field (ticket_id, field_key)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='工单自定义字段';
