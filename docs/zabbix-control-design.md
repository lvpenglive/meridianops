# Zabbix 纳管设计实现方案

值班人员只登录 MeridianOps 完成资源监控和 Agent 批量启停、升级。Zabbix 继续负责采集、触发器和历史存储。不重做报表、拨测、模板编辑器。

## 1. 目标

- 按业务系统查看主机是否在线、Agent 是否通、关键指标和当前问题。
- 对一批主机执行 Agent 启动、停止、重启、升级、回滚。
- 操作前自动进入 Zabbix 维护期，结束后退出，避免误报。
- 发起人、主机清单和结果写入 MeridianOps 审计。

不做：新建监控模板、预处理、低级发现、Prometheus 或 JDBC 采集、另一套全行告警中心。全行告警仍由 Eventide 汇聚。本方案里的「当前问题」只用于看某一批资源，不替代 Eventide。

## 2. 架构

```
浏览器
  └─ MeridianOps Portal
        └─ Gateway  /api/monitor/*
              鉴权、按配置项收口可操作主机、写审计
              └─ zabbix-ctl（独立进程）
                    ├─ Zabbix JSON-RPC（API 令牌）
                    └─ 回调 Gateway 作业执行（已有 SSH 通道）
```

`zabbix-ctl` 单独部署，不把 Zabbix 协议写进 Gateway 业务路由。Gateway 只做门禁和转发。Zabbix API 令牌只放在 `zabbix-ctl` 的环境变量里，不到浏览器，也不进前端包。

多套 Zabbix 在 `zabbix-ctl` 配置中并列，每套一个 `code`、API 地址和令牌。请求必须带 `code`。

## 3. 权限

在现有 RBAC 上增加三个权限点：

| 权限 | 能做的事 |
|------|----------|
| `monitor:read` | 看名册、指标、当前问题、任务结果 |
| `monitor:operate` | 对授权业务系统下的主机启停、重启 |
| `monitor:agent` | 升级、回滚。生产主机沿用现有审批 |

主机范围不以 Zabbix 主机组为准。Gateway 用配置项上的业务系统和部门，算出本次允许的 `hostid` 列表，再交给 `zabbix-ctl`。列表之外的 hostid 直接拒绝。

## 4. 与配置库的对照

配置项增加外部标识：`source = zabbix`，`external_id = {zabbixCode}:{hostid}`。

`zabbix-ctl` 定时调用 `host.get`，把主机名、IP、Agent 可用性和版本摘要写入对照结果。已有配置项按 IP 或主机名匹配；对不上的列入「未纳管」，不自动建业务系统。

Agent 在线与版本不落历史库。打开页面时调用：

- `agent.ping` 判断通断
- `agent.version` 或 `zabbix[host,agent,available]` 取版本

## 5. 数据表

放在现有 MySQL 库，由 `zabbix-ctl` 使用。

`zbx_instances`

- `code`、`name`、`api_url`、`enabled`

令牌不入库，只走环境变量 `ZBX_TOKEN_{CODE}`。

`zbx_host_links`

- `instance_code`、`host_id`、`hostname`、`ip`、`ci_id`（可空）
- `agent_version`、`agent_available`、`synced_at`

`zbx_tasks`

- `id`、`instance_code`、`action`（start / stop / restart / upgrade / rollback）
- `target_version`、`concurrency`、`maintenance_id`
- `status`（pending / running / succeeded / failed / partial）
- `requested_by`、`created_at`、`finished_at`

`zbx_task_hosts`

- `task_id`、`host_id`、`ci_id`、`job_run_id`
- `version_before`、`version_after`
- `status`、`error`

## 6. 接口

门户只调用 Gateway。Gateway 校验权限和主机范围后转发。

| 方法 | 路径 | 作用 |
|------|------|------|
| GET | `/api/monitor/hosts` | 按业务系统、机房、版本、在线状态查名册 |
| GET | `/api/monitor/hosts/:hostId/metrics` | 关键指标最近曲线 |
| GET | `/api/monitor/hosts/:hostId/problems` | 该资源当前未恢复问题 |
| POST | `/api/monitor/tasks` | 创建批量启停或升级 |
| GET | `/api/monitor/tasks/:id` | 查看每台成功或失败 |

