-- fingerprint 建表是 VARCHAR(64)；Eventide 原指纹加 eventide: 前缀后经常超长，
-- 严格模式会拒写，宽松模式会截断导致误合并。扩到 255 与接入代码注释对齐。

ALTER TABLE alert_events
    MODIFY COLUMN fingerprint VARCHAR(255) NOT NULL COMMENT '去重指纹（含接入前缀，最长 255）';
