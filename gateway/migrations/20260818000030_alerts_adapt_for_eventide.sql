-- 20260818000030: 告警中心适配 Eventide webhook 接入
-- 1) alert_events 表新增 ends_at 列（接收 Eventide 的 resolved 时的结束时间）
-- 2) 清空之前预置的假样例告警（来源应该是 Eventide 推送，不是手工编造）
-- 3) 字典 alert_severity 替换为 Eventide 实际使用的 Zabbix 风格枚举
-- 4) 字典 alert_source 增加 ingress 子类型
-- 5) 字典 alert_status 增加 pending 值

-- 1) alert_events 增加 ends_at 列
ALTER TABLE alert_events
    ADD COLUMN ends_at VARCHAR(40) NULL COMMENT '告警结束时间 RFC3339（resolved 时由 Eventide 推送）' AFTER fired_at;

-- 2) 清空之前预置的假样例告警（保留表结构，等真实 Eventide 推送）
DELETE FROM alert_events;

-- 3) 字典 alert_severity：删除旧的 P0/P1/P2/P3/info，换成 Eventide 实际使用的 Zabbix 风格
DELETE FROM sys_dict_items WHERE type_code = 'alert_severity';
INSERT INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'alert_severity', 'disaster',     '灾难 Disaster',     1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'critical',     '严重 Critical',     1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'high',         '重要 High',         1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'average',      '一般 Average',      1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'warning',      '警告 Warning',      1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'information',  '提示 Information',  1, 6, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'info',         '提示 Info',         1, 7, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE item_label = VALUES(item_label);

-- 4) 字典 alert_source：增加 ingress 子类型
INSERT INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'alert_source', 'snmptrap',  'SNMP Trap',  1, 6, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_source', 'kafka',     'Kafka 接入', 1, 7, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_source', 'eventide',  'Eventide 推送', 1, 8, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE item_label = VALUES(item_label);

-- 5) 字典 alert_status：增加 pending（Alertmanager 风格的待评估状态）
INSERT INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'alert_status', 'pending', '待评估', 1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE item_label = VALUES(item_label);
