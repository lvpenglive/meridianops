# 安全加固 + 个人中心 实现计划

## Context

MeridianOps 已完成 Phase 1（基础设施地基）和 Phase 2（RBAC + 部门管理 + 审计中心）。当前存在明显安全短板：

1. **登录无失败锁定**：暴力破解可无限重试，[auth_routes.rs:131](file:///e:/meridianops/gateway/src/auth_routes.rs#L131) 密码错误仅返回 401，无计数
2. **密码策略过弱**：[auth_routes.rs:260](file:///e:/meridianops/gateway/src/auth_routes.rs#L260) 仅校验 `len < 6`，无复杂度要求
3. **用户无法自助改密**：只有管理员 `reset_password`，用户自己改不了
4. **"个人中心"菜单空操作**：[MainLayout.vue:76](file:///e:/meridianops/portal/src/layout/MainLayout.vue#L76) `command="profile"` 但 `handleCommand` 未处理
5. **系统设置是 mock**：[SystemPage.vue:52](file:///e:/meridianops/portal/src/views/system/SystemPage.vue#L52) 保存只弹提示，未持久化

目标：补齐这些地基短板，使系统具备生产可用的基本安全策略与用户自助能力。

## 实现方案

### 后端（Rust / Axum）

#### 1. 新增迁移文件

**`migrations/20260811000007_add_login_lockout_fields.sql`** — users 表加锁定字段：
```sql
ALTER TABLE users
  ADD COLUMN failed_login_attempts INT NOT NULL DEFAULT 0 AFTER enabled,
  ADD COLUMN locked_until VARCHAR(64) NULL DEFAULT NULL AFTER failed_login_attempts;
```

**`migrations/20260811000008_create_system_settings.sql`** — 系统配置表 + 密码策略种子：
```sql
CREATE TABLE system_settings (
    setting_key   VARCHAR(64) PRIMARY KEY,
    setting_value TEXT NOT NULL,
    description   VARCHAR(255) NOT NULL DEFAULT '',
    updated_at    VARCHAR(64) NOT NULL,
    updated_by    VARCHAR(128) NOT NULL DEFAULT ''
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

-- 密码策略种子
INSERT INTO system_settings (setting_key, setting_value, description, updated_at, updated_by) VALUES
('password_min_length', '8', '密码最小长度', '<now>', 'system'),
('password_require_uppercase', 'true', '是否需要大写字母', '<now>', 'system'),
('password_require_lowercase', 'true', '是否需要小写字母', '<now>', 'system'),
('password_require_digit', 'true', '是否需要数字', '<now>', 'system'),
('password_require_special', 'false', '是否需要特殊字符', '<now>', 'system'),
('login_max_attempts', '5', '登录失败最大次数', '<now>', 'system'),
('login_lockout_minutes', '15', '锁定时长（分钟）', '<now>', 'system');
```

#### 2. `db.rs` 新增函数

```rust
// 登录失败计数 + 锁定
pub async fn increment_failed_login(pool, username, max_attempts, lockout_minutes) -> Result<Option<String>>  // 返回锁定时间
pub async fn reset_failed_login(pool, user_id) -> Result<()>
pub async fn is_user_locked(pool, user_id) -> Result<Option<String>>  // 返回锁定到期时间

// 系统配置
pub async fn get_all_settings(pool) -> Result<Vec<SystemSetting>>
pub async fn get_setting(pool, key) -> Result<Option<String>>
pub async fn upsert_settings(pool, entries: Vec<(key, value, updated_by)>) -> Result<()>
```

新增 `SystemSetting` struct（FromRow）。

#### 3. `auth.rs` 新增密码强度校验

```rust
pub struct PasswordPolicy {
    pub min_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_digit: bool,
    pub require_special: bool,
}

pub fn validate_password_strength(password: &str, policy: &PasswordPolicy) -> Result<(), String>
```

从 system_settings 读取配置构造 policy，在 `create_user` / `reset_password` / `change_password` 调用。

#### 4. `auth_routes.rs` 改造

- **login**：增加锁定检查（`locked_until > now` 则拒绝，返回剩余时间）；失败时 `increment_failed_login`；成功时 `reset_failed_login`
- **新增 `POST /api/auth/change-password`**：校验旧密码 → 校验新密码强度 → `update_password` → 记审计日志（action=`change_password`）

#### 5. 新增 `system_routes.rs`

- `GET /api/system/settings` — 读取全部配置（需 `system:read`）
- `PUT /api/system/settings` — 批量更新（需 `system:update`）
- `GET /api/system/security-policy` — 公开接口，返回密码策略（登录页/个人中心前端校验用，无需鉴权）

在 `routes.rs:create_router()` 加 `.merge(crate::system_routes::routes())`，`main.rs` 加 `mod system_routes;`。

### 前端（Vue 3 / Element Plus）

#### 6. 新增 API 与类型

- **`api/system.ts`**：`getSettings()` / `updateSettings(entries)` / `getSecurityPolicy()`
- **`api/auth.ts`** 加 `changePassword(data: { oldPassword, newPassword })`
- **`api/types.ts`** 加 `SystemSetting` / `SecurityPolicy` / `ChangePasswordRequest`

#### 7. 个人中心页 `views/profile/ProfilePage.vue`

3 个 Tab，参考 [UsersPage.vue](file:///e:/meridianops/portal/src/views/system/UsersPage.vue) 的对话框样式风格：

- **基本信息**：显示用户名（只读）/姓名/邮箱/角色/部门/创建时间/最后登录；可编辑姓名、邮箱 → `PUT /api/users/:id`（自己改自己）
- **修改密码**：旧密码 + 新密码 + 确认密码；前端用 `getSecurityPolicy()` 做实时强度提示；提交 → `POST /api/auth/change-password`
- **登录历史**：调 `GET /api/audit-logs?actor=<username>&action=login`，参考 [AuditPage.vue](file:///e:/meridianops/portal/src/views/audit/AuditPage.vue) 表格 + 分页

路由 `/profile` 加到 `router/index.ts`，`meta: { title: '个人中心' }`，无 permission（登录即可访问）。

#### 8. `MainLayout.vue` handleCommand 增加 profile 跳转

```ts
async function handleCommand(command: string) {
  if (command === 'profile') {
    router.push('/profile')
  } else if (command === 'logout') { ... }
}
```

#### 9. 改造 `SystemPage.vue`

- onMounted 调 `getSettings()` 加载真实配置
- 分两个 section：**安全策略**（密码策略 + 登录锁定参数）/ **系统参数**（系统名称等）
- 保存调 `updateSettings(entries)`，成功后 ElMessage 提示
- 保留右下角"系统信息"卡片不变

#### 10. `stores/user.ts` 扩展

加 `updateProfile(displayName, email)` 方法，调用后同步更新本地 user 状态。

## 关键复用点

- 密码哈希复用 [auth.rs:72 hash_password](file:///e:/meridianops/gateway/src/auth.rs#L72) / [auth.rs:81 verify_password](file:///e:/meridianops/gateway/src/auth.rs#L81)
- 权限守卫复用 [auth.rs:202 require_permission](file:///e:/meridianops/gateway/src/auth.rs#L202)
- 审计日志复用 [audit::log_async](file:///e:/meridianops/gateway/src/audit.rs)（现有模式）
- 路由注册复用 [routes.rs:22 create_router](file:///e:/meridianops/gateway/src/routes.rs#L22) 的 `.merge()` 模式
- 前端请求复用 [api/request.ts](file:///e:/meridianops/portal/src/api/request.ts) 拦截器（自动拆 body.data）
- 对话框样式复用 UsersPage 的 `dialog-header` / `form-section` 模式

## 验证计划

1. **后端编译**：`cd e:\meridianops\gateway && cargo build`
2. **前端类型检查**：`cd e:\meridianops\portal && npx vue-tsc --noEmit`
3. **E2E 功能验证**（curl + 浏览器）：
   - 连续输错 5 次密码 → 账号锁定，返回"锁定剩余 X 分钟"
   - 锁定期内正确密码也被拒
   - 等锁定过期后正确密码可登录，计数清零
   - 个人中心：改姓名/邮箱、改密码（旧密码错误时拒绝、新密码不满足策略时提示）
   - 登录历史 Tab 显示该用户登录记录
   - 系统设置：修改密码最小长度为 10 → 创建用户时 8 位密码被拒
   - 用户菜单"个人中心"可正常跳转
