-- CMDB 服务化：业务系统 → 应用服务 → 主机/中间件/数据库
-- 关系复用现有类型：biz contains service；service runs_on host；service depends_on mw/db

INSERT INTO ci_models (id, code, name, icon, description, enabled, sort_order, created_at, updated_at)
VALUES (
    'cmdb-model-service',
    'service',
    '应用服务',
    'Service',
    '业务系统下的应用/微服务能力单元；主机与中间件挂在其下',
    1,
    2,
    UTC_TIMESTAMP(),
    UTC_TIMESTAMP()
)
ON DUPLICATE KEY UPDATE
    name = VALUES(name),
    icon = VALUES(icon),
    description = VALUES(description),
    enabled = VALUES(enabled),
    sort_order = VALUES(sort_order),
    updated_at = UTC_TIMESTAMP();

INSERT INTO ci_model_attrs (
    id, model_id, code, name, value_type, default_value, options,
    is_required, is_unique, is_searchable, sort_order, created_at
) VALUES
(
    'attr-svc-code', 'cmdb-model-service', 'service_code', '服务编码',
    'string', '', NULL, 1, 1, 1, 1, UTC_TIMESTAMP()
),
(
    'attr-svc-type', 'cmdb-model-service', 'service_type', '服务类型',
    'enum', '微服务',
    '["微服务","Web应用","批处理","API网关","消息消费","Other"]',
    1, 0, 1, 2, UTC_TIMESTAMP()
),
(
    'attr-svc-port', 'cmdb-model-service', 'port', '服务端口',
    'number', '', NULL, 0, 0, 0, 3, UTC_TIMESTAMP()
),
(
    'attr-svc-env', 'cmdb-model-service', 'deploy_env', '部署环境',
    'enum', 'prod', '["prod","dr","uat","test","dev"]',
    0, 0, 1, 4, UTC_TIMESTAMP()
),
(
    'attr-svc-desc', 'cmdb-model-service', 'description', '服务说明',
    'string', '', NULL, 0, 0, 0, 5, UTC_TIMESTAMP()
)
ON DUPLICATE KEY UPDATE
    name = VALUES(name),
    value_type = VALUES(value_type),
    options = VALUES(options),
    is_required = VALUES(is_required),
    is_unique = VALUES(is_unique),
    is_searchable = VALUES(is_searchable),
    sort_order = VALUES(sort_order);

-- 样例：网银下挂支付网关 / 登录服务（仅当样例业务系统已存在）
INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at)
SELECT
    'ins-svc-ebank-pay',
    'cmdb-model-service',
    '网银-支付网关',
    'running',
    '{"service_code":"ebank-pay-gateway","service_type":"API网关","port":8443,"deploy_env":"prod","description":"网银对外支付入口"}',
    '网银,服务化样例',
    UTC_TIMESTAMP(),
    UTC_TIMESTAMP()
FROM DUAL
WHERE EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-bs-ebank')
ON DUPLICATE KEY UPDATE name = VALUES(name), attributes = VALUES(attributes), updated_at = UTC_TIMESTAMP();

INSERT INTO ci_instances (id, model_id, name, status, attributes, tags, created_at, updated_at)
SELECT
    'ins-svc-ebank-login',
    'cmdb-model-service',
    '网银-登录服务',
    'running',
    '{"service_code":"ebank-login","service_type":"微服务","port":8080,"deploy_env":"prod","description":"网银登录与会话"}',
    '网银,服务化样例',
    UTC_TIMESTAMP(),
    UTC_TIMESTAMP()
FROM DUAL
WHERE EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-bs-ebank')
ON DUPLICATE KEY UPDATE name = VALUES(name), attributes = VALUES(attributes), updated_at = UTC_TIMESTAMP();

-- 业务系统 contains 服务
INSERT IGNORE INTO ci_relations (id, source_id, target_id, relation_type, created_at)
SELECT 'rel-svc-ebank-pay', 'ins-bs-ebank', 'ins-svc-ebank-pay', 'contains', UTC_TIMESTAMP()
FROM DUAL
WHERE EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-svc-ebank-pay');

INSERT IGNORE INTO ci_relations (id, source_id, target_id, relation_type, created_at)
SELECT 'rel-svc-ebank-login', 'ins-bs-ebank', 'ins-svc-ebank-login', 'contains', UTC_TIMESTAMP()
FROM DUAL
WHERE EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-svc-ebank-login');

-- 服务 depends_on 中间件 / 库（不删除旧的 biz→mw 直连，兼容存量）
INSERT IGNORE INTO ci_relations (id, source_id, target_id, relation_type, created_at)
SELECT 'rel-svc-pay-nginx1', 'ins-svc-ebank-pay', 'ins-mw-ebank-nginx1', 'depends_on', UTC_TIMESTAMP()
FROM DUAL
WHERE EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-mw-ebank-nginx1')
  AND EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-svc-ebank-pay');

INSERT IGNORE INTO ci_relations (id, source_id, target_id, relation_type, created_at)
SELECT 'rel-svc-pay-db', 'ins-svc-ebank-pay', 'ins-db-ebank-mysql-master', 'depends_on', UTC_TIMESTAMP()
FROM DUAL
WHERE EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-db-ebank-mysql-master')
  AND EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-svc-ebank-pay');

INSERT IGNORE INTO ci_relations (id, source_id, target_id, relation_type, created_at)
SELECT 'rel-svc-login-wl01', 'ins-svc-ebank-login', 'ins-mw-ebank-wl01', 'depends_on', UTC_TIMESTAMP()
FROM DUAL
WHERE EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-mw-ebank-wl01')
  AND EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-svc-ebank-login');

-- 服务 runs_on 主机
INSERT IGNORE INTO ci_relations (id, source_id, target_id, relation_type, created_at)
SELECT 'rel-svc-pay-host1', 'ins-svc-ebank-pay', 'ins-host-ebank-web1', 'runs_on', UTC_TIMESTAMP()
FROM DUAL
WHERE EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-host-ebank-web1')
  AND EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-svc-ebank-pay');

INSERT IGNORE INTO ci_relations (id, source_id, target_id, relation_type, created_at)
SELECT 'rel-svc-pay-host2', 'ins-svc-ebank-pay', 'ins-host-ebank-web2', 'runs_on', UTC_TIMESTAMP()
FROM DUAL
WHERE EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-host-ebank-web2')
  AND EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-svc-ebank-pay');

INSERT IGNORE INTO ci_relations (id, source_id, target_id, relation_type, created_at)
SELECT 'rel-svc-login-host1', 'ins-svc-ebank-login', 'ins-host-ebank-app1', 'runs_on', UTC_TIMESTAMP()
FROM DUAL
WHERE EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-host-ebank-app1')
  AND EXISTS (SELECT 1 FROM ci_instances WHERE id = 'ins-svc-ebank-login');
