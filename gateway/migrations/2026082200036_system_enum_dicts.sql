-- ============================================================
-- 2026-08-22: 全局系统枚举字典化
-- 将审计、告警、CMDB、同步、作业等模块的硬编码枚举
-- 迁移到字典管理，支持运行时动态调整
-- ============================================================

-- ── 1. 审计操作类型 audit_action ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('audit_action', '审计操作类型', '审计日志中的操作动作枚举', 1, 20, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'audit_action', 'login',          '登录',     1, 1,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'audit_action', 'logout',          '登出',     1, 2,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'audit_action', 'create',          '创建',     1, 3,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'audit_action', 'update',          '更新',     1, 4,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'audit_action', 'enable',          '启用',     1, 5,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'audit_action', 'disable',         '禁用',     1, 6,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'audit_action', 'reset_password',  '重置密码', 1, 7,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'audit_action', 'delete',          '删除',     1, 8,  UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 2. 审计结果 audit_result ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('audit_result', '审计结果', '审计操作的结果状态', 1, 21, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'audit_result', 'success', '成功', 1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'audit_result', 'failure', '失败', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 3. 告警严重度 alert_severity ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('alert_severity', '告警严重度', '告警事件的严重程度枚举', 1, 22, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'alert_severity', 'critical', '严重', 1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'warning',  '警告', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'info',     '信息', 1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'resolved', '恢复', 1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 4. 告警状态 alert_status ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('alert_status', '告警状态', '告警事件的生命周期状态', 1, 23, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'alert_status', 'firing',       '活动',   1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_status', 'acknowledged', '处理中', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_status', 'resolved',     '已解决', 1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 5. 告警来源 alert_source ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('alert_source', '告警来源', '告警事件的来源系统', 1, 24, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'alert_source', 'eventide',   'Eventide',   1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_source', 'zabbix',     'Zabbix',     1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_source', 'axleops',    'AxleOps',    1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_source', 'prometheus', 'Prometheus', 1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_source', 'elk',        'ELK',        1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 6. 同步操作类型 sync_action ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('sync_action', '同步操作类型', 'CMDB数据同步的操作方式', 1, 25, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'sync_action', 'webhook', '推送', 1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sync_action', 'pull',   '拉取', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sync_action', 'upsert', '更新', 1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 7. 同步结果状态 sync_status ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('sync_status', '同步结果状态', '数据同步操作的执行结果', 1, 26, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'sync_status', 'success', '成功',   1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sync_status', 'partial', '部分成功', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sync_status', 'failed',  '失败',   1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sync_status', 'skipped', '跳过',   1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 8. CMDB操作类型 cmdb_action ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('cmdb_action', 'CMDB操作类型', 'CMDB配置项操作审计动作', 1, 27, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'cmdb_action', 'create_ci',          '创建',     1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_action', 'update_ci',          '更新',     1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_action', 'delete_ci',          '删除',     1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_action', 'sync_ci',             '同步',     1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_action', 'pull_ci',             '拉取',     1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_action', 'create_ci_relation',  '建立关系', 1, 6, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_action', 'delete_ci_relation',  '删除关系', 1, 7, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 9. 作业执行状态 job_run_status ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('job_run_status', '作业执行状态', '脚本/作业执行的结果状态', 1, 28, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'job_run_status', 'running', '执行中',   1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'job_run_status', 'success', '成功',     1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'job_run_status', 'failed',  '失败',     1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'job_run_status', 'partial', '部分成功', 1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'job_run_status', 'timeout', '超时',     1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'job_run_status', 'pending', '等待',     1, 6, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 10. 代理/系统状态 agent_status ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('agent_status', '代理状态', 'Agent/系统在线状态', 1, 29, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'agent_status', 'online',  '在线', 1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'agent_status', 'offline', '离线', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 11. 敏感操作类型 sensitive_action ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('sensitive_action', '敏感操作类型', '敏感操作审计的动作类型', 1, 30, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'sensitive_action', 'delete_user',       '删除用户',   1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sensitive_action', 'disable_user',      '禁用用户',   1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sensitive_action', 'reset_password',    '重置密码',   1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sensitive_action', 'create_role',       '创建角色',   1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sensitive_action', 'update_role',        '更新角色',   1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sensitive_action', 'delete_role',        '删除角色',   1, 6, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sensitive_action', 'assign_permission',  '权限分配',   1, 7, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sensitive_action', 'update_settings',   '系统设置变更', 1, 8, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sensitive_action', 'change_password',   '密码修改',   1, 9, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 12. 同步来源类型 sync_source_type ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('sync_source_type', '同步来源类型', 'CMDB外部数据来源的类型', 1, 31, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'sync_source_type', 'blueking',     '蓝鲸',     1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'sync_source_type', 'external_cmdb', '外部CMDB', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ── 13. CMDB模型图标 cmdb_model_icon ──
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('cmdb_model_icon', 'CMDB模型图标', 'CMDB模型可用的图标名称列表', 1, 32, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), enabled = VALUES(enabled), updated_at = UTC_TIMESTAMP();

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'cmdb_model_icon', 'Monitor',    'Monitor',    1, 1,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Cpu',        'Cpu',        1, 2,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Coin',       'Coin',       1, 3,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Connection', 'Connection', 1, 4,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Grid',       'Grid',       1, 5,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Box',        'Box',        1, 6,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Files',      'Files',      1, 7,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'List',       'List',       1, 8,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Service',    'Service',    1, 9,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Platform',   'Platform',   1, 10, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Document',   'Document',   1, 11, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Folder',     'Folder',     1, 12, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Setting',    'Setting',    1, 13, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'cmdb_model_icon', 'Cloudy',     'Cloudy',     1, 14, UTC_TIMESTAMP(), UTC_TIMESTAMP());
