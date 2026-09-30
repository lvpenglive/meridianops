# 运维工具单点登录（后续计划）

当前「运维工具」只提供可配置外链，点击后新标签打开目标系统，**不传递 MeridianOps 会话**。

## 目标

用户在 MeridianOps 已登录时，打开 Zabbix / Grafana / 云监控等，尽量免二次登录或自动带上同一身份。

## 前置条件

- 企业已有统一认证（CAS / OIDC / LDAP+门户票据）
- 目标系统支持同一 IdP，或支持 ticket / JWT 跳转

## 建议阶段

1. **配置扩展**：`ops_tools` 增加 `sso_mode`（`none` / `oidc` / `cas` / `ticket_url`）、可选 `sso_client_id` 等，默认仍为 `none`。
2. **跳转编排**：Gateway 增加 `GET /api/ops-tools/:id/launch`，校验权限后按模式生成一次性跳转 URL（或 302），审计记录打开人与目标。
3. **对接优先**：先做与现有 CAS/OIDC 一致的系统；无统一认证的系统保持外链。
4. **安全**：一次性 code、短 TTL、禁止把长期 token 放进浏览器地址栏；新窗口仍用 `noopener`。

## 非目标

- 用 iframe 嵌第三方完整控制台
- 在 MeridianOps 内复刻 Zabbix/Grafana 功能
