-- 合规补全：密码过期 + 会话超时
-- 1) users 表加 password_changed_at，记录密码最后修改时间（用于判断是否过期）
--    首次迁移时用 created_at 回填，已有用户不会立刻被判定过期
ALTER TABLE users
  ADD COLUMN password_changed_at VARCHAR(64) NULL DEFAULT NULL AFTER password_hash;

UPDATE users SET password_changed_at = created_at WHERE password_changed_at IS NULL;

-- 2) 新增 2 项系统配置：密码过期天数（0=不过期）、会话超时分钟数（0=不超时，仅前端 idle 计时）
INSERT INTO system_settings (setting_key, setting_value, description, updated_at, updated_by) VALUES
('password_expiry_days',        '90', '密码过期天数（0=不过期）',   '2026-08-12T00:00:00Z', 'system'),
('session_timeout_minutes',     '60', '会话超时分钟数（0=不超时）', '2026-08-12T00:00:00Z', 'system');
