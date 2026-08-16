import request from './request'

// ---- 类型 ----

export interface DictType {
  code: string
  name: string
  description: string | null
  enabled: boolean
  sortOrder: number
  createdAt: string
  updatedAt: string
}

export interface DictItem {
  id: string
  value: string
  label: string
  sortOrder: number
}

export interface CreateDictTypeRequest {
  code: string
  name: string
  description?: string
  sortOrder?: number
}

export interface UpdateDictTypeRequest {
  name: string
  description?: string
  enabled?: boolean
  sortOrder?: number
}

export interface CreateDictItemRequest {
  itemValue: string
  itemLabel: string
  sortOrder?: number
}

export interface UpdateDictItemRequest {
  itemLabel: string
  enabled?: boolean
  sortOrder?: number
}

// ---- 字典类型 API ----

export function listDictTypes(): Promise<DictType[]> {
  return request.get('/dict/types')
}

export function createDictType(data: CreateDictTypeRequest): Promise<{ code: string }> {
  return request.post('/dict/types', data)
}

export function updateDictType(code: string, data: UpdateDictTypeRequest): Promise<{ code: string }> {
  return request.put(`/dict/types/${code}`, data)
}

export function deleteDictType(code: string): Promise<void> {
  return request.delete(`/dict/types/${code}`)
}

// ---- 字典项 API ----

export function listDictItems(typeCode: string): Promise<DictItem[]> {
  return request.get(`/dict/types/${typeCode}/items`)
}

export function createDictItem(typeCode: string, data: CreateDictItemRequest): Promise<{ id: string }> {
  return request.post(`/dict/types/${typeCode}/items`, data)
}

export function updateDictItem(id: string, data: UpdateDictItemRequest): Promise<{ id: string }> {
  return request.put(`/dict/items/${id}`, data)
}

export function deleteDictItem(id: string): Promise<void> {
  return request.delete(`/dict/items/${id}`)
}
