-- 20260817000027: 丰富运维样例数据
-- 1) 5 个银行运维作业定义
-- 2) 12 次作业执行记录（含成功/部分成功/失败，过去 3 天）
-- 3) 40+ 个执行子任务（按主机粒度）
-- 4) 20 条审计日志（覆盖登录/创建/更新/删除/执行等操作）
-- 5) 4 篇运维知识库文章

-- ===== 1. 作业定义 =====
INSERT INTO job_definitions (name, description, script_type, script_content, timeout_secs, target_scope, target_asset_ids, run_as, port, enabled, created_by, executor_type, credential_id) VALUES
('核心系统日终巡检', '检查核心系统关键进程、磁盘空间、日志错误', 'shell',
 '#!/bin/bash\necho "=== 核心系统日终巡检 ==="\necho "[1] 关键进程检查"\nps aux | grep -E "(core|batch|clear)" | grep -v grep | head -10\necho "[2] 磁盘空间"\ndf -h | grep -E "^/dev|Filesystem"\necho "[3] 错误日志扫描(最近1小时)"\nfind /var/log -name "*.log" -mmin -60 -exec grep -l "ERROR\|FATAL" {} \\; 2>/dev/null\necho "=== 巡检完成 ==="',
 300, 'manual', '["ins-host-core-app1","ins-host-core-app2","ins-host-core-db1","ins-host-core-db2","ins-host-core-web1"]',
 'root', 22, 1, 'admin', 'mock', 1),

('数据库慢查询检查', '采集 MySQL 慢查询日志，统计 TOP 10 慢查询', 'python',
 'import subprocess\nimport re\n\nprint("=== MySQL 慢查询检查 ===")\ntry:\n    result = subprocess.run(\n        ["mysql", "-uroot", "-e", "SHOW VARIABLES LIKE ''slow_query_log%''"],\n        capture_output=True, text=True, timeout=10\n    )\n    print(result.stdout)\n    result2 = subprocess.run(\n        ["mysqldumpslow", "-s", "t", "-t", "10", "/var/log/mysql/slow.log"],\n        capture_output=True, text=True, timeout=10\n    )\n    print(result2.stdout)\nexcept Exception as e:\n    print(f"检查完成: {e}")\nprint("=== 慢查询检查完成 ===")',
 120, 'manual', '["ins-host-core-db1","ins-host-core-db2"]',
 'root', 22, 1, 'admin', 'mock', 1),

('中间件健康检查', '检查 WebLogic/WLS 中间件运行状态和 JVM 堆使用', 'shell',
 '#!/bin/bash\necho "=== 中间件健康检查 ==="\nfor i in $(seq 1 4); do\n  echo "[节点$i] WLS-$i 状态:"\n  curl -s -o /dev/null -w "HTTP %{http_code} | %{time_total}s" http://wls$i:7001/health 2>/dev/null || echo "连接失败"\n  echo ""\ndone\necho "=== JVM 堆使用 ==="\nps -C java -o pid,%mem,vsz,rss --sort=-rss | head -5\necho "=== 中间件检查完成 ==="',
 180, 'manual', '["ins-mw-core-wl01","ins-mw-core-wl02","ins-mw-core-wl03","ins-mw-core-wl04"]',
 'root', 22, 1, 'admin', 'mock', 1),

('安全补丁核查', '检查关键主机未安装的安全补丁', 'shell',
 '#!/bin/bash\necho "=== 安全补丁核查 ==="\necho "[1] 系统版本"\ncat /etc/redhat-release 2>/dev/null || cat /etc/os-release 2>/dev/null | head -3\necho "[2] 待安装补丁"\nyum check-update --security 2>/dev/null | grep -c "update" || echo "0"\necho "[3] 最近安装补丁"\nrpm -qa --last | head -10\necho "=== 补丁核查完成 ==="',
 200, 'manual', '["ins-host-core-app1","ins-host-core-app2","ins-host-core-web1","ins-host-core-web2"]',
 'root', 22, 1, 'admin', 'mock', 1),

('日志清理与归档', '清理超过 7 天的日志并归档到备份目录', 'shell',
 '#!/bin/bash\necho "=== 日志清理与归档 ==="\nARCHIVE_DIR="/data/archive/logs"\nmkdir -p $ARCHIVE_DIR\necho "[1] 扫描大日志文件"\nfind /var/log -name "*.log" -size +500M -exec ls -lh {} \\; 2>/dev/null\necho "[2] 归档 7 天前的日志"\nfind /var/log -name "*.log.*" -mtime +7 -exec gzip -c {} > $ARCHIVE_DIR/$(basename {}).gz \\; -exec rm {} \\; 2>/dev/null\necho "[3] 清理完成"\ndf -h /var/log 2>/dev/null\necho "=== 日志清理完成 ==="',
 300, 'manual', '["ins-host-core-app1","ins-host-core-db1"]',
 'root', 22, 1, 'admin', 'mock', 1);

