-- AIOps 分析日志表
CREATE TABLE IF NOT EXISTS aiops_analysis_logs (
    id          VARCHAR(36)  PRIMARY KEY,
    type        VARCHAR(32)  NOT NULL COMMENT 'rca|anomaly|recommend|llm',
    trigger_id  VARCHAR(64)  COMMENT '触发的告警ID或工单ID',
    input_json  JSON         COMMENT '输入快照',
    result_json JSON         COMMENT '分析结果快照',
    created_by  VARCHAR(64),
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_aiops_type_time (type, created_at DESC),
    INDEX idx_aiops_trigger (trigger_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='AIOps 分析日志';

-- LLM 配置项写入 system_settings
INSERT INTO system_settings (setting_key, setting_value, description, updated_at) VALUES
    ('aiops_llm_enabled', 'false', 'AIOps LLM 诊断开关', NOW()),
    ('aiops_llm_api_url', '', 'LLM API 地址（OpenAI 兼容端点）', NOW()),
    ('aiops_llm_api_key', '', 'LLM API Key', NOW()),
    ('aiops_llm_model', 'deepseek-chat', 'LLM 模型名', NOW())
ON DUPLICATE KEY UPDATE setting_value = setting_value;
