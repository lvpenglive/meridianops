-- Agent 压缩包库 + Proxy Group 与 conf 模板后缀映射

CREATE TABLE IF NOT EXISTS monitor_agent_packages (
    id VARCHAR(36) NOT NULL PRIMARY KEY,
    name VARCHAR(128) NOT NULL,
    version VARCHAR(64) NOT NULL,
    os_arch VARCHAR(64) NULL,
    agent_kind VARCHAR(32) NULL COMMENT 'agentd / agent2',
    filename VARCHAR(255) NOT NULL,
    stored_path VARCHAR(512) NOT NULL,
    file_size BIGINT NOT NULL DEFAULT 0,
    sha256 VARCHAR(64) NULL,
    description VARCHAR(512) NULL,
    enabled TINYINT NOT NULL DEFAULT 1,
    created_by VARCHAR(64) NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NULL,
    KEY idx_agent_pkg_enabled (enabled),
    KEY idx_agent_pkg_version (version)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE IF NOT EXISTS monitor_proxy_conf_maps (
    id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
    instance_code VARCHAR(64) NOT NULL,
    proxy_group_id VARCHAR(64) NOT NULL,
    conf_suffix VARCHAR(64) NOT NULL COMMENT '对应 etc/zabbix_agentd.conf_<suffix>',
    remark VARCHAR(255) NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NULL,
    UNIQUE KEY uk_inst_proxy_group (instance_code, proxy_group_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
