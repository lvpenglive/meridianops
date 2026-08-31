import request from './request'

export interface NotificationItem {
  id: string
  type: string
  title: string
  content?: string
  link?: string
  isRead: boolean
  createdAt: string
  readAt?: string | null
}

export interface NotificationPage {
  total: number
  page: number
  pageSize: number
  list: NotificationItem[]
}

export function getNotifications(params?: {
  unreadOnly?: boolean
  page?: number
  pageSize?: number
}): Promise<NotificationPage> {
  return request.get('/notifications', { params })
}

export function getUnreadCount(): Promise<{ count: number }> {
  return request.get('/notifications/unread-count')
}

export function markRead(id: string): Promise<void> {
  return request.post(`/notifications/${id}/read`)
}

export function markAllRead(): Promise<{ updated: number }> {
  return request.post('/notifications/read-all')
}

export function deleteNotification(id: string): Promise<void> {
  return request.delete(`/notifications/${id}`)
}

// ============================================================
// 通知引擎 — 通道管理
// ============================================================

export interface NotificationChannel {
  id: string
  name: string
  channelType: 'email' | 'feishu' | 'webhook'
  config: Record<string, any>
  enabled: boolean
  createdBy: string
  createdAt: string
  updatedAt: string
}

export function listChannels(): Promise<NotificationChannel[]> {
  return request.get('/notification/channels')
}

export function createChannel(data: {
  name: string
  channelType: string
  config: Record<string, any>
  enabled?: boolean
}): Promise<{ id: string }> {
  return request.post('/notification/channels', data)
}

export function updateChannel(id: string, data: Partial<{
  name: string
  channelType: string
  config: Record<string, any>
  enabled: boolean
}>): Promise<void> {
  return request.put(`/notification/channels/${id}`, data)
}

export function deleteChannel(id: string): Promise<void> {
  return request.delete(`/notification/channels/${id}`)
}

export function testChannel(id: string): Promise<{ message: string }> {
  return request.post(`/notification/channels/${id}/test`)
}

// ============================================================
// 通知引擎 — 规则管理
// ============================================================

export interface NotificationRule {
  id: string
  name: string
  eventType: string
  severityFilter: string[] | null
  channelIds: string[]
  recipientList: string
  enabled: boolean
  createdBy: string
  createdAt: string
  updatedAt: string
}

export function listRules(): Promise<NotificationRule[]> {
  return request.get('/notification/rules')
}

export function createRule(data: {
  name: string
  eventType: string
  severityFilter?: string[]
  channelIds: string[]
  recipientList?: string
  enabled?: boolean
}): Promise<{ id: string }> {
  return request.post('/notification/rules', data)
}

export function updateRule(id: string, data: Partial<{
  name: string
  eventType: string
  severityFilter: string[]
  channelIds: string[]
  recipientList: string
  enabled: boolean
}>): Promise<void> {
  return request.put(`/notification/rules/${id}`, data)
}

export function deleteRule(id: string): Promise<void> {
  return request.delete(`/notification/rules/${id}`)
}

// ============================================================
// 通知引擎 — 发送日志
// ============================================================

export interface NotificationLog {
  id: string
  ruleId?: string | null
  ruleName?: string | null
  channelId: string
  channelName: string
  channelType: 'email' | 'feishu' | 'webhook' | string
  eventType: string
  severity?: string | null
  recipients?: string | null
  title: string
  content?: string | null
  link?: string | null
  status: 'success' | 'failed' | string
  errorMsg?: string | null
  responseSnippet?: string | null
  durationMs?: number | null
  triggeredBy?: 'rule' | 'manual_test' | string | null
  sentAt: string
}

export interface NotificationLogPage {
  list: NotificationLog[]
  total: number
  page: number
  pageSize: number
}

export function listNotificationLogs(params?: {
  page?: number
  pageSize?: number
  status?: string
  channelType?: string
  eventType?: string
  channelId?: string
  triggeredBy?: string
  keyword?: string
  sentSince?: string
  sentUntil?: string
}): Promise<NotificationLogPage> {
  return request.get('/notification/logs', { params })
}

export interface NotificationCleanerConfig {
  enabled: boolean
  retentionDays: number
  intervalSecs: number
  batchSize: number
}

export interface NotificationCleanerRunResult {
  retentionDays: number
  cutoff: string
  deleted: number
  batches: number
  durationMs: number
}

export function getNotificationCleanerConfig(): Promise<NotificationCleanerConfig> {
  return request.get('/notification/cleaner-config')
}

export function runNotificationCleaner(data?: {
  retentionDays?: number
  batchSize?: number
}): Promise<NotificationCleanerRunResult> {
  return request.post('/notification/cleaner/run', data ?? {})
}
