import request from './request'
import { listDictItems, type DictItem } from './dict'

// ============================================================
// 告警短信策略（对齐老系统「短信策略」）
// ============================================================

export interface SmsStrategy {
  id: string
  eventType: string
  eventSubType: string
  triggerOp: string
  severityFilter: string
  hostFilter: string
  nameKeyword: string
  alertGroupId?: string | null
  alertGroupName?: string
  recipientUserIds: string[]
  description: string
  enabled: boolean
  createdBy: string
  createdAt: string
  updatedAt: string
}

export interface SmsStrategyPage {
  list: SmsStrategy[]
  total: number
  page: number
  pageSize: number
}

export function listSmsStrategies(params?: {
  page?: number
  pageSize?: number
  keyword?: string
  eventType?: string
  enabled?: boolean
}): Promise<SmsStrategyPage> {
  return request.get('/sms-strategies', { params })
}

export interface SmsStrategyPayload {
  eventType?: string
  eventSubType?: string
  triggerOp?: string
  severityFilter?: string
  hostFilter?: string
  nameKeyword?: string
  alertGroupId?: string
  recipientUserIds: string[]
  description?: string
  enabled?: boolean
}

export function createSmsStrategy(data: SmsStrategyPayload): Promise<{ id: string }> {
  return request.post('/sms-strategies', data)
}

export function updateSmsStrategy(id: string, data: SmsStrategyPayload): Promise<void> {
  return request.put(`/sms-strategies/${id}`, data)
}

export function toggleSmsStrategy(id: string, enabled: boolean): Promise<void> {
  return request.patch(`/sms-strategies/${id}/enable`, { enabled })
}

export function deleteSmsStrategy(id: string): Promise<void> {
  return request.delete(`/sms-strategies/${id}`)
}

/// 事件类型（一级，来自字典 event_type）
export function listSmsEventTypes(): Promise<DictItem[]> {
  return listDictItems('event_type')
}

/// 事件子类型（二级，来自字典 event_sub_type）
export function listSmsSubTypes(): Promise<DictItem[]> {
  return listDictItems('event_sub_type')
}

/// 触发级别运算符（与后端 trigger_op 一致）
export const SMS_TRIGGER_OPS: { value: string; label: string }[] = [
  { value: 'eq', label: '等于' },
  { value: 'gte', label: '大于等于' },
  { value: 'gt', label: '大于' },
  { value: 'lte', label: '小于等于' },
  { value: 'lt', label: '小于' },
]

/// 事件级别选项（老系统：信息 + 一级~五级）
export const SMS_SEVERITIES: { value: string; label: string }[] = [
  { value: 'info', label: '信息事件' },
  { value: '1', label: '一级事件' },
  { value: '2', label: '二级事件' },
  { value: '3', label: '三级事件' },
  { value: '4', label: '四级事件' },
  { value: '5', label: '五级事件' },
]
