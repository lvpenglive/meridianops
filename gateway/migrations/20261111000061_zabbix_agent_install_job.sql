-- Zabbix Agent 安装剧本。Server / Hostname 由执行时注入的环境变量提供。
-- credential_id 需在作业中心绑定真实凭据后才能实跑。

INSERT INTO job_definitions (
    name, description, script_type, script_content, timeout_secs,
    target_scope, target_asset_ids, run_as, port, enabled, created_by,
    executor_type, credential_id
)
SELECT
    'Zabbix Agent 安装',
    '监控纳管：安装 zabbix-agent2，写入 Server/ServerActive/Hostname 并启动',
    'shell',
    '#!/bin/bash\nset -euo pipefail\ntrap \'echo \"安装失败：请检查包仓库是否有 zabbix-agent2、SSH 是否可用、主机到 Zabbix Server 的 10051 是否可达\" >&2\' ERR\n: \"${ZBX_SERVER:?缺少 ZBX_SERVER}\"\n: \"${ZBX_HOSTNAME:?缺少 ZBX_HOSTNAME}\"\nSERVER_ACTIVE=\"${ZBX_SERVER_ACTIVE:-$ZBX_SERVER}\"\nif command -v rpm >/dev/null 2>&1; then\n  rpm -q zabbix-agent2 >/dev/null 2>&1 || { yum install -y zabbix-agent2 || dnf install -y zabbix-agent2; }\nelif command -v dpkg >/dev/null 2>&1; then\n  dpkg -s zabbix-agent2 >/dev/null 2>&1 || { apt-get update && apt-get install -y zabbix-agent2; }\nelse\n  echo \"不支持的系统，仅支持 rpm 或 deb\" >&2\n  exit 1\nfi\nconf=/etc/zabbix/zabbix_agent2.conf\nif [ ! -f \"$conf\" ]; then\n  echo \"未找到 $conf\" >&2\n  exit 1\nfi\nset_kv() {\n  local key=\"$1\" val=\"$2\"\n  if grep -q \"^${key}=\" \"$conf\"; then\n    sed -i \"s|^${key}=.*|${key}=${val}|\" \"$conf\"\n  else\n    echo \"${key}=${val}\" >> \"$conf\"\n  fi\n}\nset_kv Server \"$ZBX_SERVER\"\nset_kv ServerActive \"$SERVER_ACTIVE\"\nset_kv Hostname \"$ZBX_HOSTNAME\"\nsystemctl enable zabbix-agent2\nsystemctl restart zabbix-agent2\nsystemctl is-active zabbix-agent2\n',
    600,
    'manual',
    NULL,
    'root',
    22,
    1,
    'system',
    'ssh',
    NULL
WHERE NOT EXISTS (SELECT 1 FROM job_definitions WHERE name = 'Zabbix Agent 安装');
