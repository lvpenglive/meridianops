-- 20260817000026: 作业模块适配 CMDB ci_instances schema
-- 问题：job_run_targets.asset_id 为 BIGINT，但 CMDB ci_instances.id 是 CHAR(36) UUID
-- 另外去掉不再使用的 idx_target_asset (BIGINT)，改为 VARCHAR(36) 并重建索引

ALTER TABLE job_run_targets
    DROP INDEX idx_target_asset,
    MODIFY COLUMN asset_id VARCHAR(36) NOT NULL COMMENT 'CI 实例 ID (ci_instances.id，UUID 字符串)',
    ADD INDEX idx_target_asset (asset_id);
