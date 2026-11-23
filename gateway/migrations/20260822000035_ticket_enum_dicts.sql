-- ============================================================
-- 2026-08-22: 工单/流程系统枚举字典化
-- 将工单状态、优先级、分类、节点类型、节点状态、操作动作、审批决策
-- 从代码硬编码迁移到字典管理，支持运行时动态调整
-- ============================================================

-- ── 1. 工单状态 ticket_status ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('ticket_status', '工单状态', '工单生命周期状态枚举', 1, 12, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'ticket_status', 'open',           '待处理',   1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_status', 'assigned',        '已指派',   1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_status', 'in_progress',     '处理中',   1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_status', 'pending_review',  '待审核',   1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_status', 'resolved',        '已解决',   1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_status', 'closed',          '已关闭',   1, 6, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_status', 'cancelled',       '已取消',   1, 7, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 2. 工单优先级 ticket_priority ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('ticket_priority', '工单优先级', '工单优先级 P1-P4，item_value 为数字字符串', 1, 13, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'ticket_priority', '1', 'P1-紧急', 1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_priority', '2', 'P2-高',   1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_priority', '3', 'P3-中',   1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_priority', '4', 'P4-低',   1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 3. 工单分类 ticket_category ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('ticket_category', '工单分类', '工单分类标签，用于筛选和统计', 1, 14, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'ticket_category', 'database',   '数据库', 1, 1,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_category', 'network',    '网络',   1, 2,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_category', 'security',   '安全',   1, 3,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_category', 'host',       '主机',   1, 4,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_category', 'app',        '应用',   1, 5,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_category', 'storage',    '存储',   1, 6,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_category', 'middleware', '中间件', 1, 7,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_category', 'office',      '办公网', 1, 8,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'ticket_category', 'monitor',     '监控',   1, 9,  UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 4. 流程节点类型 workflow_node_kind ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('workflow_node_kind', '流程节点类型', '流程编排中的节点类型枚举', 1, 15, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'workflow_node_kind', 'start',             '开始',     1, 1,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_kind', 'end',               '结束',     1, 2,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_kind', 'auto_pass',         '自动流转', 1, 3,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_kind', 'single_approval',   '单人审批', 1, 4,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_kind', 'all_approval',      '会签审批', 1, 5,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_kind', 'any_approval',      '或签审批', 1, 6,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_kind', 'countersign',       '加签',     1, 7,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_kind', 'condition_gateway', '条件网关', 1, 8,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_kind', 'parallel_split',    '并行分支', 1, 9,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_kind', 'parallel_join',     '并行汇聚', 1, 10, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 5. 流程节点状态 workflow_node_status ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('workflow_node_status', '流程节点状态', '流程实例节点的运行时状态', 1, 16, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'workflow_node_status', 'pending',  '待处理', 1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_status', 'active',   '进行中', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_status', 'done',     '已完成', 1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_status', 'rejected', '已驳回', 1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_node_status', 'skipped',  '已跳过', 1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 6. 流程操作动作 workflow_action ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('workflow_action', '流程操作动作', '工单操作/评论的动作类型', 1, 17, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'workflow_action', 'create',       '创建',   1, 1,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_action', 'comment',      '评论',   1, 2,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_action', 'assign',       '指派',   1, 3,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_action', 'approve',      '通过',   1, 4,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_action', 'reject',       '驳回',   1, 5,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_action', 'reassign',     '改派',   1, 6,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_action', 'close',        '关闭',   1, 7,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_action', 'cancel',       '取消',   1, 8,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_action', 'link_alert',   '关联告警', 1, 9,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_action', 'unlink_alert', '解除告警', 1, 10, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 7. 流程审批决策 workflow_decision ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('workflow_decision', '流程审批决策', '审批节点的决策结果', 1, 18, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'workflow_decision', 'approve', '通过', 1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_decision', 'reject',  '驳回', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_decision', 'skip',    '跳过', 1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP());
