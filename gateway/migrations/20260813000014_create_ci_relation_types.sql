-- CI 关系类型字典表
CREATE TABLE IF NOT EXISTS ci_relation_types (
    id          VARCHAR(36)  NOT NULL,
    code        VARCHAR(32)  NOT NULL,             -- depends_on / contains / runs_on / manages / connects_to
    name        VARCHAR(64)  NOT NULL,             -- 中文名：依赖 / 包含 / 运行于 / 管理 / 连接
    description VARCHAR(255) NOT NULL DEFAULT '',  -- 描述
    directional TINYINT(1)   NOT NULL DEFAULT 1,   -- 是否有方向：1=有向（源→目标），0=无向
    enabled     TINYINT(1)   NOT NULL DEFAULT 1,   -- 是否启用
    sort_order  INT          NOT NULL DEFAULT 0,
    created_at  VARCHAR(64)  NOT NULL,
    updated_at  VARCHAR(64)  NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uk_cireltype_code (code)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 种子数据：5 个内置关系类型
INSERT INTO ci_relation_types (id, code, name, description, directional, enabled, sort_order, created_at, updated_at) VALUES
    ('reltype-depends-on',  'depends_on',  '依赖',   'A 依赖 B 才能正常运行',         1, 1, 1, NOW(3), NOW(3)),
    ('reltype-contains',    'contains',    '包含',   'A 包含 B 作为子组件',           1, 1, 2, NOW(3), NOW(3)),
    ('reltype-runs-on',     'runs_on',     '运行于', 'A 运行在 B 之上（如应用→主机）', 1, 1, 3, NOW(3), NOW(3)),
    ('reltype-manages',     'manages',     '管理',   'A 管理 B（如集群→节点）',       1, 1, 4, NOW(3), NOW(3)),
    ('reltype-connects-to', 'connects_to', '连接',   'A 与 B 存在网络连接',           0, 1, 5, NOW(3), NOW(3))
ON DUPLICATE KEY UPDATE id = id;