指标接口内部：

- 最近 24 小时用 `history.get`
- 超过 24 小时用 `trend.get`
- 固定查询 CPU、内存、磁盘、Agent 通断，不开放任意监控项键，避免把 Zabbix 查询能力暴露成通用接口

## 7. 批量启停与升级

1. 门户提交主机和动作。Gateway 核对每台都落在本人可操作的配置项上。
2. `zabbix-ctl` 调用 `maintenance.create`，主机范围为这批 `hostid`，时长覆盖任务超时（默认 30 分钟）。
3. 调用 MeridianOps 已有作业执行，剧本按动作固定：
   - 重启：`systemctl restart zabbix-agent`（或 `zabbix-agent2`，以名册中的类型为准）
   - 升级：安装任务指定的版本包，失败则安装 `version_before`
   - 停止、启动同理
4. 并发由任务参数限制，默认 10。已成功的主机不重复执行。
5. 每台结束后 `item.get` 回读版本。与目标一致记成功，否则记失败原因。
6. 全部完成或超时后 `maintenance.delete`，或把维护期结束时间改到当前。
7. Gateway 写审计：动作、发起人、主机数、成功数、失败数、任务 id。

作业账号使用现有 `ssh_credentials`。`zabbix-ctl` 不保存 SSH 私钥。

停止生产 Agent 时，Gateway 先走现有审批，通过后再创建任务。

## 8. 看监控时的页面

监控纳管页放在现有门户中，分三块：

- **名册。** 业务系统、主机、IP、Agent 版本、通断。可按版本落后和离线筛选。
- **主机会话。** 四条曲线和当前问题。问题行带来源触发器名称和严重级别，处理入口链到 Eventide 告警，不在本页做全行确认。
- **任务。** 选择主机后执行启停或升级，展示每台结果。

登录页和未授权用户不出现该菜单。

## 9. 分期与进度

| 步骤 | 内容 | 状态 |
|------|------|------|
| 0 | `zabbix-ctl` 骨架：健康检查、实例列表、版本 | 已完成 |
| 1 | `zabbix-ctl` 只读：名册、当前问题、CPU/内存/磁盘 | 已完成 |
| 2 | Gateway `/api/monitor/*` + 门户「监控纳管」页 | 已完成 |
| 3 | MySQL 任务表 + 维护期 + Agent 启停升级 | 已完成（需配置作业剧本 ID 后联调） |
| 4 | 模板差异、Proxy、历史库水位只读 | 已完成 |
| 5 | 预置 Agent 剧本、对照校验、名册筛选 | 已完成 |

Gateway 创建任务前会校验：主机必须已「同步对照」，且 `ci_id` 非空。作业 ID 为 0 时，`zabbix-ctl` 按名称匹配 `job_definitions`（迁移 `20260929000053_zabbix_agent_jobs.sql` 预置五条剧本）。剧本默认 `executor_type=ssh`，需在作业中心绑定 SSH 凭据后才能实跑。

## 10. 实现落点

| 仓库或目录 | 内容 |
|------------|------|
| 新进程 `zabbix-ctl` | Zabbix JSON-RPC 客户端、同步、维护期、任务状态机 |
| `gateway` | `/api/monitor/*` 鉴权、主机范围、转发、审计 |
| `portal` | 监控纳管页 |
| 作业中心 | 四条固定剧本：启动、停止、重启、升级 |

`zabbix-ctl` 与 Gateway 用内网 HTTP，带服务令牌。Gateway 是唯一对外入口。

## 11. 风险

- 升级剧本依赖目标机的安装方式（系统包或二进制）。第一段先支持一种已在行内统一的安装方式，名册里标出安装类型，类型不明的主机拒绝升级。
- 维护期创建失败时不允许继续停 Agent。
- 回读版本超时记为未知，不把任务标成成功。
- API 令牌权限使用只读加维护期写权限。能改模板的 Zabbix 超级用户令牌不接入本服务。
