-- 修复 audit_logs.detail 列类型：JSON 改为 TEXT，与 Rust Option<String> 兼容
ALTER TABLE audit_logs MODIFY COLUMN detail TEXT NULL;