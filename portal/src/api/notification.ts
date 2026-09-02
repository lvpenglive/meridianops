import request from './request'
import { listDictItems, type DictItem } from './dict'

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

export interface NotificationChannelPage {
  list: NotificationChannel[]
  total: number
  page: number
  pageSize: number
}

export function listChannels(params?: {
  page?: number
  pageSize?: number
  keyword?: string
  channelType?: string
  enabled?: boolean
}): Promise<NotificationChannelPage> {
  return request.get('/notification/channels', { params })
}

/// 便捷方法：一次性取最多 1000 条通道（用于下拉选择场景）
export async function listAllChannels(): Promise<NotificationChannel[]> {
  const r = await listChannels({ pageSize: 1000 })
  return r.list
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
  triggerScene: string
  severityFilter: string[] | null
  severityOp: string
  hostFilter: string
  nameKeyword: string
  channelIds: string[]
  recipientList: string
  enabled: boolean
  createdBy: string
  createdAt: string
  updatedAt: string
}

export interface NotificationRulePage {
  list: NotificationRule[]
  total: number
  page: number
  pageSize: number
}

export function listRules(params?: {
  page?: number
  pageSize?: number
  keyword?: string
  eventType?: string
  triggerScene?: string
  enabled?: boolean
}): Promise<NotificationRulePage> {
  return request.get('/notification/rules', { params })
}

/// 触发场景（系统内置 6 个生命周期，不来自字典）
export const TRIGGER_SCENES: { value: string; label: string }[] = [
  { value: 'alert_firing',       label: '告警触发' },
  { value: 'alert_acknowledged', label: '告警认领' },
  { value: 'alert_resolved',     label: '告警恢复' },
  { value: 'ticket_assigned',    label: '工单分派' },
  { value: 'ticket_closed',      label: '工单关闭' },
  { value: 'job_failed',         label: '作业失败' },
]

/// 事件类型从字典 event_type 读取
export function listEventTypes(): Promise<DictItem[]> {
  return listDictItems('event_type')
}

export function createRule(data: {
  name: string
  eventType?: string
  triggerScene?: string
  severityFilter?: string[]
  severityOp?: string
  hostFilter?: string
  nameKeyword?: string
  channelIds: string[]
  recipientList?: string
  enabled?: boolean
}): Promise<{ id: string }> {
  return request.post('/notification/rules', data)
}

export function updateRule(id: string, data: Partial<{
  name: string
  eventType: string
  triggerScene: string
  severityFilter: string[]
  severityOp: string
  hostFilter: string
  nameKeyword: string
  channelIds: string[]
  recipientList: string
  enabled: boolean
}>): Promise<void> {
  return request.put(`/notification/rules/${id}`, data)
}

/// 级别比较运算符选项（与后端 severity_op 字段一致）
export const SEVERITY_OPS: { value: string; label: string; desc: string }[] = [
  { value: 'in',      label: '包含于', desc: '多选包含匹配（默认）' },
  { value: 'gte',     label: '≥ 大于等于', desc: '告警级别 ≥ 选定级别' },
  { value: 'gt',      label: '> 大于', desc: '告警级别 > 选定级别' },
  { value: 'eq',      label: '= 等于', desc: '告警级别 = 选定级别' },
  { value: 'lte',     label: '≤ 小于等于', desc: '告警级别 ≤ 选定级别' },
  { value: 'lt',      label: '< 小于', desc: '告警级别 < 选定级别' },
  { value: 'between', label: '区间', desc: '告警级别在 [min, max] 闭区间内' },
]

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
