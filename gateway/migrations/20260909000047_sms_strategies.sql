-- ============================================================
-- 2026-09-09: 告警短信策略（对齐老系统「短信策略」）
-- 独立于 notification_rules 的策略表：按「事件类型 + 触发级别 + 事件级别
-- + 设备IP + 事件名称」匹配告警，命中后通知「选择组/已选人员」。
-- 通知方式复用现有 dispatch_event 的站内信 + 邮件/飞书/webhook 通道能力。
-- ============================================================

-- ---- 1. 短信策略表 ----
CREATE TABLE IF NOT EXISTS alert_sms_strategies (
    id                  CHAR(36)      NOT NULL COMMENT '策略 UUID',
    event_type          VARCHAR(64)   NULL COMMENT '一级事件类型（字典 event_type value，如 network/host/database/middleware）；空=全部',
    event_sub_type      VARCHAR(64)   NULL COMMENT '二级事件子类（字典 event_sub_type value，如 db2/oracle/tomcat）；空=全部',
    trigger_op          VARCHAR(8)    NOT NULL DEFAULT 'eq' COMMENT '触发级别运算符：eq(等于,默认)/gte(大于等于)/gt/lte/lt',
    severity_filter     VARCHAR(16)   NULL COMMENT '事件级别：info/1~5；空=全部级别',
    host_filter         VARCHAR(255)  NULL COMMENT '设备IP过滤：逗号分隔多值，支持 % 通配；空=全部设备',
    name_keyword        VARCHAR(128)  NULL COMMENT '事件名称关键字：标题包含此词才命中（大小写不敏感）；空=全部',
    department_id       CHAR(36)      NULL COMMENT '选择组：关联 departments.id；空=不限组',
    recipient_user_ids  JSON          NOT NULL COMMENT '已选人员 user_id 列表 JSON 数组',
    description         VARCHAR(512)  NULL COMMENT '描述',
    enabled             TINYINT(1)    NOT NULL DEFAULT 1 COMMENT '1=启用 0=停用',
    created_by          VARCHAR(64)   NOT NULL COMMENT '创建人',
    created_at          VARCHAR(40)   NOT NULL COMMENT '创建时间',
    updated_at          VARCHAR(40)   NOT NULL COMMENT '更新时间',
    PRIMARY KEY (id),
    INDEX idx_sms_event (event_type),
    INDEX idx_sms_dept (department_id),
    INDEX idx_sms_enabled (enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='告警短信策略';

-- ---- 2. 权限点：短信策略 ----
-- 注意：303 已被 notification_log:read 占用，此处 read 用 305 / manage 用 304，避免主键冲突。
INSERT INTO permissions (id, code, name, module, description, created_at) VALUES
('00000000-0000-0000-0000-000000000305', 'sms_strategy:read',   '查看短信策略', 'notification', '查看告警短信策略', NOW()),
('00000000-0000-0000-0000-000000000304', 'sms_strategy:manage', '管理短信策略', 'notification', '新增/编辑/删除/启停告警短信策略', NOW())
ON DUPLICATE KEY UPDATE name = VALUES(name);

-- 给 admin / operator 分配管理权限，viewer 只读
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000305'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000304'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000305'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000304'),
('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000305');

-- ---- 3. 扩展 event_type 字典：补齐老系统「应用 / 其他 / 硬件」一级类型 ----
INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'event_type', 'application', '应用告警', 1, 10, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_type', 'hardware',    '硬件告警', 1, 11, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_type', 'other',       '其他告警', 1, 12, UTC_TIMESTAMP(), UTC_TIMESTAMP());

-- ---- 4. 预置 event_sub_type 字典：老系统二级事件子类 ----
INSERT IGNORE INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('event_sub_type', '事件子类型', '告警事件二级子类（数据库/中间件具体产品）', 1, 11, UTC_TIMESTAMP(), UTC_TIMESTAMP());

INSERT IGNORE INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'event_sub_type', 'db2',        'DB2数据库告警',   1, 1,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_sub_type', 'oracle',     'Oracle数据库',    1, 2,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_sub_type', 'sequoiadb',  '巨杉数据库',      1, 3,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_sub_type', 'informix',   'INFORMIX',        1, 4,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_sub_type', 'sybase',     'SYBASE',          1, 5,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_sub_type', 'sqlserver',  'SQLSERVER',       1, 6,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_sub_type', 'gbase',      'GBASE',           1, 7,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_sub_type', 'was',        'WAS中间件',       1, 8,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_sub_type', 'cics',       'CICS中间件',      1, 9,  UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_sub_type', 'mq',         'MQ中间件',        1, 10, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'event_sub_type', 'tomcat',     'TOMCAT中间件',    1, 11, UTC_TIMESTAMP(), UTC_TIMESTAMP());
