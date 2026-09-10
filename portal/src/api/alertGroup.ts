import request from './request'

// ============================================================
// 告警组（独立实体，短信策略「选择组」数据源）
// ============================================================

export interface AlertGroup {
  id: string
  code: string
  name: string
  description: string
  enabled: boolean
  parentId?: string | null
  createdBy: string
  createdAt: string
  updatedAt: string
  memberCount: number
  memberNames: string[]
}

/** 树节点（后端全量树接口返回，含嵌套 children） */
export interface AlertGroupTreeNode {
  id: string
  code: string
  name: string
  description: string
  enabled: boolean
  parentId?: string | null
  memberCount: number
  children: AlertGroupTreeNode[]
}

export interface AlertGroupPage {
  list: AlertGroup[]
  total: number
  page: number
  pageSize: number
}

export interface AlertGroupMember {
  id: string
  displayName?: string | null
  username?: string | null
  email?: string | null
  enabled: boolean
}

export function listAlertGroups(params?: {
  page?: number
  pageSize?: number
  keyword?: string
  enabled?: boolean
}): Promise<AlertGroupPage> {
  return request.get('/alert-groups', { params })
}

export interface AlertGroupPayload {
  code: string
  name: string
  description?: string
  enabled?: boolean
}

export function createAlertGroup(data: AlertGroupPayload): Promise<{ id: string }> {
  return request.post('/alert-groups', data)
}

export function updateAlertGroup(id: string, data: AlertGroupPayload): Promise<void> {
  return request.put(`/alert-groups/${id}`, data)
}

export function toggleAlertGroup(id: string, enabled: boolean): Promise<void> {
  return request.patch(`/alert-groups/${id}/enable`, { enabled })
}

export function deleteAlertGroup(id: string): Promise<void> {
  return request.delete(`/alert-groups/${id}`)
}

export function listAlertGroupMembers(id: string, includeDescendants = false): Promise<AlertGroupMember[]> {
  return request.get(`/alert-groups/${id}/members`, {
    params: includeDescendants ? { includeDescendants: true } : {},
  })
}

export function replaceAlertGroupMembers(id: string, userIds: string[]): Promise<{ count: number }> {
  return request.put(`/alert-groups/${id}/members`, { userIds })
}

/** 全量树（无分页），用于左树展示与短信策略父组选择 */
export function getAlertGroupTree(): Promise<AlertGroupTreeNode[]> {
  return request.get('/alert-groups/tree')
}