export interface SystemInfo {
  id: string
  name: string
  type: string
  baseUrl: string
  status: 'online' | 'offline'
  version?: string
}

export interface AgentInfo {
  id: string
  hostname: string
  ip: string
  status: 'online' | 'offline'
  services: number
  cpu: number
  memory: number
  uptime: string
}

export interface ServiceInfo {
  id: string
  name: string
  type: string
  agentId: string
  status: 'running' | 'stopped' | 'error'
  cpu: number
  memory: number
  version: string
  startedAt: string
}

export interface AlertInfo {
  id: string
  severity: 'critical' | 'warning' | 'info' | 'resolved'
  title: string
  source: string
  agent: string
  service: string
  createdAt: string
  status: 'firing' | 'acknowledged' | 'resolved'
}

export interface OverviewData {
  agents: { total: number; online: number; offline: number }
  services: { total: number; running: number; stopped: number; error: number }
  alerts: { firing: number; warning: number; resolved: number }
  hosts: { total: number; healthy: number; warning: number; critical: number }
  recentAlerts: AlertInfo[]
}

// ---- 用户与鉴权 ----
export type UserRole = 'admin' | 'operator' | 'viewer'

export interface UserInfo {
  id: string
  username: string
  displayName: string
  email: string
  role: UserRole
  roleId?: string
  departmentId?: string
  enabled: boolean
  lastLoginAt?: string | null
  passwordChangedAt?: string | null
  createdAt: string
  updatedAt: string
}

export interface LoginRequest {
  username: string
  password: string
}

export interface LoginResponse {
  token: string
  expiresAt: string
  user: UserInfo
  /** 密码是否已过期（true 时 token 仅能访问改密端点） */
  passwordExpired?: boolean
  /** 会话超时分钟数（0=不超时），前端据此做 idle 计时 */
  sessionTimeoutMinutes: number
  /** 当前产品授权摘要（前端用于页脚标识 + 到期预警） */
  license: LicenseStatus
}

/** 产品授权摘要（登录响应/全局 license/status 接口通用） */
export interface LicenseStatus {
  edition: 'Community' | 'Enterprise' | 'Ultimate' | string
  customer: string
  /** 到期时间 RFC3339，空字符串=永不到期 */
  expiresAt: string
  /** 激活时间 RFC3339，空字符串=未激活 */
  activatedAt: string
  /** 剩余天数。永不到期 = 9223372036854775807 (i64::MAX)，已过期 = 负数 */
  daysRemaining: number
  isExpired: boolean
  warnLevel: 'none' | 'soon' | 'urgent' | 'expired'
}

// ---- 用户管理 ----

export interface CreateUserRequest {
  username: string
  password: string
  displayName?: string
  email?: string
  role?: UserRole
  roleId?: string
  departmentId?: string
  enabled?: boolean
}

export interface UpdateUserRequest {
  displayName?: string
  email?: string
  role?: UserRole
  roleId?: string
  departmentId?: string
  enabled?: boolean
}

// ---- 角色管理（RBAC）----

export interface Role {
  id: string
  name: string
  displayName: string
  description: string
  enabled: boolean
  builtIn: boolean
  createdAt: string
  updatedAt: string
}

export interface Permission {
  id: string
  code: string
  name: string
  module: string
  description: string
  createdAt: string
}

export interface CreateRoleRequest {
  name: string
  displayName?: string
  description?: string
  enabled?: boolean
}

export interface UpdateRoleRequest {
  displayName?: string
  description?: string
  enabled?: boolean
}

// ---- 部门管理（树形）----

export interface Department {
  id: string
  name: string
  parentId?: string | null
  sortOrder: number
  enabled: boolean
  createdAt: string
  updatedAt: string
}

/** 前端构建的部门树节点（在扁平 Department 基础上挂 children）。 */
export interface DepartmentNode extends Department {
  children: DepartmentNode[]
}

export interface CreateDepartmentRequest {
  name: string
  parentId?: string
  sortOrder?: number
  enabled?: boolean
}

export interface UpdateDepartmentRequest {
  name?: string
  parentId?: string
  sortOrder?: number
  enabled?: boolean
}

// ---- 审计日志 ----

export interface AuditLog {
  id: number
  actorUsername: string
  action: string
  targetType: string
  targetId: string
  detail?: string
  ip: string
  status: string
  createdAt: string
}

