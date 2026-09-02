-- ============================================================
-- 2026-09-01: 通知规则事件类型改造为字典定义
-- 1. notification_rules 表新增 trigger_scene 字段（触发场景：alert_firing 等 6 个生命周期）
-- 2. notification_logs 表新增 trigger_scene 字段（日志记录触发场景）
-- 3. sys_dict_types / sys_dict_items 预置 event_type 字典类型与项
-- ============================================================

-- ---- notification_rules 新增触发场景字段 ----
ALTER TABLE notification_rules
    ADD COLUMN trigger_scene VARCHAR(64) NULL COMMENT '触发场景：alert_firing/alert_acknowledged/alert_resolved/ticket_assigned/ticket_closed/job_failed；空=匹配所有场景' AFTER event_type;

-- ---- notification_logs 新增触发场景字段（兼容历史日志，允许 NULL） ----
ALTER TABLE notification_logs
    ADD COLUMN trigger_scene VARCHAR(64) NULL COMMENT '触发场景（与 event_type 配合使用）' AFTER event_type;

-- ---- 预置 event_type 字典类型 ----
INSERT IGNORE INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('event_type', '事件类型', '通知规则匹配的事件分类，由字典统一维护', 1, 10, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ---- 预置 event_type 字典项 ----
INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'event_type', 'host',        '主机告警',   1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_type', 'software',    '软件告警',   1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_type', 'database',    '数据库告警', 1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_type', 'network',     '网络告警',   1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_type', 'middleware',  '中间件告警', 1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_type', 'storage',     '存储告警',   1, 6, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_type', 'security',    '安全告警',   1, 7, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_type', 'ticket',      '工单事件',   1, 8, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_type', 'job',         '作业事件',   1, 9, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ---- 把已有规则迁到新模型：原 event_type 值迁移到 trigger_scene，event_type 设为 NULL（匹配所有事件类型） ----
UPDATE notification_rules SET trigger_scene = event_type WHERE trigger_scene IS NULL AND event_type IS NOT NULL AND event_type <> '';