-- ===== 2. 作业执行记录 (过去 3 天, 含成功/部分成功/失败) =====
-- job_id 2=核心系统日终巡检, 3=数据库慢查询, 4=中间件健康检查, 5=安全补丁核查, 6=日志清理

INSERT INTO job_runs (job_id, job_name, script_type, script_content, trigger_mode, target_count, success_count, failed_count, overall_status, started_by, started_at, finished_at) VALUES
-- Day -3 (2天前)
(2, '核心系统日终巡检', 'shell', '#!/bin/bash\necho "=== 核心系统日终巡检 ==="', 'cron', 5, 5, 0, 'success', 'system', '2026-08-14 22:00:00', '2026-08-14 22:02:30'),
(4, '中间件健康检查', 'shell', '#!/bin/bash\necho "=== 中间件健康检查 ==="', 'cron', 4, 4, 0, 'success', 'system', '2026-08-14 22:05:00', '2026-08-14 22:06:12'),
-- Day -2 (1天前)
(2, '核心系统日终巡检', 'shell', '#!/bin/bash\necho "=== 核心系统日终巡检 ==="', 'cron', 5, 5, 0, 'success', 'system', '2026-08-15 22:00:00', '2026-08-15 22:02:15'),
(4, '中间件健康检查', 'shell', '#!/bin/bash\necho "=== 中间件健康检查 ==="', 'cron', 4, 3, 1, 'partial', 'system', '2026-08-15 22:05:00', '2026-08-15 22:06:48'),
(5, '安全补丁核查', 'shell', '#!/bin/bash\necho "=== 安全补丁核查 ==="', 'manual', 4, 4, 0, 'success', 'admin', '2026-08-15 14:30:00', '2026-08-15 14:31:22'),
(3, '数据库慢查询检查', 'python', 'import subprocess\nprint("=== MySQL 慢查询检查 ===")', 'manual', 2, 2, 0, 'success', 'admin', '2026-08-15 15:00:00', '2026-08-15 15:00:45'),
-- Day -1 (今天)
(2, '核心系统日终巡检', 'shell', '#!/bin/bash\necho "=== 核心系统日终巡检 ==="', 'cron', 5, 4, 1, 'partial', 'system', '2026-08-16 22:00:00', '2026-08-16 22:03:10'),
(4, '中间件健康检查', 'shell', '#!/bin/bash\necho "=== 中间件健康检查 ==="', 'cron', 4, 4, 0, 'success', 'system', '2026-08-16 22:05:00', '2026-08-16 22:06:20'),
(5, '安全补丁核查', 'shell', '#!/bin/bash\necho "=== 安全补丁核查 ==="', 'manual', 4, 3, 1, 'partial', 'admin', '2026-08-16 10:00:00', '2026-08-16 10:01:35'),
(6, '日志清理与归档', 'shell', '#!/bin/bash\necho "=== 日志清理与归档 ==="', 'manual', 2, 2, 0, 'success', 'admin', '2026-08-16 11:00:00', '2026-08-16 11:02:00'),
(3, '数据库慢查询检查', 'python', 'import subprocess\nprint("=== MySQL 慢查询检查 ===")', 'manual', 2, 1, 1, 'failed', 'admin', '2026-08-16 16:00:00', '2026-08-16 16:01:12'),
-- Day 0 (今天 - 已有 run id=1 的 test-echo-job 不重复)
(2, '核心系统日终巡检', 'shell', '#!/bin/bash\necho "=== 核心系统日终巡检 ==="', 'cron', 5, 5, 0, 'success', 'system', '2026-08-17 06:00:00', '2026-08-17 06:02:25');

-- ===== 3. 作业执行子任务 =====
-- Run 2: 核心系统日终巡检 Day-3 (全成功)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(2, 'ins-host-core-app1', '核心应用-APP01', '10.1.1.11', 'success', 0, '[22:00:01] 执行巡检...\n[22:00:02] 进程检查: core-batch(正常) core-clear(正常) core-sched(正常)\n[22:00:05] 磁盘: /dev/sda1 45% /dev/sdb1 32%\n[22:00:08] 错误日志: 无\n[22:02:25] 巡检完成', '', 2500, '2026-08-14 22:00:01', '2026-08-14 22:00:03'),
(2, 'ins-host-core-app2', '核心应用-APP02', '10.1.1.12', 'success', 0, '[22:00:01] 执行巡检...\n[22:00:02] 进程检查: core-batch(正常) core-clear(正常)\n[22:00:05] 磁盘: /dev/sda1 38%\n[22:00:08] 错误日志: 无\n[22:02:25] 巡检完成', '', 2300, '2026-08-14 22:00:01', '2026-08-14 22:00:03'),
(2, 'ins-host-core-db1', '核心DB-NODE01', '10.1.1.21', 'success', 0, '[22:00:01] 执行巡检...\n[22:00:03] MySQL进程: 正常(线程数 152)\n[22:00:05] 磁盘: /dev/sda1 61% /dev/sdb1 44%\n[22:00:08] 错误日志: 无\n[22:02:25] 巡检完成', '', 2800, '2026-08-14 22:00:01', '2026-08-14 22:00:04'),
(2, 'ins-host-core-db2', '核心DB-NODE02', '10.1.1.22', 'success', 0, '[22:00:01] 执行巡检...\n[22:00:03] MySQL进程: 正常(线程数 148)\n[22:00:05] 磁盘: /dev/sda1 58%\n[22:00:08] 错误日志: 无\n[22:02:25] 巡检完成', '', 2600, '2026-08-14 22:00:01', '2026-08-14 22:00:04'),
(2, 'ins-host-core-web1', '核心WEB-01', '10.1.1.31', 'success', 0, '[22:00:01] 执行巡检...\n[22:00:02] Nginx进程: 正常(worker 8)\n[22:00:05] 磁盘: /dev/sda1 42%\n[22:02:25] 巡检完成', '', 2100, '2026-08-14 22:00:01', '2026-08-14 22:00:03');

