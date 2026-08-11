# MeridianOps

[![Version](https://img.shields.io/badge/version-v0.2.0-blue)](./CHANGELOG.md)
[![Framework](https://img.shields.io/badge/Vue-3.x-42b883)](https://vuejs.org/)
[![Framework](https://img.shields.io/badge/Rust-1.x-orange)](https://www.rust-lang.org/)

**MeridianOps** 是基于rust语言面向银行/中大型团队的**一体化融合运维管理平台**，旨在实现 **1-5-10** 目标：

- **1 分钟内发现**故障（实时数据融合 + 多源告警汇聚 + 态势大屏统一提醒）
- **5 分钟内定位**问题（拓扑感知 + 关联分析 + 智能诊断 + 知识推荐）
- **10 分钟内解决**故障（标准作业剧本 + 一键执行 + 自动自愈 + 知识库驱动）

三大核心能力：
- 🔗 **数据融合**：多源监控/资产/服务/日志数据统一接入与关联
- 🧠 **智能分析**：根因分析、故障诊断、MTTR/可用率报告、异常趋势检测
- 📚 **知识档案**：故障处理过程沉淀、知识条目管理、相似告警智能推荐

## 项目结构

```
meridianops/
├── portal/          # 统一门户（Vue 3 SPA）
├── gateway/         # API 聚合网关（Rust + Axum）
└── README.md        # 本文档（全面计划表）
```

## 子系统

| 子系统 | 说明 | 技术栈 | 端口 |
|--------|------|--------|------|
| **portal** | 统一前端门户 | Vue 3 + TypeScript + Element Plus + Pinia | 5173 (dev) |
| **gateway** | API 聚合网关 / 融合层 | Rust + Axum 0.7 + sqlx 0.8 + MySQL | 8000 |

### 数据接入原则（避免重复建设）

> **告警类数据只从 Eventide 接入**：Zabbix / Prometheus / APM / NMS 等监控源作为 Eventide 的上游，由 Eventide 负责去重、压缩、降噪、通知分发，MeridianOps 不再直连这些监控系统的告警接口。
>
> **指标/日志/服务元数据按需直连**：日志（ELK）、服务生命周期（AxleOps）、资产（CMDB）等 Eventide 不覆盖的领域，由 MeridianOps Gateway 直接代理。

| 系统 | 类型 | MeridianOps 从哪里接入 | 说明 |
|------|------|----------------------|------|
| **AxleOps** | 服务管理 / 发布 / 作业执行 | ✅ Gateway 直连 | 服务元数据、发布状态、剧本执行 |
| **Eventide** | 告警中枢 / 多源告警接入 | ✅ Gateway 直连（**唯一告警入口**） | 统一告警列表、确认/关闭、分级统计 |
| **Zabbix** | 基础设施监控 | ⚡ 仅通过 Eventide 间接接入 | 指标历史图可按需从 Eventide 透传 |
| **Prometheus** | 指标监控 | ⚡ 仅通过 Eventide 间接接入 | 同 Zabbix |
| **ELK** | 日志分析 | ✅ Gateway 直连（iframe + 检索跳转） | 日志查询、告警详情快捷跳转 |
| **CMDB** | 资产 / 业务拓扑 | ✅ Gateway 直连（或 Excel 导入） | 主机归属、业务链路、负责人关联 |
| **堡垒机** | 操作审计 | 🔮 规划中 | 高危操作统一发起与留痕 |

## 快速开始

### 环境要求

- **Node.js** >= 18
- **Rust** >= 1.75
- **MySQL** >= 5.7（120.26.105.115:3306，独立库 `meridianops`）
- **pnpm / npm**（前端包管理）

### 启动前端门户

```bash
cd portal
npm install
npm run dev
```

访问 http://localhost:5173

默认种子账号：`admin` / `admin123!`

### 启动 API 网关

```bash
cd gateway
cargo run
```

网关会自动执行 `migrations/` 下的 SQL 迁移、创建数据库、种子 admin 账号与内置角色/权限/部门。

### 同时启动（开发模式）

```bash
# 终端 1 - 前端
cd portal && npm run dev

# 终端 2 - 网关
cd gateway && cargo run
```

访问 http://localhost:5173 即可使用统一门户。

## 整体架构

```
                    ┌───────────────────────────────────────────────────────┐
                    │           MeridianOps Portal (Vue 3 SPA)             │
                    │  统一入口 / 16 模块导航 / 态势大屏 / 报告 / 工作台     │
                    └───────────────────────────┬───────────────────────────┘
                                                │ HTTP / JSON
                                                ▼
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                  MeridianOps Gateway (Rust + Axum)                                     │
│  ┌───────────────────────────────────────────────────────────────────────────────────┐  │
│  │ 融合层（MeridianOps 核心价值）                                                   │  │
│  │  ├─ 统一鉴权 RBAC：JWT + Argon2 + 3 级角色 + 18 个权限点 + 部门树形              │  │
│  │  ├─ 审计中间件：所有写操作自动留痕（谁 / 何时 / 对什么 / 做了什么 / 结果）        │  │
│  │  ├─ 数据聚合：告警统计 + 服务健康 + MTTR / 可用率报表                            │  │
│  │  ├─ 关联分析：告警 ↔ 资产 ↔ 业务 ↔ 服务 ↔ 发布历史 跨源 Join                    │  │
│  │  ├─ 知识引擎：告警 → 相似知识推荐 + 故障处理过程一键沉淀                         │  │
│  │  └─ 自愈编排：告警 → 剧本推荐 → 一键 / 自动执行（AxleOps 作业通道）              │  │
│  └───────────────────────────────────────────────────────────────────────────────────┘  │
│  ┌──────────────────────────────── 通用代理层 ───────────────────────────────────────┐  │
│  │  GET|POST|PUT|DELETE /api/proxy/:system/*rest   （按配置动态加系统，不需改代码） │  │
│  └───────────────────────────────────────────────────────────────────────────────────┘  │
└───────────┬────────────────────────┬──────────────────────┬──────────────────────────────┘
            ▼                        ▼                      ▼
        AxleOps                Eventide                   ELK / CMDB
      (服务/发布/作业)         (告警中枢)                 (日志 / 资产)
                               ▲    ▲    ▲
                               │    │    │
                           Zabbix Prometheus APM/NMS...
                          （Eventide 上游，MeridianOps 不直连）
```

## 功能模块矩阵（16 模块 × 状态 × 核心能力归属）

| 分类 | 模块 | 路径 | 状态 | 归属核心能力 | 说明 |
|------|------|------|------|------------|------|
| 🎯 **态势总览** | 态势中心 | `/overview` | ✅ 已实现（框架齐，待填真实数据） | 数据融合 | 全局告警分级计数、服务健康数、最近告警列表 |
| 🎯 **态势总览** | 全局搜索 | 顶部搜索框 | ⏳ 待开发（P0） | 数据融合 | 跨资产/告警/知识/服务一键搜索 |
| 🎯 **态势总览** | 个人工作台 | `/dashboard` | ⏳ 待开发（P1） | 数据融合 | 待处理告警 + 待审批 + 待办工单聚合首页 |
| 🔗 **数据融合** | 资产管理 | `/assets` | ⏳ 待开发（P0） | 数据融合 | 主机/IP/业务系统/负责人/标签 + Excel 导入 |
| 🔗 **数据融合** | 告警中心 | `/alerts` | ⏳ 待开发（P0） | 数据融合 | 统一告警列表（从 Eventide）+ 确认/关闭/备注 + 分组折叠 |
| 🔗 **数据融合** | 告警详情 | 弹窗 | ⏳ 待开发（P1） | 数据融合 | 告警 + 归属业务 + 关联服务 + 同主机关联告警 + 跳 ELK |
| 🔗 **数据融合** | 拓扑视图 | `/topology` | ⏳ 待开发（P1） | 数据融合 | 业务系统 → 主机 两层拓扑，告警节点高亮 |
| 🔗 **数据融合** | 日志中心 | `/logs` | ✅ 已实现（框架齐，待接 ELK） | 数据融合 | 快捷查询 + 告警跳转 |
| 🔗 **数据融合** | 容器管理 | `/containers` | 🚧 规划中（P2） | 数据融合 | K8s 集群概览 |
| 🔗 **数据融合** | DB 数据库 | `/database` | ✅ 已实现（框架齐） | 数据融合 | 数据库实例状态 |
| 🧠 **智能分析** | 报表中心 | `/reports` | ⏳ 待开发（P0） | 智能分析 | 日报/周报/月报 + MTTR/可用率统计 + PDF/Excel 导出 |
| 🧠 **智能分析** | 根因分析 RCA | 告警详情 Tab | 🚧 规划中（P2） | 智能分析 | 拓扑 + 告警时序归因，高亮根因节点 |
| 🧠 **智能分析** | AIOps 诊断 | `/aiops` | ✅ 已实现（框架齐） | 智能分析 | 相似告警推荐 + 异常趋势检测 |
| ⚡ **流程自动化** | 作业中心 | `/jobs` | ✅ 已实现（框架齐，P1 接剧本库） | 流程自动化 | 标准作业剧本库 + 一键执行 + 审计留痕 |
| ⚡ **流程自动化** | 自愈编排 | 告警联动 | 🚧 规划中（P2） | 流程自动化 | 白名单告警触发自动执行剧本 |
| ⚡ **流程自动化** | 工单系统 | `/tickets` | ✅ 已实现（框架齐，P2 接 ITIL） | 流程自动化 | 变更申请/审批/执行 ITIL 流程 |
| 📚 **知识档案** | 知识库 | `/knowledge` | ⏳ 待开发（P1） | 知识档案 | 知识 CRUD + 标签 + 全文检索 + 版本化 |
| 🛡️ **合规管理** | 用户管理 | `/system/users` | ✅ 已实现 | 合规地基 | 用户 CRUD + 启停 + 重置密码 + 角色/部门分配 |
| 🛡️ **合规管理** | 角色管理 | `/system/roles` | ✅ 已实现 | 合规地基 | 角色 CRUD + 权限分配 + 内置角色保护 |
| 🛡️ **合规管理** | 部门管理 | `/system/departments` | ✅ 已实现 | 合规地基 | 树形部门 CRUD + 删除保护 |
| 🛡️ **合规管理** | 个人中心 | `/profile` | ✅ 已实现 | 合规地基 | 资料 / 自助改密 / 登录历史 |
| 🛡️ **合规管理** | 系统设置 | `/system` | ✅ 已实现（接真实 API） | 合规地基 | 密码策略 + 登录锁定参数（system_settings 持久化） |
| 🛡️ **合规管理** | 审计中心 | `/audit` | ✅ 已实现 | 合规地基 | 操作日志分页查询 + 筛选 + 导出（后续） |
| 🛡️ **合规管理** | 安全加固 | — | ✅ 已实现 | 合规地基 | 登录失败 5 次锁定 15 分钟 + 密码强度策略 + JWT 鉴权 |
| 🛡️ **合规管理** | 双人审批 | — | ⏳ 待开发（P1） | 合规地基 | 用户删除/角色分配/配置修改需第二人审批 |
| 🛡️ **合规管理** | 密码过期/双因子 | — | ⏳ 待开发（P1） | 合规地基 | 90 天强制改密 + TOTP 双因子登录 |
| 💼 **运营辅助** | 配置中心 | `/config` | ✅ 已实现（框架齐） | 运营辅助 | Agent 配置管理 |
| 💼 **运营辅助** | 费用中心 | `/cost` | ✅ 已实现（框架齐） | 运营辅助 | 云资源成本统计（后续接账单） |
| 💼 **运营辅助** | 通知中心 | 顶部铃铛 | ⏳ 待开发（P1） | 运营辅助 | 站内信 + 飞书/钉钉/邮件 Webhook |

## 目标 × 核心能力 × 功能模块 映射关系

| 核心目标 | 依赖哪项核心能力 | 对应的关键功能模块 |
|---------|---------------|-----------------|
| **⏱️ 1 分钟内发现故障** | 🔗 数据融合 | 告警中心（Eventide）+ 态势大屏 + 全局搜索 + 通知中心 |
| **🔍 5 分钟内定位问题** | 🔗 数据融合 + 🧠 智能分析 + 📚 知识档案 | 告警详情上下文融合 + 拓扑视图 + AIOps 根因分析 + 知识推荐 |
| **✅ 10 分钟内解决故障** | ⚡ 流程自动化 + 📚 知识档案 | 作业剧本一键执行 + 自动自愈 + 知识库方案 + 工单/审批 |
| **📊 提升服务可用率 / 降低 MTTR** | 🧠 智能分析 + 🛡️ 合规管理 | 报表中心（MTTR/可用率/告警分级趋势）+ 审计合规 |

---

## 开发路线图（全面计划表）

### Phase 0 ✅ 完成：基础设施地基（2026-08-11 ~ 1 周）

> 目标：让平台"能跑起来、能登录、能管用户"

- [x] Gateway 框架：Rust + Axum 0.7 + sqlx 0.8 + MySQL 连接池 + 自动建库
- [x] 统一鉴权：JWT（24h token）+ Argon2 密码哈希 + 三级角色（admin/operator/viewer）
- [x] 登录/登出/me 接口 + 前端登录页 + 路由守卫 + token 本地持久化
- [x] 端到端验证：curl 登录/me/401/错误密码 + 浏览器登录联调
- [x] 交付物：可部署的网关二进制 + 可访问的统一门户

### Phase 1 ✅ 完成：RBAC + 部门 + 审计中心（2026-08-11 ~ 1 周）

> 目标：满足最基本的合规/安全要求，银行等保三级起点

- [x] RBAC 三表：`roles` / `permissions` / `role_permissions` + 18 个权限点种子
- [x] 部门树形：`departments` + parent_id + 有子部门/用户不可删保护
- [x] `users` 扩展：role_id / department_id 外键 + 内置角色回填
- [x] JWT claims 扩展 permissions 数组 + require_permission 守卫 + 前端 v-permission 指令 + 菜单权限过滤
- [x] 角色 CRUD + 权限分配 API / 页面
- [x] 部门 CRUD API / 页面
- [x] 审计中间件：所有写操作自动记录到 `audit_logs` 表（actor/action/target/ip/status）
- [x] 审计中心 API / 页面：分页查询 + 按 actor/action/status/时间筛选
- [x] 交付物：用户/角色/部门完整管理闭环 + 操作全留痕

### Phase 2 ✅ 完成：安全加固 + 个人中心（2026-08-11 ~ 1 天）

> 目标：堵住暴力破解/弱密码风险，用户自助能力上线

- [x] 登录失败锁定：`failed_login_attempts` + `locked_until`（默认 5 次失败锁 15 分钟）
- [x] 密码强度策略：`PasswordPolicy` + 可配置最小长度/大小写/数字/特殊字符（system_settings 表）
- [x] 系统设置 API：GET/PUT `/api/system/settings`（system:read/update 权限）+ GET 密码策略
- [x] 自助改密：POST `/api/auth/change-password`（校验旧密码 + 强度策略 + 审计）
- [x] 个人中心页面：资料摘要 + 修改密码 + 登录历史 Tab（头像下拉菜单跳转）
- [x] 系统设置页面改造：密码策略 + 锁定参数可视化配置（接真实 API，按权限只读/可编辑）
- [x] 交付物：防暴破上线 + 自助改密可用 + 系统策略可配置

---

### Phase 3 🔥 **立即推进（P0，预计 2-3 周）：数据融合第一块砖**

> 目标：**让"1 分钟发现"从口号变成事实**，值班员第一次真正觉得 MeridianOps 有用

- [ ] 🔗 **Eventide 告警接入**：
  - gateway-config.toml 加 eventide 配置（base_url/token）
  - 新建 `eventide_routes.rs`：GET `/api/alerts` 分页查询 / PATCH 确认 / POST 关闭（透传 Eventide）
  - 全局告警 badge 接「未处理告警数」（MainLayout 右上角铃铛）
- [ ] 🔗 **态势大屏填真数**：OverviewPage 告警分级卡片（firing/warning/resolved）+ 服务健康数（从 AxleOps）+ 最近告警列表从 Eventide 拉
- [ ] 📊 **报表中心 MVP**：
  - 新建 `report_routes.rs`：`GET /api/reports/monthly` 从 `audit_logs` + 告警数据生成 JSON
  - 页面：告警分级统计图 + MTTR 统计 + 操作审计 TOP 10 + 导出 Excel（PDF 后续）
- [ ] 🔗 **资产清单 MVP**：
  - 迁移 `assets` / `business_systems` 表（主机/IP/业务系统/负责人/标签/环境）
  - 资产 CRUD API + 页面替换 AssetsPage.vue mock
  - 支持 **Excel 批量导入**（从行里老 CMDB 直接灌数据）
- [ ] 🔍 **全局搜索 MVP**：
  - `GET /api/search?q=` 搜资产（IP/主机名）+ 搜告警 + 搜用户
  - 顶部搜索框回车 → 搜索结果页分 Tab 展示

**Phase 3 结束验收标准：** 值班员早上打开 MeridianOps，看到大屏红点数 = 昨夜未处理告警；搜某主机 IP，0.5 秒出来归属业务 + 负责人 + 最近告警。

---

### Phase 4 ⚡ **短期推进（P1，预计 4-6 周）：融合深度 + 体验升级**

> 目标：**让"5 分钟定位"成为可能**，值班员不用再切 5 个系统

#### 数据融合深化
- [ ] 告警详情上下文弹窗：
  - 归属业务 / 负责人（从资产表）
  - 关联服务 + 最近一次发布时间/人（从 AxleOps）
  - 同一主机 1 小时内其他告警
  - 快捷按钮：跳 ELK（按主机+时间窗拼 Kibana URL） / 跳指标图 / 开堡垒机
- [ ] 外部系统 iframe 嵌入：ELK / AxleOps / Zabbix 深度操作在 MeridianOps 内用 iframe 无缝切换
- [ ] 拓扑视图（最简版）：业务系统 → 主机两层拓扑图，告警主机高亮红点

#### 智能分析 + 知识档案
- [ ] 告警全闭环：`alerts` 状态机（firing → acknowledged → resolved）+ 处理人/处理时间/备注
- [ ] MTTR/MTBF 自动计算 + 报表中心「按业务 TOP 10 MTTR」图表
- [ ] 告警聚类折叠 UI：同一主机 5 分钟内的告警合并显示
- [ ] 知识库 MVP：`knowledge_items` 表 + CRUD + 标签 + 全文检索 + 告警详情「关联知识」Tab（先按标签匹配）
- [ ] 故障 → 知识一键沉淀：告警处理完毕后点「沉淀为知识」按钮自动填草稿

#### 合规补充
- [ ] 高危操作双人审批：用户删除/角色分配/系统配置修改 → 审批单 → 第二人批准后执行，审批单全留痕
- [ ] 密码过期强制（90 天）+ 首次登录强制改密
- [ ] 30 分钟无操作会话超时自动登出
- [ ] 双因子登录（TOTP 可选：Google Authenticator / 行里统一认证）

#### 自动化 + 体验
- [ ] 作业剧本 MVP：5 个高频场景剧本（重启服务/清理磁盘/回滚/探活/配置同步）→ 调 AxleOps 执行
- [ ] 告警详情 → 推荐剧本 → 一键执行
- [ ] 个人工作台首页：待处理告警 + 待审批单 + 待办工单
- [ ] 通知中心 MVP：站内信 + 飞书/钉钉/邮件 Webhook 配置

**Phase 4 结束验收标准：** 一个告警弹出，值班员 5 分钟内能在同一个页面看到「这影响了哪个核心业务、负责人是谁、是不是刚刚发布导致的、以前怎么解决的、现在可以一键执行哪个剧本」—— 切其他系统次数 ≤ 1。

---

### Phase 5 🚧 **中期推进（P2，预计 2-3 个月）：智能化 + 自动化**

> 目标：**让"10 分钟解决"不依赖老师傅**，新人也能按流程快速处理

#### 智能分析（真·AIOps）
- [ ] 根因分析 RCA：拓扑 + 告警时序归因，高亮「最可能根因节点」+ 影响面拓扑图
- [ ] 相似告警智能推荐知识：知识库 > 50 条后，用 TF-IDF / 向量相似度推 Top 3 历史方案
- [ ] 异常趋势检测：告警量 / MTTR 环比同比异常波动主动提醒
- [ ] 报表中心升级：PDF 导出 + 邮件定时发送周报月报给行领导

#### 自动化
- [ ] 自动自愈：白名单告警（磁盘满/进程挂/日志清理）触发自动执行剧本，结果回写通知
- [ ] ITIL 工单系统：变更申请 → 审批 → 执行 → 验证 → 关闭 全流程，关联告警/资产/作业

#### 融合能力
- [ ] 四层拓扑视图：网络层 → 主机层 → 应用层 → 业务层
- [ ] 容器管理接 K8s API：集群状态 / Pod 列表 / 重启次数
- [ ] 全链路追踪接入（Skywalking / Pinpoint）：服务调用链 + 慢查询

**Phase 5 结束验收标准：** 80% 的日常告警处理 ≤ 10 分钟；新人入职 1 周内能独立处理常见告警；自动自愈覆盖 50% 高频故障。

---

### Phase 6 🔮 **长期展望（P3，预计 6 个月+）：AI + 成本优化**

- [ ] AI 大模型运维助手：知识库 + 历史故障 + 资产数据喂向量库，聊天式问"这个告警怎么办""昨天核心支付系统 5xx 是什么原因"
- [ ] 成本优化分析：费用中心 + 资源利用率，推荐缩容/降配/腾挪方案
- [ ] 多数据中心/多地域统一视图切换
- [ ] 移动门户 / 小程序：值班员手机看告警 + 审批 + 执行剧本
- [ ] 高可用部署：Gateway 多实例 + Portal CDN + 主备切换

---

## 默认账号（首次启动自动种子）

| 角色 | 用户名 | 默认密码 | 说明 |
|------|-------|---------|------|
| 管理员（admin） | `admin` | `admin123!` | 全部 18 个权限，上线后**必须立即修改默认密码** |

三个内置角色（不可删除）：

| 角色 | 权限范围 | 对应人员 |
|------|---------|---------|
| admin | 18 个权限全开 | 运维主管 / 系统管理员 |
| operator | 用户全 CRUD + 部门全 CRUD + 审计读 + 系统读（不含角色管理和系统修改） | 日常运维值班员 |
| viewer | 所有 `:read` 权限（不可写） | 开发 / 二线支持只读查看 |

## 配置

### 网关配置 `gateway/gateway-config.toml`

```toml
[server]
bind = "0.0.0.0:8000"

[database]
# 可通过环境变量 MERIDIANOPS_DB_URL 覆盖
url = "mysql://root:886363@120.26.105.115:3306/meridianops"
max_connections = 10
min_connections = 2

[auth]
# 生产环境必须通过环境变量 MERIDIANOPS_JWT_SECRET 覆盖默认值
jwt_secret = "meridianops-dev-secret-change-me"
token_ttl_hours = 24
seed_username = "admin"
seed_password = "admin123!"

# 接入的下游系统（按需增加，MeridianOps Gateway 自动代理）
[[systems]]
id = "axleops"
name = "AxleOps 服务管理"
base_url = "http://axleops-admin:9000"
auth_type = "session"

[[systems]]
id = "eventide"
name = "Eventide 告警中枢"
base_url = "http://eventide:8080"
auth_type = "token"
auth_token = "替换为 Eventide API Token"
```

### 支持的认证类型（代理下游系统时）

| 类型 | 说明 |
|------|------|
| `none` | 无需认证 |
| `token` | 使用 `Authorization: Bearer <token>` + `X-AxleOps-Token` 请求头 |
| `api_key` | 使用 API Key |
| `basic_auth` | HTTP Basic Auth |
| `session` | Session Cookie（浏览器转发，适用于 iframe 嵌入） |

## 数据库迁移与种子数据

Migrations 位于 `gateway/migrations/`，按文件名序号顺序执行：

| 序号 | 文件 | 说明 | 状态 |
|------|------|------|------|
| 01 | 000001_init_users.sql | 建 users 表 | ✅ |
| 02 | 000002_add_last_login.sql | users 加 last_login_at | ✅ |
| 03 | 000003_create_audit_logs.sql | 建 audit_logs 表 | ✅ |
| 04 | 000004_fix_audit_logs_detail.sql | audit_logs.detail 类型修正 | ✅ |
| 05 | 000005_create_rbac_departments.sql | 建 roles/permissions/role_permissions/departments + 外键 | ✅ |
| 06 | 000006_seed_rbac_departments.sql | 3 内置角色 + 18 权限点 + 角色权限分配 + 根部门 + 用户回填 | ✅ |
| 07 | 000007_add_login_lockout_fields.sql | users 加 failed_login_attempts + locked_until | ✅ |
| 08 | 000008_create_system_settings.sql | 建 system_settings 表 + 密码策略/锁定参数种子 | ✅ |
| 09 | _（下一个 Phase 3）_ | assets + business_systems | ⏳ Phase 3 |
| 10 | _（下一个 Phase 3）_ | alerts 状态同步表 | ⏳ Phase 3 |
| 11 | _（下一个 Phase 4）_ | knowledge_items + approvals | ⏳ Phase 4 |

启动 Gateway 时 `sqlx::migrate!("./migrations").run(&pool)` 自动按顺序执行，幂等安全。

## 关联项目（自研上下游）

- [AxleOps](https://github.com/lvpenglive/axleops) — 服务生命周期管理 / 发布 / 剧本执行（MeridianOps 数据融合 + 作业执行通道）
- [Eventide](https://github.com/lvpenglive/eventide) — 告警事件中枢 / 多源告警接入 / 去重降噪（MeridianOps **唯一告警数据源**）

## License

MIT
