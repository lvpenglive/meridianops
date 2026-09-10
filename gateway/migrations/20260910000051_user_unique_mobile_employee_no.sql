-- 用户手机号 / 工号 唯一性约束
-- 背景：手机号是告警短信的触达标识，工号是人事身份标识，均不允许与其他用户重复。
-- 说明：两列均可空；MySQL 唯一索引允许多个 NULL，因此先把历史空串规范化为 NULL，
--       避免 '' 之间互相冲突导致建索引失败。
-- 数据校验：上线前已确认存量手机号(17 个占位号)、工号(均为 NULL)无重复。

UPDATE users SET mobile = NULL WHERE mobile = '';
UPDATE users SET employee_no = NULL WHERE employee_no = '';

-- 原为普通索引，替换为唯一索引
ALTER TABLE users DROP INDEX idx_users_mobile;
ALTER TABLE users ADD UNIQUE KEY uk_users_mobile (mobile);

ALTER TABLE users DROP INDEX idx_users_employee_no;
ALTER TABLE users ADD UNIQUE KEY uk_users_employee_no (employee_no);
