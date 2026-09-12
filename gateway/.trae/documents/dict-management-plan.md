# 字典管理功能实现方案

## Context

当前知识库分类是前端硬编码的 5 个选项（[KnowledgePage.vue:163-169](file:///d:/rustworkspace/meridianops/portal/src/views/knowledge/KnowledgePage.vue#L163-L169)），新增分类需改代码。用户希望在后台管理中加一个通用的"字典管理"功能，支持动态配置分类等枚举值，无需改代码即可增减选项。

## 数据模型

两张表，规范的一对多设计：

```sql
-- 字典类型（如 knowledge_category, alert_severity）
CREATE TABLE sys_dict_types (
    code         VARCHAR(64)   NOT NULL PRIMARY KEY,        -- 类型编码（如 knowledge_category）
    name         VARCHAR(128)  NOT NULL,                    -- 显示名称（如 知识库分类）
    description  VARCHAR(256)  NULL,
    enabled      TINYINT(1)    NOT NULL DEFAULT 1,
    sort_order   INT           NOT NULL DEFAULT 0,
    created_at   DATETIME(3)   NOT NULL,
    updated_at   DATETIME(3)   NOT NULL
);

-- 字典项（如 database→数据库, linux→Linux/系统）
CREATE TABLE sys_dict_items (
    id           CHAR(36)      NOT NULL PRIMARY KEY,
    type_code    VARCHAR(64)   NOT NULL,                    -- FK → sys_dict_types.code
    item_value   VARCHAR(128)  NOT NULL,                    -- 存储值（如 database）
    item_label   VARCHAR(128)  NOT NULL,                    -- 显示文本（如 数据库）
    enabled      TINYINT(1)    NOT NULL DEFAULT 1,
    sort_order   INT           NOT NULL DEFAULT 0,
    created_at   DATETIME(3)   NOT NULL,
    updated_at   DATETIME(3)   NOT NULL,
    UNIQUE KEY uk_type_value (type_code, item_value),
    INDEX idx_type_sort (type_code, sort_order)
);
```

## API 设计

| 方法 | 路径 | 权限 | 说明 |
|---|---|---|---|
| GET | `/api/dict/types` | `dict:read` | 列出所有字典类型 |
| POST | `/api/dict/types` | `dict:create` | 新建字典类型 |
| PUT | `/api/dict/types/:code` | `dict:update` | 编辑字典类型 |
| DELETE | `/api/dict/types/:code` | `dict:delete` | 删除字典类型（连带删项） |
| GET | `/api/dict/types/:code/items` | 仅登录 | 列出某类型的有效项（供下拉框用） |
| POST | `/api/dict/types/:code/items` | `dict:create` | 新建字典项 |
| PUT | `/api/dict/items/:id` | `dict:update` | 编辑字典项 |
| DELETE | `/api/dict/items/:id` | `dict:delete` | 删除字典项 |

`GET items` 仅需登录（不需 `dict:read`），因为知识库创建/编辑页面需要加载分类选项，用户已有 `knowledge:create` 权限即可。

## 改动清单

### 后端

#### 1. `gateway/migrations/20260815000021_create_sys_dict.sql`（新建）

- 建表 `sys_dict_types` + `sys_dict_items`
- 注册 4 个权限点：`dict:read` / `dict:create` / `dict:update` / `dict:delete`，分配给 admin 角色
- 种子数据：插入 `knowledge_category` 类型 + 5 个现有分类项（database/linux/network/middleware/general）

#### 2. `gateway/src/dict_routes.rs`（新建）

参照 [knowledge_routes.rs](file:///d:/rustworkspace/meridianops/gateway/src/knowledge_routes.rs) 的模式：
- `routes()` 函数返回 `Router<Arc<AppState>>`
- 8 个 handler 函数，用 `auth::require_permission` 做权限检查
- 写操作用 `audit::log_async` 记审计日志
- 响应格式统一 `{ code: 0, data: ... }`

#### 3. `gateway/src/main.rs`（改）

添加 `mod dict_routes;`（L15 附近，和其他 mod 声明一起）

#### 4. `gateway/src/routes.rs`（改）

在 [routes.rs:34](file:///d:/rustworkspace/meridianops/gateway/src/routes.rs#L34) 的 `.merge(crate::knowledge_routes::routes())` 后加 `.merge(crate::dict_routes::routes())`

### 前端

#### 5. `portal/src/api/dict.ts`（新建）

参照 [knowledge.ts](file:///d:/rustworkspace/meridianops/portal/src/api/knowledge.ts) 的模式，封装 API 函数 + TypeScript 接口

#### 6. `portal/src/views/system/DictPage.vue`（新建）

参照 [DepartmentsPage.vue](file:///d:/rustworkspace/meridianops/portal/src/views/system/DepartmentsPage.vue) 的模式：
- 左侧：字典类型列表（el-table）
- 右侧：选中类型的字典项列表（el-table + 新增/编辑/删除对话框）
- 按钮用 `v-permission="'dict:create'"` 等控制

#### 7. `portal/src/router/index.ts`（改）

在 [L189](file:///d:/rustworkspace/meridianops/portal/src/router/index.ts#L189) 后加路由：
```js
{
  path: 'system/dict',
  name: 'SystemDict',
  component: () => import('../views/system/DictPage.vue'),
  meta: { title: '字典管理', icon: 'Collection', permission: 'dict:read' }
}
```

#### 8. `portal/src/layout/MainLayout.vue`（改）

在 [L162](file:///d:/rustworkspace/meridianops/portal/src/layout/MainLayout.vue#L162) 的"后台管理"分组里加菜单项：
```js
{ path: '/system/dict', title: '字典管理', icon: 'Collection', permission: 'dict:read' },
```

#### 9. `portal/src/views/knowledge/KnowledgePage.vue`（改）

- 新增 `loadDictCategories()` 函数，调用 `GET /api/dict/types/knowledge_category/items` 获取分类列表
- 将 [L163-169](file:///d:/rustworkspace/meridianops/portal/src/views/knowledge/KnowledgePage.vue#L163-L169) 的硬编码 `<el-option>` 改为 `v-for` 遍历字典数据
- 将 [L281-287](file:///d:/rustworkspace/meridianops/portal/src/views/knowledge/KnowledgePage.vue#L281-L287) 的 `categoryLabels` 改为从字典数据动态构建

## 验证方案

1. `cargo build` 编译通过
2. 启动 gateway，观察迁移 `20260815000021` 应用成功
3. 前端访问 `/system/dict`，验证字典管理页面正常展示
4. 在字典管理页面新增一个分类（如 `security` → `安全`），验证 CRUD 正常
5. 访问知识库页面，点击"新建知识"，验证分类下拉框包含新增的 `安全` 选项
6. 知识库侧边栏分类列表仍正常显示（`list_categories` 走 GROUP BY 不受影响）
7. 删除刚新增的分类，验证知识库下拉框不再显示该项
