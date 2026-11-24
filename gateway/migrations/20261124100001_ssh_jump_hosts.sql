-- Proxy SSH 跳板：Gateway 经 Proxy 再 SSH 到业务机（直连仍为默认）

CREATE TABLE IF NOT EXISTS monitor_ssh_jumps (
    id BIGINT NOT NULL AUTO_INCREMENT PRIMARY KEY,
    instance_code VARCHAR(64) NOT NULL COMMENT 'Zabbix 实例 code',
    proxy_id VARCHAR(64) NOT NULL COMMENT 'Zabbix proxyid，跳板端点',
    proxy_name VARCHAR(255) NULL COMMENT '展示名',
    jump_host VARCHAR(255) NOT NULL COMMENT 'Proxy 管理 IP/主机名（SSH）',
    jump_port INT NOT NULL DEFAULT 22,
    credential_id BIGINT NOT NULL COMMENT 'ssh_credentials.id，跳板登录凭据',
    enabled TINYINT NOT NULL DEFAULT 1,
    remark VARCHAR(255) NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NULL,
    UNIQUE KEY uk_jump_inst_proxy (instance_code, proxy_id),
    KEY idx_jump_cred (credential_id),
    KEY idx_jump_enabled (enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='监控 Proxy SSH 跳板';

-- Proxy Group → 指定组内哪台 Proxy 当跳板（可空=不经跳板/未配置）
ALTER TABLE monitor_proxy_conf_maps
    ADD COLUMN jump_proxy_id VARCHAR(64) NULL
        COMMENT '组内用作 SSH 跳板的 proxyid；空则该组不默认跳板'
        AFTER conf_suffix;

-- 配置项执行方式：auto=按监控通道解析跳板；direct=强制直连
ALTER TABLE ci_instances
    ADD COLUMN ssh_exec_mode VARCHAR(16) NULL
        COMMENT 'auto|direct；空=auto'
        AFTER ssh_credential_id;
