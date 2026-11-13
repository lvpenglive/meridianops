# Zabbix 控制台方案（改方案、装 Agent、模板设计器）

> 日期：2026-11-11  
> 状态：四期已落地（2026-11-11）。只读纳管和 Agent 启停升级仍以 [zabbix-control-design.md](./zabbix-control-design.md) 为准。  
> 本文覆盖其后的目标：值班和监控管理员只登录 MeridianOps，即可改监控方案、安装 Agent、设计模板。

Zabbix Server / Proxy / Agent 继续负责采集、触发器计算和历史存储。MeridianOps 不保存第二份模板定义，只通过现有链路读写 Zabbix API。全行告警仍由 Eventide 汇聚，本方案不建第二套告警中心。

## 1. 架构

沿用已有链路，不把 Zabbix JSON-RPC 写进 Gateway，也不向浏览器开放任意方法调用。

```
浏览器
  └─ Portal  监控纳管 / 改方案 / 模板设计器
        └─ Gateway  /api/monitor/*
              鉴权、配置项范围、审计
              └─ zabbix-ctl
                    ├─ Zabbix 7.0 JSON-RPC（读令牌 / 写令牌分开）
                    └─ 回调 Gateway 作业（SSH 安装、启停、升级）
```

配置的存放处是 Zabbix。门户打开时读取，保存时写回。一次请求只保存一个对象（模板、监控项、触发器或发现规则）。Zabbix API 没有跨对象事务，界面必须展示部分成功和失败原因。

MySQL 只保留已经在用的对照和任务，外加审计：

| 数据 | 存放 |
|------|------|
| 模板、监控项、触发器、预处理、低级发现、宏 | 只在 Zabbix |
| 主机对照 `zbx_host_links`、任务 `zbx_tasks` | 现有表 |
| 谁在何时改了哪个对象 | `audit_logs`。更新或删除模板前，把该模板的 `configuration.export` 结果记入审计详情，便于按这份导出再导入 |

删除模板、批量导入前先导出。导出内容进审计，不进业务表。

## 2. 版本

契约锁定 **Zabbix 7.0 LTS** 的 API。`zabbix-ctl` 启动时调 `apiinfo.version`。

- 主版本不是 7.0：设计器、导入、改模板的写接口直接拒绝，并写明实际版本。
- 只读名册、指标、当前问题保持可用。
- 监控项类型、预处理步骤、触发器函数放在 `zabbix-ctl` 的目录文件 `catalog/zabbix-7.0.json`。前端按目录画表单。升级 Zabbix 时换目录并改适配，不改页面结构。

## 3. 权限和令牌

在现有三个权限上增加 `monitor:template`（建议权限 ID `00000000-0000-0000-0000-000000000405`）。

| 权限 | 能做的事 |
|------|----------|
| `monitor:read` | 名册、指标、当前问题、查看模板和监控项（只读） |
| `monitor:operate` | 套用或卸下模板、改主机宏、登记主机、确认问题 |
| `monitor:agent` | 安装、升级、回滚。生产主机沿用现有审批 |
| `monitor:template` | 新建和修改模板、模板组、主机组、监控项、触发器、低级发现、模板宏、导入导出 |

主机范围规则不变：Gateway 按配置项上的业务系统和部门计算允许的主机，不认 Zabbix 主机组。模板不属于某台配置项，`monitor:template` 按人授予，只给监控管理员。

令牌仍只放在 `zabbix-ctl` 环境变量里：

| 变量 | 用途 |
|------|------|
| `ZBX_TOKEN_{CODE}` | 现有读令牌：查询、维护期、主机组、模板组 |
| `ZBX_TOKEN_{CODE}_WRITE` | 写令牌：主机、主机组、模板、模板组、监控项、触发器、发现规则、宏、`configuration.import` / `export` |

写令牌不授予 Zabbix 的用户、用户组、全局脚本、动作和媒介类型权限。读接口继续用读令牌。

## 4. 分期

每期都能单独上线。前一期未完成时不开始依赖它的页面。

### 第 1 期：改方案

