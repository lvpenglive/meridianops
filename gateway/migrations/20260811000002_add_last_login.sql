-- 用户表追加 last_login_at：登录成功时更新，用于用户管理页"最后登录"列与审计
-- 可空（种子 admin 首次登录前为 NULL）
ALTER TABLE users ADD COLUMN last_login_at VARCHAR(64) NULL AFTER enabled;