export interface AuditQueryParams {
  actor?: string
  action?: string
  targetType?: string
  targetId?: string
  status?: string
  startFrom?: string
  page?: number
  pageSize?: number
}

export interface AuditPageResponse {
  total: number
  page: number
  pageSize: number
  items: AuditLog[]
}

// ---- 个人中心：修改密码 ----

export interface ChangePasswordRequest {
  oldPassword: string
  newPassword: string
}

// ---- 系统设置 ----

/** 系统配置项（键值对）。与后端 system_settings 表对应。 */
export interface SystemSetting {
  settingKey: string
  settingValue: string
  description: string
  updatedAt: string
  updatedBy: string
}

/** 密码策略。后端 GET /api/system/password-policy 返回。 */
export interface PasswordPolicy {
  minLength: number
  requireUppercase: boolean
  requireLowercase: boolean
  requireDigit: boolean
  requireSpecial: boolean
  description: string
  /** 密码过期天数（0=不过期） */
  expiryDays: number
}

/** 批量更新系统配置的请求体。key → value 映射。 */
export type UpdateSettingsRequest = Record<string, string>

// ---- 个人工作台 Dashboard ----

export interface DashboardStats {
  totalUsers: number
  enabledUsers: number
  totalRoles: number
  totalDepartments: number
  todayOps: number
  todayLogins: number
}

export interface OpsStats {
  totalAssets: number
  totalModels: number
  totalJobDefs: number
  enabledJobDefs: number
  todayJobRuns: number
  todayJobSuccess: number
  totalSyncSources: number
  enabledSyncSources: number
}

export interface ModelStatItem {
  code: string
  name: string
  count: number
  icon: string
}

export interface JobRunSummaryItem {
  id: number
  jobName: string
  triggerMode: string
  overallStatus: string
  targetCount: number
  successCount: number
  failedCount: number
  startedBy: string
  startedAt: string
  finishedAt: string | null
}

export interface DashboardData {
  stats: DashboardStats
  opsStats: OpsStats
  modelStats: ModelStatItem[]
  recentJobRuns: JobRunSummaryItem[]
  recentActivities: AuditLog[]
  myActivities: AuditLog[]
}

// ============ 报表中心 ============

export interface LoginTrendItem {
  date: string
  success: number
  failed: number
}

export interface FailedTopItem {
  username: string
  failedCount: number
  lastFailedAt: string
}

export interface SensitiveTrendItem {
  date: string
  count: number
}

export interface SensitiveTopItem {
  username: string
  count: number
  lastActionAt: string
}

export interface SensitiveListResponse {
  total: number
  page: number
  pageSize: number
  items: AuditLog[]
}

export interface ComplianceSummary {
  totalUsers: number
  weakPasswordCount: number
  expiredPasswordCount: number
  inactive90dCount: number
  passwordExpiryDays: number
}

export interface RoleAssignmentItem {
  roleName: string
  userCount: number
}

// ============ CMDB 配置管理 ============

/** CI 模型（资产类型定义） */
export interface CiModel {
  id: string
  code: string
  name: string
  icon: string
  description: string
  enabled: boolean
  sortOrder: number
  createdAt: string
  updatedAt: string
  /** 属性数（list 接口附带，避免前端逐个请求） */
  attrCount?: number
}

/** 模型属性定义 */
export interface CiModelAttr {
  id: string
  modelId: string
  code: string
  name: string
  valueType: 'string' | 'number' | 'boolean' | 'enum' | 'date' | 'json'
  defaultValue: string
  options: string[] | null
  isRequired: boolean
  isUnique: boolean
  isSearchable: boolean
  sortOrder: number
  createdAt: string
}

/** 模型详情（含属性定义） */
export interface CiModelDetail {
  model: CiModel
  attributes: CiModelAttr[]
}

/** CI 实例（资产记录） */
export interface CiInstance {
  id: string
  modelId: string
  name: string
  status: string
  departmentId?: string | null
  ownerId?: string | null
  attributes: Record<string, unknown>
  tags: string
  source?: string | null
  externalId?: string | null
  lastSyncedAt?: string | null
  createdAt: string
  updatedAt: string
}

