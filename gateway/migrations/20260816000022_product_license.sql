-- 20260816000022: 添加产品授权到期配置项
-- 用途: 商业化授权 - 限制产品使用时间
-- INSERT IGNORE 保证重复运行无副作用
-- 注意：system_settings 表无 setting_type/created_at 列，只有 updated_at/updated_by

INSERT IGNORE INTO system_settings (setting_key, setting_value, description, updated_at, updated_by) VALUES
(
  'license_edition',
  'Community',
  '产品版本: Community/Enterprise/Ultimate',
  NOW(),
  'system'
),
(
  'license_expires_at',
  '',
  '授权到期时间 (RFC3339 / Y-m-d H:i:s), 空字符串表示永不到期',
  NOW(),
  'system'
),
(
  'license_customer',
  'MeridianOps 测试客户',
  '授权客户名称',
  NOW(),
  'system'
),
(
  'license_activated_at',
  '',
  '授权激活时间, 空字符串表示未激活',
  NOW(),
  'system'
),
(
  'license_key',
  '',
  '授权激活码(脱敏存储)',
  NOW(),
  'system'
);
