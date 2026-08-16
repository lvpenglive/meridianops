-- 20260816000023: 创建作业中心核心表
-- V1: 作业定义(job_definitions) + 执行历史(job_runs) + 每个资产的子任务(job_run_targets)

-- 作业定义表
CREATE TABLE IF NOT EXISTS job_definitions (
    id              BIGINT AUTO_INCREMENT PRIMARY KEY,
    name            VARCHAR(255) NOT NULL COMMENT '作业名称',
    description     VARCHAR(1024) NOT NULL DEFAULT '' COMMENT '作业描述',
    script_type     VARCHAR(32)  NOT NULL DEFAULT 'shell' COMMENT '脚本类型: shell/python/powershell',
    script_content  MEDIUMTEXT   NOT NULL COMMENT '脚本内容',
    timeout_secs    INT          NOT NULL DEFAULT 300 COMMENT '超时时间(秒), 默认300',
    target_scope    VARCHAR(16)  NOT NULL DEFAULT 'manual' COMMENT '目标范围: static/cmdb_query/manual',
    target_asset_ids JSON        NULL COMMENT '静态资产ID列表, JSON数组',
    target_cmdb_query VARCHAR(1024) NOT NULL DEFAULT '' COMMENT 'CMDB 查询条件',
    run_as          VARCHAR(128) NOT NULL DEFAULT 'root' COMMENT '执行用户, 默认root',
    port            INT          NOT NULL DEFAULT 22 COMMENT 'SSH端口, 默认22',
    enabled         TINYINT(1)   NOT NULL DEFAULT 1 COMMENT '是否启用',
    created_by      VARCHAR(128) NOT NULL COMMENT '创建人',
    created_at      DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_job_name (name),
    INDEX idx_job_creator (created_by),
    INDEX idx_job_enabled (enabled)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='作业定义';

-- 作业执行历史表
CREATE TABLE IF NOT EXISTS job_runs (
    id              BIGINT AUTO_INCREMENT PRIMARY KEY,
    job_id          BIGINT       NOT NULL COMMENT '关联作业定义ID, 0=即时脚本无定义',
    job_name        VARCHAR(255) NOT NULL COMMENT '执行时的作业名称快照',
    script_type     VARCHAR(32)  NOT NULL DEFAULT 'shell',
    script_content  MEDIUMTEXT   NOT NULL COMMENT '执行时的脚本快照',
    trigger_mode    VARCHAR(16)  NOT NULL DEFAULT 'manual' COMMENT '触发方式: manual/cron/api',
    target_count    INT          NOT NULL DEFAULT 0 COMMENT '目标资产总数',
    success_count   INT          NOT NULL DEFAULT 0 COMMENT '成功数',
    failed_count    INT          NOT NULL DEFAULT 0 COMMENT '失败数',
    overall_status  VARCHAR(16)  NOT NULL DEFAULT 'running' COMMENT '整体状态',
    started_by      VARCHAR(128) NOT NULL COMMENT '触发人',
    started_at      DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at     DATETIME     NULL COMMENT '全部完成时间',
    INDEX idx_run_job (job_id),
    INDEX idx_run_status (overall_status),
    INDEX idx_run_started (started_at),
    INDEX idx_run_creator (started_by)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='作业执行历史';

-- 每个资产的子任务表
CREATE TABLE IF NOT EXISTS job_run_targets (
    id              BIGINT AUTO_INCREMENT PRIMARY KEY,
    job_run_id      BIGINT       NOT NULL COMMENT '所属job_run.id',
    asset_id        BIGINT       NOT NULL COMMENT '资产ID',
    asset_name      VARCHAR(255) NOT NULL COMMENT '资产名称快照',
    asset_ip        VARCHAR(128) NOT NULL DEFAULT '' COMMENT '资产IP快照',
    status          VARCHAR(16)  NOT NULL DEFAULT 'pending' COMMENT '状态',
    exit_code       INT          NULL COMMENT '退出码',
    stdout          MEDIUMTEXT   NULL COMMENT '标准输出',
    stderr          MEDIUMTEXT   NULL COMMENT '标准错误',
    duration_ms     BIGINT       NOT NULL DEFAULT 0 COMMENT '执行耗时(ms)',
    started_at      DATETIME     NULL,
    finished_at     DATETIME     NULL,
    INDEX idx_target_run (job_run_id),
    INDEX idx_target_status (status),
    INDEX idx_target_asset (asset_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='作业执行子任务(按资产)';

-- 作业权限种子（使用正确字段: id/code/name/module/description/created_at）
INSERT IGNORE INTO permissions (id, code, name, module, description, created_at) VALUES
('20000000-0000-0000-0000-000000000027', 'job:read',     '查看作业',     '作业中心', '查看作业定义和执行历史', UTC_TIMESTAMP()),
('20000000-0000-0000-0000-000000000028', 'job:create',   '创建编辑作业', '作业中心', '创建和编辑作业定义',     UTC_TIMESTAMP()),
('20000000-0000-0000-0000-000000000029', 'job:execute',  '执行作业',     '作业中心', '手动执行作业定义',       UTC_TIMESTAMP()),
('20000000-0000-0000-0000-000000000030', 'job:admin',    '作业管理',     '作业中心', '作业管理(含删除作业)',   UTC_TIMESTAMP());

-- admin：全部 4 个作业权限
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000027'),
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000028'),
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000029'),
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000030');

-- operator：读 + 创建编辑 + 执行（不可删除）
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000027'),
('00000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000028'),
('00000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000029');

-- viewer：只读
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000027');
