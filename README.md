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

### 数据接入原则（基于全行真实生态）

> **所有告警数据统一走 Eventide**：天旦交易监控、优云 BPM、Zabbix 综合监控、华为 NPM、平行线机房监控等告警源，**全部由 Eventide 统一接入和汇聚**，MeridianOps 只对接 Eventide 的告警 API，不直连任何监控系统。
>
> **非告警类数据按需直连**：资产配置（优云 CMDB）、日志检索（ELK）、服务管理（AxleOps）等 Eventide 不覆盖的领域，由 MeridianOps Gateway 直接代理。
>
> **上游系统双向对接**：2oauth SSO、优云 ITIL、理想自动化、中国移动短信、帕拉迪堡垒机等上游系统，MeridianOps 提供双向接口（接收工单/触发作业/推送操作记录/下发通知）。

#### 数据源接入

| 系统 | 类型 | MeridianOps 接入方式 | 说明 |
|------|------|---------------------|------|
| **Eventide** | 告警中枢（**唯一告警入口**） | ✅ Gateway 直连 | 汇聚全行 6+ 监控系统告警，提供统一告警列表/确认/关闭/分级统计 API |
| **AxleOps** | 服务管理 / 发布 / 作业执行 | ✅ Gateway 直连 | 服务元数据、发布状态、剧本执行通道 |
| **优云 CMDB** | 资产配置 / 业务拓扑 / 配置关系 | ✅ Gateway 直连 | 主机归属、业务链路、负责人、配置项关联 |
| **ELK** | 日志分析 | ✅ Gateway 直连（iframe + 检索跳转） | 告警详情快捷跳 Kibana、日志检索 |
| **帕拉迪堡垒机** | 操作审计 | ⚡ 对接预留 | 操作记录回流 MeridianOps 审计中心 |
| **天旦交易监控** | 交易系统告警 | ❌ 不直连（经 Eventide） | Eventide 上游数据源 |
| **优云 BPM** | BPM 告警 | ❌ 不直连（经 Eventide） | Eventide 上游数据源 |
| **Zabbix 综合监控** | 系统告警/指标 | ❌ 不直连（经 Eventide） | Eventide 上游数据源 |
| **华为/科来 NPM** | 网络性能告警 | ❌ 不直连（经 Eventide） | Eventide 上游数据源 |
| **平行线机房监控** | 动环告警 | ❌ 不直连（经 Eventide） | Eventide 上游数据源 |

#### 上游系统对接

| 系统 | 交互方向 | 交互内容 | 优先级 |
|------|---------|---------|--------|
| **2oauth 统一身份认证** | 双向 | 登录认证 + 用户信息同步（MeridianOps 接 SSO） | P1 |
| **优云 ITIL 流程平台** | 双向 | 工单数据同步 + 配置数据同步 | P2 |
| **理想自动化批处理** | 双向 | MeridianOps 触发自动化操作 + 接收操作记录 | P2 |
| **中国移动短信/邮件网关** | 单向（出） | 告警/审批/通知通过短信/邮件下发 | P1 |
| **长亮大数据平台** | 预留接口 | 数据对接预留 | P3 |

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

## 整体架构（全行融合运维数据中台）