-- Run 3: 中间件健康检查 Day-3 (全成功)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(3, 'ins-mw-core-wl01', '核心-WLS-01', '10.1.2.1', 'success', 0, '[22:05:01] WLS-01 健康检查\nHTTP 200 | 0.045s\nJVM堆: 已用 2.1G/4G (52%)\n[22:06:10] 检查完成', '', 1200, '2026-08-14 22:05:01', '2026-08-14 22:05:02'),
(3, 'ins-mw-core-wl02', '核心-WLS-02', '10.1.2.2', 'success', 0, '[22:05:01] WLS-02 健康检查\nHTTP 200 | 0.052s\nJVM堆: 已用 1.8G/4G (45%)\n[22:06:10] 检查完成', '', 1100, '2026-08-14 22:05:01', '2026-08-14 22:05:02'),
(3, 'ins-mw-core-wl03', '核心-WLS-03', '10.1.2.3', 'success', 0, '[22:05:01] WLS-03 健康检查\nHTTP 200 | 0.038s\nJVM堆: 已用 2.4G/4G (60%)\n[22:06:10] 检查完成', '', 1300, '2026-08-14 22:05:01', '2026-08-14 22:05:02'),
(3, 'ins-mw-core-wl04', '核心-WLS-04', '10.1.2.4', 'success', 0, '[22:05:01] WLS-04 健康检查\nHTTP 200 | 0.041s\nJVM堆: 已用 1.5G/4G (38%)\n[22:06:10] 检查完成', '', 1000, '2026-08-14 22:05:01', '2026-08-14 22:05:02');

-- Run 4: 核心系统日终巡检 Day-2 (全成功)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(4, 'ins-host-core-app1', '核心应用-APP01', '10.1.1.11', 'success', 0, '[22:00:01] 执行巡检...\n进程检查: 正常\n磁盘: /dev/sda1 47%\n错误日志: 无\n巡检完成', '', 2400, '2026-08-15 22:00:01', '2026-08-15 22:00:03'),
(4, 'ins-host-core-app2', '核心应用-APP02', '10.1.1.12', 'success', 0, '[22:00:01] 执行巡检...\n进程检查: 正常\n磁盘: /dev/sda1 40%\n错误日志: 无\n巡检完成', '', 2200, '2026-08-15 22:00:01', '2026-08-15 22:00:03'),
(4, 'ins-host-core-db1', '核心DB-NODE01', '10.1.1.21', 'success', 0, '[22:00:01] 执行巡检...\nMySQL进程: 正常(线程数 165)\n磁盘: /dev/sda1 63%\n错误日志: 无\n巡检完成', '', 2700, '2026-08-15 22:00:01', '2026-08-15 22:00:04'),
(4, 'ins-host-core-db2', '核心DB-NODE02', '10.1.1.22', 'success', 0, '[22:00:01] 执行巡检...\nMySQL进程: 正常(线程数 159)\n磁盘: /dev/sda1 60%\n错误日志: 无\n巡检完成', '', 2500, '2026-08-15 22:00:01', '2026-08-15 22:00:03'),
(4, 'ins-host-core-web1', '核心WEB-01', '10.1.1.31', 'success', 0, '[22:00:01] 执行巡检...\nNginx进程: 正常(worker 8)\n磁盘: /dev/sda1 44%\n巡检完成', '', 2000, '2026-08-15 22:00:01', '2026-08-15 22:00:03');

