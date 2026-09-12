# MeridianOps 架构审查报告

**审查日期**：2026-09-12  
**审查范围**：`gateway/` (Rust/Axum API 网关) + `portal/` (Vue 3 SPA)  
**上下文**：银行级运维平台，告警经 Eventide 系统流转，CMDB/AxleOps/ELK 日志经网关访问

---

## 执行摘要

MeridianOps 是一个功能完整的运维中台，涵盖告警管理、工单工作流、CMDB、通知引擎、作业执行等核心能力。代码整体质量良好，安全实践（参数化 SQL、密码 Argon2 哈希、JWT 鉴权）基本到位。主要风险集中在：

1. **P0（紧急）**：生产环境配置硬编码风险、CORS 策略过于宽松
2. **P1（高优先级）**：God 文件需拆分、缺乏限流与熔断机制、测试覆盖不足
3. **P2（中优先级）**：缺少结构化 metrics/tracing、数据库连接池偏小、前端状态管理可优化

---

## 1. 模块边界与耦合度

### 1.1 Gateway 模块结构

```
gateway/src/
├── main.rs          # 启动入口 + 调度器挂载
├── routes.rs        # 路由聚合 + AppState 定义
├── auth.rs          # JWT/密码/API Token 认证核心
├── db.rs            # 数据库操作（⚠️ 3211 行 God 文件）
├── alert_routes.rs  # 告警管理（⚠️ 2507 行）
├── cmdb_routes.rs   # CMDB CI 管理（⚠️ 2057 行）
├── ticket_routes.rs # 工单 + 工作流（⚠️ 1969 行）
├── notification_*.rs # 通知引擎相关
├── workflow_engine.rs # 工作流编译 + 运行时
└── *_scheduler.rs   # 后台调度任务
```

**问题**：

| 文件 | 行数 | 问题 |
|------|------|------|
| `db.rs` | 3211 | God 文件，混合了 users/roles/audit/cmdb/tickets 等所有领域的数据访问 |
| `alert_routes.rs` | 2507 | 告警路由 + Eventide 双向同步 + webhook 接收，职责过重 |
| `cmdb_routes.rs` | 2057 | CI 模型/实例 CRUD + 外部同步 + 拓扑计算混在一起 |
| `ticket_routes.rs` | 1969 | 工单 CRUD + 工作流推进 + 导出，建议拆分 |

**建议**：

- 将 `db.rs` 按领域拆分为 `db/users.rs`, `db/cmdb.rs`, `db/tickets.rs` 等子模块
- 将 Eventide 集成逻辑抽取为独立的 `eventide_client.rs` 模块
- 告警接入（webhook ingress）与告警业务路由分离

### 1.2 Portal 模块结构

```
portal/src/
├── api/            # 后端 API 封装（types.ts 821 行偏大）
├── stores/         # Pinia 状态管理
├── views/          # 46 个页面组件
├── components/     # 公共组件
├── composables/    # 组合式函数
└── router/         # 路由配置（348 行）
```

**亮点**：前端结构清晰，API 层与视图层解耦良好，Pinia store 按功能划分。

**建议**：`api/types.ts` 可按领域拆分（`types/alert.ts`, `types/ticket.ts` 等）。

---

## 2. 安全与配置风险

### 2.1 P0 — 生产环境配置硬编码

**位置**：`gateway/src/config.rs`

```rust
// L144-146：默认 MySQL URL 包含真实服务器 IP 和密码
fn default_mysql_url() -> String {
    "mysql://root:886363@120.26.67.180:3306/meridianops".to_string()
}
```

```rust
// L249-254：Eventide 默认 token 硬编码
SystemConfig {
    id: "eventide".to_string(),
    auth_token: Some("eventide-admin-token".to_string()),
    // ...
}
```

**风险**：代码泄露即暴露生产凭据。

**修复**：
1. 移除所有硬编码凭据，改用占位符（如 `REPLACE_ME`）
2. 强制环境变量覆盖：`MERIDIANOPS_DB_URL`, `EVENTIDE_AUTH_TOKEN`
3. 启动时校验敏感配置项非占位符，否则 panic

