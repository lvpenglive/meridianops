# 项目部署日志 / Deployment Log

> 树形告警组（alert_groups 层级 + 短信策略父组选择）联调记录。
> 日志只记录真实执行结果与限制，不掩盖构建/测试退出码。

## 2026-09-09 树形告警组部署联调（第二轮：实际部署 + API 验证）

### 环境
- 后端二进制：`gateway/target/debug/meridianops-gateway.exe`（cargo build 产物）
- 服务地址：`http://127.0.0.1:8000`（通过 `MERIDIANOPS_SERVER_BIND=127.0.0.1:8000` 启动，未修改已提交的 `gateway-config.toml`）
- 数据库：迁移在启动时由 `run_migrations_ignore_checksum` 应用（沿用既有幂等机制）
- 前端 dev：`http://127.0.0.1:5173`（vite，`/api` 代理到 8000）

### 关键动作与真实结果
1. **审阅迁移**：`20260909000047_sms_strategies.sql` / `48_alert_groups.sql` / `49_add_alert_group_parent.sql` 三件套；49 仅给 `alert_groups` 加可空 `parent_id` + 索引，不破坏历史数据。
2. **停止旧进程**：精确识别监听 `127.0.0.1:8000` 的 PID 21476 = `meridianops-gateway.exe`（tasklist 确认），`taskkill /F` 停止，释放 Windows exe 锁。
3. **构建**：`cargo build` 真实退出码 **0**（输出落 ``.tmp/build.log``）。
4. **启动 + 迁移**：后端健康 `GET /api/health` = 200；启动日志确认 `applying migration 20260909000049: add alert group parent` → `applied 1 new migrations`，`parent_id` 列已落地。
5. **修复接口运行错误（sms_strategy_routes JOIN 歧义）**：
   - `list_strategies` 查询 `alert_sms_strategies s LEFT JOIN alert_groups g`，两表都有 `description`/`enabled`/`created_at`/`id`。
   - 原代码 `WHERE description LIKE` / `enabled =` / `ORDER BY created_at DESC, id DESC` 未限定别名 → MySQL 1052 歧义（必现 500）。
   - 已限定为 `s.description` / `s.enabled` / `s.created_at, s.id`。`alert_group_routes.rs` 各查询为单表或已限定别名，无此问题（已 grep 全仓确认仅此一处 JOIN alert_groups）。
6. **API 验证（admin 登录，密码经环境变量传入，未打印 token/密码；仅用唯一前缀临时组，finally 清理）**：
   - 全量树接口可用；临时父子组创建/更新成功。
   - 防循环：把父组上级设为子孙（P→C1）、自指父级均被拒（HTTP 400）。
   - 父组含子孙成员按真实 `user_id` 去重：同一用户同时存在于父/子组，返回仅一次。
   - 空组成员返回 `[]`，不报错。
   - 删除保护：父组有子组 → 拒删（400）；被短信策略引用 → 拒删（400）。
   - 短信策略列表接口修复后返回 200（验证 JOIN 歧义已消除）。
   - 清理：临时组/策略全部删除，`LEFT_TEMP=0`，用户既有数据未动。
   - 说明：首轮脚本清理顺序（先删父后删子）触发了一次误报 FAIL，系脚本清理顺序问题非后端缺陷；改用子组优先清理后 `LEFT_TEMP=0`，所有功能断言均通过（25/27，2 项为脚本清理顺序，现已纠正）。

### 前端
- `vue-tsc -b` 共 **78 处**类型错误，分布在 `CredentialsPage/UsersPage/TicketDetailPage/TopologyPage/WorkflowTemplatesPage/JobsPage/AlertsPage/AssetsPage/ModelsPage/ReportIndexPage/RulesPage/MainLayout` 等**与本次功能无关的历史文件**。
- 本次功能文件 `AlertGroupsPage.vue` / `SmsStrategyPage.vue` / `api/alertGroup.ts` / `api/smsStrategy.ts` **无任何类型错误**（grep 错误列表确认 `NONE_IN_FEATURE_FILES`）。
- 结论：前端生产构建（`npm run build`）因历史类型错误整体失败（退出码 2），本次树形告警组改动未引入新类型错误。
- `npm run dev`（vite，esbuild 不去类型检查）可正常启动 5173 并返回 SPA 外壳，`/api` 代理到 8000，经代理 `GET /api/health` = 200，端到端联通。

### 浏览器可视化验证
- 未执行。沙箱为无头环境，`agent-browser` 未安装（需约 500MB Chromium 下载 + 真实浏览器/显示），不具备浏览器可视化能力。
- 已用「等价手段」替代：直接对 8000 后端做完整 HTTP API 行为验证（即前端调用的同一组端点），并验证 5173 dev 代理可达后端。未假称已做浏览器实测。

### 剩余限制 / 待办
- 前端生产构建被 78 处**历史**类型错误阻断，与本次功能无关，建议单独立项清理（不在本次范围，避免越界改动 15 个无关页面）。
- 浏览器交互/视觉验证未在本次执行（环境无浏览器），如需可安装 agent-browser + Chromium 后补测。
- 本次改动（迁移 + 路由 + 前端页面/API）目前为未提交的工作区改动，尚未 git commit。
