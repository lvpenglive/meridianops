# MeridianOps 基础设施地基 · 阶段一：持久化层 + 真实登录鉴权

## Context

MeridianOps 当前所有数据走 mock，登录页写死 `admin/admin` 不调任何 API（`setTimeout` 600ms 后塞 `mock-token-<ts>` 到 localStorage），网关无任何中间件、无持久化层、无鉴权。这是所有自有功能（AIOps 引擎、工单系统、知识库、自愈编排）走向真实化的前置阻塞。

阶段一目标：**打通"前端登录 → 网关 JWT 签发 → MySQL 用户校验 → 后续请求带 token → 网关验签"的完整闭环**，并为后续模块提供可复用的鉴权基建。

**已确认选型**：MySQL（与 Eventide 统一运维栈）· JWT 自签发 + 三级固定角色（admin/operator/viewer）· argon2 密码哈希 · sqlx 异步访问。

**阶段一不做**（留到阶段二）：限流/熔断/Prometheus metrics、完整 RBAC、用户管理 CRUD 页面、token 黑名单（服务端 logout 真失效）、基于角色的前端路由守卫、审计日志写入。

---

## 关键现状（已核实）

- 网关 `Cargo.toml` 仅有 tokio/axum 0.7/reqwest/serde/chrono/clap/toml/anyhow/thiserror。无 sqlx/jsonwebtoken/argon2/tower-http。
- `config.rs` 的 `ServerConfig` 有 `cors_origins` 字段但代码从未挂 CORS layer（形同虚设）。`load()` 无环境变量覆盖。
- `routes.rs` 的 `AppState { config, client }`。`proxy_request` 对所有系统无差别塞 `X-AxleOps-Token` + `Authorization: Bearer`，不区分 `auth_type`。
- `vite.config.ts` 极简，**无 dev server proxy**——前端 `/api` 在开发模式到不了网关。
- `LoginPage.vue` form 默认值写死 `{ username: 'admin', password: 'admin' }`，`handleLogin` 不调 API。
- `request.ts` 响应拦截器只 `ElMessage.error(error.message)`，**无 401 跳转**。
- `stores/user.ts` 的 `login(user, t)` 只写 localStorage，无过期处理。
- `router/index.ts` 守卫只检查 token 是否存在，不校验有效性。
- **grep 确认**：`portal/src/views/**` 下**没有任何文件调用 request**，全部 `import { mockXxx } from '../../mock/data'`。→ request 拦截器改造零破坏。

---

## 实现方案

### 一、网关侧（Rust + Axum 0.7）

#### 1. 依赖补充 — `gateway/Cargo.toml`

追加：
```toml
sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio-rustls", "mysql", "macros", "chrono", "uuid", "migrate"] }
jsonwebtoken = "9"
argon2 = { version = "0.5", features = ["std"] }
uuid = { version = "1", features = ["v4", "serde"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }
```
> `runtime-tokio-rustls` 比 native-tls 更易在 Windows 编译（免 OpenSSL）。argon2 0.5 依赖 cc 编译 C，本机 AxleOps 已用过 argon2，编译链已就绪。

#### 2. 配置扩展 — `gateway/src/config.rs`

新增两个 serde default 结构体（保证旧 toml 不改也能启动）：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    pub systems: Vec<SystemConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    #[serde(default = "default_mysql_url")]
    pub url: String,
    #[serde(default = "default_max_conn")]
    pub max_connections: u32,
    #[serde(default = "default_min_conn")]
    pub min_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,
    #[serde(default = "default_jwt_ttl")]
    pub token_ttl_hours: u64,
    #[serde(default = "default_seed_username")]
    pub seed_username: String,
    #[serde(default = "default_seed_password")]
    pub seed_password: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}
