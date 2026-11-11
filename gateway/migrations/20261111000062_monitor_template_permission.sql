-- 模板设计器权限，只授给管理员。生产实例的写入另走标准变更工单。

SET @now = UTC_TIMESTAMP();

INSERT IGNORE INTO permissions (id, code, name, module, description, created_at) VALUES
('00000000-0000-0000-0000-000000000405', 'monitor:template', '设计监控模板', '监控纳管', '新建和修改模板、模板组、主机组、监控项、触发器，以及导入导出', @now);

INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000405');

CREATE TABLE IF NOT EXISTS monitor_template_changes (
    id VARCHAR(36) NOT NULL PRIMARY KEY,
    instance_code VARCHAR(64) NOT NULL,
    ticket_id VARCHAR(36) NOT NULL,
    title VARCHAR(255) NOT NULL,
    http_method VARCHAR(8) NOT NULL,
    resource_path VARCHAR(255) NOT NULL,
    body MEDIUMTEXT NULL,
    snapshot MEDIUMTEXT NULL,
    status VARCHAR(16) NOT NULL,
    requested_by VARCHAR(64) NOT NULL,
    created_at DATETIME NOT NULL,
    applied_at DATETIME NULL,
    KEY idx_mtc_instance (instance_code, status)
);
