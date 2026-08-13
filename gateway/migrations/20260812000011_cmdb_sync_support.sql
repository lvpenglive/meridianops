-- ============================================================
-- CMDB 同步支持：外部系统（蓝鲸等）数据同步
-- 1. ci_instances 加 external_id / source / last_synced_at 字段
-- 2. 新建 sync_logs 表记录每次同步日志
-- 3. 新建 sync_sources 表配置外部数据源
-- ============================================================

-- ---- 1. ci_instances 加同步字段 ----
ALTER TABLE ci_instances
    ADD COLUMN external_id     VARCHAR(128) NULL COMMENT '外部系统中的唯一 ID（如蓝鲸 bk_host_id）',
    ADD COLUMN source          VARCHAR(32)  NULL COMMENT '数据来源：blueking/manual/import',
    ADD COLUMN last_synced_at  VARCHAR(64)  NULL COMMENT '最近一次同步时间（RFC3339）',
    ADD INDEX idx_ciinst_external (source, external_id);

-- ---- 2. 同步数据源配置表 ----
CREATE TABLE sync_sources (
    id              CHAR(36)     NOT NULL,
    code            VARCHAR(32)  NOT NULL COMMENT '来源编码：blueking/cmdb_manual/import',
    name            VARCHAR(128) NOT NULL COMMENT '显示名称：蓝鲸CMDB',
    source_type     VARCHAR(16)  NOT NULL DEFAULT 'webhook' COMMENT 'webhook/pull/manual',
    api_url         VARCHAR(255) NOT NULL DEFAULT '' COMMENT '外部 API 地址（pull 模式用）',
    api_token       VARCHAR(255) NOT NULL DEFAULT '' COMMENT '访问 token（加密存储）',
    webhook_secret  VARCHAR(255) NOT NULL DEFAULT '' COMMENT 'webhook 签名密钥',
    enabled         TINYINT      NOT NULL DEFAULT 1,
    last_sync_at    VARCHAR(64)  NULL COMMENT '最近同步时间',
    last_sync_count INT          NOT NULL DEFAULT 0 COMMENT '上次同步条数',
    last_sync_status VARCHAR(16) NOT NULL DEFAULT '' COMMENT 'success/failed',
    created_at      VARCHAR(64)  NOT NULL,
    updated_at      VARCHAR(64)  NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uk_syncsrc_code (code)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- ---- 3. 同步日志表（每次同步批次一条）----
CREATE TABLE sync_logs (
    id              BIGINT AUTO_INCREMENT PRIMARY KEY,
    source_code     VARCHAR(32)  NOT NULL COMMENT '来源编码',
    batch_id        VARCHAR(64)  NOT NULL COMMENT '批次 ID（UUID，便于关联明细）',
    action          VARCHAR(16)  NOT NULL COMMENT 'create/update/delete/upsert',
    model_code      VARCHAR(64)  NOT NULL COMMENT 'CI 模型编码',
    external_id     VARCHAR(128) NOT NULL DEFAULT '',
    instance_id     CHAR(36)     NULL COMMENT 'MeridianOps 中的 ci_instance.id',
    instance_name   VARCHAR(255) NOT NULL DEFAULT '',
    status          VARCHAR(16)  NOT NULL COMMENT 'success/skipped/failed',
    message         VARCHAR(512) NOT NULL DEFAULT '' COMMENT '错误信息或跳过原因',
    payload         JSON         NULL COMMENT '原始数据快照',
    created_at      VARCHAR(64)  NOT NULL,
    INDEX idx_synclog_source (source_code),
    INDEX idx_synclog_batch (batch_id),
    INDEX idx_synclog_created (created_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- ---- 4. 种子：蓝鲸 CMDB 数据源 ----
INSERT INTO sync_sources (id, code, name, source_type, api_url, api_token, webhook_secret, enabled, created_at, updated_at) VALUES
('sync-src-blueking', 'blueking', '蓝鲸 CMDB', 'webhook', '', '', '', 1, NOW(), NOW())
ON DUPLICATE KEY UPDATE name = VALUES(name);
