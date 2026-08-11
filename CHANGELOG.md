# Changelog

## v0.2.0 (2026-08-11)

### 🎉 新增

- ✅ 基础设施地基完成：Gateway 框架（Rust + Axum 0.7 + sqlx 0.8 + MySQL）
- ✅ 统一鉴权：JWT（24h token）+ Argon2 密码哈希 + 三级角色（admin/operator/viewer）
- ✅ RBAC 完整实现：roles/permissions/role_permissions 三表 + 18 个权限点种子
- ✅ 部门管理：departments 树形 + 有子部门/用户不可删保护
- ✅ 审计中心：audit_logs 表 + 中间件自动记录 + 分页查询
- ✅ 安全加固：登录失败 5 次锁定 15 分钟 + 密码强度策略 + 系统设置持久化
- ✅ 个人中心：资料摘要 + 自助改密 + 登录历史
- ✅ 前端框架：Vue 3 + TypeScript + Element Plus + Pinia + Vite
- ✅ 用户/角色/部门/审计/系统设置 5 个完整页面
- ✅ v-permission 指令 + 菜单权限过滤

### 📝 文档

- 完整 README 项目蓝图（架构图 + 功能矩阵 + 7 Phase 路线图）
- 对齐全行真实生态：天旦/优云/华为/平行线/Eventide/AxleOps/2oauth/帕拉迪

---

## v0.1.0 (2026-08-11)

### 🎉 新增

- 项目初始化：Git 仓库 + 目录结构
- Gateway 基础框架：Rust + Axum 0.7 + sqlx 0.8
- Portal 基础框架：Vue 3 + TypeScript + Element Plus
- 数据库连接池 + 自动建库 + 迁移框架
- JWT 基础实现 + 登录/登出/me API
- 前端登录页 + 路由守卫