```

`load()` 末尾追加环境变量覆盖（参考 AxleOps 做法）：`MERIDIANOPS_DB_URL`、`MERIDIANOPS_JWT_SECRET`、`MERIDIANOPS_JWT_TTL_HOURS`、`MERIDIANOPS_SEED_USERNAME`、`MERIDIANOPS_SEED_PASSWORD`、`MERIDIANOPS_AUTH_ENABLED`、`MERIDIANOPS_SERVER_BIND`。

启动期检测：若 `jwt_secret` 仍是默认值且 `bind` 非 `127.0.0.1`，打 `tracing::warn!`。

`gateway-config.toml` 追加：
```toml
[database]
url = "mysql://meridianops:meridianops@127.0.0.1:3306/meridianops"
max_connections = 10
min_connections = 1

[auth]
jwt_secret = "change-me-to-a-long-random-string"
token_ttl_hours = 24
seed_username = "admin"
seed_password = "Admin123!"
enabled = true
```

#### 3. 数据库层 — 新建 `gateway/src/db.rs`

```rust
pub type DbPool = sqlx::MySqlPool;

pub async fn connect(cfg: &DatabaseConfig) -> anyhow::Result<DbPool> {
    ensure_database_exists(&cfg.url).await?;  // 库不存在则自动建（参考 Eventide）
    MySqlPoolOptions::new()
        .max_connections(cfg.max_connections)
        .min_connections(cfg.min_connections)
        .connect(&cfg.url).await
}

