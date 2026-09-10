-- ============================================================
-- 2026-09-09: 告警组支持层级结构（parent_id 可选，兼容旧组无父）
-- 用途：短信策略「选择组」可选父组，命中后包含其全部子孙组成员。
-- 设计：
--   * parent_id 可空，历史告警组（迁移前创建）parent_id 为 NULL，保持兼容，不改动历史迁移。
--   * 不建外键，父子关系与循环防护在应用层校验，避免 MySQL 5.7 不支持递归 CTE 的限制。
-- ============================================================

ALTER TABLE alert_groups
    ADD COLUMN parent_id CHAR(36) NULL COMMENT '父组 id；NULL=顶级组（兼容旧组无父）' AFTER description;

ALTER TABLE alert_groups
    ADD INDEX idx_alert_group_parent (parent_id);