### 2.2 P0 — CORS 策略过于宽松

**位置**：`gateway/src/routes.rs` L97

```rust
.layer(tower_http::cors::CorsLayer::permissive())
```

**位置**：`gateway/src/config.rs` L127

```rust
cors_origins: vec!["*".to_string()],
```

**风险**：允许任意域名跨域请求，可能被 CSRF/跨站攻击利用。

**修复**：
1. 生产环境配置具体的允许域名列表
2. 启动时检测 `cors_origins` 包含 `*` 时发出警告

### 2.3 P1 — 加密密钥硬编码开发默认值

**位置**：`gateway/src/crypto.rs` L7-12

```rust
const DEFAULT_DEV_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn get_encryption_key() -> Result<[u8; 32], CryptoError> {
    let hex_key = std::env::var("MERIDIANOPS_ENCRYPTION_KEY")
        .unwrap_or_else(|_| {
            tracing::warn!("MERIDIANOPS_ENCRYPTION_KEY 未设置，使用开发默认值（仅限开发环境）");
            DEFAULT_DEV_KEY.to_string()
        });
```

**亮点**：有环境变量覆盖机制并输出警告。

**建议**：非 loopback bind 时若检测到默认密钥，应 panic 而非仅警告。

### 2.4 P1 — JWT Secret 默认值检测

**位置**：`gateway/src/main.rs` L70-76

```rust
if config.auth.jwt_secret == "meridianops-dev-secret-change-me"
    && !config.server.bind.starts_with("127.0.0.1")
{
    tracing::warn!("JWT secret 仍是默认值且 bind 非 loopback，生产环境必须通过 MERIDIANOPS_JWT_SECRET 覆盖");
}
```

**亮点**：已有检测逻辑。

**建议**：升级为 panic，阻止生产环境启动。

### 2.5 安全亮点 ✅

| 实践 | 位置 | 说明 |
|------|------|------|
| 参数化 SQL | 全部 | 所有 `sqlx::query` 均使用 `.bind()` 防注入 |
| 密码 Argon2 哈希 | `auth.rs` | `argon2::hash_encoded` + 验证 |
| API Token 安全存储 | `auth.rs` L147 | 仅存储 SHA256 哈希，不可逆 |
| XSS 防护 | `TicketDetailPage.vue` | Markdown 渲染前先 `escapeHtml()` |
| JWT 权限传递 | `stores/user.ts` | 前端从 JWT 解析权限，后端二次校验 |
| 密码策略 | `auth.rs` | 长度/复杂度/过期/历史记录检查 |

---

## 3. 可靠性与运维

### 3.1 P1 — 缺乏 Rate Limiting

**搜索结果**：在整个代码库中未找到 `rate_limit`, `throttle` 等关键词。

**风险**：API 端点（尤其是登录、告警上报）可被暴力攻击。

**建议**：
- 引入 `tower-governor` 或自定义限流中间件
- 关键端点：`/api/auth/login` (5/min/IP)、`/api/alerts/ingress/*` (100/min/token)

### 3.2 P1 — 缺乏 Circuit Breaker / Retry

**搜索结果**：未找到熔断或重试逻辑。

**位置**：`gateway/src/alert_routes.rs` L2209-2250（Eventide 回写）

```rust
// best-effort 同步，失败仅记录日志
Err(e) => {
    tracing::warn!(target: "eventide_writeback", error = %e, "Eventide 回写请求失败");
}
```

**风险**：
- Eventide 服务故障时大量同步请求堆积
- 无指数退避重试导致瞬时故障放大

**建议**：
- 对外部系统调用（Eventide、AxleOps、ELK）引入熔断器
- 添加带 jitter 的指数退避重试

### 3.3 健康检查

**位置**：`gateway/src/routes.rs` L27, L64-66

```rust
.route("/api/health", get(health_check))
// ...
async fn health_check() -> &'static str { "ok" }
```

**问题**：仅返回静态 `ok`，未检查数据库/Redis/Eventide 连接状态。