pub async fn seed_admin_if_empty(pool: &DbPool, cfg: &AuthConfig) -> anyhow::Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(pool).await?;
    if count > 0 { return Ok(()); }
    // argon2 哈希密码 + INSERT admin 行
}
```

**迁移方案**：`sqlx::migrate!("./migrations")` 宏，编译期嵌入 SQL，启动期一行 `migrate!.run(&pool).await?`。

**SQL 访问模式**：**用运行期 `query_as::<_, User>(sql).bind(...)`，不用 `query!` 宏**。理由：项目早期 SQL 少且变动频繁，编译期宏需要本地 MySQL + `cargo sqlx prepare` 维护 `.sqlx/` 离线缓存，开发门槛过高；后期稳定后可切编译期。用 struct + `FromRow` 派生即可。

#### 4. 迁移文件 — 新建 `gateway/migrations/20260811000001_init_users.sql`

```sql
CREATE TABLE users (
    id              CHAR(36)     NOT NULL,
    username        VARCHAR(128) NOT NULL,
    display_name    VARCHAR(255) NOT NULL DEFAULT '',
    email           VARCHAR(255) NOT NULL DEFAULT '',
    password_hash   TEXT         NOT NULL,
    role            VARCHAR(32)  NOT NULL DEFAULT 'viewer',
    enabled         TINYINT      NOT NULL DEFAULT 1,
    created_at      VARCHAR(64)  NOT NULL,
    updated_at      VARCHAR(64)  NOT NULL,
    PRIMARY KEY (id),
    UNIQUE KEY uk_users_username (username),
    KEY idx_users_role (role)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
```

**设计取舍**：
- `id` 用 CHAR(36) UUID——与 AxleOps/Eventide 一致，便于跨系统对齐。
- `role` 用 VARCHAR 而非 ENUM——应用层用 Rust enum 强约束，DB 层灵活。
- `created_at/updated_at` 用 VARCHAR(64) RFC3339——与 Eventide 一致，避免 MySQL DATETIME 时区坑。
- **不建 sessions/blacklist 表**——JWT 无状态，logout 仅前端清 token（与 Eventide 同路）。阶段二若需服务端失效再加。

#### 5. 鉴权模块 — 新建 `gateway/src/auth.rs`（单文件，约 250 行）

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role { Admin, Operator, Viewer }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // username
    pub uid: String,   // user id
    pub role: Role,
    pub iat: usize,
    pub exp: usize,
}

pub fn hash_password(password: &str) -> anyhow::Result<String>;
pub fn verify_password(password: &str, hash: &str) -> bool;
pub fn issue_token(uid: &str, username: &str, role: Role, secret: &str, ttl_hours: u64) -> anyhow::Result<String>;

pub struct AuthUser(pub Claims);
// impl FromRequestParts<Arc<AppState>>：从 Authorization: Bearer 取 token，decode，返回 AuthUser
// 失败返回 AppError::unauthorized

fn require_admin(user: &AuthUser) -> Result<(), AppError>;  // 简单 guard 函数，不用泛型 extractor
```

**模式选择**：用 `FromRequestParts` extractor（AxleOps 路线），**不挂 axum::middleware**。受保护 handler 签名里写 `auth: AuthUser` 即触发校验，不写则不校验。简单直接。

#### 6. 错误处理 — 新建 `gateway/src/error.rs`

```rust
pub struct AppError { pub status: StatusCode, pub code: i32, pub message: String }
impl AppError {
    pub fn unauthorized(msg: &str) -> Self;
    pub fn forbidden(msg: &str) -> Self;
    pub fn bad(msg: &str) -> Self;
    pub fn internal(e: impl Display) -> Self;
}
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "code": self.code, "message": self.message }))).into_response()
    }
}
```
保持现有 `{ code: 0, data }` 成功格式不变，错误用 `{ code: 4xx/5xx, message }`。

#### 7. Auth 路由 — 新建 `gateway/src/auth_routes.rs`

```rust
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/auth/login", post(login))     // 公开
        .route("/api/auth/logout", post(logout))   // 公开（无状态，仅返回 ok）
        .route("/api/auth/me", get(me))            // 受保护（AuthUser）
        .route("/api/users", get(list_users))      // 受保护（admin only）
}

#[derive(Deserialize)] pub struct LoginRequest { pub username: String, pub password: String }
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
pub struct UserInfo { pub id, username, displayName, email, role, enabled, createdAt, updatedAt }
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
pub struct LoginResponse { pub token, pub expiresAt, pub user: UserInfo }
```

> **字段命名**：后端 `#[serde(rename_all = "camelCase")]`，与前端现有 `types.ts`（`baseUrl`/`createdAt`）风格一致，减少前端心智负担。

#### 8. AppState 扩展与路由挂载 — `gateway/src/routes.rs`

```rust
pub struct AppState {
    pub config: Arc<GatewayConfig>,
    pub client: reqwest::Client,
    pub db: sqlx::MySqlPool,
    pub jwt_secret: String,
    pub jwt_ttl_hours: u64,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(health_check))
        .merge(crate::auth_routes::routes())
        .route("/api/systems", get(list_systems))
        .route("/api/systems/{id}", get(get_system))
        .route("/api/proxy/{id}/{*path}", proxy_any_method())
        .route("/api/aggregate/overview", get(aggregate_overview))
        .route("/api/aggregate/alerts", get(aggregate_alerts))
        .layer(CorsLayer::permissive())   // 阶段二收紧到 cors_origins
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
```

**已知妥协（必须文档标注）**：阶段一 `/api/systems`、`/api/proxy/*`、`/api/aggregate/*` **暂不加 AuthUser**。理由：加了会让前端 12 个 mock 页面全 401，且这些页面尚未对接真实后端。阶段二第一步就给它们加鉴权。阶段一仅 `/api/auth/me` 和 `/api/users` 强制鉴权。

#### 9. 启动流程 — `gateway/src/main.rs`

```rust
mod auth; mod auth_routes; mod config; mod db; mod error; mod routes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // tracing init（已有）
    let config = GatewayConfig::load(&cli.config)?;  // 内部增加环境变量覆盖
    let db = db::connect(&config.database).await?;
    sqlx::migrate!("./migrations").run(&db).await?;
    db::seed_admin_if_empty(&db, &config.auth).await?;
    let state = Arc::new(AppState { config: Arc::new(config.clone()), client: ..., db: db.clone(), jwt_secret: config.auth.jwt_secret.clone(), jwt_ttl_hours: config.auth.token_ttl_hours });
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(&config.server.bind).await?;
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;
    Ok(())
}
```
新增 `shutdown_signal()`（ctrl_c + unix SIGTERM）。

---

### 二、前端侧（Vue 3 + TS）

#### 1. vite 代理 — `portal/vite.config.ts`

```ts
export default defineConfig({
  plugins: [vue()],
  server: {
    port: 5173,
    proxy: {
      '/api': { target: 'http://127.0.0.1:8000', changeOrigin: true },
    },
  },
})
```

#### 2. 类型与 API 模块

`portal/src/api/types.ts` 追加：
```ts
export type UserRole = 'admin' | 'operator' | 'viewer'
export interface UserInfo { id, username, displayName, email, role, enabled, createdAt, updatedAt }
export interface LoginRequest { username, password }
export interface LoginResponse { token, expiresAt, user: UserInfo }
```

新建 `portal/src/api/auth.ts`：
```ts
import request from './request'
export const login = (data: LoginRequest) => request.post('/auth/login', data)
export const getMe = () => request.get('/auth/me')
export const logout = () => request.post('/auth/logout')
export const listUsers = () => request.get('/users')
```

#### 3. 请求拦截器改造 — `portal/src/api/request.ts`

响应拦截器拆 `body.data`（**零破坏**，已确认无页面调用 request）+ 401 跳登录：
```ts
request.interceptors.response.use(
  (response) => {
    const body = response.data
    if (body?.code === 0) return body.data        // 成功拆包
    if (body?.code) { ElMessage.error(body.message); return Promise.reject(new Error(body.message)) }
    return body
  },
  (error) => {
    if (error.response?.status === 401) {
      localStorage.removeItem('meridianops_token')
      localStorage.removeItem('meridianops_user')
      if (location.pathname !== '/login') location.href = '/login'
    } else if (error.response?.status === 403) {
      ElMessage.error('无权限访问')
    } else {
      ElMessage.error(error.response?.data?.message || error.message || '请求失败')
    }
    return Promise.reject(error)
  }
)
```

#### 4. 用户 Store 重写 — `portal/src/stores/user.ts`

- `token` + `user: UserInfo | null`（不再只存 username）
- `isAuthenticated` computed：token 存在 + JWT `exp` 未过期（手写 `atob` 解析 payload，不引 jwt-decode）
- `async login(req)` 调真实 API
- `async fetchMe()` 调 `/auth/me` 刷新用户
- `async logout()` 调 API + 清 localStorage

#### 5. 登录页改造 — `portal/src/views/login/LoginPage.vue`

- form 默认值清空（移除 `admin/admin`）
- `handleLogin` 改 `await userStore.login({ username, password })`，try/catch + finally 控制 loading
- 错误提示依赖 request 拦截器，handler 内不重复弹窗

#### 6. 路由守卫 — `portal/src/router/index.ts`

```ts
router.beforeEach((to, _from, next) => {
  const userStore = useUserStore()
  if (to.path === '/login') {
    userStore.isAuthenticated ? next('/overview') : next(); return
  }
  if (to.meta.requiresAuth !== false && !userStore.isAuthenticated) {
    next({ path: '/login', query: { redirect: to.fullPath } }); return
  }
  next()
})
```
> 需确认 `main.ts` 是 `app.use(createPinia())` 在 `app.use(router)` 之前（否则守卫内 `useUserStore()` 报错）。

#### 7. 布局改造 — `portal/src/layout/MainLayout.vue`

- `username` 改为 `userStore.user?.displayName || userStore.user?.username || 'Admin'`
- `onMounted` 调 `userStore.fetchMe()`（token 无效会被 401 拦截器跳登录）
- 登出调 `await userStore.logout()` 再跳 `/login`

---

### 三、文件清单

**网关新建**：
- `gateway/src/auth.rs`
- `gateway/src/auth_routes.rs`
- `gateway/src/db.rs`
- `gateway/src/error.rs`
- `gateway/migrations/20260811000001_init_users.sql`

**网关修改**：
- `gateway/Cargo.toml` · `gateway/src/config.rs` · `gateway/src/main.rs` · `gateway/src/routes.rs` · `gateway/gateway-config.toml`

**前端新建**：
- `portal/src/api/auth.ts`

**前端修改**：
- `portal/vite.config.ts` · `portal/src/api/request.ts` · `portal/src/api/types.ts` · `portal/src/stores/user.ts` · `portal/src/views/login/LoginPage.vue` · `portal/src/router/index.ts` · `portal/src/layout/MainLayout.vue`

---

## 验证方案

### 网关侧（curl）
1. 准备 MySQL（`docker run -d --name mops-mysql -e MYSQL_ROOT_PASSWORD=root -p 3306:3306 mysql:8` + 建库建用户）
2. `cargo build` 通过
3. `cargo run`，期望日志：`mysql ready` → `migrations applied` → `seed admin user created username=admin` → `listening on 0.0.0.1:8000`
4. `POST /api/auth/login` 用 `admin/Admin123!` → 返回 `{ code: 0, data: { token, expiresAt, user } }`
5. `GET /api/auth/me` 带 Bearer token → 返回用户信息
6. `GET /api/auth/me` 无 token → 401 `missing token`
7. 错误密码 → 401 `用户名或密码错误`
8. `GET /api/users` 带 admin token → 返回数组
9. `GET /api/users` 无 token → 401

### 前端侧（浏览器）
1. `npm run dev` 启动 5173
2. 访问 `/` 自动跳 `/login`
3. 输入 `admin/Admin123!` 登录 → 跳 `/overview`，ElMessage 成功
4. F5 刷新，保持在 `/overview`（token 持久化 + isAuthenticated 通过）
5. 顶部用户下拉显示"管理员"
6. 登出 → 跳 `/login`，localStorage 清空
7. 手动破坏 localStorage token 签名后刷新 → fetchMe 401 → 跳登录
8. 手动塞过期 token（exp 过去）→ isAuthenticated false → 跳登录

### 端到端
- DevTools Network：登录请求 `POST http://127.0.0.1:5173/api/auth/login`（vite proxy 转发到 8000）
- 后续请求自动带 `Authorization: Bearer ...`
- 网关日志有 trace 输出（TraceLayer 生效）

---

## 风险点

1. **sqlx 运行期模式无编译期 SQL 校验** → 用 `FromRow` 派生 + 单测覆盖关键查询。
2. **JWT 默认密钥泄露风险** → 启动期检测默认值 + 非 loopback bind 时 warn；文档强调生产用 `MERIDIANOPS_JWT_SECRET` 覆盖。
3. **现有路由暂未鉴权**（/api/systems 等）→ 阶段一仅用于开发联调，不部署生产；阶段二首要任务补鉴权。
4. **argon2 Windows 编译** → 依赖 cc + MSVC，本机 AxleOps 已验证可编译。
5. **sqlx::migrate! 路径** → 相对 `Cargo.toml` 目录，必须确保 `gateway/migrations/` 存在且含至少一个 .sql，否则编译失败。
6. **token 存 localStorage 的 XSS 风险** → 阶段一沿用，阶段二可迁 httpOnly cookie + CSRF。
7. **sqlx 0.8 + axum 0.7 Send 约束** → `MySqlPool` 内部 Arc，直接 clone 进 AppState，不要包 Mutex。

---

## 实施顺序

1. 网关 DB 闭环（Cargo.toml → config.rs → db.rs → migrations → main.rs）→ `cargo run` 看到 seed 日志、MySQL 查到 admin 行
2. 网关鉴权闭环（auth.rs → error.rs → auth_routes.rs → routes.rs 挂载）→ curl login/me/list_users 全通
3. 前端 vite proxy → 浏览器 Network 见转发
4. 前端 store + api（types.ts → auth.ts → request.ts → user.ts）
5. 前端 UI（LoginPage → router → MainLayout）→ 浏览器端到端登录→刷新→登出
6. 回归：现有 mock 页面不受影响（确认 request 拆包无破坏）