/** CI 关系 */
export interface CiRelation {
  id: string
  sourceId: string
  sourceName?: string
  targetId: string
  targetName?: string
  /** 关系类型英文 code（如 depends_on / contains / runs_on） */
  relationType: string
  /** 关系类型中文名称（接口直接 JOIN 返回，无需前端再映射） */
  relationTypeName?: string
  createdAt: string
}

/** 实例分页查询参数 */
export interface CiInstanceQuery {
  modelId?: string
  status?: string
  keyword?: string
  departmentId?: string
  page?: number
  pageSize?: number
}

/** 实例分页响应 */
export interface CiInstancePage {
  total: number
  page: number
  pageSize: number
  items: CiInstance[]
}

/** 创建实例请求 */
export interface CreateCiInstanceRequest {
  modelId: string
  name: string
  status?: string
  departmentId?: string
  ownerId?: string
  attributes?: Record<string, unknown>
  tags?: string
}

/** 更新实例请求 */
export interface UpdateCiInstanceRequest {
  name: string
  status?: string
  departmentId?: string
  ownerId?: string
  attributes?: Record<string, unknown>
  tags?: string
}

/** 创建关系请求 */
export interface CreateCiRelationRequest {
  sourceId: string
  targetId: string
  relationType: string
}

/** 批量导入单条实例项 */
export interface BatchInstanceItem {
  name: string
  status?: string
  tags?: string
  attributes?: Record<string, unknown>
}

/** 批量导入请求 */
export interface BatchCreateInstancesRequest {
  modelId: string
  items: BatchInstanceItem[]
}

/** 批量导入单行错误 */
export interface BatchImportError {
  row: number
  name: string
  message: string
}

/** 批量导入结果 */
export interface BatchImportResult {
  total: number
  success: number
  failed: number
  status: 'success' | 'failed' | 'partial'
  errors: BatchImportError[]
}

/** CMDB 统计 */
export interface CmdbStats {
  total: number
  modelCount: number
  byModel: Array<{
    modelId: string
    modelCode: string
    modelName: string
    icon: string
    count: number
  }>
}

// ---- CMDB 同步：外部系统（蓝鲸等）数据接入 ----

/** 同步数据源 */
export interface SyncSource {
  id: string
  code: string
  name: string
  sourceType: string
  apiUrl: string
  apiToken: string
  webhookSecret: string
  enabled: boolean
  lastSyncAt?: string | null
  lastSyncCount: number
  lastSyncStatus: string
  pullConfig?: Record<string, unknown> | string | null
  pullCron: string
  pullEnabled: boolean
  createdAt: string
  updatedAt: string
}

/** 同步日志条目 */
export interface SyncLog {
  id: number
  sourceCode: string
  batchId: string
  action: string
  modelCode: string
  externalId: string
  instanceId?: string | null
  instanceName: string
  status: string
  message: string
  payload?: string | null
  createdAt: string
}

/** 批量同步请求体 */
export interface SyncRequest {
  source: string
  modelCode: string
  items: Record<string, unknown>[]
}

/** 批量同步结果 */
export interface SyncResult {
  batchId: string
  total: number
  success: number
  failed: number
  status: 'success' | 'partial' | 'failed'
}

/** 拉取同步请求 */
export interface PullRequest {
  source: string
  modelCode?: string
}

/** 拉取同步结果 */
export interface PullResult extends SyncResult {
  mode: 'pull'
}

/** 数据源拉取配置更新请求 */
export interface UpdateSyncSourceRequest {
  apiUrl: string
  apiToken: string
  pullConfig: string
  pullCron: string
  pullEnabled: boolean
}

/** 新增同步数据源请求 */
export interface CreateSyncSourceRequest {
  code: string
  name: string
  sourceType?: string
  apiUrl?: string
  apiToken?: string
  webhookSecret?: string
  pullConfig?: string
  pullCron?: string
  pullEnabled?: boolean
}

/** 同步日志查询参数 */
export interface SyncLogQuery {
  sourceCode?: string
  batchId?: string
  status?: string
  instanceId?: string
  page?: number
  pageSize?: number
}

/** 同步日志分页响应 */
export interface SyncLogPage {
  total: number
  page: number
  pageSize: number
  items: SyncLog[]
}

// ---- API 令牌管理（外部系统对接）----

