import request from './request'

/** 告警事件 */
export interface AlertEvent {
  id: string
  fingerprint: string
  source: string
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
}

export interface AlertEventPage {
  total: number
  page: number
  pageSize: number
  items: AlertEvent[]
}

export interface AlertEventQuery {
  page?: number
  page_size?: number
  source?: string
  severity?: string
  status?: string
  ci_id?: string
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