-- Run 5: 中间件健康检查 Day-2 (部分成功 - WLS-04 超时)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(5, 'ins-mw-core-wl01', '核心-WLS-01', '10.1.2.1', 'success', 0, '[22:05:01] WLS-01 健康检查\nHTTP 200 | 0.048s\nJVM堆: 已用 2.2G/4G (55%)\n检查完成', '', 1150, '2026-08-15 22:05:01', '2026-08-15 22:05:02'),
(5, 'ins-mw-core-wl02', '核心-WLS-02', '10.1.2.2', 'success', 0, '[22:05:01] WLS-02 健康检查\nHTTP 200 | 0.055s\nJVM堆: 已用 1.9G/4G (48%)\n检查完成', '', 1200, '2026-08-15 22:05:01', '2026-08-15 22:05:02'),
(5, 'ins-mw-core-wl03', '核心-WLS-03', '10.1.2.3', 'success', 0, '[22:05:01] WLS-03 健康检查\nHTTP 200 | 0.042s\nJVM堆: 已用 2.5G/4G (63%)\n检查完成', '', 1300, '2026-08-15 22:05:01', '2026-08-15 22:05:02'),
(5, 'ins-mw-core-wl04', '核心-WLS-04', '10.1.2.4', 'failed', 1, '', '连接超时: curl: (28) Connection timed out after 10 seconds\nWLS-04 管理端口不可达，请检查 WLS 进程状态', 10000, '2026-08-15 22:05:01', '2026-08-15 22:05:11');

-- Run 6: 安全补丁核查 Day-2 (全成功)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(6, 'ins-host-core-app1', '核心应用-APP01', '10.1.1.11', 'success', 0, '[14:30:01] 安全补丁核查\n系统: CentOS 7.9\n待安装补丁: 0\n最近安装: kernel-3.10.0-1160.95.1 (2026-08-10)\n核查完成', '', 1500, '2026-08-15 14:30:01', '2026-08-15 14:30:02'),
(6, 'ins-host-core-app2', '核心应用-APP02', '10.1.1.12', 'success', 0, '[14:30:01] 安全补丁核查\n系统: CentOS 7.9\n待安装补丁: 2\n  - RHSA-2026:1234 openssl\n  - RHSA-2026:1235 curl\n核查完成', '', 1600, '2026-08-15 14:30:01', '2026-08-15 14:30:02'),
(6, 'ins-host-core-web1', '核心WEB-01', '10.1.1.31', 'success', 0, '[14:30:01] 安全补丁核查\n系统: CentOS 7.9\n待安装补丁: 0\n核查完成', '', 1400, '2026-08-15 14:30:01', '2026-08-15 14:30:02'),
(6, 'ins-host-core-web2', '核心WEB-02', '10.1.1.32', 'success', 0, '[14:30:01] 安全补丁核查\n系统: CentOS 7.9\n待安装补丁: 1\n  - RHSA-2026:1236 nginx\n核查完成', '', 1500, '2026-08-15 14:30:01', '2026-08-15 14:30:02');

-- Run 7: 数据库慢查询检查 Day-2 (全成功)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(7, 'ins-host-core-db1', '核心DB-NODE01', '10.1.1.21', 'success', 0, '=== MySQL 慢查询检查 ===\nslow_query_log: ON\n慢查询 TOP 3:\n1. SELECT * FROM t_account WHERE... (3.2s, 出现 15 次)\n2. UPDATE t_balance SET... (2.8s, 出现 8 次)\n3. INSERT INTO t_journal... (2.1s, 出现 23 次)\n检查完成', '', 1800, '2026-08-15 15:00:01', '2026-08-15 15:00:03'),
(7, 'ins-host-core-db2', '核心DB-NODE02', '10.1.1.22', 'success', 0, '=== MySQL 慢查询检查 ===\nslow_query_log: ON\n慢查询 TOP 3:\n1. SELECT COUNT(*) FROM t_journal... (2.5s, 出现 12 次)\n2. SELECT * FROM t_account WHERE... (1.9s, 出现 6 次)\n检查完成', '', 1500, '2026-08-15 15:00:01', '2026-08-15 15:00:02');

-- Run 8: 核心系统日终巡检 Day-1 (部分成功 - DB2 磁盘告警)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(8, 'ins-host-core-app1', '核心应用-APP01', '10.1.1.11', 'success', 0, '[22:00:01] 执行巡检...\n进程: 正常\n磁盘: /dev/sda1 49%\n错误日志: 无\n巡检完成', '', 2300, '2026-08-16 22:00:01', '2026-08-16 22:00:03'),
(8, 'ins-host-core-app2', '核心应用-APP02', '10.1.1.12', 'success', 0, '[22:00:01] 执行巡检...\n进程: 正常\n磁盘: /dev/sda1 42%\n错误日志: 无\n巡检完成', '', 2100, '2026-08-16 22:00:01', '2026-08-16 22:00:03'),
(8, 'ins-host-core-db1', '核心DB-NODE01', '10.1.1.21', 'success', 0, '[22:00:01] 执行巡检...\nMySQL: 正常(线程 170)\n磁盘: /dev/sda1 65% /dev/sdb1 48%\n错误日志: 无\n巡检完成', '', 2600, '2026-08-16 22:00:01', '2026-08-16 22:00:04'),
(8, 'ins-host-core-db2', '核心DB-NODE02', '10.1.1.22', 'failed', 1, '[22:00:01] 执行巡检...\nMySQL: 正常(线程 162)\n磁盘: /dev/sda1 85% [WARNING] 磁盘使用率超过 80%\n', '磁盘告警: /dev/sda1 使用率 85%，已超过阈值 80%，请及时清理', 2400, '2026-08-16 22:00:01', '2026-08-16 22:00:03'),
(8, 'ins-host-core-web1', '核心WEB-01', '10.1.1.31', 'success', 0, '[22:00:01] 执行巡检...\nNginx: 正常\n磁盘: /dev/sda1 46%\n巡检完成', '', 1900, '2026-08-16 22:00:01', '2026-08-16 22:00:03');

