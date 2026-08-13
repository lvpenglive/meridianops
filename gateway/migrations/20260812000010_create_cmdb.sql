-- ============================================================
-- CMDB 配置管理数据库：ci_models / ci_model_attrs / ci_instances / ci_relations
-- 动态模型设计：CI 模型可配置属性，实例属性值存 JSON
-- 支持 5 种核心 CI 类型：业务系统/主机/中间件/数据库/网络设备
-- ============================================================

-- ---- CI 模型表（定义资产类型）----
CREATE TABLE ci_models (
    id           CHAR(36)     NOT NULL,
    code         VARCHAR(64)  NOT NULL,          -- 模型编码：host/business_system/middleware/database/network_device
    name         VARCHAR(128) NOT NULL,          -- 显示名称：主机/业务系统/...
    icon         VARCHAR(64)  NOT NULL DEFAULT '', -- 图标名（Element Plus 图标）
    description  VARCHAR(255) NOT NULL DEFAULT '',
    enabled      TINYINT      NOT NULL DEFAULT 1,
    sort_order   INT          NOT NULL DEFAULT 0,
    created_at   VARCHAR(64)  NOT NULL,
    updated_at   VARCHAR(64)  NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uk_cimodel_code (code),
    KEY idx_cimodel_sort (sort_order)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- ---- 模型属性定义表（每种 CI 模型有哪些字段）----
CREATE TABLE ci_model_attrs (
    id           CHAR(36)     NOT NULL,
    model_id     CHAR(36)     NOT NULL,
    code         VARCHAR(64)  NOT NULL,          -- 属性编码：hostname/ip/os/cpu...
    name         VARCHAR(128) NOT NULL,          -- 属性名称：主机名/IP地址/操作系统...
    value_type   VARCHAR(16)  NOT NULL DEFAULT 'string', -- string/number/boolean/enum/date/json
    default_value VARCHAR(255) NOT NULL DEFAULT '',
    options      JSON         NULL,              -- 枚举选项（value_type=enum 时使用）
    is_required  TINYINT      NOT NULL DEFAULT 0,
    is_unique    TINYINT      NOT NULL DEFAULT 0,
    is_searchable TINYINT     NOT NULL DEFAULT 1,
    sort_order   INT          NOT NULL DEFAULT 0,
    created_at   VARCHAR(64)  NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uk_ciattr_model_code (model_id, code),
    KEY idx_ciattr_model (model_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- ---- CI 实例表（具体资产记录）----
CREATE TABLE ci_instances (
    id            CHAR(36)     NOT NULL,
    model_id      CHAR(36)     NOT NULL,
    name          VARCHAR(255) NOT NULL,         -- 实例名称/显示名
    status        VARCHAR(32)  NOT NULL DEFAULT 'running', -- running/stopped/maintenance/unknown
    department_id CHAR(36)     NULL,             -- 归属部门（外键 departments.id）
    owner_id      CHAR(36)     NULL,             -- 负责人（外键 users.id）
    attributes    JSON         NULL,             -- 动态属性值 { "hostname":"web-01", "ip":"10.0.0.1", ... }
    tags          VARCHAR(512) NOT NULL DEFAULT '', -- 标签（逗号分隔）
    created_at    VARCHAR(64)  NOT NULL,
    updated_at    VARCHAR(64)  NOT NULL,
    PRIMARY KEY (id),
    KEY idx_ciinst_model (model_id),
    KEY idx_ciinst_status (status),
    KEY idx_ciinst_dept (department_id),
    KEY idx_ciinst_owner (owner_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- ---- CI 关系表（资产间依赖关系）----
CREATE TABLE ci_relations (
    id            CHAR(36)     NOT NULL,
    source_id     CHAR(36)     NOT NULL,         -- 源 CI 实例
    target_id     CHAR(36)     NOT NULL,         -- 目标 CI 实例
    relation_type VARCHAR(32)  NOT NULL,         -- depends_on/contains/runs_on/manages/connects_to
    created_at    VARCHAR(64)  NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uk_cirel_pair_type (source_id, target_id, relation_type),
    KEY idx_cirel_source (source_id),
    KEY idx_cirel_target (target_id),
    KEY idx_cirel_type (relation_type)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- ============================================================
-- 种子数据：5 种 CI 模型 + 属性定义 + CMDB 权限点
-- ============================================================

-- ---- CMDB 权限点 ----
INSERT INTO permissions (id, code, name, module, description, created_at) VALUES
('00000000-0000-0000-0000-000000000101', 'asset:read',   '查看资产', 'asset', '查看 CMDB 资产列表和详情', NOW()),
('00000000-0000-0000-0000-000000000102', 'asset:create', '创建资产', 'asset', '新增 CMDB 资产实例', NOW()),
('00000000-0000-0000-0000-000000000103', 'asset:update', '修改资产', 'asset', '修改 CMDB 资产信息和属性', NOW()),
('00000000-0000-0000-0000-000000000104', 'asset:delete', '删除资产', 'asset', '删除 CMDB 资产实例', NOW())
ON DUPLICATE KEY UPDATE name = VALUES(name);

-- ---- 给 admin 角色（00000000-0000-0000-0000-000000000001）分配 asset 权限 ----
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000101'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000102'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000103'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000104');

-- ---- 给 operator 角色（00000000-0000-0000-0000-000000000002）分配 asset 读写权限 ----
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000101'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000102'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000103');

-- ---- 给 viewer 角色（00000000-0000-0000-0000-000000000003）分配 asset 只读权限 ----
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000101');

-- ============================================================
-- CI 模型种子：5 种核心类型
-- ============================================================
INSERT INTO ci_models (id, code, name, icon, description, enabled, sort_order, created_at, updated_at) VALUES
('cmdb-model-business',  'business_system', '业务系统', 'Coin',      '银行业务系统，如核心系统、网银、手机银行等', 1, 1, NOW(), NOW()),
('cmdb-model-host',      'host',            '主机',     'Monitor',   '物理机/虚拟机，运行中间件和数据库', 1, 2, NOW(), NOW()),
('cmdb-model-middleware', 'middleware',      '中间件',   'Cpu',       '应用中间件，如 Tomcat/WebLogic/Redis/MQ', 1, 3, NOW(), NOW()),
('cmdb-model-database',  'database',        '数据库',   'Coin',      '数据库实例，如 MySQL/Oracle/Redis', 1, 4, NOW(), NOW()),
('cmdb-model-network',   'network_device',  '网络设备', 'Connection', '交换机/路由器/防火墙/负载均衡', 1, 5, NOW(), NOW());

-- ============================================================
-- 模型属性种子
-- ============================================================

-- 业务系统属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-bs-code',   'cmdb-model-business', 'system_code', '系统编码',   'string', '', NULL, 1, 1, 1, 1, NOW()),
('attr-bs-level',  'cmdb-model-business', 'system_level','系统等级',   'enum',   '2', '["1","2","3","4"]', 1, 0, 1, 2, NOW()),
('attr-bs-desc',   'cmdb-model-business', 'description', '系统描述',   'string', '', NULL, 0, 0, 0, 3, NOW()),
('attr-bs-rto',    'cmdb-model-business', 'rto',         'RTO(分钟)',  'number', '60', NULL, 0, 0, 0, 4, NOW());

-- 主机属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-host-name', 'cmdb-model-host', 'hostname', '主机名',     'string', '', NULL, 1, 1, 1, 1, NOW()),
('attr-host-ip',   'cmdb-model-host', 'ip',       'IP地址',     'string', '', NULL, 1, 0, 1, 2, NOW()),
('attr-host-os',   'cmdb-model-host', 'os',       '操作系统',   'enum',   '', '["CentOS 7","CentOS 8","RHEL 7","RHEL 8","Ubuntu 20.04","Ubuntu 22.04","Windows Server 2019","AIX","Other"]', 1, 0, 1, 3, NOW()),
('attr-host-cpu',  'cmdb-model-host', 'cpu',      'CPU核数',    'number', '4', NULL, 0, 0, 0, 4, NOW()),
('attr-host-mem',  'cmdb-model-host', 'memory',   '内存(GB)',   'number', '8', NULL, 0, 0, 0, 5, NOW()),
('attr-host-disk', 'cmdb-model-host', 'disk',     '磁盘(GB)',   'number', '100', NULL, 0, 0, 0, 6, NOW()),
('attr-host-room', 'cmdb-model-host', 'datacenter','机房位置',   'string', '', NULL, 0, 0, 1, 7, NOW());

-- 中间件属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-mw-type',    'cmdb-model-middleware', 'mw_type',    '中间件类型', 'enum', '', '["Tomcat","WebLogic","Nginx","Redis","Kafka","RabbitMQ","ActiveMQ","Other"]', 1, 0, 1, 1, NOW()),
('attr-mw-version', 'cmdb-model-middleware', 'version',    '版本',       'string', '', NULL, 1, 0, 1, 2, NOW()),
('attr-mw-port',    'cmdb-model-middleware', 'port',       '端口',       'number', '8080', NULL, 0, 0, 0, 3, NOW()),
('attr-mw-path',    'cmdb-model-middleware', 'install_path','安装路径',  'string', '', NULL, 0, 0, 0, 4, NOW());

-- 数据库属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-db-type',    'cmdb-model-database', 'db_type',    '数据库类型', 'enum', '', '["MySQL","Oracle","PostgreSQL","Redis","MongoDB","DB2","Other"]', 1, 0, 1, 1, NOW()),
('attr-db-version', 'cmdb-model-database', 'version',    '版本',       'string', '', NULL, 1, 0, 1, 2, NOW()),
('attr-db-instance','cmdb-model-database', 'instance',   '实例名',     'string', '', NULL, 1, 1, 1, 3, NOW()),
('attr-db-port',    'cmdb-model-database', 'port',       '端口',       'number', '3306', NULL, 0, 0, 0, 4, NOW()),
('attr-db-charset', 'cmdb-model-database', 'charset',    '字符集',     'string', 'UTF8MB4', NULL, 0, 0, 0, 5, NOW());

-- 网络设备属性
INSERT INTO ci_model_attrs (id, model_id, code, name, value_type, default_value, options, is_required, is_unique, is_searchable, sort_order, created_at) VALUES
('attr-nd-name',   'cmdb-model-network', 'device_name', '设备名',   'string', '', NULL, 1, 1, 1, 1, NOW()),
('attr-nd-ip',     'cmdb-model-network', 'mgmt_ip',     '管理IP',   'string', '', NULL, 1, 0, 1, 2, NOW()),
('attr-nd-vendor', 'cmdb-model-network', 'vendor',      '厂商',     'enum',   '', '["Cisco","Huawei","H3C","Juniper","F5","Other"]', 0, 0, 1, 3, NOW()),
('attr-nd-model',  'cmdb-model-network', 'device_model','设备型号', 'string', '', NULL, 0, 0, 0, 4, NOW()),
('attr-nd-ports',  'cmdb-model-network', 'port_count',  '端口数',   'number', '48', NULL, 0, 0, 0, 5, NOW());
