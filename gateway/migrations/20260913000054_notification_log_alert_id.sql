-- 通知发送日志关联告警，便于告警中心查看这条告警有没有发出去。

ALTER TABLE notification_logs
    ADD COLUMN alert_id CHAR(36) NULL COMMENT '关联告警事件 ID' AFTER link;

ALTER TABLE notification_logs
    ADD INDEX idx_log_alert (alert_id);

UPDATE notification_logs
SET alert_id = SUBSTRING_INDEX(SUBSTRING_INDEX(link, 'id=', -1), '&', 1)
WHERE (alert_id IS NULL OR alert_id = '')
  AND link LIKE '%/alerts?id=%'
  AND CHAR_LENGTH(SUBSTRING_INDEX(SUBSTRING_INDEX(link, 'id=', -1), '&', 1)) BETWEEN 8 AND 64;
