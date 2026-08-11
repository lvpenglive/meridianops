-- 登录失败锁定：记录失败次数与锁定到期时间
-- failed_login_attempts 达到阈值后设置 locked_until，期间拒绝登录
-- RFC3339 时间字符串，与现有 created_at 风格一致
ALTER TABLE users
  ADD COLUMN failed_login_attempts INT NOT NULL DEFAULT 0 AFTER enabled,
  ADD COLUMN locked_until VARCHAR(64) NULL DEFAULT NULL AFTER failed_login_attempts;
