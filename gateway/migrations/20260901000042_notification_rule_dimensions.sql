-- 20260901000042: notification_rules 增加 host_filter / name_keyword 两个匹配维度
--
-- 为了对齐老系统"短信策略"的 4 维匹配能力（eventType / eventLevel / eventIp / eventName），
-- 在现有 event_type + severity_filter 基础上补全 host / name 两个可选匹配条件。
-- 字段留空 = 不筛选，保证向后兼容已有规则。

ALTER TABLE notification_rules
    ADD COLUMN host_filter  VARCHAR(255) NULL COMMENT '设备/IP 过滤：逗号分隔多值；支持 % 通配（如 10.0.5.%）；空=不筛选' AFTER severity_filter,
    ADD COLUMN name_keyword VARCHAR(128) NULL COMMENT '事件名关键字：title 包含此词才命中，空=不筛选（大小写不敏感）' AFTER host_filter;