-- Run 9: 中间件健康检查 Day-1 (全成功)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(9, 'ins-mw-core-wl01', '核心-WLS-01', '10.1.2.1', 'success', 0, '[22:05:01] WLS-01 健康检查\nHTTP 200 | 0.043s\nJVM堆: 已用 2.3G/4G (58%)\n检查完成', '', 1100, '2026-08-16 22:05:01', '2026-08-16 22:05:02'),
(9, 'ins-mw-core-wl02', '核心-WLS-02', '10.1.2.2', 'success', 0, '[22:05:01] WLS-02 健康检查\nHTTP 200 | 0.050s\nJVM堆: 已用 2.0G/4G (50%)\n检查完成', '', 1200, '2026-08-16 22:05:01', '2026-08-16 22:05:02'),
(9, 'ins-mw-core-wl03', '核心-WLS-03', '10.1.2.3', 'success', 0, '[22:05:01] WLS-03 健康检查\nHTTP 200 | 0.040s\nJVM堆: 已用 2.6G/4G (65%)\n检查完成', '', 1300, '2026-08-16 22:05:01', '2026-08-16 22:05:02'),
(9, 'ins-mw-core-wl04', '核心-WLS-04', '10.1.2.4', 'success', 0, '[22:05:01] WLS-04 健康检查\nHTTP 200 | 0.039s\nJVM堆: 已用 1.6G/4G (40%)\n检查完成', '', 1000, '2026-08-16 22:05:01', '2026-08-16 22:05:02');

-- Run 10: 安全补丁核查 Day-1 (部分成功)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(10, 'ins-host-core-app1', '核心应用-APP01', '10.1.1.11', 'success', 0, '[10:00:01] 安全补丁核查\n待安装补丁: 0\n核查完成', '', 1400, '2026-08-16 10:00:01', '2026-08-16 10:00:02'),
(10, 'ins-host-core-app2', '核心应用-APP02', '10.1.1.12', 'success', 0, '[10:00:01] 安全补丁核查\n待安装补丁: 2 (openssl, curl)\n核查完成', '', 1500, '2026-08-16 10:00:01', '2026-08-16 10:00:02'),
(10, 'ins-host-core-web1', '核心WEB-01', '10.1.1.31', 'success', 0, '[10:00:01] 安全补丁核查\n待安装补丁: 0\n核查完成', '', 1300, '2026-08-16 10:00:01', '2026-08-16 10:00:02'),
(10, 'ins-host-core-web2', '核心WEB-02', '10.1.1.32', 'failed', 1, '', 'SSH连接失败: Permission denied (publickey,password)\n请检查凭据或主机访问策略', 5000, '2026-08-16 10:00:01', '2026-08-16 10:00:06');

-- Run 11: 日志清理与归档 (全成功)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(11, 'ins-host-core-app1', '核心应用-APP01', '10.1.1.11', 'success', 0, '[11:00:01] 日志清理与归档\n扫描大日志: /var/log/app/core.log (1.2G)\n归档 7 天前日志: 3 个文件\n清理前: 85% -> 清理后: 82%\n清理完成', '', 50000, '2026-08-16 11:00:01', '2026-08-16 11:00:51'),
(11, 'ins-host-core-db1', '核心DB-NODE01', '10.1.1.21', 'success', 0, '[11:00:01] 日志清理与归档\n扫描大日志: /var/log/mysql/slow.log (800M)\n归档 7 天前日志: 2 个文件\n清理前: 65% -> 清理后: 63%\n清理完成', '', 65000, '2026-08-16 11:00:01', '2026-08-16 11:01:06');

-- Run 12: 数据库慢查询检查 Day-1 (失败 - DB2 连接失败)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(12, 'ins-host-core-db1', '核心DB-NODE01', '10.1.1.21', 'success', 0, '=== MySQL 慢查询检查 ===\nslow_query_log: ON\n慢查询 TOP 3:\n1. SELECT * FROM t_journal WHERE... (3.5s, 出现 18 次)\n2. UPDATE t_balance SET... (2.9s, 出现 10 次)\n检查完成', '', 1700, '2026-08-16 16:00:01', '2026-08-16 16:00:03'),
(12, 'ins-host-core-db2', '核心DB-NODE02', '10.1.1.22', 'failed', 1, '', 'MySQL连接失败: Can''t connect to MySQL server on 10.1.1.22:3306 (Connection refused)\n请检查 MySQL 服务状态', 3000, '2026-08-16 16:00:01', '2026-08-16 16:00:04');

