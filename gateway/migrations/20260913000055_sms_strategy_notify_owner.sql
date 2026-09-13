-- 短信策略：命中后可同时通知告警关联资产的责任人（列表「联系人」）
-- 默认关闭，已有策略行为不变。

ALTER TABLE alert_sms_strategies
    ADD COLUMN notify_owner TINYINT(1) NOT NULL DEFAULT 0
        COMMENT '1=命中后同时短信通知关联资产责任人'
        AFTER recipient_user_ids;
