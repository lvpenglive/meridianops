-- 侧栏缺权限菜单补齐：态势中心 / 报表中心 / 告警大屏 / AIOps
-- 同时把已有权限点绑到对应菜单（告警中心、作业中心、通知发送日志由前端改绑）
-- UUID 段：00000000-…-412 ~ 415（410/411 已被 ops_tool 占用）

SET @now = UTC_TIMESTAMP();

INSERT IGNORE INTO permissions (id, code, name, module, description, created_at) VALUES
('00000000-0000-0000-0000-000000000412', 'overview:read',      '查看态势中心', '态势中心', '查看运营态势总览', @now),
('00000000-0000-0000-0000-000000000413', 'report:read',        '查看报表中心', '报表中心', '查看运营分析报表', @now),
('00000000-0000-0000-0000-000000000414', 'alert_screen:read',  '查看告警大屏', '告警大屏', '打开告警全屏大屏', @now),
('00000000-0000-0000-0000-000000000415', 'aiops:read',         '查看 AIOps',   'AIOps运维', '使用 AIOps 分析能力', @now);

-- admin：全部新权限
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000412'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000413'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000414'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000415');

-- operator / viewer：只读类一并授予
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000412'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000413'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000414'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000415'),
('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000412'),
('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000413'),
('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000414'),
('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000415');

-- 自定义角色兼容：态势中心原先人人可见，给已有角色都补上
INSERT IGNORE INTO role_permissions (role_id, permission_id)
SELECT id, '00000000-0000-0000-0000-000000000412' FROM roles;

-- 原先靠 audit:read 看报表的，补 report:read
INSERT IGNORE INTO role_permissions (role_id, permission_id)
SELECT rp.role_id, '00000000-0000-0000-0000-000000000413'
FROM role_permissions rp
WHERE rp.permission_id = '20000000-0000-0000-0000-000000000016';

-- 原先靠 alert:read 看大屏 / AIOps 的，补对应权限
INSERT IGNORE INTO role_permissions (role_id, permission_id)
SELECT rp.role_id, '00000000-0000-0000-0000-000000000414'
FROM role_permissions rp
WHERE rp.permission_id = '00000000-0000-0000-0000-000000000201';

INSERT IGNORE INTO role_permissions (role_id, permission_id)
SELECT rp.role_id, '00000000-0000-0000-0000-000000000415'
FROM role_permissions rp
WHERE rp.permission_id = '00000000-0000-0000-0000-000000000201';

-- 通知发送日志菜单改为 notification_log:read：有通知通道读权限的一并补上
INSERT IGNORE INTO role_permissions (role_id, permission_id)
SELECT rp.role_id, '00000000-0000-0000-0000-000000000303'
FROM role_permissions rp
WHERE rp.permission_id = '00000000-0000-0000-0000-000000000301';
