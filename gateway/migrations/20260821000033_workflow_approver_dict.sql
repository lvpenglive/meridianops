-- ============================================================
-- 2026-08-21: 流程审批人选择器字典
-- 工作流节点审批人选项，通过字典配置，支持动态增减
-- ============================================================

-- 字典类型
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('workflow_approver_selector', '流程审批人选择器', '工作流审批节点的审批人选择方式', 1, 10, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

-- 字典项（使用 INSERT IGNORE 避免重复键冲突）
INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'workflow_approver_selector', 'assignee',                        '指派给当前处理人',         1, 1,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_approver_selector', 'department_head_of_reporter',      '上报人部门主管',           1, 2,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_approver_selector', 'team_leader_of_assignee',          '处理人所在部门组长',       1, 3,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_approver_selector', 'role:team_leader',                 '角色：team_leader',        1, 4,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_approver_selector', 'role:operator',                   '角色：operator',           1, 5,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_approver_selector', 'role:admin',                      '角色：admin',              1, 6,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_approver_selector', 'incident_manager',               '事件经理',                 1, 7,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_approver_selector', 'problem_manager',                '问题经理',                 1, 8,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_approver_selector', 'cab_member',                     'CAB 成员',                 1, 9,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_approver_selector', 'senior_engineer_group',           '高级工程师组',             1, 10, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'workflow_approver_selector', 'vp_oncall',                      'VP OnCall',                1, 11, UTC_TIMESTAMP(), UTC_TIMESTAMP());
