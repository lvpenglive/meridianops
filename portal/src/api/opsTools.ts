import request from './request'

export interface OpsTool {
  id: string
  name: string
  url: string
  description: string
  icon: string
  sortOrder: number
  enabled: boolean
  createdAt?: string
  updatedAt?: string
}

export interface OpsToolBody {
  name: string
  url: string
  description?: string
  icon?: string
  sortOrder?: number
  enabled?: boolean
}

export function listOpsTools(): Promise<OpsTool[]> {
  return request.get('/ops-tools')
}

export function createOpsTool(data: OpsToolBody): Promise<{ id: string }> {
  return request.post('/ops-tools', data)
}

export function updateOpsTool(id: string, data: OpsToolBody): Promise<{ id: string }> {
  return request.put(`/ops-tools/${id}`, data)
}

export function deleteOpsTool(id: string): Promise<boolean> {
  return request.delete(`/ops-tools/${id}`)
}
