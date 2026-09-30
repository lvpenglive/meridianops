-- 运维工具：第三方平台外链入口（单点登录后续再做）。

CREATE TABLE IF NOT EXISTS ops_tools (
    id            CHAR(36)      NOT NULL PRIMARY KEY,
    name          VARCHAR(128)  NOT NULL,
    url           VARCHAR(1024) NOT NULL,
    description   VARCHAR(512)  NOT NULL DEFAULT '',
    icon          VARCHAR(64)   NOT NULL DEFAULT 'Link',
    sort_order    INT           NOT NULL DEFAULT 0,
    enabled       TINYINT(1)    NOT NULL DEFAULT 1,
    created_at    DATETIME(3)   NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
    updated_at    DATETIME(3)   NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
    UNIQUE KEY uk_ops_tools_name (name),
    KEY idx_ops_tools_sort (enabled, sort_order, name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

SET @now = UTC_TIMESTAMP();

INSERT IGNORE INTO permissions (id, code, name, module, description, created_at) VALUES
('00000000-0000-0000-0000-000000000410', 'ops_tool:read', '查看运维工具', '运维工具', '查看并打开第三方运维平台外链', @now),
('00000000-0000-0000-0000-000000000411', 'ops_tool:manage', '管理运维工具', '运维工具', '增删改运维工具外链配置', @now);

INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000410'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000411'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000410'),
('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000410');

-- 示例条目：请在「运维工具 → 管理」里改成实际地址后再给业务用。
INSERT IGNORE INTO ops_tools (id, name, url, description, icon, sort_order, enabled) VALUES
('00000000-0000-0000-0000-000000000501', 'Zabbix', 'https://zabbix.example/', '传统资源监控（请改成实际地址）', 'Monitor', 10, 1),
('00000000-0000-0000-0000-000000000502', 'Grafana', 'https://grafana.example/', '指标可视化（请改成实际地址）', 'TrendCharts', 20, 1),
('00000000-0000-0000-0000-000000000503', '云监控', 'https://cloud.example/', '公有云/托管服务监控入口', 'Cloudy', 30, 1);