**建议**：
```rust
// 推荐的健康检查
async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db_ok = sqlx::query("SELECT 1").fetch_one(&state.db).await.is_ok();
    if db_ok { "ok" } else { (StatusCode::SERVICE_UNAVAILABLE, "db_down") }
}
```

### 3.4 Graceful Shutdown ✅

**位置**：`gateway/src/main.rs` L139-145, L258-280

```rust
axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
    .with_graceful_shutdown(shutdown_signal())
    .await?;
```

**亮点**：已实现 SIGTERM/SIGINT 优雅关闭。

---

## 4. 性能与可扩展性

### 4.1 P2 — 数据库连接池偏小

**位置**：`gateway/src/config.rs` L147-152

```rust
fn default_max_conn() -> u32 { 10 }
fn default_min_conn() -> u32 { 1 }
```

**风险**：高并发场景下连接池耗尽导致请求排队。

**建议**：
- 默认 `max_connections` 提升至 20-50
- 添加连接池监控指标

### 4.2 P2 — 后台任务调度

**位置**：`gateway/src/main.rs` L120-136

```rust
tokio::spawn(cmdb_routes::pull_scheduler_loop(state.db.clone()));
tokio::spawn(knowledge_routes::resegment_knowledge_content(state.db.clone()));
tokio::spawn(ticket_scheduler::start_scheduler(state.db.clone()));
tokio::spawn(notification_cleaner::start_scheduler(...));
tokio::spawn(log_alert_scheduler::start_scheduler(state.clone()));
```

**问题**：
- 所有后台任务在同一进程内运行，单实例部署
- 无分布式锁，多实例部署会重复执行

**建议**：
- 关键调度任务使用 Redis 分布式锁
- 考虑引入轻量级任务队列（如 `sqlx` + `SKIP LOCKED`）

### 4.3 日志/Tracing ✅

**位置**：`gateway/src/main.rs` L54-56

```rust
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
    .init();
```

**亮点**：已集成 `tracing`，支持 `RUST_LOG` 环境变量控制日志级别。

**建议**：
- 添加结构化日志字段（request_id, user_id）
- 集成 OpenTelemetry 实现分布式追踪

---

## 5. 可维护性债务

### 5.1 P1 — 测试覆盖不足

**搜索结果**：

| 类型 | 文件数 |
|------|--------|
| Rust `#[test]` | 仅 `crypto.rs` 有 4 个测试 |
| Vue `*.spec.ts` / `*.test.ts` | 0 |

**风险**：重构、新功能开发无安全网。

**建议**：
- 优先补充 `auth.rs`, `workflow_engine.rs` 的单元测试
- 添加核心 API 端点的集成测试
- 前端关键组件添加 Vitest 测试

### 5.2 P2 — unwrap()/expect() 使用

**搜索结果**：`crypto.rs` 7 处，`main.rs` 3 处，其他文件少量。

**评估**：多数位于测试代码或启动阶段（可接受），但建议生产代码全部替换为 `?` 或 `.ok()`。

### 5.3 P2 — 数据库迁移 Checksum 绕过

**位置**：`gateway/src/main.rs` L150-254

```rust
// 由于迁移文件在历史上多次被调整注释/内容，若严格按 sqlx 默认的 checksum 对比会在
// 启动时 panic。这里按 version 做幂等：已 applied 跳过，未 applied 按 sqlx 顺序 apply。
```

**问题**：绕过 checksum 校验存在风险，可能导致不一致的迁移状态。

**建议**：
- 长期应重建干净的迁移历史
- 短期在绕过逻辑中添加显式警告日志

---

## 6. 主要功能结构评估

### 6.1 告警系统 ✅

- **Eventide 集成**：双向同步（webhook 接收 + API 回写）设计合理
- **去重指纹**：`eventide:` / `local:` 前缀区分来源，避免跨路径碰撞
- **通知引擎**：支持 Email/Feishu/Webhook/SMS 多渠道，规则驱动

### 6.2 工单工作流 ✅

- **LogicFlow 编译器**：`workflow_engine.rs` 实现了完整的节点编译、条件网关、审批人解析
- **SLA 调度**：`ticket_scheduler.rs` 实现超时自动升级，有幂等保护
- **状态派生**：根据当前节点类型自动推导工单状态

