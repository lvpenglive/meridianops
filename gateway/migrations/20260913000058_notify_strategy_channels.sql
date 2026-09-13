-- 通知策略：一条策略可多选渠道（站内信 / 短信 / 邮件 / 飞书 / Webhook）
ALTER TABLE alert_sms_strategies
    ADD COLUMN channel_kinds JSON NULL
        COMMENT 'inbox/sms/email/feishu/webhook'
        AFTER notify_owner,
    ADD COLUMN extra_channel_ids JSON NULL
        COMMENT '飞书/Webhook 等 notification_channels.id'
        AFTER channel_kinds,
    ADD COLUMN trigger_scene VARCHAR(64) NULL
        COMMENT '触发场景；空=仅告警触发（兼容旧数据）'
        AFTER extra_channel_ids;

UPDATE alert_sms_strategies
SET channel_kinds = JSON_ARRAY('inbox', 'sms', 'email'),
    trigger_scene = 'alert_firing'
WHERE channel_kinds IS NULL;

UPDATE permissions SET module = '通知策略', name = '查看通知策略' WHERE code = 'sms_strategy:read';
UPDATE permissions SET module = '通知策略', name = '管理通知策略' WHERE code = 'sms_strategy:manage';
