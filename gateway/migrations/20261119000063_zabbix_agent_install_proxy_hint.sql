-- 安装剧本错误提示改为兼容 Proxy 拓扑（Agent 连 Proxy，不必直连 Server）。
UPDATE job_definitions
SET script_content = REPLACE(
    script_content,
    '主机到 Zabbix Server 的 10051 是否可达',
    '主机到 Zabbix Proxy/Server 的 10050/10051 是否可达'
)
WHERE name = 'Zabbix Agent 安装'
  AND script_content LIKE '%主机到 Zabbix Server 的 10051 是否可达%';
