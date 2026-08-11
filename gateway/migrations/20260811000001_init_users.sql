-- 用户表：MeridianOps 自身账号体系（阶段一：admin/operator/viewer 三级固定角色）
-- id 用 CHAR(36) UUID，与 AxleOps/Eventide 对齐
-- created_at/updated_at 用 VARCHAR(64) RFC3339，避免 MySQL DATETIME 时区坑
CREATE TABLE users (
    id              CHAR(36)     NOT NULL,
    username        VARCHAR(128) NOT NULL,
    display_name    VARCHAR(255) NOT NULL DEFAULT '',
    email           VARCHAR(255) NOT NULL DEFAULT '',
    password_hash   TEXT         NOT NULL,
    role            VARCHAR(32)  NOT NULL DEFAULT 'viewer',
    enabled         TINYINT      NOT NULL DEFAULT 1,
    created_at      VARCHAR(64)  NOT NULL,
    updated_at      VARCHAR(64)  NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uk_users_username (username),
    KEY idx_users_role (role)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
