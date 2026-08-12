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

export interface DashboardData {
  stats: DashboardStats
  recentActivities: AuditLog[]
  myActivities: AuditLog[]
}