-- Run 13: 核心系统日终巡检 Day 0 (全成功)
INSERT INTO job_run_targets (job_run_id, asset_id, asset_name, asset_ip, status, exit_code, stdout, stderr, duration_ms, started_at, finished_at) VALUES
(13, 'ins-host-core-app1', '核心应用-APP01', '10.1.1.11', 'success', 0, '[06:00:01] 执行巡检...\n进程: 正常\n磁盘: /dev/sda1 51%\n错误日志: 无\n巡检完成', '', 2200, '2026-08-17 06:00:01', '2026-08-17 06:00:03'),
(13, 'ins-host-core-app2', '核心应用-APP02', '10.1.1.12', 'success', 0, '[06:00:01] 执行巡检...\n进程: 正常\n磁盘: /dev/sda1 44%\n错误日志: 无\n巡检完成', '', 2000, '2026-08-17 06:00:01', '2026-08-17 06:00:03'),
(13, 'ins-host-core-db1', '核心DB-NODE01', '10.1.1.21', 'success', 0, '[06:00:01] 执行巡检...\nMySQL: 正常(线程 175)\n磁盘: /dev/sda1 67%\n错误日志: 无\n巡检完成', '', 2500, '2026-08-17 06:00:01', '2026-08-17 06:00:03'),
(13, 'ins-host-core-db2', '核心DB-NODE02', '10.1.1.22', 'success', 0, '[06:00:01] 执行巡检...\nMySQL: 正常(线程 168)\n磁盘: /dev/sda1 83% [注意] 磁盘使用率较高\n巡检完成', '', 2400, '2026-08-17 06:00:01', '2026-08-17 06:00:03'),
(13, 'ins-host-core-web1', '核心WEB-01', '10.1.1.31', 'success', 0, '[06:00:01] 执行巡检...\nNginx: 正常\n磁盘: /dev/sda1 48%\n巡检完成', '', 1800, '2026-08-17 06:00:01', '2026-08-17 06:00:03');

-- ===== 4. 审计日志 (20 条, 覆盖各种操作) =====
INSERT INTO audit_logs (actor_username, action, target_type, target_id, detail, ip, status, created_at) VALUES
('admin', 'create_job_def', 'job_definition', '2', '{"id":2,"name":"核心系统日终巡检","scriptType":"shell","timeoutSecs":300}', '127.0.0.1', 'success', '2026-08-14 10:00:00'),
('admin', 'create_job_def', 'job_definition', '3', '{"id":3,"name":"数据库慢查询检查","scriptType":"python","timeoutSecs":120}', '127.0.0.1', 'success', '2026-08-14 10:05:00'),
('admin', 'create_job_def', 'job_definition', '4', '{"id":4,"name":"中间件健康检查","scriptType":"shell","timeoutSecs":180}', '127.0.0.1', 'success', '2026-08-14 10:10:00'),
('admin', 'create_job_def', 'job_definition', '5', '{"id":5,"name":"安全补丁核查","scriptType":"shell","timeoutSecs":200}', '127.0.0.1', 'success', '2026-08-14 10:15:00'),
('admin', 'create_job_def', 'job_definition', '6', '{"id":6,"name":"日志清理与归档","scriptType":"shell","timeoutSecs":300}', '127.0.0.1', 'success', '2026-08-14 10:20:00'),
('system', 'execute_job', 'job_definition', '2', '{"jobName":"核心系统日终巡检","jobRunId":2,"triggerMode":"cron","assetCount":5}', '127.0.0.1', 'success', '2026-08-14 22:00:00'),
('system', 'execute_job', 'job_definition', '4', '{"jobName":"中间件健康检查","jobRunId":3,"triggerMode":"cron","assetCount":4}', '127.0.0.1', 'success', '2026-08-14 22:05:00'),
('admin', 'login', 'user', '5507e15d-481e-4dc2-b66b-5b6b87632af9', '{"role":"admin","passwordExpired":false}', '127.0.0.1', 'success', '2026-08-15 09:00:00'),
('admin', 'execute_job', 'job_definition', '5', '{"jobName":"安全补丁核查","jobRunId":6,"triggerMode":"manual","assetCount":4}', '127.0.0.1', 'success', '2026-08-15 14:30:00'),
('admin', 'execute_job', 'job_definition', '3', '{"jobName":"数据库慢查询检查","jobRunId":7,"triggerMode":"manual","assetCount":2}', '127.0.0.1', 'success', '2026-08-15 15:00:00'),
('system', 'execute_job', 'job_definition', '2', '{"jobName":"核心系统日终巡检","jobRunId":4,"triggerMode":"cron","assetCount":5}', '127.0.0.1', 'success', '2026-08-15 22:00:00'),
('system', 'execute_job', 'job_definition', '4', '{"jobName":"中间件健康检查","jobRunId":5,"triggerMode":"cron","assetCount":4}', '127.0.0.1', 'success', '2026-08-15 22:05:00'),
('admin', 'login', 'user', '5507e15d-481e-4dc2-b66b-5b6b87632af9', '{"role":"admin","passwordExpired":false}', '127.0.0.1', 'success', '2026-08-16 09:30:00'),
('admin', 'execute_job', 'job_definition', '5', '{"jobName":"安全补丁核查","jobRunId":10,"triggerMode":"manual","assetCount":4}', '127.0.0.1', 'success', '2026-08-16 10:00:00'),
('admin', 'execute_job', 'job_definition', '6', '{"jobName":"日志清理与归档","jobRunId":11,"triggerMode":"manual","assetCount":2}', '127.0.0.1', 'success', '2026-08-16 11:00:00'),
('admin', 'update', 'ci_instance', 'ins-host-core-db2', '{"field":"status","oldValue":"active","newValue":"warning","reason":"磁盘使用率85%"}', '127.0.0.1', 'success', '2026-08-16 22:01:00'),
('admin', 'execute_job', 'job_definition', '3', '{"jobName":"数据库慢查询检查","jobRunId":12,"triggerMode":"manual","assetCount":2}', '127.0.0.1', 'success', '2026-08-16 16:00:00'),
('admin', 'update', 'ci_instance', 'ins-mw-core-wl04', '{"field":"status","oldValue":"running","newValue":"stopped","reason":"WLS-04管理端口不可达"}', '127.0.0.1', 'success', '2026-08-15 22:06:00'),
('admin', 'login', 'user', '5507e15d-481e-4dc2-b66b-5b6b87632af9', '{"role":"admin","passwordExpired":false}', '127.0.0.1', 'success', '2026-08-17 07:00:00'),
('admin', 'update', 'ci_instance', 'ins-host-core-db2', '{"field":"status","oldValue":"warning","newValue":"active","reason":"磁盘清理后恢复"}', '127.0.0.1', 'success', '2026-08-17 06:05:00');