```
    ╔══════════════════════════════════════════════════════════════════════════════════════════╗
    ║                              上游系统（对外对接）                                        ║
    ║                                                                                        ║
    ║  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────────────┐  ┌──────────────┐            ║
    ║  │ 2oauth SSO  │  │ 优云 ITIL    │  │ 理想自动化  │  │ 移动短信/邮件网关 │  │ 长亮大数据  │            ║
    ║  │ 统一身份认证│  │ 流程平台    │  │ 批处理系统  │  │                    │  │ 平台(预留)  │            ║
    ║  └──────┬──────┘  └──────┬───────┘  └──────┬───────┘  └─────────┬──────────┘  └──────┬───────┘            ║
    ║         │ 登录/用户     │ 工单/配置        │ 触发/记录          │ 通知下发              │ 预留接口                     ║
    ║         ▼               ▼                 ▼                   ▼                       ▼                            ║
    ╚═════════╤═══════════════╤═════════════════╤═══════════════════╤═══════════════════════════╝
              │               │                 │                   │                       │
              ▼               ▼                 ▼                   ▼                       ▼
    ┌─────────────────────────────────────────────────────────────────────────────────────────────┐
    │                   MeridianOps Gateway (Rust + Axum)                                         │
    │                                                                                             │
    │  ┌────────────────────────── 融合层（核心价值） ─────────────────────────────────────────┐   │
    │  │  🛡️ 统一鉴权：JWT + Argon2 + RBAC (3级角色/18权限) + 对接 2oauth SSO                │   │
    │  │  📋 审计中间件：所有写操作自动留痕 → 对接帕拉迪堡垒机操作记录                        │   │
    │  │  📊 数据聚合：告警统计 + 服务健康 + MTTR/可用率报表                                  │   │
    │  │  🔗 关联分析：告警 ↔ CMDB资产 ↔ 业务系统 ↔ AxleOps发布历史 跨源 Join                 │   │
    │  │  📚 知识引擎：告警 → 相似知识推荐 + 故障处理一键沉淀                                  │   │
    │  │  ⚡ 自愈编排：告警 → 剧本推荐 → 一键执行 → 对接理想自动化批处理                       │   │
    │  │  📨 通知中心：告警/审批 → 对接中国移动短信/邮件网关                                    │   │
    │  │  📝 工单联动：告警 → 创建优云 ITIL 工单 → 状态同步                                    │   │
    │  └──────────────────────────────────────────────────────────────────────────────────────┘   │
    │                                                                                             │
    │  ┌────────────────────────── 通用代理层 ──────────────────────────────────────────────┐   │
    │  │  GET|POST|PUT|DELETE /api/proxy/:system/*rest   （按配置动态加系统，不需改代码）     │   │
    │  └──────────────────────────────────────────────────────────────────────────────────────┘   │
    └────────┬──────────────────────┬──────────────────────────┬──────────────────────────────────┘
             ▼                      ▼                          ▼
        AxleOps               Eventide                    优云 CMDB + ELK
      (服务/发布/作业)        (告警中枢)                  (资产配置 / 日志)
                               ▲
                               │
              ┌────────────────┼────────────────┐
              │                │                │
          ┌───┴────┐      ┌───┴────┐       ┌───┴────┐
          │天旦交易│      │Zabbix  │       │华为/科来│
          │监控/BPM│      │综合监控│       │NPM     │
          └────────┘      └────────┘       └────────┘
          ┌────────┐
          │平行线  │
          │机房监控│
          └────────┘
          ↑ 所有告警源统一由 Eventide 汇聚，MeridianOps 不直连 ↑
```

## 功能模块矩阵（30+ 模块 × 状态 × 归属 Phase）

> 状态标记：✅ 已实现（框架齐 = 有页面但用 mock 数据） / ⏳ 待开发（Phase N）/ 🚧 规划中（Phase N）

