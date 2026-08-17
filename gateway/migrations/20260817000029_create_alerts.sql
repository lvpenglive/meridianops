-- 20260817000029: 告警中心 - 事件库 + 静默规则 + 权限点 + 字典种子

-- ============================================================
-- alert_events：告警事件
--   - fingerprint 用于同源去重（同一指纹的新告警仅更新 fired_at/count）
--   - severity: P0(核心故障)/P1(重要)/P2(次要)/P3(警告)/info(提示)
--   - status:   firing(触发中)/acknowledged(已认领)/resolved(已解决)/suppressed(被静默)
--   - ci_id 关联 ci_instances.id（UUID 字符串）
-- ============================================================
CREATE TABLE IF NOT EXISTS alert_events (
    id              CHAR(36)       NOT NULL COMMENT '事件 UUID',
    fingerprint     VARCHAR(64)   NOT NULL COMMENT '去重指纹（source+ciId+metric+title 哈希前 16 字符）',
    source          VARCHAR(32)   NOT NULL DEFAULT 'manual' COMMENT '来源：zabbix/prometheus/manual/job/system',
    severity        VARCHAR(16)   NOT NULL DEFAULT 'P3' COMMENT '严重程度：P0/P1/P2/P3/info',
    status          VARCHAR(20)   NOT NULL DEFAULT 'firing' COMMENT '状态：firing/acknowledged/resolved/suppressed',
    title           VARCHAR(255)  NOT NULL COMMENT '告警标题',
    message         TEXT          NULL COMMENT '告警详情',
    labels          JSON          NULL COMMENT '标签 JSON（如 host/service/category）',
    ci_id           CHAR(36)      NULL COMMENT '关联资产 ID → ci_instances.id',
    ci_name_snapshot VARCHAR(255) NULL COMMENT '资产名称快照（避免 N+1）',
    fire_count      BIGINT        NOT NULL DEFAULT 1 COMMENT '同指纹触发次数',
    first_fired_at  VARCHAR(40)   NOT NULL COMMENT '首次触发时间 RFC3339',
    fired_at        VARCHAR(40)   NOT NULL COMMENT '最近触发时间 RFC3339',
    acknowledged_by VARCHAR(64)   NULL COMMENT '认领人',
    acknowledged_at VARCHAR(40)   NULL COMMENT '认领时间',
    resolved_by     VARCHAR(64)   NULL COMMENT '解决人',
    resolved_at     VARCHAR(40)   NULL COMMENT '解决时间',
    resolution_note TEXT          NULL COMMENT '解决备注',
    created_at      VARCHAR(40)   NOT NULL COMMENT '入库时间',
    updated_at      VARCHAR(40)   NOT NULL COMMENT '更新时间',
    PRIMARY KEY (id),
    UNIQUE KEY uk_alert_fp (fingerprint),
    INDEX idx_alert_status (status),
    INDEX idx_alert_severity (severity),
    INDEX idx_alert_source (source),
    INDEX idx_alert_ci (ci_id),
    INDEX idx_alert_fired (fired_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='告警事件';

-- ============================================================
-- alert_silences：静默规则
--   - match_labels JSON：如 {"source":["zabbix"],"severity":["P0","P1"],"ciId":["uuid1"]}
--   - 时间窗口：starts_at ~ ends_at；active 字段为运行期计算缓存（仅 ends_at 未到且未手动停用为 1）
-- ============================================================
CREATE TABLE IF NOT EXISTS alert_silences (
    id            CHAR(36)       NOT NULL COMMENT '静默规则 UUID',
    name          VARCHAR(255)   NOT NULL COMMENT '规则名称',
    reason        VARCHAR(500)   NULL COMMENT '静默理由',
    match_labels  JSON           NULL COMMENT '匹配条件 JSON',
    starts_at     VARCHAR(40)    NOT NULL COMMENT '生效开始时间',
    ends_at       VARCHAR(40)    NOT NULL COMMENT '生效结束时间',
    active        TINYINT        NOT NULL DEFAULT 1 COMMENT '是否启用（1 启用 0 已停用/已过期）',
    created_by    VARCHAR(64)    NOT NULL COMMENT '创建人',
    created_at    VARCHAR(40)    NOT NULL COMMENT '创建时间',
    updated_at    VARCHAR(40)    NOT NULL COMMENT '更新时间',
    PRIMARY KEY (id),
    INDEX idx_silence_active (active),
    INDEX idx_silence_window (starts_at, ends_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='静默规则';

-- ============================================================
-- 告警权限点（4 个）
-- ============================================================
INSERT INTO permissions (id, code, name, module, description, created_at) VALUES
('00000000-0000-0000-0000-000000000201', 'alert:read',   '查看告警', 'alert', '查看告警事件列表和详情', NOW()),
('00000000-0000-0000-0000-000000000202', 'alert:create', '创建告警', 'alert', '手动创建告警事件', NOW()),
('00000000-0000-0000-0000-000000000203', 'alert:update', '处置告警', 'alert', '认领/解决/添加备注/管理静默规则', NOW()),
('00000000-0000-0000-0000-000000000204', 'alert:delete', '删除告警', 'alert', '删除告警事件', NOW())
ON DUPLICATE KEY UPDATE name = VALUES(name);

-- 给 admin 全部权限
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000201'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000202'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000203'),
('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000204');

-- 给 operator 全部权限（运维人员需处理告警）
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000201'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000202'),
('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000203');

-- 给 viewer 只读权限
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000201');

-- ============================================================
-- 字典种子：告警级别 / 来源 / 状态
-- ============================================================
INSERT INTO sys_dict_types (code, name, description, enabled, sort_order, created_at, updated_at) VALUES
('alert_severity', '告警级别', '告警严重程度枚举', 1, 10, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
('alert_source',   '告警来源', '告警事件来源系统', 1, 11, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
('alert_status',   '告警状态', '告警事件处置状态', 1, 12, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE name = VALUES(name);

INSERT INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at) VALUES
(UUID(), 'alert_severity', 'P0',    'P0 紧急', 1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'P1',    'P1 重要', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'P2',    'P2 次要', 1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'P3',    'P3 警告', 1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_severity', 'info',  '提示',   1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_source',   'zabbix',     'Zabbix',     1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_source',   'prometheus', 'Prometheus', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_source',   'manual',     '人工上报',   1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_source',   'job',        '作业执行',   1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_source',   'system',     '系统内置',   1, 5, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_status',   'firing',       '触发中', 1, 1, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_status',   'acknowledged', '已认领', 1, 2, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_status',   'resolved',     '已解决', 1, 3, UTC_TIMESTAMP(), UTC_TIMESTAMP()),
(UUID(), 'alert_status',   'suppressed',   '已静默', 1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP())
ON DUPLICATE KEY UPDATE item_label = VALUES(item_label);

-- ============================================================
-- 样例告警数据：覆盖多种级别/状态/来源，便于演示
-- ============================================================
INSERT INTO alert_events
(id, fingerprint, source, severity, status, title, message, labels, ci_id, ci_name_snapshot,
 fire_count, first_fired_at, fired_at, acknowledged_by, acknowledged_at, resolved_by, resolved_at, resolution_note,
 created_at, updated_at) VALUES
-- P0 触发中：核心数据库 CPU 100%
('alert-evt-0001', 'fp-zabbix-coredb-cpu-100', 'zabbix', 'P0', 'firing',
 '核心数据库 CPU 使用率 100%',
 'core-db-01 主机 CPU 持续 5 分钟高于 95%，当前 100%，可能影响核心交易',
 JSON_OBJECT('host', 'core-db-01', 'category', 'database', 'metric', 'cpu_usage'),
 'ci-inst-core-db-01', 'core-db-01 核心数据库',
 3, '2026-08-17T02:15:00Z', '2026-08-17T02:35:00Z',
 NULL, NULL, NULL, NULL, NULL,
 '2026-08-17T02:15:05Z', '2026-08-17T02:35:00Z'),

-- P1 触发中：支付网关 HSM 连接异常
('alert-evt-0002', 'fp-prom-hsm-conn-fail', 'prometheus', 'P1', 'firing',
 '支付网关 HSM 连接失败',
 'pay-gw-01 与加密机 hsm-01 连接失败，已重试 3 次均未成功',
 JSON_OBJECT('service', 'pay-gateway', 'target', 'hsm-01', 'metric', 'hsm_conn_fail'),
 'ci-inst-pay-gw-01', 'pay-gw-01 支付网关',
 1, '2026-08-17T01:50:00Z', '2026-08-17T01:50:00Z',
 NULL, NULL, NULL, NULL, NULL,
 '2026-08-17T01:50:10Z', '2026-08-17T01:50:10Z'),

-- P2 已认领：F5 负载 VIP 切换
('alert-evt-0003', 'fp-zabbix-f5-vip-switch', 'zabbix', 'P2', 'acknowledged',
 'F5 负载均衡 VIP 主备切换',
 'f5-pair-01 发生 VIP 主备切换，原主节点恢复中',
 JSON_OBJECT('host', 'f5-pair-01', 'vip', '10.20.30.40'),
 'ci-inst-f5-pair-01', 'f5-pair-01 负载均衡',
 1, '2026-08-17T00:30:00Z', '2026-08-17T00:30:00Z',
 'admin', '2026-08-17T00:45:00Z', NULL, NULL, NULL,
 '2026-08-17T00:30:15Z', '2026-08-17T00:45:00Z'),

-- P1 触发中：WebLogic 队列堆积
('alert-evt-0004', 'fp-wls-stuck-queue', 'zabbix', 'P1', 'firing',
 'WebLogic 队列堆积超过阈值',
 'wls-cluster-01 EXEC 队列长度 85，阈值 50，请求处理延迟上升',
 JSON_OBJECT('cluster', 'wls-cluster-01', 'metric', 'stuck_thread_count'),
 'ci-inst-wls-cluster-01', 'wls-cluster-01 WebLogic 集群',
 5, '2026-08-16T22:00:00Z', '2026-08-17T02:40:00Z',
 NULL, NULL, NULL, NULL, NULL,
 '2026-08-16T22:00:20Z', '2026-08-17T02:40:00Z'),

-- P3 已解决：磁盘空间警告
('alert-evt-0005', 'fp-prom-disk-warn', 'prometheus', 'P3', 'resolved',
 '磁盘空间使用率 85%',
 'app-srv-05 /data 分区使用率 85%，阈值 80%',
 JSON_OBJECT('host', 'app-srv-05', 'partition', '/data'),
 'ci-inst-app-srv-05', 'app-srv-05 应用服务器',
 1, '2026-08-16T18:00:00Z', '2026-08-16T18:00:00Z',
 NULL, NULL, 'operator', '2026-08-16T19:30:00Z', '清理临时日志后释放 15GB',
 '2026-08-16T18:00:05Z', '2026-08-16T19:30:00Z'),

-- info 触发中：作业执行失败告警
('alert-evt-0006', 'fp-job-fail-backup', 'job', 'info', 'firing',
 '备份作业执行失败',
 '作业「数据库全量备份」执行失败，错误码 28（磁盘空间不足）',
 JSON_OBJECT('jobName', '数据库全量备份', 'runId', 'job-run-0007'),
 NULL, NULL,
 2, '2026-08-17T03:00:00Z', '2026-08-17T03:15:00Z',
 NULL, NULL, NULL, NULL, NULL,
 '2026-08-17T03:00:10Z', '2026-08-17T03:15:00Z'),

-- P2 已解决：Redis 主从延迟
('alert-evt-0007', 'fp-prom-redis-delay', 'prometheus', 'P2', 'resolved',
 'Redis 主从同步延迟',
 'redis-cluster-01 主从复制延迟 30s，影响缓存一致性',
 JSON_OBJECT('cluster', 'redis-cluster-01', 'metric', 'replication_lag'),
 'ci-inst-redis-cluster-01', 'redis-cluster-01 缓存集群',
 4, '2026-08-16T10:00:00Z', '2026-08-16T11:00:00Z',
 'admin', '2026-08-16T10:15:00Z', 'admin', '2026-08-16T11:30:00Z', '调整 repl-backlog-size 后恢复',
 '2026-08-16T10:00:20Z', '2026-08-16T11:30:00Z'),

-- P0 已认领：核心交易系统不可用
('alert-evt-0008', 'fp-manual-coretrade-down', 'manual', 'P0', 'acknowledged',
 '核心交易系统不可用',
 '核心交易系统首页访问返回 503，疑似应用服务异常',
 JSON_OBJECT('system', 'core-trade', 'httpCode', '503'),
 'ci-inst-core-trade-sys', 'core-trade 核心交易系统',
 1, '2026-08-17T03:30:00Z', '2026-08-17T03:30:00Z',
 'admin', '2026-08-17T03:40:00Z', NULL, NULL, NULL,
 '2026-08-17T03:30:15Z', '2026-08-17T03:40:00Z'),

-- P3 已解决：ELK 集群节点重启
('alert-evt-0009', 'fp-elk-node-restart', 'prometheus', 'P3', 'resolved',
 'ES 集群节点重启',
 'es-node-03 在过去 10 分钟内重启过 1 次',
 JSON_OBJECT('cluster', 'es-cluster', 'node', 'es-node-03'),
 'ci-inst-es-cluster', 'es-cluster ELK 日志集群',
 1, '2026-08-15T15:00:00Z', '2026-08-15T15:00:00Z',
 NULL, NULL, 'admin', '2026-08-15T15:30:00Z', 'JVM 堆内存不足触发 OOM，已扩容',
 '2026-08-15T15:00:20Z', '2026-08-15T15:30:00Z');
