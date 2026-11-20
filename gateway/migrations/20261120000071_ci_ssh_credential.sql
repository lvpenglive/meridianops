-- 配置项绑定 SSH 凭据：作业执行时按目标 CI 解析，剧本 credential_id 作兜底
ALTER TABLE ci_instances
    ADD COLUMN ssh_credential_id BIGINT NULL
        COMMENT '关联 ssh_credentials.id；作业 SSH 执行优先用此凭据'
        AFTER owner_id;

CREATE INDEX idx_ci_instances_ssh_credential ON ci_instances (ssh_credential_id);
