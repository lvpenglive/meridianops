-- ============================================================
-- 2026-08-21: 流程工单类型字典
-- 工作流模板适用的工单类型，通过字典配置，支持动态增减
-- ============================================================

INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('workflow_ticket_type', '流程工单类型', '工作流模板适用的工单类型', 1, 11, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

-- 使用 INSERT IGNORE 避免重复键冲突
INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'workflow_ticket_type', 'incident',          '事件工单',         1, 1,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_ticket_type', 'problem',           '故障工单',         1, 2,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_ticket_type', 'change_normal',     '标准变更',         1, 3,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_ticket_type', 'change_emergency',  '紧急变更',         1, 4,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_ticket_type', 'task_simple',       '运维任务',         1, 5,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_ticket_type', 'task',              '任务',             1, 6,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_ticket_type', 'change',            '变更',             1, 7,  UTC_TIMESTAMP(), UTC_TIMESTAMP());