| 分类 | 模块 | 路径 | 状态 | 归属 Phase | 说明 |
|------|------|------|------|-----------|------|
| 🧭 **工作台** | 个人工作台 | `/dashboard` | ⏳ 待开发（**Phase 3**） | Phase 3 | 待办 + 最近活动 + 快捷入口，登录默认首页 |
| 🧭 **工作台** | 全局搜索 | 顶部搜索框 | ⏳ 待开发（**Phase 3**） | Phase 3 | 跨用户/角色/部门/审计/知识一键搜索 |
| 🧭 **工作台** | 通知中心 | 铃铛 + `/notifications` | ⏳ 待开发（**Phase 3**） | Phase 3 | 操作通知 + 未读 badge + 已读管理 |
| 🧭 **工作台** | 密码过期/会话超时 | — | ⏳ 待开发（**Phase 3**） | Phase 3 | 90 天强制改密 + 30 分钟无操作登出 |
| 📊 **分析** | 报表中心 | `/reports` | ⏳ 待开发（**Phase 3**） | Phase 3 | 审计 TOP 10 + 登录统计 + Excel 导出 |
| 📊 **分析** | 态势中心 | `/overview` | ✅ 已实现（框架齐，**Phase 5** 接真数） | Phase 5 | 全局告警分级 + 服务健康 + 最近活动 |
| 📊 **分析** | AIOps 诊断 | `/aiops` | ✅ 已实现（框架齐，**Phase 5** 接真数） | Phase 5 | 相似告警推荐 + 异常趋势检测 |
| 📊 **分析** | 根因分析 RCA | 告警详情 Tab | 🚧 规划中（Phase 7） | Phase 7 | 拓扑 + 告警时序归因 |
| 🔗 **融合** | 资产管理 | `/assets` | ✅ 已实现（框架齐，**Phase 4** 接 CRUD） | Phase 4 | 主机/IP/业务/负责人 + Excel 导入 |
| 🔗 **融合** | 告警中心 | `/alerts` | ⏳ 待开发（Phase 6） | Phase 6 | 统一告警列表（Eventide）+ 确认/关闭 |
| 🔗 **融合** | 告警详情 | 弹窗 | ⏳ 待开发（Phase 6） | Phase 6 | 归属业务 + 关联服务 + 跳 ELK |
| 🔗 **融合** | 拓扑视图 | `/topology` | ⏳ 待开发（Phase 6） | Phase 6 | 业务→主机两层拓扑，告警节点高亮 |
| 🔗 **融合** | 日志中心 | `/logs` | ✅ 已实现（框架齐，**Phase 5** 完善） | Phase 5 | 日志查询 UI + ELK 跳转预留 |
| 🔗 **融合** | 容器管理 | `/containers` | ✅ 已实现（框架齐，**Phase 5** 完善） | Phase 5 | K8s 资源视图 |
| 🔗 **融合** | DB 数据库 | `/database` | ✅ 已实现（框架齐，**Phase 5** 完善） | Phase 5 | 数据库实例管理 + 连接测试 |
| ⚡ **自动化** | 作业中心 | `/jobs` | ✅ 已实现（框架齐，**Phase 4** 接剧本库） | Phase 4 | 剧本 CRUD + 执行 + 历史 |
| ⚡ **自动化** | 工单系统 | `/tickets` | ✅ 已实现（框架齐，**Phase 4** 接 ITIL） | Phase 4 | 状态流转 + 指派 + 评论 |
| ⚡ **自动化** | 自愈编排 | 告警联动 | 🚧 规划中（Phase 7） | Phase 7 | 白名单告警自动执行剧本 |
| 📚 **知识** | 知识库 | `/knowledge` | ⏳ 待开发（**Phase 4**） | Phase 4 | CRUD + 标签 + 全文检索 + 版本化 |
| 📚 **知识** | 知识推荐 | 告警详情 Tab | 🚧 规划中（Phase 7） | Phase 7 | 相似告警智能匹配知识条目 |
| 🛡️ **合规** | 用户管理 | `/system/users` | ✅ 已实现 | Phase 1 | CRUD + 启停 + 重置密码 |
| 🛡️ **合规** | 角色管理 | `/system/roles` | ✅ 已实现 | Phase 1 | CRUD + 权限分配 |
| 🛡️ **合规** | 部门管理 | `/system/departments` | ✅ 已实现 | Phase 1 | 树形 CRUD + 删除保护 |
| 🛡️ **合规** | 个人中心 | `/profile` | ✅ 已实现 | Phase 2 | 资料 / 自助改密 / 登录历史 |
| 🛡️ **合规** | 系统设置 | `/system` | ✅ 已实现 | Phase 2 | 密码策略 + 锁定参数 |
| 🛡️ **合规** | 审计中心 | `/audit` | ✅ 已实现 | Phase 1 | 操作日志分页查询 + 筛选 |
| 🛡️ **合规** | 安全加固 | — | ✅ 已实现 | Phase 2 | 登录锁定 + 密码强度 + JWT |
| 🛡️ **合规** | 审批中心 | `/approvals` | ⏳ 待开发（**Phase 4**） | Phase 4 | 高危操作双人审批流 |
| 🛡️ **合规** | TOTP 双因子登录 | — | ⏳ 待开发（**Phase 4**） | Phase 4 | Google Authenticator 绑定/验证 |
| 💼 **运营** | 配置中心 | `/config` | ✅ 已实现（框架齐，**Phase 4** 接 CRUD） | Phase 4 | Agent 配置 + 版本历史 + 热更 |
| 💼 **运营** | 费用中心 | `/cost` | ✅ 已实现（框架齐，**Phase 5** 完善） | Phase 5 | 成本统计图表 |
| 💼 **运营** | Webhook 配置 | `/webhooks` | ⏳ 待开发（**Phase 5**） | Phase 5 | 飞书/钉钉/邮件通知通道 |

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

