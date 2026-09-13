-- 权限分组/名称与侧栏菜单对齐
-- 分组按菜单项命名；历史英文 module（asset/alert/ticket 等）一并纠正

-- ---- 后台管理 ----
UPDATE permissions SET module = '系统设置', name = '查看系统设置' WHERE code = 'system:read';
UPDATE permissions SET module = '系统设置', name = '修改系统设置' WHERE code = 'system:update';
UPDATE permissions SET name = '查看用户管理' WHERE code = 'user:read';
UPDATE permissions SET name = '启用/禁用用户' WHERE code = 'user:toggle_enable';
UPDATE permissions SET name = '查看角色管理' WHERE code = 'role:read';
UPDATE permissions SET name = '查看部门管理' WHERE code = 'dept:read';
UPDATE permissions SET name = '查看字典管理' WHERE code = 'dict:read';
UPDATE permissions SET name = '查看审计中心' WHERE code = 'audit:read';

-- ---- 资产管理 ----
UPDATE permissions SET module = '资产管理', name = '查看资产管理' WHERE code = 'asset:read';
UPDATE permissions SET module = '资产管理' WHERE code IN ('asset:create', 'asset:update', 'asset:delete');

-- ---- 监控告警 ----
UPDATE permissions SET module = '告警中心', name = '查看告警中心' WHERE code = 'alert:read';
UPDATE permissions SET module = '告警中心' WHERE code IN ('alert:create', 'alert:update', 'alert:delete');

UPDATE permissions SET module = '通知通道', name = '查看通知通道' WHERE code = 'notification:read';
UPDATE permissions SET module = '通知通道', name = '管理通知通道' WHERE code = 'notification:manage';

UPDATE permissions SET module = '通知发送日志', name = '查看通知发送日志' WHERE code = 'notification_log:read';

UPDATE permissions SET module = '告警短信策略', name = '查看告警短信策略' WHERE code = 'sms_strategy:read';
UPDATE permissions SET module = '告警短信策略', name = '管理告警短信策略' WHERE code = 'sms_strategy:manage';

UPDATE permissions SET module = '告警组维护', name = '查看告警组维护' WHERE code = 'alert_group:read';
UPDATE permissions SET module = '告警组维护', name = '管理告警组维护' WHERE code = 'alert_group:manage';

UPDATE permissions SET name = '查看日志中心' WHERE code = 'log:read';

-- ---- 运维流程 ----
UPDATE permissions SET name = '查看作业中心' WHERE code = 'job:read';
UPDATE permissions SET name = '创建作业' WHERE code = 'job:create';
UPDATE permissions SET name = '管理作业中心' WHERE code = 'job:admin';

UPDATE permissions SET module = 'SSH 凭据', name = '查看 SSH 凭据' WHERE code = 'credential:read';
UPDATE permissions SET module = 'SSH 凭据', name = '创建 SSH 凭据' WHERE code = 'credential:create';
UPDATE permissions SET module = 'SSH 凭据', name = '删除 SSH 凭据' WHERE code = 'credential:delete';

UPDATE permissions SET module = '工单系统', name = '查看工单系统' WHERE code = 'ticket:read';
UPDATE permissions SET module = '工单系统', name = '创建工单' WHERE code = 'ticket:create';
UPDATE permissions SET module = '工单系统', name = '更新工单' WHERE code = 'ticket:update';
UPDATE permissions SET module = '工单系统', name = '删除工单' WHERE code = 'ticket:delete';

UPDATE permissions SET module = '流程模板', name = '查看流程模板' WHERE code = 'workflow:read';
UPDATE permissions SET module = '流程模板', name = '管理流程模板' WHERE code = 'workflow:admin';
