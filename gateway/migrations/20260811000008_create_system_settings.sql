-- 系统配置表：键值对存储，支持密码策略、登录锁定参数等运行时可改配置
-- updated_by 记录最后修改人（username），system 表示种子数据
CREATE TABLE system_settings (
    setting_key   VARCHAR(64) PRIMARY KEY,
    setting_value TEXT NOT NULL,
    description   VARCHAR(255) NOT NULL DEFAULT '',
    updated_at    VARCHAR(64) NOT NULL,
    updated_by    VARCHAR(128) NOT NULL DEFAULT ''
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 密码策略 + 登录锁定参数种子
INSERT INTO system_settings (setting_key, setting_value, description, updated_at, updated_by) VALUES
('password_min_length',         '8',  '密码最小长度',         '2026-08-11T00:00:00Z', 'system'),
('password_require_uppercase',  'true', '密码是否需要大写字母', '2026-08-11T00:00:00Z', 'system'),
('password_require_lowercase',  'true', '密码是否需要小写字母', '2026-08-11T00:00:00Z', 'system'),
('password_require_digit',      'true', '密码是否需要数字',     '2026-08-11T00:00:00Z', 'system'),
('password_require_special',    'false','密码是否需要特殊字符', '2026-08-11T00:00:00Z', 'system'),
('login_max_attempts',          '5',  '登录失败最大次数',       '2026-08-11T00:00:00Z', 'system'),
('login_lockout_minutes',       '15', '账号锁定时长（分钟）',  '2026-08-11T00:00:00Z', 'system');
