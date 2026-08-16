-- ============================================================
-- 2026-08-15: 知识库种子数据（12 条常见运维知识）
-- ============================================================

SET @now = UTC_TIMESTAMP();
SET @admin_id = '00000000-0000-0000-0000-000000000001';
SET @admin_name = 'admin';

-- 1. MySQL 主从延迟告警处理
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), 'MySQL 主从延迟告警处理', 'database', '["mysql","主从延迟","告警"]',
'## 现象
从库 `SHOW SLAVE STATUS` 中 `Seconds_Behind_Master` 持续增大，触发告警。

## 排查步骤
1. 登录从库执行 `SHOW SLAVE STATUS\G`
2. 检查 `Seconds_Behind_Master` 值
3. 检查 `Slave_IO_Running` 和 `Slave_SQL_Running` 是否为 Yes
4. 如果 SQL 线程停止，检查 `Last_Error` 信息

## 处理方案
- **延迟 < 300s**：一般等待自动追平即可，关注大事务
- **延迟 300s-1800s**：
  1. `STOP SLAVE;`
  2. `SET GLOBAL slave_parallel_workers = 4;`
  3. `START SLAVE;`
- **延迟 > 1800s 或无法追平**：
  1. 检查网络带宽是否有异常
  2. 检查是否有大事务（`SHOW PROCESSLIST` 查看长时间运行的 SQL）
  3. 必要时基于主库重新搭建从库

## 预防措施
- 开启并行复制（`slave_parallel_workers`）
- 监控大事务（`max_binlog_size`）
- 避免在业务高峰期执行大批量 DML',
'MySQL主从延迟告警的排查步骤和处理方案，包括并行复制配置和大事务排查',
'published', 1, @admin_id, @admin_name, @now, @now);

-- 2. Linux 磁盘空间不足告警
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), 'Linux 磁盘空间不足告警处理', 'linux', '["linux","磁盘","告警"]',
'## 现象
监控告警 `disk usage > 90%`，需快速释放空间。

## 排查步骤
1. `df -h` 查看各分区使用率
2. `du -sh /* | sort -rh | head -10` 定位大目录
3. `find /var/log -name "*.log" -size +500M` 查找大日志文件

## 快速处理
- **清理日志**：
  ```bash
  # 截断大日志文件（不删除，避免进程仍持有句柄）
  > /var/log/large_app.log
  # 清理 7 天前的日志
  find /var/log -name "*.log.*" -mtime +7 -delete
  ```
- **清理临时文件**：
  ```bash
  rm -rf /tmp/old_*
  rm -rf /var/cache/yum/*
  ```
- **清理 journal 日志**：
  ```bash
  journalctl --vacuum-time=3d
  ```

## 注意事项
- **不要直接 rm 大文件**，进程可能仍持有句柄，空间不会释放
- 用 `> file` 截断或 `echo "" > file` 代替
- 清理后确认 `df -h` 空间已释放',
'Linux磁盘空间不足的排查和快速清理方法，包括日志截断和临时文件清理',
'published', 1, @admin_id, @admin_name, @now, @now);

-- 3. Nginx 502 Bad Gateway
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), 'Nginx 502 Bad Gateway 故障处理', 'middleware', '["nginx","502","故障"]',
'## 现象
用户访问返回 502 Bad Gateway，Nginx error_log 报 `connect() refused`。

## 排查步骤
1. 检查后端服务是否存活：`systemctl status <service>`
2. 检查后端端口是否监听：`ss -tlnp | grep <port>`
3. 检查 Nginx upstream 配置是否正确
4. 查看 Nginx error_log：`tail -50 /var/log/nginx/error.log`

## 常见原因及处理
- **后端进程挂掉**：`systemctl restart <service>`
- **端口不对**：检查 Nginx `proxy_pass` 与后端实际监听端口
- **后端过载**：临时调大 `proxy_connect_timeout` 和 `proxy_read_timeout`
- **防火墙拦截**：`iptables -L -n` 检查规则
- **SELinux**：`getenforce` 检查，临时 `setenforce 0`