### Phase 3 🔥 **立即推进（P0，预计 3-4 周）：自身功能完善**

> 目标：**把 MeridianOps 自身功能做扎实**，不依赖外部系统对接，让平台自闭环可用

#### 🧭 个人工作台（Dashboard）—— 值班员首页
- [ ] 后端：`GET /api/dashboard` 聚合接口（用户待办 + 最近审计 + 系统状态）
- [ ] 前端：新建 `DashboardPage.vue`
  - 快捷入口卡片：用户管理 / 角色 / 审计 / 系统设置
  - 最近活动列表（从 audit_logs 拉最近 20 条）
  - 系统状态概览（在线用户数 + 总用户数 + 今日操作数）
- [ ] 路由：`/dashboard` 设为登录后默认跳转页

#### 📊 报表中心 MVP —— 给领导看成果
- [ ] 后端：新建 `report_routes.rs`
  - `GET /api/reports/overview`：平台使用概览（总用户数/登录次数/操作次数/按日统计）
  - `GET /api/reports/audit-by-top`：审计操作 TOP 10（按用户/按类型）
  - `GET /api/reports/login-stats`：登录统计（成功/失败次数/锁定次数）
  - `GET /api/reports/export`：导出 Excel
- [ ] 前端：新建 `ReportsPage.vue`
  - 统计卡片：总用户 / 今日登录 / 今日操作 / 告警数（预留）
  - 操作审计 TOP 10 表格
  - 登录失败统计图表
  - 导出 Excel 按钮

#### 🔍 全局搜索 MVP —— 跨模块搜索
- [ ] 后端：`GET /api/search?q=` 聚合搜索
  - 搜用户（username/display_name/email）
  - 搜角色（name/display_name）
  - 搜部门（name）
  - 搜审计日志（actor/action/target_id）
  - 搜系统设置（setting_key）
- [ ] 前端：顶部搜索框 → 回车跳搜索结果页（分 Tab 展示各组结果）
- [ ] 新建 `SearchPage.vue` 搜索结果页

#### � 通知中心 MVP —— 站内信
- [ ] 后端：新建 `notification_routes.rs`
  - `GET /api/notifications`：当前用户通知列表（分页）
  - `PUT /api/notifications/:id/read`：标记已读
  - `PUT /api/notifications/read-all`：全部已读
  - `POST /api/notifications`：创建通知（系统内部使用）
  - 新建 `notifications` 表（id/user_id/type/title/content/is_read/created_at）
- [ ] 前端：MainLayout 铃铛接真实数据
  - 下拉显示最近 5 条通知 + 未读计数 badge
  - 点击全部跳通知中心页
  - 新建 `NotificationsPage.vue`：通知列表 + 已读/全部已读
- [ ] 触发场景：登录成功/密码修改/用户创建/权限变更 → 自动生成通知

#### 🛡️ 合规补全（第一批）
- [ ] 数据库：`users` 表加 `password_changed_at` + `must_change_password` 字段
- [ ] 后端 auth.rs：密码过期校验（90 天）+ 首次登录强制改密拦截
- [ ] 前端路由守卫：JWT 过期 / 30 分钟无操作自动登出（后端 JWT token 自带 exp，过期后 401 → 前端跳登录）
- [ ] 迁移文件：`000009_add_password_expiry.sql`

**Phase 3 结束验收标准：** 值班员登录后看到个人工作台首页；能搜用户/审计/部门；能导出 Excel 报表；通知中心能收到操作提醒；密码过期会被强制改密。

---

### Phase 4 ⚡ **短期推进（P1，预计 4-6 周）：自身功能深化**

> 目标：让 MeridianOps 具备完整的「自我管理 + 知识沉淀 + 流程闭环」能力

