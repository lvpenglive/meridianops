-- ============================================================
-- alert_events 表新增 clue_logs 字段
-- 用于存储告警-日志自动关联结果：告警前后 5 分钟内同主机 error 日志摘要
-- 由 ingress_eventide 创建告警后 tokio::spawn 异步任务查询 ClickHouse 写入
-- 前端告警详情抽屉「关联日志」Tab 直接展示
-- ============================================================

ALTER TABLE alert_events
    ADD COLUMN clue_logs JSON NULL COMMENT '关联日志摘要：告警前后5分钟内同主机error日志JSON快照' AFTER labels;