## 预防
- 配置 `upstream` 健康检查（`health_check`）
- 设置 `max_fails` 和 `fail_timeout`
- 监控后端进程存活状态',
'Nginx 502故障的排查步骤，涵盖后端进程检查、端口确认、防火墙和SELinux排查',
'published', 1, @admin_id, @admin_name, @now, @now);

-- 4. Redis 内存溢出
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), 'Redis 内存溢出处理', 'middleware', '["redis","内存","OOM"]',
'## 现象
Redis 告警 `used_memory > maxmemory`，部分写入返回 OOM。

## 排查步骤
1. `redis-cli INFO memory` 查看内存使用
2. `redis-cli CONFIG GET maxmemory` 查看上限
3. `redis-cli CONFIG GET maxmemory-policy` 查看淘汰策略
4. `redis-cli --bigkeys` 找出大 key

## 处理方案
- **临时扩容**：`CONFIG SET maxmemory 8gb`
- **调整淘汰策略**：
  ```
  CONFIG SET maxmemory-policy allkeys-lru
  ```
- **清理无用 key**：
  ```bash
  # 扫描大 key
  redis-cli --bigkeys
  # 手动删除指定 key
  redis-cli DEL <key>
  ```
- **检查过期 key 是否正常清理**：`INFO stats` 看 `expired_keys`

## 预防
- 合理设置 TTL
- 避免 bigkey（单个 key > 10MB）
- 监控 `used_memory_rss` 与 `used_memory` 比值（碎片率）',
'Redis内存溢出的排查和处理，包括淘汰策略调整、bigkey清理和TTL管理',
'published', 1, @admin_id, @admin_name, @now, @now);

-- 5. CPU 使用率过高
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), 'Linux CPU 使用率过高排查', 'linux', '["linux","cpu","性能"]',
'## 现象
监控告警 CPU 使用率 > 90%。

## 排查步骤
1. `top -c` 按 P 排序，找 CPU 最高的进程
2. `top -H -p <pid>` 查看该进程的线程 CPU
3. `pidstat -u 1` 持续观察
4. 如果是 Java：`jstack <pid>` 抓线程栈
5. 如果是已知进程：检查是否有异常循环或死锁

## 常见原因
- **业务高峰**：正常负载，考虑扩容
- **死循环 Bug**：`strace -p <pid>` 查看系统调用
- **挖矿木马**：检查异常进程 `ps aux | grep -E "miner|xmrig"`
- **日志风暴**：大量 ERROR 日志拖慢 CPU

## 处理
- 临时：`renice 10 <pid>` 降低优先级
- 紧急：`kill -9 <pid>` 后重启服务
- 根因：分析代码逻辑或抓火焰图（`perf record`）',
'CPU使用率过高的排查方法，使用top/pidstat/strace定位高CPU进程和线程',
'published', 1, @admin_id, @admin_name, @now, @now);

-- 6. SSH 登录失败排查
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), 'SSH 登录失败排查指南', 'linux', '["ssh","登录","安全"]',
'## 现象
SSH 登录被拒绝，提示 `Permission denied` 或 `Connection refused`。

## 排查步骤
1. 确认网络连通：`telnet <ip> 22`
2. 检查 sshd 服务：`systemctl status sshd`
3. 查看安全日志：`tail -50 /var/log/secure`（CentOS）或 `/var/log/auth.log`（Ubuntu）
4. 检查 PAM 配置：`/etc/pam.d/sshd`
5. 检查 hosts.allow/hosts.deny

## 常见原因
- **密码错误**：检查日志 `Failed password`
- **SSH 密钥权限**：
  ```bash
  chmod 700 ~/.ssh
  chmod 600 ~/.ssh/authorized_keys
  ```
- **sshd_config 限制**：
  - `AllowUsers` / `DenyUsers` 配置
  - `PermitRootLogin no`
  - `PasswordAuthentication no`