-- ===== 5. 运维知识库文章 =====
INSERT INTO knowledge_items (id, title, category, tags, content, content_text, summary, status, view_count, helpful_count, version, created_by, created_by_name, updated_at, created_at) VALUES
(UUID(), '核心系统日终巡检标准操作流程', 'linux',
 '["巡检", "日终", "核心系统", "SOP"]',
 '# 核心系统日终巡检标准操作流程\n\n## 1. 巡检时间\n每日 22:00 自动执行，次日 09:00 前确认结果。\n\n## 2. 巡检范围\n- 核心应用服务器 (APP01-02)\n- 核心数据库服务器 (DB01-02)\n- 核心Web服务器 (WEB01-02)\n- 中间件服务器 (WLS01-04)\n\n## 3. 检查项\n### 3.1 进程检查\n确认以下关键进程正常运行：\n- `core-batch` 批处理进程\n- `core-clear` 清算进程\n- `core-sched` 调度进程\n- `mysqld` 数据库进程\n- `nginx` Web服务器\n\n### 3.2 磁盘空间检查\n- 根分区使用率 < 80%\n- 数据分区使用率 < 75%\n- 超过阈值时触发告警\n\n### 3.3 错误日志扫描\n扫描最近 1 小时内的 ERROR/FATAL 级别日志。\n\n## 4. 异常处理\n- 进程异常：尝试自动重启，失败则升级工单\n- 磁盘告警：执行日志清理脚本，仍超则人工介入\n- 错误日志：根据错误类型创建工单',
 '核心系统日终巡检标准操作流程：每日22:00自动执行，检查进程、磁盘空间、错误日志。覆盖核心应用、数据库、Web和中间件服务器。超过阈值触发告警并升级处理。',
 '日终巡检SOP：进程/磁盘/日志三大检查项及异常处理流程',
 'published', 128, 45, 3, '5507e15d-481e-4dc2-b66b-5b6b87632af9', 'admin', '2026-08-15 10:00:00', '2026-08-14 09:00:00'),

(UUID(), 'MySQL慢查询排查指南', 'database',
 '["mysql", "慢查询", "性能优化", "数据库"]',
 '# MySQL慢查询排查指南\n\n## 1. 开启慢查询日志\n```sql\nSET GLOBAL slow_query_log = ON;\nSET GLOBAL long_query_time = 2;\nSET GLOBAL slow_query_log_file = "/var/log/mysql/slow.log";\n```\n\n## 2. 使用 mysqldumpslow 分析\n```bash\n# 按耗时排序 TOP 10\nmysqldumpslow -s t -t 10 /var/log/mysql/slow.log\n\n# 按出现次数排序\nmysqldumpslow -s c -t 10 /var/log/mysql/slow.log\n```\n\n## 3. 常见慢查询优化\n### 3.1 缺少索引\n```sql\n-- 检查执行计划\nEXPLAIN SELECT * FROM t_journal WHERE create_time > "2026-08-01";\n-- 添加索引\nALTER TABLE t_journal ADD INDEX idx_create_time (create_time);\n```\n\n### 3.2 全表扫描\n使用 `EXPLAIN` 检查 `type` 字段，如果是 `ALL` 则为全表扫描。\n\n### 3.3 子查询优化\n将子查询改为 JOIN 提升性能。\n\n## 4. 银行核心场景常见慢查询\n- 账户余额查询：t_account 表数据量大，需确保索引覆盖\n- 交易流水查询：t_journal 表按日期分区\n- 日终汇总统计：避免高峰期执行',
 'MySQL慢查询排查指南：开启慢查询日志、使用mysqldumpslow分析、常见优化方法（索引、全表扫描、子查询）。银行核心场景常见慢查询及优化建议。',
 'MySQL慢查询排查：日志开启、分析工具、索引优化最佳实践',
 'published', 95, 38, 2, '5507e15d-481e-4dc2-b66b-5b6b87632af9', 'admin', '2026-08-16 10:00:00', '2026-08-14 14:00:00'),

