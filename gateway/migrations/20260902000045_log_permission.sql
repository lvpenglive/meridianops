-- ============================================================
-- 新增 log:read 权限点，用于日志中心页面访问控制
-- 路由 /logs 加 meta.permission = 'log:read'
-- 沿用种子 UUID 序号：20000000-0000-0000-0000-000000000019
-- admin / operator / viewer 三个内置角色均授予该权限（日志为只读，安全）
-- ============================================================

SET @now = UTC_TIMESTAMP();

-- 新增权限点
INSERT IGNORE INTO permissions (id, code, name, module, description, created_at) VALUES
('20000000-0000-0000-0000-000000000019', 'log:read', '查看日志', '日志中心', '查询 ClickHouse/Loki 中的日志', @now);

-- 授予 admin
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000019');

-- 授予 operator
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000019');

-- 授予 viewer（日志只读，安全）
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000019');
