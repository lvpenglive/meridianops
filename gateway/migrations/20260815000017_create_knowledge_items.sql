-- ============================================================
-- 2026-08-15: 知识库（Phase 4）
-- 设计预留 AI 扩展：embedding 字段供 Phase 7 向量检索使用
-- ============================================================

-- ---- 知识库权限点（4 个）----
INSERT IGNORE INTO permissions (id, code, name, module, description, created_at) VALUES
('20000000-0000-0000-0000-000000000019', 'knowledge:read',   '查看知识库', '知识库', '查看知识条目',   UTC_TIMESTAMP()),
('20000000-0000-0000-0000-000000000020', 'knowledge:create', '创建知识',   '知识库', '新建知识条目',   UTC_TIMESTAMP()),
('20000000-0000-0000-0000-000000000021', 'knowledge:update', '编辑知识',   '知识库', '修改知识条目',   UTC_TIMESTAMP()),
('20000000-0000-0000-0000-000000000022', 'knowledge:delete', '删除知识',   '知识库', '删除知识条目',   UTC_TIMESTAMP());

-- admin 角色分配知识库全部权限
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000019'),
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000020'),
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000021'),
('00000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000022');

-- operator 角色：知识库读 + 创建 + 编辑（不可删除）
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000019'),
('00000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000020'),
('00000000-0000-0000-0000-000000000002', '20000000-0000-0000-0000-000000000021');

-- viewer 角色：知识库只读
INSERT IGNORE INTO role_permissions (role_id, permission_id) VALUES
('00000000-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000019');

-- ---- 知识条目表 ----
CREATE TABLE IF NOT EXISTS knowledge_items (
    id                CHAR(36)      NOT NULL COMMENT '知识条目 ID (UUID)',
    title             VARCHAR(256)  NOT NULL COMMENT '标题',
    category          VARCHAR(64)   NOT NULL DEFAULT 'general' COMMENT '分类：database/linux/network/middleware/general 等',
    tags              JSON          NOT NULL COMMENT '标签数组，如 ["mysql", "主从延迟"]',
    content           LONGTEXT      NOT NULL COMMENT 'Markdown 正文',
    content_text      LONGTEXT      NULL COMMENT '纯文本（去除 Markdown 标记，用于全文检索和未来 embedding）',
    summary           VARCHAR(512)  NULL COMMENT '摘要（列表展示和 AI 推荐用）',
    -- AI 扩展预留：Phase 7 向量检索使用，当前始终为 NULL
    embedding         LONGBLOB      NULL COMMENT '向量嵌入（Phase 7 AI 语义检索用，当前预留）',
    embedding_model   VARCHAR(64)   NULL COMMENT '生成 embedding 的模型名称',
    embedding_updated_at VARCHAR(64) NULL COMMENT 'embedding 最后更新时间',
    status            VARCHAR(16)   NOT NULL DEFAULT 'published' COMMENT 'draft/published/archived',
    view_count        INT           NOT NULL DEFAULT 0 COMMENT '查看次数',
    helpful_count     INT           NOT NULL DEFAULT 0 COMMENT '认为有帮助的次数',
    version           INT           NOT NULL DEFAULT 1 COMMENT '版本号，每次编辑 +1',
    created_by        CHAR(36)      NOT NULL COMMENT '创建者 user.id',
    created_by_name   VARCHAR(128)  NULL COMMENT '创建者用户名（冗余，展示用）',
    updated_at        VARCHAR(64)   NOT NULL,
    created_at        VARCHAR(64)   NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_knowledge_category (category),
    INDEX idx_knowledge_status (status),
    INDEX idx_knowledge_created (created_at),
    INDEX idx_knowledge_created_by (created_by),
    FULLTEXT INDEX ft_knowledge_search (title, content_text) COMMENT '全文检索（MySQL 5.7+ ngram）'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='运维知识库条目';

-- ---- 知识版本历史表（追踪每次修改）----
CREATE TABLE IF NOT EXISTS knowledge_versions (
    id              CHAR(36)      NOT NULL COMMENT '版本记录 ID',
    knowledge_id    CHAR(36)      NOT NULL COMMENT '关联知识条目 ID',
    version         INT           NOT NULL COMMENT '版本号',
    title           VARCHAR(256)  NOT NULL COMMENT '该版本标题',
    content         LONGTEXT      NOT NULL COMMENT '该版本正文',
    tags            JSON          NOT NULL COMMENT '该版本标签',
    edited_by       CHAR(36)      NOT NULL COMMENT '编辑者 user.id',
    edited_by_name  VARCHAR(128)  NULL COMMENT '编辑者用户名',
    created_at      VARCHAR(64)   NOT NULL,
    PRIMARY KEY (id),
    INDEX idx_knowledge_ver_kid (knowledge_id),
    INDEX idx_knowledge_ver_ver (knowledge_id, version)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='知识条目版本历史';
