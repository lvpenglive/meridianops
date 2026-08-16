-- ============================================================
-- 2026-08-15: 字典管理（通用枚举值配置）
-- 支持知识库分类、告警级别等动态配置，无需改代码即可增减选项
-- ============================================================

-- ---- 字典管理权限点（4 个）----
INSERT IGNORE INTO permissions (id, code, name, module, description, created_at) VALUES
('20000000-0000-0000-0000-000000000023', 'dict:read',   '查看字典', '字典管理', '查看字典类型和字典项',   UTC_TIMESTAMP()),
('20000000-0000-0000-0000-000000000024', 'dict:create', '创建字典', '字典管理', '新建字典类型或字典项',   UTC_TIMESTAMP()),
('20000000-0000-0000-0000-000000000025', 'dict:update', '编辑字典', '字典管理', '修改字典类型或字典项',   UTC_TIMESTAMP()),
('20000000-0000-0000-0000-000000000026', 'dict:delete', '删除字典', '字典管理', '删除字典类型或字典项',   UTC_TIMESTAMP());

-- admin 角色分配字典全部权限
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000023'),
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000024'),
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000025'),
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000026');

-- operator 角色：字典只读
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000023');

-- viewer 角色：字典只读
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000023');

-- ---- 字典类型表 ----
CREATE TABLE IF NOT EXISTS sys_dict_types (
    code         VARCHAR(64)   NOT NULL COMMENT '类型编码（如 knowledge_category）',
    name         VARCHAR(128)  NOT NULL COMMENT '显示名称（如 知识库分类）',
    description  VARCHAR(256)  NULL     COMMENT '描述',
    enabled      TINYINT(1)    NOT NULL DEFAULT 1 COMMENT '1=启用 0=禁用',
    sort_order   INT           NOT NULL DEFAULT 0 COMMENT '排序',
    created_at   DATETIME(3)   NOT NULL,
    updated_at   DATETIME(3)   NOT NULL,
    PRIMARY KEY (code)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='字典类型';

-- ---- 字典项表 ----
CREATE TABLE IF NOT EXISTS sys_dict_items (
    id           CHAR(36)      NOT NULL COMMENT '字典项 ID (UUID)',
    type_code    VARCHAR(64)   NOT NULL COMMENT '所属类型编码 → sys_dict_types.code',
    item_value   VARCHAR(128)  NOT NULL COMMENT '存储值（如 database）',
    item_label   VARCHAR(128)  NOT NULL COMMENT '显示文本（如 数据库）',
    enabled      TINYINT(1)    NOT NULL DEFAULT 1 COMMENT '1=启用 0=禁用',
    sort_order   INT           NOT NULL DEFAULT 0 COMMENT '排序',
    created_at   DATETIME(3)   NOT NULL,
    updated_at   DATETIME(3)   NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uk_type_value (type_code, item_value),
    INDEX idx_type_sort (type_code, sort_order)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='字典项';

-- ---- 种子数据：知识库分类 ----
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('knowledge_category', '知识库分类', '知识条目的分类选项', 1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP());

INSERT INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'knowledge_category', 'database',   '数据库',     1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'knowledge_category', 'linux',      'Linux/系统', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'knowledge_category', 'network',    '网络',       1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'knowledge_category', 'middleware', '中间件',     1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'knowledge_category', 'general',    '通用',       1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP());