- **fail2ban 封禁**：`fail2ban-client status sshd`
- **PAM 模块限制**：检查 `/etc/security/limits.conf`

## 处理
- 通过堡垒机或带外管理登录
- `systemctl restart sshd` 重启服务
- 临时关闭 selinux：`setenforce 0`',
'SSH登录失败的排查指南，涵盖网络、sshd配置、密钥权限、fail2ban封禁等常见原因',
'published', 1, @admin_id, @admin_name, @now, @now);

-- 7. 网络不通排查
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), '网络不通排查流程', 'network', '["网络","ping","tcpdump"]',
'## 排查流程（从下到上）

### 1. 物理层
```bash
ethtool eth0          # 检查链路状态
ip link show          # 接口状态
```

### 2. 网络层
```bash
ping <target_ip>      # 基本连通性
traceroute <target>   # 路由路径
ip route              # 路由表
```

### 3. 传输层
```bash
telnet <ip> <port>    # TCP 端口连通
ss -tlnp              # 本地监听端口
tcpdump -i eth0 port <port>  # 抓包分析
```

### 4. 应用层
```bash
curl -v http://<target>     # HTTP 连通性
nslookup <domain>           # DNS 解析
```

## 常见原因
- 防火墙规则：`iptables -L -n`
- 安全组限制（云环境）
- 路由配置错误
- DNS 解析失败
- MTU 不匹配',
'网络不通的分层排查流程，从物理层到应用层逐步定位',
'published', 1, @admin_id, @admin_name, @now, @now);

-- 8. MySQL 慢查询优化
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), 'MySQL 慢查询优化方法', 'database', '["mysql","慢查询","优化"]',
'## 发现慢查询
1. 开启慢查询日志：
   ```sql
   SET GLOBAL slow_query_log = ON;
   SET GLOBAL long_query_time = 1;
   ```
2. 分析慢查询日志：`mysqldumpslow -s t /var/log/mysql/slow.log`

## 分析执行计划
```sql
EXPLAIN SELECT ...;
```
关注：
- `type`：避免 `ALL`（全表扫描）
- `key`：是否使用了索引
- `rows`：扫描行数
- `Extra`：是否 `Using filesort` 或 `Using temporary`

## 优化手段
- **加索引**：WHERE / JOIN / ORDER BY 字段
- **避免 SELECT ***`：只查需要的列
- **分页优化**：用 `WHERE id > last_id LIMIT 10` 代替 `OFFSET`
- **JOIN 优化**：小表驱动大表
- **子查询改 JOIN**：子查询性能通常更差

## 验证
优化后再次 `EXPLAIN`，对比 `rows` 扫描数',
'MySQL慢查询的发现、分析和优化方法，包括执行计划解读和索引优化',
'published', 1, @admin_id, @admin_name, @now, @now);

-- 9. Kafka 消费积压处理
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), 'Kafka 消费积压处理', 'middleware', '["kafka","积压","消费者"]',
'## 现象
Consumer Lag 持续增大，消息处理跟不上生产速度。

## 排查步骤
1. `kafka-consumer-groups.sh --describe --group <group>` 查看 Lag
2. 检查消费者是否存活：`ps aux | grep consumer`
3. 检查消费者日志是否有异常

## 处理方案
- **增加消费者实例**：确保实例数 ≤ 分区数
- **提高消费并发**：调整 `max.poll.records`
- **批量消费**：改为批量处理而非逐条
- **异步处理**：消息先落库，异步处理
- **临时扩容**：增加分区数 + 消费者

## 注意事项
- 增加分区数不可逆，需谨慎
- 消费者数不能超过分区数，多出的不工作
- 如果是处理慢，优先优化业务逻辑而非加机器',
'Kafka消费积压的排查和处理，包括增加消费者、批量消费和分区扩容',
'published', 1, @admin_id, @admin_name, @now, @now);

-- 10. 系统内存不足 OOM
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), 'Linux 系统 OOM Killer 处理', 'linux', '["linux","oom","内存"]',
'## 现象
进程被异常 kill，`dmesg` 中出现 `Out of memory: Killed process`。

## 排查步骤
1. `dmesg -T | grep -i oom` 查看 OOM 记录
2. `grep -i "killed process" /var/log/messages`
3. 确认被 kill 的进程
4. `free -m` 查看当前内存状态
5. `ps aux --sort=-%mem | head -10` 找内存占用最高的进程

## 处理方案
- **紧急恢复**：重启被 kill 的服务
- **调整 OOM 策略**：
  ```bash
  # 保护关键进程不被 kill
  echo -17 > /proc/<pid>/oom_score_adj
  ```
- **限制进程内存**：使用 cgroup 或 systemd `MemoryLimit`
- **增加 swap**：
  ```bash
  fallocate -l 4G /swapfile
  chmod 600 /swapfile
  mkswap /swapfile
  swapon /swapfile
  ```

## 预防
- 监控内存使用趋势，提前告警
- 合理配置 JVM 堆内存（`-Xmx`）
- 避免单机部署过多服务',
'OOM Killer的处理方法，包括排查被kill进程、调整OOM策略和内存限制',
'published', 1, @admin_id, @admin_name, @now, @now);

-- 11. 证书过期处理
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), 'TLS/SSL 证书过期处理流程', 'network', '["证书","ssl","tls"]',
'## 现象
浏览器提示证书不可信，或服务间 HTTPS 调用失败。

## 检查证书过期时间
```bash
# 查看远程证书
echo | openssl s_client -connect <host>:443 2>/dev/null | openssl x509 -noout -dates

