-- ============================================================
-- 2026-09-13: 三个 HTTP 出站样例通道（优云 CMDB / AxleOps / 理想自动化）
-- 默认未启用，填对端地址和 Token 后可在「数据源同步」手推主机投影。
-- ============================================================

INSERT INTO sync_sources (
    id, code, name, source_type, api_url, api_token, webhook_secret,
    enabled, pull_config, pull_cron, pull_enabled, last_sync_count, last_sync_status,
    created_at, updated_at
) VALUES
(
    'sync-src-youyun-cmdb',
    'youyun_cmdb',
    '优云 CMDB',
    'http_push',
    'http://youyun-cmdb.example.local',
    '',
    '',
    0,
    '{"method":"POST","path":"/openapi/v1/cmdb/ci/sync","bodyMode":"items","authStyle":"bearer","rejectEmpty":true,"platform":"youyun"}',
    '',
    0,
    0,
    '',
    UTC_TIMESTAMP(),
    UTC_TIMESTAMP()
),
(
    'sync-src-axleops-push',
    'axleops_push',
    'AxleOps 服务台账',
    'http_push',
    'http://axleops-admin:9000',
    '',
    '',
    0,
    '{"method":"PUT","path":"/api/v1/cmdb/hosts","bodyMode":"rows","authStyle":"axleops","rejectEmpty":true,"platform":"axleops"}',
    '',
    0,
    0,
    '',
    UTC_TIMESTAMP(),
    UTC_TIMESTAMP()
),
(
    'sync-src-ideal-auto',
    'ideal_auto',
    '理想自动化',
    'http_push',
    'http://ideal-auto.example.local',
    '',
    '',
    0,
    '{"method":"POST","path":"/openapi/batch/cmdb/hosts","bodyMode":"hosts","authStyle":"bearer","rejectEmpty":true,"platform":"ideal"}',
    '',
    0,
    0,
    '',
    UTC_TIMESTAMP(),
    UTC_TIMESTAMP()
)
ON DUPLICATE KEY UPDATE
    name = VALUES(name),
    source_type = VALUES(source_type);

INSERT INTO sys_dict_items (id, type_code, item_value, item_label, enabled, sort_order, created_at, updated_at)
SELECT '00000000-0000-0000-0000-000000000157', 'sync_action', 'push', '出站推送', 1, 4, UTC_TIMESTAMP(), UTC_TIMESTAMP()
FROM DUAL
WHERE NOT EXISTS (
    SELECT 1 FROM sys_dict_items WHERE type_code = 'sync_action' AND item_value = 'push'
);