(UUID(), 'WebLogic中间件健康检查方法', 'middleware',
 '["weblogic", "wls", "中间件", "健康检查"]',
 '# WebLogic中间件健康检查方法\n\n## 1. 检查项\n### 1.1 管理端口可达性\n```bash\ncurl -s -o /dev/null -w "%{http_code}" http://wls01:7001/health\n```\n期望返回 200。\n\n### 1.2 JVM 堆使用率\n```bash\n# 通过 JMX 获取\njmx-console -url service:jmx:rmi:///jndi/rmi://wls01:7001/jndi/jmx -attr HeapMemoryUsage\n```\n- 堆使用率 > 85% 需关注\n- 堆使用率 > 95% 需立即处理\n\n### 1.3 线程池状态\n检查 `ExecuteThreadCount` 和 `QueueLength`：\n- 队列长度持续增长说明处理能力不足\n\n## 2. 常见问题处理\n### 2.1 管理端口不可达\n1. 检查 WLS 进程是否存活：`ps -ef | grep weblogic`\n2. 检查端口监听：`netstat -tlnp | grep 7001`\n3. 检查防火墙规则\n4. 必要时重启 WLS 服务\n\n### 2.2 JVM OOM\n1. 查看堆内存详情：`jmap -heap <pid>`\n2. 导出堆转储：`jmap -dump:format=b,file=heap.hprof <pid>`\n3. 调整 JVM 参数：`-Xms4g -Xmx4g -XX:+HeapDumpOnOutOfMemoryError`\n\n## 3. 巡检脚本\n使用 MeridianOps 作业中心「中间件健康检查」作业自动执行。',
 'WebLogic中间件健康检查方法：检查管理端口可达性、JVM堆使用率、线程池状态。常见问题处理包括管理端口不可达和JVM OOM排查。可通过作业中心自动化巡检。',
 'WebLogic健康检查：端口可达性、JVM堆、线程池，含故障排查',
 'published', 76, 29, 1, '5507e15d-481e-4dc2-b66b-5b6b87632af9', 'admin', '2026-08-15 16:00:00', '2026-08-15 15:00:00'),

(UUID(), 'Linux磁盘空间告警处理流程', 'linux',
 '["linux", "磁盘", "告警", "运维"]',
 '# Linux磁盘空间告警处理流程\n\n## 1. 告警阈值\n- 使用率 > 80%：WARNING\n- 使用率 > 90%：CRITICAL\n\n## 2. 排查步骤\n### 2.1 确认磁盘使用情况\n```bash\ndf -h\n# 查看大文件\nfind / -type f -size +500M -exec ls -lh {} \\; 2>/dev/null\n```\n\n### 2.2 查找大目录\n```bash\n# 使用 ncdu（推荐）\nncdu /\n\n# 或使用 du\ndu -sh /* 2>/dev/null | sort -rh | head -20\n```\n\n### 2.3 日志文件清理\n```bash\n# 查看日志文件大小\nfind /var/log -name "*.log" -size +100M -exec ls -lh {} \\;\n\n# 归档并清理旧日志\nfind /var/log -name "*.log.*" -mtime +7 -exec gzip -c {} > /data/archive/$(basename {}).gz \\; -exec rm {} \\;\n```\n\n### 2.4 清理系统缓存\n```bash\n# 清理 yum 缓存\nyum clean all\n\n# 清理临时文件\nfind /tmp -type f -mtime +3 -delete\n```\n\n## 3. 自动化处理\n通过 MeridianOps 作业中心「日志清理与归档」作业定期执行：\n- 每日 11:00 自动执行\n- 清理 7 天前的日志文件\n- 归档到 `/data/archive/logs/`\n\n## 4. 银行核心系统特别注意\n- **禁止**直接删除业务日志，必须先归档\n- 清理前确认备份完成\n- 核心数据库磁盘告警优先处理 slow log 和 binlog',
 'Linux磁盘空间告警处理流程：告警阈值80%/90%、排查步骤（df/find/du）、日志清理方法、自动化处理方案。银行核心系统注意事项。',
 '磁盘告警处理：排查大文件、清理日志、自动化归档流程',
 'published', 112, 52, 2, '5507e15d-481e-4dc2-b66b-5b6b87632af9', 'admin', '2026-08-16 14:00:00', '2026-08-14 16:00:00');