### 6.3 CMDB ⚠️

- **功能完整**：CI 模型/实例 CRUD、关系管理、外部同步（蓝鲸等）
- **问题**：`cmdb_routes.rs` 过于庞大（2057 行），建议拆分为：
  - `cmdb/models.rs` — CI 模型管理
  - `cmdb/instances.rs` — CI 实例 CRUD
  - `cmdb/sync.rs` — 外部同步
  - `cmdb/topology.rs` — 拓扑计算

### 6.4 认证授权 ✅

- **多认证方式**：JWT (用户) + API Token (mk- 前缀) + 禁用模式（开发用）
- **RBAC**：角色 → 权限点，权限点按模块组织
- **安全策略**：密码复杂度、过期强制修改、历史密码检测

---

## 7. 优先级优化 Backlog

### P0 — 紧急（阻塞生产部署）

| # | 问题 | 位置 | 修复建议 |
|---|------|------|----------|
| 1 | MySQL URL 硬编码真实凭据 | `config.rs:144-146` | 替换为占位符 + 强制环境变量 |
| 2 | Eventide token 硬编码 | `config.rs:254` | 同上 |
| 3 | CORS 允许所有来源 | `routes.rs:97` | 生产环境配置具体域名 |
| 4 | 加密密钥默认值非 panic | `crypto.rs:7-12` | 非 loopback 时 panic |

### P1 — 高优先级

| # | 问题 | 位置 | 修复建议 |
|---|------|------|----------|
| 5 | `db.rs` 3211 行 God 文件 | `db.rs` | 按领域拆分子模块 |
| 6 | 无 API 限流 | 全局 | 引入 `tower-governor` |
| 7 | 无熔断/重试 | `alert_routes.rs` | 外部调用添加熔断器 |
| 8 | 健康检查无依赖探测 | `routes.rs:64-66` | 添加 DB/Redis 检查 |
| 9 | 测试覆盖接近零 | 全局 | 优先补充 auth/workflow 测试 |
| 10 | JWT secret 默认值仅警告 | `main.rs:70-76` | 升级为 panic |

### P2 — 中优先级

| # | 问题 | 位置 | 修复建议 |
|---|------|------|----------|
| 11 | 连接池 max_conn=10 偏小 | `config.rs:147` | 提升至 20-50 |
| 12 | 无分布式锁 | 调度器 | 添加 Redis 锁 |
| 13 | 无结构化 metrics | 全局 | 集成 Prometheus |
| 14 | `api/types.ts` 821 行 | `portal/src/api/` | 按领域拆分 |
| 15 | 迁移 checksum 绕过 | `main.rs:150-254` | 重建迁移历史 |

---

## 附录：代码行数统计

```
gateway/src/*.rs 按行数排序：
  3211  db.rs
  2507  alert_routes.rs
  2057  cmdb_routes.rs
  1969  ticket_routes.rs
  1170  notification_routes.rs
   802  log_routes.rs
   521  job_routes.rs
   443  workflow_engine.rs
   354  auth.rs
   348  ticket_scheduler.rs
   ...
```

```
portal/src/ 按行数排序（TS）：
   821  api/types.ts
   419  api/ticket.ts
   348  router/index.ts
   280  api/notification.ts
   278  api/alert.ts
   239  stores/user.ts
   ...
```

---

## 8. 补充发现（深度审查）

### 8.1 后台调度器与引擎问题

| 问题 | 位置 | 严重程度 | 说明 |
|------|------|----------|------|
| SSH 执行器无重试机制 | `ssh_executor.rs:206-244` | P0 | 网络瞬时故障导致任务永久失败 |
| 通知发送失败无重试 | `notification_engine.rs:484-568` | P0 | 关键告警通知可能丢失 |
| 日志告警静默期检查竞态 | `log_alert_scheduler.rs:79-90` | P0 | 多实例可能产生重复告警 |
| 工单调度器无分布式锁 | `ticket_scheduler.rs:18-30` | P1 | 多实例重复处理 |
| SSH 主机密钥验证被跳过 | `ssh_executor.rs:106-118` | P1 | 中间人攻击风险 |
| 工单编号生成器并发冲突 | `workflow_engine.rs:16-50` | P1 | COUNT+INSERT 高并发下不可靠 |
| ClickHouse 查询无超时 | `log_alert_scheduler.rs:57-64` | P1 | 可能阻塞调度轮次 |
| 邮件仅发给第一个收件人 | `notification_engine.rs:163-169` | P2 | 多收件人仅首个收到 |

