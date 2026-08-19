-- 20260818000031: alert_events 接入渠道 / 接入者字段
-- 注意：ALTER TABLE 已在 main.rs 的 Rust 代码中幂等处理（MySQL 不支持 ADD COLUMN IF NOT EXISTS）
-- 本迁移仅负责字典种子数据。

-- 字典：新增 sys_dict alert_ingress_channel
-- sys_dict_types 主键是 code 列
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at)
VALUES ('alert_ingress_channel', '告警接入渠道', '告警事件进入系统的渠道', 1, 0, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), updated_at = VALUES(updated_at);

INSERT INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'alert_ingress_channel', 'webhook',   'Webhook 推送',  1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_ingress_channel', 'manual',    '人工上报',      1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_ingress_channel', 'job',       '作业执行',      1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_ingress_channel', 'api_token', 'API 令牌',      1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_ingress_channel', 'system',    '系统内置',      1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE item_label = VALUES(item_label), updated_at = VALUES(updated_at);