> 代码已落地（2026-11-11）。`zabbix-ctl` 提供读写接口，Gateway `monitor_scheme.rs` 做权限、配置项范围和审计，门户为「监控纳管」补充主机组与确认问题，并新增「改方案」页。

日常改方案不打开模板设计器。

- 列出全部模板组（`templategroup.get`）和全部主机组（`hostgroup.get`）。组名、组 ID 对有 `monitor:read` 的人可见。
- 列出模板（名称、所属模板组、已链接的上级模板），可按模板组筛选。
- 打开一台已对照主机时，显示它所属的主机组。从主机组点进去，只列出落在操作者配置项范围内的主机；组内其余主机不返回。主机组不作为权限边界。
- 对一批已对照主机关联或卸下模板，并增删所属主机组（组必须已经存在）。
- 查看和修改主机宏（阈值几乎都在 `{$...}` 宏上）。
- 为已有配置项登记 Zabbix 主机：技术名、Agent 接口（IP、端口 10050）、主机组、模板。同名或同 IP 已存在则更新，不重复创建。登记时从主机组列表里选，不手填组 ID。
- 当前问题：确认、改严重级别、评论、关闭。调用 `event.acknowledge`。这里只处理这一批资源上的问题，全行告警仍走 Eventide。

验收：门户能列出全部主机组和模板组；套用模板、增删主机组、改宏、登记主机在门户完成，Zabbix 界面中能看到同样结果。

### 第 2 期：安装 Agent

> 代码已落地（2026-11-11）。门户「安装 Agent」提交任务；`zabbix-ctl` 对已有主机先建维护期，再调用剧本「Zabbix Agent 安装」，登记主机并轮询 `agent.ping`。作业中心需给该剧本绑定 SSH 凭据，并配置 `MERIDIANOPS_JOB_TOKEN`。

新增剧本 **Zabbix Agent 安装**，不复用「升级」占位脚本。动作名 `install`。目标是配置项，不是已经存在的 `hostid`。当前「必须先同步对照」的校验只约束启停和升级，不约束安装。

1. 操作者选择配置项、Zabbix 实例、要套的模板、`Server` / `ServerActive` 地址。主机名默认用配置项名称。
2. Gateway 校验 `monitor:agent`，以及这些配置项落在操作者的业务系统和部门范围内。SSH 凭据使用作业中心已经绑定的凭据。
3. 该 IP 在 Zabbix 里已有主机时，先建维护期，失败则停止，不继续装。
4. SSH 安装 `zabbix-agent2`，写入 `Server`、`ServerActive`、`Hostname`，`systemctl enable --now`。包不存在或系统类型不在支持列表里，记失败原因并停止，不调用建主机。
5. `host.create` 或 `host.update`，套上所选模板。
6. 轮询 `agent.ping`，默认 3 分钟。通了才把这台记成功，并写入 `zbx_host_links`。超时记未知，不算成功。
7. 若第 3 步建过维护期，结束时删除或把结束时间改到当前。
8. 审计：发起人、配置项、实例、成功数、失败数、任务 id。

第一期安装只支持行内已经统一的一种系统包（rpm 或 deb 二选一，实施时写死在剧本里）。名册里标不出安装类型的主机，拒绝升级；安装时系统探测失败同样拒绝。

门户替代不了这三件事：包仓库里要有 Agent 包、目标机 SSH 可达、主机到 Zabbix Server 的 10051 端口可达。失败原因按这三项回显。

验收：一台未纳管主机从门户安装后出现在名册里，`agent.ping` 为通，并已套上选定模板。

### 第 3 期：模板设计器（普通对象）

> 代码已落地（2026-11-11）。门户「模板库」和模板设计页按 `catalog/zabbix-7.0.json` 画表单。写入前核对 Zabbix 主版本必须是 7.0。`protected = true` 的实例不直接写，先开标准变更工单，工单关闭后再执行；执行前把模板导出快照留在变更记录里。

页面按目录渲染，不按 Zabbix 原界面逐屏照搬。

