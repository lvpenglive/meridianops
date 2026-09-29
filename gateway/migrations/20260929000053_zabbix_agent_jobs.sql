-- Zabbix Agent 批量操作预置剧本（SSH）。credential_id 需在作业中心绑定真实凭据后才能实跑。
-- zabbix-ctl 默认按作业名称匹配；也可在 toml 里写死 job_* ID。

INSERT INTO job_definitions (
    name, description, script_type, script_content, timeout_secs,
    target_scope, target_asset_ids, run_as, port, enabled, created_by,
    executor_type, credential_id
)
SELECT * FROM (
    SELECT
        'Zabbix Agent 启动' AS name,
        '监控纳管：启动 zabbix-agent2 或 zabbix-agent' AS description,
        'shell' AS script_type,
        '#!/bin/bash\nset -e\nif systemctl list-unit-files | grep -q zabbix-agent2; then\n  systemctl start zabbix-agent2\n  systemctl is-active zabbix-agent2\nelif systemctl list-unit-files | grep -q zabbix-agent; then\n  systemctl start zabbix-agent\n  systemctl is-active zabbix-agent\nelse\n  echo \"未找到 zabbix-agent 服务\" >&2\n  exit 1\nfi' AS script_content,
        120 AS timeout_secs,
        'manual' AS target_scope,
        NULL AS target_asset_ids,
        'root' AS run_as,
        22 AS port,
        1 AS enabled,
        'system' AS created_by,
        'ssh' AS executor_type,
        NULL AS credential_id
) t
WHERE NOT EXISTS (SELECT 1 FROM job_definitions WHERE name = 'Zabbix Agent 启动');

INSERT INTO job_definitions (
    name, description, script_type, script_content, timeout_secs,
    target_scope, target_asset_ids, run_as, port, enabled, created_by,
    executor_type, credential_id
)
SELECT * FROM (
    SELECT
        'Zabbix Agent 停止',
        '监控纳管：停止 zabbix-agent2 或 zabbix-agent',
        'shell',
        '#!/bin/bash\nset -e\nif systemctl list-unit-files | grep -q zabbix-agent2; then\n  systemctl stop zabbix-agent2\nelif systemctl list-unit-files | grep -q zabbix-agent; then\n  systemctl stop zabbix-agent\nelse\n  echo \"未找到 zabbix-agent 服务\" >&2\n  exit 1\nfi\necho stopped',
        120,
        'manual',
        NULL,
        'root',
        22,
        1,
        'system',
        'ssh',
        NULL
) t
WHERE NOT EXISTS (SELECT 1 FROM job_definitions WHERE name = 'Zabbix Agent 停止');

INSERT INTO job_definitions (
    name, description, script_type, script_content, timeout_secs,
    target_scope, target_asset_ids, run_as, port, enabled, created_by,
    executor_type, credential_id
)
SELECT * FROM (
    SELECT
        'Zabbix Agent 重启',
        '监控纳管：重启 zabbix-agent2 或 zabbix-agent',
        'shell',
        '#!/bin/bash\nset -e\nif systemctl list-unit-files | grep -q zabbix-agent2; then\n  systemctl restart zabbix-agent2\n  systemctl is-active zabbix-agent2\nelif systemctl list-unit-files | grep -q zabbix-agent; then\n  systemctl restart zabbix-agent\n  systemctl is-active zabbix-agent\nelse\n  echo \"未找到 zabbix-agent 服务\" >&2\n  exit 1\nfi',
        180,
        'manual',
        NULL,
        'root',
        22,
        1,
        'system',
        'ssh',
        NULL
) t
WHERE NOT EXISTS (SELECT 1 FROM job_definitions WHERE name = 'Zabbix Agent 重启');

INSERT INTO job_definitions (
    name, description, script_type, script_content, timeout_secs,
    target_scope, target_asset_ids, run_as, port, enabled, created_by,
    executor_type, credential_id
)
SELECT * FROM (
    SELECT
        'Zabbix Agent 升级',
        '监控纳管：按环境定制的升级脚本占位。请改成行内包管理命令后再用于生产。',
        'shell',
        '#!/bin/bash\nset -e\necho \"请在作业中心编辑本剧本，改为行内统一的 Agent 升级命令\"\nif command -v yum >/dev/null 2>&1; then\n  yum install -y zabbix-agent2 || yum install -y zabbix-agent\nelif command -v apt-get >/dev/null 2>&1; then\n  apt-get update && (apt-get install -y zabbix-agent2 || apt-get install -y zabbix-agent)\nelse\n  echo \"未知包管理器，请手工升级\" >&2\n  exit 1\nfi\nsystemctl restart zabbix-agent2 2>/dev/null || systemctl restart zabbix-agent\n(systemctl is-active zabbix-agent2 || systemctl is-active zabbix-agent)',
        600,
        'manual',
        NULL,
        'root',
        22,
        1,
        'system',
        'ssh',
        NULL
) t
WHERE NOT EXISTS (SELECT 1 FROM job_definitions WHERE name = 'Zabbix Agent 升级');

INSERT INTO job_definitions (
    name, description, script_type, script_content, timeout_secs,
    target_scope, target_asset_ids, run_as, port, enabled, created_by,
    executor_type, credential_id
)
SELECT * FROM (
    SELECT
        'Zabbix Agent 回滚',
        '监控纳管：回滚占位剧本。请按行内包版本策略修改。',
        'shell',
        '#!/bin/bash\necho \"请在作业中心编辑本剧本，指定要回滚的 Agent 版本包\"\nexit 1',
        600,
        'manual',
        NULL,
        'root',
        22,
        1,
        'system',
        'ssh',
        NULL
) t
WHERE NOT EXISTS (SELECT 1 FROM job_definitions WHERE name = 'Zabbix Agent 回滚');