/** API 令牌列表项（token 已脱敏） */
export interface ApiToken {
  id: string
  name: string
  /** 脱敏后显示值：如 mk-****8y2Q */
  token: string
  ownerUserId: string
  /** 权限码数组，如 ["asset:create", "asset:read"] */
  scopes: string[]
  /** admin / operator / viewer */
  role: string
  /** 过期时间 RFC3339，null=永不过期 */
  expiresAt: string | null
  /** 是否已吊销 */
  revoked: boolean
  /** 吊销时间 RFC3339 */
  revokedAt: string | null
  /** 最近使用时间 RFC3339 */
  lastUsedAt: string | null
  createdAt: string
  updatedAt: string
}

/** 新建令牌请求 */
export interface CreateApiTokenRequest {
  name: string
  scopes: string[]
  /** never / hours / days / custom */
  ttlType?: 'never' | 'hours' | 'days' | 'custom'
  ttlValue?: number
  /** RFC3339，仅 ttlType=custom 时必填 */
  expiresAt?: string
  /** admin/operator/viewer，默认 operator */
  role?: 'admin' | 'operator' | 'viewer'
}

/** 新建令牌响应 */
export interface CreateApiTokenResponse {
  id: string
  /** 明文 token，仅创建时返回一次！ */
  token: string
  expiresAt: string | null
}

/** 权限点分组（新建对话框使用） */
export interface PermissionGroup {
  group: string
  items: string[]
}

/** 当前用户可授予的权限范围 */
export interface MyPermissionsResponse {
  allPerms: string[]
  groups: PermissionGroup[]
  role: 'admin' | 'operator' | 'viewer' | string
}

/** 更新有效期请求 */
export interface UpdateExpiryRequest {
  ttlType: 'never' | 'hours' | 'days' | 'custom'
  ttlValue?: number
  expiresAt?: string
}

// ============ CMDB 模型管理（动态建模）============

/** 创建 CI 模型请求 */
export interface CreateCiModelRequest {
  code: string
  name: string
  icon?: string
  description?: string
  enabled?: boolean
  sortOrder?: number
}

/** 更新 CI 模型请求（code 不可改） */
export interface UpdateCiModelRequest {
  name: string
  icon?: string
  description?: string
  enabled?: boolean
  sortOrder?: number
}

/** 创建模型属性请求 */
export interface CreateCiModelAttrRequest {
  code: string
  name: string
  valueType?: string
  defaultValue?: string
  /** 枚举选项数组（valueType=enum 时） */
  options?: string[] | string
  isRequired?: boolean
  isUnique?: boolean
  isSearchable?: boolean
  sortOrder?: number
}

/** 更新模型属性请求（code 不可改） */
export interface UpdateCiModelAttrRequest {
  name: string
  valueType?: string
  defaultValue?: string
  options?: string[] | string
  isRequired?: boolean
  isUnique?: boolean
  isSearchable?: boolean
  sortOrder?: number
}

// ---- 拓扑视图 ----

/** 拓扑节点 */
export interface TopoNode {
  id: string
  name: string
  status: string
  modelId: string
  modelCode: string
  modelName: string
  icon: string
  source?: string | null
}

/** 拓扑边 */
export interface TopoLink {
  id: string
  sourceId: string
  targetId: string
  relationType: string
}

/** 拓扑查询响应 */
export interface TopologyData {
  nodes: TopoNode[]
  links: TopoLink[]
  nodeCount: number
  linkCount: number
}

/** 拓扑查询参数 */
export interface TopologyQuery {
  modelId?: string
  status?: string
}

// ---- CI 关系类型（关系字典管理）----

/** CI 关系类型 */
export interface CiRelationType {
  id: string
  code: string
  name: string
  description: string
  /** 是否有方向：true=有向（源→目标），false=无向 */
  directional: boolean
  /** 是否启用 */
  enabled: boolean
  sortOrder: number
  createdAt: string
  updatedAt: string
}

/** 创建关系类型请求（code 不可改，长度 2-32，字母/数字/下划线） */
export interface CreateCiRelationTypeRequest {
  code: string
  name: string
  description?: string
  directional?: boolean
  enabled?: boolean
  sortOrder?: number
}

/** 更新关系类型请求（code 不可改） */
export interface UpdateCiRelationTypeRequest {
  name: string
  description?: string
  directional?: boolean
  enabled?: boolean
  sortOrder?: number
}
