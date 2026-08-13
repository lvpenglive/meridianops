-- ============================================================
-- 拉取同步支持：sync_sources 表增加拉取配置字段
-- 1. pull_config JSON：拉取参数（API 路径/分页/过滤条件等）
-- 2. pull_cron：定时拉取 cron 表达式（空=不定时）
-- 3. pull_enabled：是否启用定时拉取
-- ============================================================

ALTER TABLE sync_sources
    ADD COLUMN pull_config   JSON         NULL COMMENT '拉取配置（API路径/分页/过滤等）',
    ADD COLUMN pull_cron     VARCHAR(64)  NOT NULL DEFAULT '' COMMENT '定时拉取 cron 表达式（空=不定时）',
    ADD COLUMN pull_enabled  TINYINT      NOT NULL DEFAULT 0 COMMENT '是否启用定时拉取';

-- 更新蓝鲸数据源：source_type 改为支持 pull
UPDATE sync_sources SET source_type = 'webhook', updated_at = NOW() WHERE code = 'blueking';