#### 📚 知识库 —— 知识沉淀 + 检索
- [ ] 迁移：`knowledge_items` 表（id/title/category/tags/content/markdown/created_by/created_at/updated_at/version）
- [ ] 后端：新建 `knowledge_routes.rs`
  - CRUD：GET/POST/PUT/DELETE `/api/knowledge`
  - 搜索：`GET /api/knowledge/search?q=` 全文检索
  - 分类：`GET /api/knowledge/categories`
  - 标签：`GET /api/knowledge/tags`
- [ ] 前端：新建 `KnowledgePage.vue`
  - 知识库列表（分类筛选 + 标签筛选 + 搜索）
  - 富文本编辑器（Markdown）
  - 知识详情页 + 版本历史
- [ ] 种子数据：预置 10+ 条常见运维知识条目

#### ⚡ 作业中心 —— 剧本库 + 执行
- [ ] 迁移：`job_scripts` 表（id/name/description/category/steps_json/status/created_by/created_at）+ `job_executions` 表（id/script_id/triggered_by/params/status/result/started_at/finished_at）
- [ ] 后端：完善 `job_routes.rs`
  - 剧本 CRUD：GET/POST/PUT/DELETE `/api/jobs/scripts`
  - 执行：POST `/api/jobs/scripts/:id/execute`（模拟执行 → 后续接 AxleOps）
  - 执行历史：`GET /api/jobs/executions`
- [ ] 前端：完善 `JobsPage.vue`
  - 剧本列表 + 分类 + 状态
  - 剧本详情 + 步骤编辑
  - 执行按钮 + 执行日志
  - 预置 5 个高频剧本：重启服务/清理磁盘/回滚/探活/配置同步

#### 📋 工单系统 —— ITIL 流程
- [ ] 迁移：`tickets` 表（id/title/type/priority/status/description/assignee/created_by/created_at/updated_at）+ `ticket_comments` 表
- [ ] 后端：新建 `ticket_routes.rs`
  - CRUD：GET/POST/PUT/DELETE `/api/tickets`
  - 状态流转：POST `/api/tickets/:id/transition`（open → in_progress → resolved → closed）
  - 指派：POST `/api/tickets/:id/assign`
  - 评论：POST `/api/tickets/:id/comments`
- [ ] 前端：完善 `TicketsPage.vue`
  - 工单列表 + 状态看板
  - 新建工单 + 编辑 + 详情
  - 状态流转按钮 + 评论区

#### ⚙️ 配置中心 —— Agent 配置管理
- [ ] 迁移：`agent_configs` 表（id/config_key/config_value/agent/env/description/created_at/updated_at）+ `config_history` 表
- [ ] 后端：完善 `config_routes.rs`
  - CRUD：GET/POST/PUT/DELETE `/api/configs`
  - 历史：`GET /api/configs/:key/history`
  - 热更：POST `/api/configs/:key/reload`（模拟 → 后续接 Agent）
- [ ] 前端：完善 `ConfigPage.vue`（替换 mock 数据）

#### 🛡️ 合规补全（第二批）
- [ ] 迁移：`approvals` 表（id/type/target_id/action/reason/status/requester/approver/created_at/approved_at）
- [ ] 后端：审批中间件 `approval.rs`
  - 高危操作（用户删除/角色分配/系统配置修改）→ 自动创建审批单 → 等待第二人批准
  - `GET /api/approvals`：审批列表
  - `POST /api/approvals/:id/approve`：批准
  - `POST /api/approvals/:id/reject`：拒绝
- [ ] 前端：审批页面 + 通知中心审批提醒
- [ ] TOTP 双因子：`users` 表加 `totp_secret` + 绑定/验证流程

**Phase 4 结束验收标准：** 知识库有 50+ 条；作业中心能执行剧本；工单能走完完整生命周期；高危操作必须双人审批。

---

### Phase 5 🚧 **中期推进（P2，预计 3-4 周）：剩余模块完善 + 融合准备**

> 目标：完成所有自身功能模块，为对接外部系统做准备

