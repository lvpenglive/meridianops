-- ============================================================
-- 2026-09-01: 通知规则 severity_filter 支持比较运算符
-- 新增 severity_op 字段，控制 severity_filter 的匹配方式
-- ============================================================

-- ---- notification_rules 新增 severity_op 字段 ----
ALTER TABLE notification_rules
    ADD COLUMN severity_op VARCHAR(8) NULL COMMENT '级别匹配方式：in(多选包含,默认)/gte/gt/lte/lt/eq/between；NULL 视为 in' AFTER severity_filter;

-- ---- 历史规则默认 op = 'in'（保持原多选包含语义） ----
UPDATE notification_rules SET severity_op = 'in' WHERE severity_op IS NULL;