| 对象 | Zabbix 方法 | 第一版可编辑的类型 |
|------|-------------|-------------------|
| 模板组 | `templategroup.get` / `create` / `update` / `delete` | 第 1 期只读全部模板组；第 3 期可新建、改名、删除空组 |
| 主机组 | `hostgroup.get` / `create` / `update` / `delete` | 第 1 期只读并给主机增删已有组；第 3 期可新建、改名、删除空组 |
| 模板 | `template.get` / `create` / `update` / `delete` | 名称、所属模板组、上级模板、宏 |
| 监控项 | `item.get` / `create` / `update` / `delete` | Agent、SNMP、简单检查、依赖项、计算项、HTTP Agent、采集器 |
| 预处理 | 监控项上的 `preprocessing` | 目录里列出的步骤：正则、JSONPath、倍数、丢弃心跳、变更、限速等 |
| 触发器 | `trigger.get` / `create` / `update` / `delete` | 表达式、严重级别、恢复表达式。函数从 7.0 目录点选后填入表达式 |
| 导入导出 | `configuration.export` / `import` | 单个或一批模板，用于实例之间搬模板 |

模板上已经存在、但不在上表类型里的监控项：列表可见，表单只读，并标明类型名称。避免设计器把看不见的项保存时弄丢。

生产实例（`zabbix-ctl` 里该 `code` 标记 `protected=true`）的模板写操作，先走现有审批，通过后再调用写令牌。主机宏和套用模板属于第 1 期，只审计，不进这条审批。

验收：在门户新建模板，加上一个 Agent 监控项（含一步预处理）和一条触发器，导出后再导入到同一实例，Zabbix 中对象一致。

### 第 4 期：低级发现

> 代码已落地（2026-11-11）。模板设计页增加「发现规则」：规则与过滤、监控项原型、触发器原型、覆盖规则。原型复用第 3 期表单，键值必须带 `{#宏}`。继承来的规则只读。图形原型和主机原型的覆盖动作在保存时按步骤保留，不在页面上改。

复用第 3 期的监控项和触发器表单，编辑对象改为原型。

- `discoveryrule.*`：发现规则、过滤
- `itemprototype.*`、`triggerprototype.*`
- 覆盖规则（override）

验收：一条低级发现规则及其监控项原型、触发器原型可在门户修改，并在 Zabbix 中生效。

## 5. 接口

浏览器只访问 Gateway。Gateway 校验权限后原样转发到 `zabbix-ctl` 的同名资源路径。请求体里不出现 Zabbix 的 `method` 字段。

| 期 | 方法 | Gateway 路径 | 权限 |
|----|------|----------------|------|
| 1 | GET | `/api/monitor/instances/:code/template-groups` | `monitor:read` |
| 1 | GET | `/api/monitor/instances/:code/host-groups` | `monitor:read` |
| 1 | GET | `/api/monitor/instances/:code/host-groups/:groupId/hosts` | `monitor:read`，结果与配置项范围取交集 |
| 1 | GET | `/api/monitor/instances/:code/templates` | `monitor:read`，可用 `groupId` 筛选 |
| 1 | GET | `/api/monitor/instances/:code/templates/:id` | `monitor:read` |
| 1 | POST | `/api/monitor/instances/:code/hosts/:hostId/templates` | `monitor:operate` |
| 1 | POST | `/api/monitor/instances/:code/hosts/:hostId/groups` | `monitor:operate`，请求体 `{ "groupIds": ["..."], "action": "link" \| "unlink" }` |
| 1 | GET / PUT | `/api/monitor/instances/:code/hosts/:hostId/macros` | 读 `monitor:read`，写 `monitor:operate` |
| 1 | POST | `/api/monitor/instances/:code/hosts` | `monitor:operate` |
| 1 | POST | `/api/monitor/instances/:code/events/:eventId/ack` | `monitor:operate` |
| 2 | POST | `/api/monitor/tasks`，`action=install` | `monitor:agent` |
| 3 | POST / PUT / DELETE | `/api/monitor/instances/:code/template-groups` 及 `/:id` | `monitor:template` |
| 3 | POST / PUT / DELETE | `/api/monitor/instances/:code/host-groups` 及 `/:id` | `monitor:template` |
| 3 | POST / PUT / DELETE | `/api/monitor/instances/:code/templates` 及 `/:id` | `monitor:template` |
| 3 | GET / POST / PUT / DELETE | `.../templates/:id/items` 及 `.../items/:itemId` | 读 `monitor:read`，写 `monitor:template` |
| 3 | GET / POST / PUT / DELETE | `.../templates/:id/triggers` 及 `.../triggers/:triggerId` | 同上 |
| 3 | GET / PUT | `.../templates/:id/macros` | 同上 |
| 3 | POST | `.../templates/export`、`.../templates/import` | `monitor:template` |
| 4 | GET / POST / PUT / DELETE | `.../templates/:id/discovery-rules` 及其 prototypes | 同上 |