#### 📊 剩余模块完善
- [ ] 态势中心 OverviewPage：接真实聚合 API（替换 mock 数据）
- [ ] AIOps 诊断 AIOpsPage：接真实相似检索（基于审计/告警历史）
- [ ] 日志中心 LogsPage：日志查询 UI（mock 数据 → 预留 ELK 接口）
- [ ] 容器管理 ContainersPage：完善 mock 数据 + K8s 资源视图
- [ ] DB 数据库 DatabasePage：数据库实例管理 + 连接测试
- [ ] 费用中心 CostPage：成本统计数据模型 + 图表展示

#### 📨 通知中心升级
- [ ] 后端：Webhook 配置 API（`GET/POST /api/webhooks`）+ 站内信 + 邮件通知通道
- [ ] 前端：Webhook 配置页（飞书/钉钉/邮件）

#### 🔗 融合准备（为 Phase 6 铺路）
- [ ] 后端：`system_routes.rs` 扩展（系统配置 + 健康检查接口）
- [ ] 前端：系统状态监控页（已接入系统在线/离线状态）
- [ ] 编写 Eventide / AxleOps / 优云 CMDB 对接 Adapter 骨架（mock 模式）

**Phase 5 结束验收标准：** 所有 16 个功能模块全部可用（无空壳页面）；导航菜单无占位符。

---

### Phase 6 🌐 **融合对接（P3，预计 4-6 周）：接外部系统**

> 目标：对接全行生态，实现 1-5-10 闭环

#### 告警接入
- [ ] Eventide 告警 API 对接：告警列表/详情/确认/关闭/分级统计
- [ ] 告警 badge 接真实未处理数
- [ ] 态势大屏告警卡片接真实数据

#### 资产融合
- [ ] 优云 CMDB API 对接：主机/业务系统/配置关系自动同步
- [ ] 资产清单自动更新（+ Excel 手动导入兜底）

#### 上游系统对接
- [ ] 2oauth SSO 对接：登录认证 + 用户信息同步
- [ ] 中国移动短信/邮件网关对接：告警/审批通知下发
- [ ] 理想自动化批处理对接：作业执行通道
- [ ] 优云 ITIL 工单对接：工单双向同步

#### 深度融合
- [ ] 告警详情上下文：归属业务 + 关联服务 + 发布历史
- [ ] 拓扑视图：业务系统 → 主机两层拓扑（从 CMDB 自动构建）
- [ ] 外部系统 iframe 嵌入：ELK / Eventide / AxleOps 深度操作无缝切换

**Phase 6 结束验收标准：** 告警从 Eventide 实时同步；资产从 CMDB 自动更新；通知能通过短信/邮件发出；告警详情能看到归属业务 + 历史发布。

---

### Phase 7 � **长期展望（P4，预计 3 个月+）：智能化 + AI**

#### 智能分析（真·AIOps）
- [ ] 根因分析 RCA：拓扑 + 告警时序归因，高亮「最可能根因节点」
- [ ] 相似告警智能推荐知识：TF-IDF / 向量相似度推 Top 3 历史方案
- [ ] 异常趋势检测：告警量 / MTTR 环比同比异常波动主动提醒
- [ ] 自动自愈：白名单告警触发自动执行剧本
- [ ] 报表中心升级：PDF 导出 + 邮件定时发送周报月报

#### 融合深化
- [ ] 四层拓扑视图：网络层 → 主机层 → 应用层 → 业务层
- [ ] 容器管理接 K8s API：集群状态 / Pod 列表
- [ ] 全链路追踪接入（Skywalking / Pinpoint）

#### AI + 成本优化
- [ ] AI 大模型运维助手：聊天式问"这个告警怎么办""昨天 5xx 原因"
- [ ] 成本优化分析：费用中心 + 资源利用率，推荐缩容方案
- [ ] 多数据中心统一视图
- [ ] 移动门户 / 小程序

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
# 生产环境必须通过 MERIDIANOPS_DB_URL 环境变量覆盖
url = "mysql://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:3306/meridianops"
max_connections = 10
min_connections = 2