### 8.2 前端额外安全问题

| 问题 | 位置 | 严重程度 | 说明 |
|------|------|----------|------|
| Token 存储在 localStorage | `stores/user.ts:52-53` | P0 | XSS 可窃取 token |
| URL 参数传递 Token | `api/ticket.ts:386-396`, `MainLayout.vue:222-227` | P0 | 泄露到浏览器历史/日志 |
| Markdown 链接处理可绕过 | `views/aiops/AIOpsPage.vue:439-455` | P1 | `javascript:` 协议未过滤 |
| 同步数据源 API Token 明文 | `api/types.ts:540-548` | P1 | 应脱敏返回 |
| 路由守卫职责过重 | `router/index.ts:306-346` | P1 | 7+ 条件分支难维护 |
| 认证常量重复定义 | `request.ts` + `stores/user.ts` | P2 | 应集中管理 |

### 8.3 设计亮点 ✅

- **幂等设计**：`log_alert_scheduler` 使用 `fingerprint = log_surge:{hostname}:{bucket}` 保证不重复触发
- **分批删除**：`notification_cleaner` 使用 LIMIT 分批 + 批间 sleep 避免锁表
- **并发限流**：`ssh_executor` 使用 Tokio Semaphore 限制最大并发 SSH 会话
- **优雅退避**：调度器启动时延迟 30s 避免抢占数据库连接

---

## 9. 更新后的优先级 Backlog

### P0 — 紧急（共 8 项）

| # | 问题 | 位置 |
|---|------|------|
| 1 | MySQL URL 硬编码真实凭据 | `config.rs:144-146` |
| 2 | Eventide token 硬编码 | `config.rs:254` |
| 3 | CORS 允许所有来源 | `routes.rs:97` |
| 4 | 加密密钥默认值非 panic | `crypto.rs:7-12` |
| 5 | SSH 执行器无重试机制 | `ssh_executor.rs:206-244` |
| 6 | 通知发送失败无重试队列 | `notification_engine.rs:484-568` |
| 7 | Token 存储在 localStorage | `stores/user.ts:52` |
| 8 | URL 参数传递 Token | `api/ticket.ts:386`, `MainLayout.vue:222` |

### P1 — 高优先级（共 14 项）

| # | 问题 | 位置 |
|---|------|------|
| 1 | `db.rs` 3211 行 God 文件 | `db.rs` |
| 2 | 无 API 限流 | 全局 |
| 3 | 无熔断/重试 | `alert_routes.rs` |
| 4 | 健康检查无依赖探测 | `routes.rs:64-66` |
| 5 | 测试覆盖接近零 | 全局 |
| 6 | JWT secret 默认值仅警告 | `main.rs:70-76` |
| 7 | 工单调度器无分布式锁 | `ticket_scheduler.rs:18-30` |
| 8 | SSH 主机密钥验证被跳过 | `ssh_executor.rs:106-118` |
| 9 | 工单编号生成器并发冲突 | `workflow_engine.rs:16-50` |
| 10 | ClickHouse 查询无超时 | `log_alert_scheduler.rs:57-64` |
| 11 | Markdown 链接处理可绕过 XSS | `views/aiops/AIOpsPage.vue` |
| 12 | 同步数据源 Token 明文返回 | `api/types.ts:540-548` |
| 13 | 日志告警静默期检查竞态 | `log_alert_scheduler.rs:79-90` |
| 14 | 路由守卫职责过重 | `router/index.ts:306-346` |

---

*本报告由 Claude 自动生成，基于代码静态分析。建议结合实际运行时监控数据进行验证。*