套用模板的请求体：`{ "templateIds": ["..."], "action": "link" | "unlink" }`。卸下用 unlink，避免用空列表覆盖该主机的全部模板。

登记主机的请求体带 `ciId`。Gateway 确认该配置项在操作者范围内，并把管理 IP、主机名交给 `zabbix-ctl`。`zabbix-ctl` 先按 IP 和技术名查找，命中则 `host.update`。

`install` 任务在现有 `CreateTaskBody` 上扩展：`ciIds`、`templateIds`、`server`、`serverActive`。`hostIds` 对安装可为空。

## 6. 页面

| 路径 | 内容 | 期 |
|------|------|----|
| `/monitor` | 现有名册、曲线、当前问题、启停升级。名册显示主机组。问题行增加确认、评论、关闭。可按主机组筛选，筛选结果仍受配置项范围限制 | 1 |
| `/monitor/scheme` | 左侧模板组、主机组两棵列表。选主机后套用或卸下模板、增删主机组、编辑主机宏，为配置项登记主机 | 1 |
| `/monitor/install` | 选配置项和模板，填写 Server 地址，看每台安装和 `agent.ping` 结果 | 2 |
| `/monitor/templates` | 模板库：按模板组浏览、搜索、导入导出。第 3 期可新建和改名模板组、主机组 | 1 起只读，3 起可写 |
| `/monitor/templates/:id` | 设计器页签：监控项、触发器、宏、上级模板；第 4 期加「发现规则」 | 3、4 |

无 `monitor:template` 的用户可以打开模板页，表单为只读。无 `monitor:read` 的用户不出现这些菜单。

## 7. 边界

以下能力留在 Zabbix 或既有系统里，不进入本方案的开发范围：

- 地图、网络拓扑图、Zabbix 仪表盘、Zabbix 报表设计器
- Zabbix 的动作、媒介、用户和用户组
- 告警汇聚、降噪和全行确认（Eventide）
- 任意监控项键的通用查询（指标接口仍只查 CPU、内存、磁盘、Agent 通断）
- JMX、IPMI、浏览器监控项、脚本监控项的编辑（列表只读，待目录后续追加）
- 用 iframe 嵌入 Zabbix 原界面

## 8. 风险

- 模板一次修改会影响所有链接了该模板的主机。生产实例必须走第 3 期的审批；审计里保留导出快照。
- 写令牌泄露等于可以改监控配置。令牌只在 `zabbix-ctl` 进程环境中，Gateway 配置和前端包都不存放。
- 触发器表达式写错会立刻作用到已链接主机。设计器保存前用 Zabbix 返回的错误原样展示，不做静默改写。
- 安装剧本与操作系统、包路径绑定。换发行版时改剧本和目录，不改任务状态机。
- 主版本升级离开 7.0 时，写接口会拒绝服务，直到换上对应 `catalog/zabbix-x.json`。

## 9. 和现有文档的关系

| 文档 | 关系 |
|------|------|
| [zabbix-control-design.md](./zabbix-control-design.md) | 已完成的只读纳管、维护期、启停升级。其中「不重做模板编辑器」和「写令牌不接入」由本文接续，不在那个范围内返工 |
| [product-next.md](./product-next.md) | 仍不做自建采集和第二套告警中心。改方案、设计器、Agent 安装按本文排期 |
| [ops-tools-sso-plan.md](./ops-tools-sso-plan.md) | 本文范围之外的 Zabbix 页面若仍需打开，走外链或后续 SSO |
