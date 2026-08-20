import request from './request'

/** 告警事件 */
export interface AlertEvent {
  id: string
  fingerprint: string
  source: string
  /** 接入渠道：webhook / manual / job / api_token / system */
  ingressChannel: string
  /** 接入者身份（通道名/用户名/token 名） */
  ingressActor: string | null
  severity: 'P0' | 'P1' | 'P2' | 'P3' | 'info' | string
  status: 'firing' | 'acknowledged' | 'resolved' | 'suppressed' | string
  title: string
  message: string | null
  labels: Record<string, unknown> | null
  ciId: string | null
  ciName: string | null
  fireCount: number
  firstFiredAt: string
  firedAt: string
  acknowledgedBy: string | null
  acknowledgedAt: string | null
  resolvedBy: string | null
  resolvedAt: string | null
  resolutionNote: string | null
  createdAt: string
  updatedAt: string
  /** 资产责任人（CI owner） */
  contactName: string | null
}

export interface AlertEventPage {
  total: number
  page: number
  pageSize: number
  items: AlertEvent[]
}

export interface AlertEventQuery {
  page?: number
  pageSize?: number
  source?: string
  severity?: string
  status?: string
  ciId?: string
  keyword?: string
}

export interface CreateAlertEventRequest {
  source?: string
  severity: string
  title: string
  message?: string
  labels?: Record<string, unknown>
  ci_id?: string
  ci_name_snapshot?: string
  fired_at?: string
}

export interface AlertStats {
  activeTotal: number
  todayNew: number
  bySeverity: Record<string, number>
  byStatus: Record<string, number>
  bySource: Record<string, number>
}

/** 静默规则 */
export interface AlertSilence {
  id: string
  name: string
  reason: string | null
  matchLabels: Record<string, unknown> | null
  startsAt: string
  endsAt: string
  active: boolean
  createdBy: string
  createdAt: string
  updatedAt: string
}

export interface CreateAlertSilenceRequest {
  name: string
  reason?: string
  match_labels?: Record<string, unknown>
  starts_at: string
  ends_at: string
}

export interface UpdateAlertSilenceRequest {
  name: string
  reason?: string
  match_labels?: Record<string, unknown>
  starts_at: string
  ends_at: string
  active?: boolean
}

/** 列出告警事件 */
export function listAlertEvents(params: AlertEventQuery): Promise<AlertEventPage> {
  return request.get('/alerts/events', { params })
}

/** 获取告警详情 */
export function getAlertEvent(id: string): Promise<AlertEvent> {
  return request.get(`/alerts/events/${id}`)
}

/** 新建告警 */
export function createAlertEvent(data: CreateAlertEventRequest): Promise<{ id: string; fingerprint: string; merged: boolean }> {
  return request.post('/alerts/events', data)
}

/** 认领告警 */
export function acknowledgeAlert(id: string): Promise<{ id: string }> {
  return request.put(`/alerts/events/${id}/acknowledge`)
}

/** 手动标记单条告警为静默（值班临时压制，alert:update 权限） */
export function suppressAlert(id: string): Promise<{ id: string }> {
  return request.put(`/alerts/events/${id}/suppress`)
}

/** 解决告警 */
export function resolveAlert(id: string, note?: string): Promise<{ id: string }> {
  return request.put(`/alerts/events/${id}/resolve`, { note: note ?? null })
}

/** 添加解决备注 */
export function updateAlertNote(id: string, note: string): Promise<{ id: string }> {
  return request.put(`/alerts/events/${id}/note`, { note })
}

/** 删除告警 */
export function deleteAlertEvent(id: string): Promise<void> {
  return request.delete(`/alerts/events/${id}`)
}

/** 获取告警统计 */
export function getAlertStats(): Promise<AlertStats> {
  return request.get('/alerts/stats')
}

/** 列出静默规则 */
export function listAlertSilences(): Promise<AlertSilence[]> {
  return request.get('/alerts/silences')
}

/** 新建静默规则 */
export function createAlertSilence(data: CreateAlertSilenceRequest): Promise<{ id: string }> {
  return request.post('/alerts/silences', data)
}

/** 更新静默规则 */
export function updateAlertSilence(id: string, data: UpdateAlertSilenceRequest): Promise<{ id: string }> {
  return request.put(`/alerts/silences/${id}`, data)
}

/** 删除静默规则 */
export function deleteAlertSilence(id: string): Promise<void> {
  return request.delete(`/alerts/silences/${id}`)
}

// ============ 接入来源概览 ============

/** API 令牌详情（接入来源概览中附带） */
export interface IngressTokenInfo {
  name: string
  role: string
  scopes: string[]
  expiresAt: string | null
  revoked: boolean
  expired: boolean
  lastUsedAt: string | null
  createdAt: string
  ownerName: string | null
}

/** 接入来源概览条目 */
export interface IngressOverviewItem {
  ingressChannel: string
  ingressActor: string | null
  totalCount: number
  firingCount: number
  acknowledgedCount: number
  resolvedCount: number
  firstFiredAt: string | null
  lastFiredAt: string | null
  tokenInfo: IngressTokenInfo | null
}

/** 接入来源概览响应 */
export interface IngressOverview {
  items: IngressOverviewItem[]
  channelSummary: Record<string, number>
  totalActors: number
}

/** 获取接入来源概览 */
export function fetchIngressOverview(): Promise<IngressOverview> {
  return request.get('/alerts/ingress-overview')
}

// ============ 告警接入配置（共享密钥） ============

/** 告警接入配置（GET 返回，密钥脱敏） */
export interface AlertIngressConfig {
  ingressEnabled: boolean
  /** 脱敏后的密钥（前 4 + **** + 后 4），仅用于展示 */
  ingressTokenMasked: string
  tokenLength: number
  /** 是否仍是默认/未配置密钥 */
  isDefault: boolean
  /** 当前值来源：config = toml 配置文件，database = 数据库覆盖 */
  source: 'config' | 'database'
  updatedBy: string | null
  updatedAt: string | null
}

/** 更新告警接入配置请求 */
export interface UpdateAlertIngressRequest {
  /** 是否启用 ingress 接收端（不传则保持不变） */
  ingressEnabled?: boolean
  /** 自定义新密钥（明文，至少 8 位）。与 regenerate 互斥 */
  ingressToken?: string
  /** 若为 true，服务端生成 32 字节随机密钥并以明文返回一次 */
  regenerate?: boolean
}

/** 更新告警接入配置响应（regenerate=true 时返回明文密钥，仅此一次） */
export interface UpdateAlertIngressResponse {
  ingressEnabled: boolean
  /** 仅在 regenerate=true 时返回，明文密钥 */
  ingressToken?: string
  regenerated: boolean
  warning?: string
}

/** 查询告警接入配置（system:read） */
export function getAlertIngress(): Promise<AlertIngressConfig> {
  return request.get('/system/alert-ingress')
}

/** 更新告警接入配置（system:update） */
export function updateAlertIngress(data: UpdateAlertIngressRequest): Promise<UpdateAlertIngressResponse> {
  return request.put('/system/alert-ingress', data)
}
