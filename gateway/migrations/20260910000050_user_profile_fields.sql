-- 用户管理：补充人员基本信息
-- 目的：支撑告警短信/通知落点、审批升级、账号生命周期管理
-- 约定：全部可空，兼容存量数据；employment_status 存量回填 active
-- 时间/日期沿用项目 VARCHAR + 字符串风格，避免 MySQL DATETIME 时区坑

ALTER TABLE users
  ADD COLUMN mobile            VARCHAR(32)  NULL DEFAULT NULL AFTER department_id,
  ADD COLUMN employee_no       VARCHAR(64)  NULL DEFAULT NULL AFTER mobile,
  ADD COLUMN position          VARCHAR(128) NULL DEFAULT NULL AFTER employee_no,
  ADD COLUMN manager_id        CHAR(36)     NULL DEFAULT NULL AFTER position,
  ADD COLUMN im_account        VARCHAR(128) NULL DEFAULT NULL AFTER manager_id,
  ADD COLUMN employment_status VARCHAR(16)  NOT NULL DEFAULT 'active' AFTER im_account,
  ADD COLUMN leave_date        VARCHAR(32)  NULL DEFAULT NULL AFTER employment_status,
  ADD COLUMN remark            VARCHAR(255) NULL DEFAULT NULL AFTER leave_date,
  ADD KEY idx_users_mobile (mobile),
  ADD KEY idx_users_employee_no (employee_no),
  ADD KEY idx_users_manager_id (manager_id);

-- 存量用户默认在职
UPDATE users SET employment_status = 'active' WHERE employment_status IS NULL OR employment_status = '';