# 查看本地证书文件
openssl x509 -in cert.pem -noout -dates
```

## 更新流程
1. 申请新证书（CA 或 Let''s Encrypt）
2. 替换证书文件
3. 验证新证书：
   ```bash
   openssl x509 -in new_cert.pem -noout -text | head -20
   ```
4. 重启/重载服务：
   - Nginx: `nginx -s reload`
   - Apache: `systemctl reload httpd`
   - Tomcat: `systemctl restart tomcat`
5. 验证 HTTPS 访问正常

## 预防
- 监控证书过期时间（提前 30 天告警）
- 使用 `certbot renew` 自动续期（Let''s Encrypt）
- 建立证书台账，记录所有证书位置和过期时间',
'TLS/SSL证书过期的检查和更新流程，包括openssl命令和自动续期',
'published', 1, @admin_id, @admin_name, @now, @now);

-- 12. 服务启动失败排查
INSERT IGNORE INTO knowledge_items (id, title, category, tags, content, summary, status, version, created_by, created_by_name, created_at, updated_at) VALUES
(UUID(), '服务启动失败通用排查流程', 'general', '["systemd","启动失败","排查"]',
'## 排查步骤

### 1. 查看服务状态
```bash
systemctl status <service>
```
关注 `Active:` 行和日志输出。

### 2. 查看详细日志
```bash
# systemd 日志
journalctl -u <service> -n 50 --no-pager

# 服务自身日志
tail -100 /var/log/<service>/<service>.log
```

### 3. 常见原因
- **端口被占用**：
  ```bash
  ss -tlnp | grep <port>
  lsof -i :<port>
  ```
- **配置文件语法错误**：
  - Nginx: `nginx -t`
  - Apache: `httpd -t`
  - MySQL: `mysqld --validate-config`
- **权限不足**：
  ```bash
  ls -la /path/to/config
  ls -la /path/to/data
  ```
- **依赖服务未启动**：检查数据库/缓存是否可用
- **磁盘空间不足**：`df -h`
- **SELinux 拦截**：`getenforce` + `audit2allow`

### 4. 调试模式启动
```bash
# 直接前台运行，查看实时输出
<service_binary> --debug
```

## 处理
根据日志中的错误信息定位具体原因，修复后 `systemctl start <service>`',
'服务启动失败的通用排查流程，涵盖端口冲突、配置错误、权限、依赖和SELinux',
'published', 1, @admin_id, @admin_name, @now, @now);