[auth]
# 生产环境必须通过 MERIDIANOPS_JWT_SECRET 环境变量覆盖
jwt_secret = "replace-via-env-var"
token_ttl_hours = 24
seed_username = "admin"
# 初始种子密码，首次启动创建 admin 后立即修改
seed_password = "${INIT_ADMIN_PASSWORD}"

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
auth_token = "${EVENTIDE_API_TOKEN}"
```

> ⚠️ **安全提示**：生产部署时请务必通过环境变量注入敏感配置，示例中的占位符 `${...}` 用于示意，实际值通过 `MERIDIANOPS_DB_URL` / `MERIDIANOPS_JWT_SECRET` 等环境变量覆盖，避免写入代码仓库。

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
| 09 | 000009_add_password_expiry.sql | users 加 password_changed_at + must_change_password + totp_secret | ⏳ Phase 3 |
| 10 | 000010_create_notifications.sql | 建 notifications 表 | ⏳ Phase 3 |
| 11 | 000011_create_knowledge_items.sql | 建 knowledge_items 表 | ⏳ Phase 4 |
| 12 | 000012_create_job_scripts.sql | 建 job_scripts + job_executions 表 | ⏳ Phase 4 |
| 13 | 000013_create_tickets.sql | 建 tickets + ticket_comments 表 | ⏳ Phase 4 |
| 14 | 000014_create_approvals.sql | 建 approvals 表（双人审批） | ⏳ Phase 4 |
| 15 | 000015_create_agent_configs.sql | 建 agent_configs + config_history 表 | ⏳ Phase 4 |

启动 Gateway 时 `sqlx::migrate!("./migrations").run(&pool)` 自动按顺序执行，幂等安全。

## 关联项目（自研上下游）

- [AxleOps](https://github.com/lvpenglive/axleops) — 服务生命周期管理 / 发布 / 剧本执行（MeridianOps 数据融合 + 作业执行通道）
- [Eventide](https://github.com/lvpenglive/eventide) — 告警事件中枢 / 多源告警接入 / 去重降噪（MeridianOps **唯一告警数据源**）

## License

MIT

## 错误处理与日志规范

### 错误响应格式（统一输出）

成功：
```json
{ "code": 0, "data": { ... } }
```

错误（由 `AppError` 统一处理）：
```json
{ "code": 401, "message": "用户名或密码错误" }
```

| code | 含义 |
|------|------|
| 0 | 成功 |
| 401 | 未登录 / token 过期 |
| 403 | 无权限 |
| 400 | 参数错误 |
| 404 | 资源不存在 |
| 500 | 服务器内部错误 |

### 日志框架

- 后端使用 `tracing` crate 输出结构化日志
- 日志级别：开发环境 `info`，生产环境可配置 `warn`
- 关键日志：登录事件、审计操作、权限变更、系统错误

### 认证方式

- JWT (HS256) + 24 小时有效期 + localStorage 持久化
- 密码哈希：Argon2id（内存 64MB, 迭代 3, 并行 1）
- 传输安全：生产环境需部署 HTTPS 反向代理（Nginx/Caddy）

## 部署架构建议

```
                    ┌──────────────┐
                    │   客户端     │
                    └──────┬───────┘
                           │ HTTPS
                    ┌──────▼───────┐
                    │  Nginx 反代   │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
         ┌─────────┐  ┌─────────┐  ┌─────────┐
         │ Gateway │  │ Gateway │  │  Portal  │
         │ (8000)  │  │ (8001)  │  │  (5173)  │
         └────┬────┘  └────┬────┘  └────┬────┘
              │             │            │
              └──────┬──────┘            │
                     ▼                   │
                 ┌─────────┐             │
                 │  MySQL  │◀────────────┘
                 │ 主从集群│
                 └─────────┘
```

- Gateway 多实例部署，前面 Nginx 做负载均衡
- MySQL 建议主从，只读查询走从库
- Portal 静态文件用 Nginx 直接托管（生产环境用 `npm run build` 打包）
- 敏感配置全部通过环境变量注入，不写入配置文件

## CHANGELOG

### v0.2.0 (2026-08-11)
- ✅ 完成基础设施地基 Phase 0-2
- ✅ JWT + RBAC + 审计中心 + 安全加固
- ✅ 架构重绘：对齐全行真实生态系统
- ✅ 路线图规划：7 Phase 全面计划表

### v0.1.0 (2026-08-11)
- ✅ 项目初始化
- ✅ Gateway 框架 + Portal 框架
