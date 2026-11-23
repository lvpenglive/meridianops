-- 站内消息通知表
CREATE TABLE IF NOT EXISTS notifications (
    id          VARCHAR(36)  PRIMARY KEY,
    user_id     VARCHAR(64)  NOT NULL COMMENT '接收人 user_id',
    type        VARCHAR(32)  NOT NULL COMMENT 'ticket_assigned|ticket_approved|ticket_rejected|ticket_closed|ticket_created|ticket_commented',
    title       VARCHAR(255) NOT NULL,
    content     TEXT,
    link        VARCHAR(512) COMMENT '点击跳转路径',
    is_read     BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    read_at     DATETIME     NULL,
    INDEX idx_notif_user_read (user_id, is_read),
    INDEX idx_notif_user_created (user_id, created_at DESC)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='站内通知';
