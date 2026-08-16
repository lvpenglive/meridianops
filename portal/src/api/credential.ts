import request from './request'
import type { PageList } from './job'

// ===== 类型定义 =====

export type AuthType = 'password' | 'key'

/** 凭据列表项 / 详情（脱敏，不含明文密码/私钥） */
export interface SshCredential {
  id: number
  name: string
  authType: AuthType
  username: string
  /** 预期主机密钥指纹（可空） */
  hostKeyFingerprint: string
  description: string
  createdBy: string
  createdAt: string
  updatedAt: string
}

/** 简易凭据项（供作业定义下拉选择） */
export interface SshCredentialSimple {
  id: number
  name: string
  authType: AuthType
  username: string
}

/** 创建凭据请求 */
export interface CreateCredentialPayload {
  name: string
  authType: AuthType
  username: string
  /** 密码（authType=password 时必填） */
  password?: string
  /** 私钥 PEM（authType=key 时必填） */
  privateKey?: string
  /** 私钥口令（可空） */
  passphrase?: string
  hostKeyFingerprint?: string
  description?: string
}

/** 更新凭据请求（敏感字段留空 = 不修改） */
export type UpdateCredentialPayload = CreateCredentialPayload

// ===== API =====

/** 凭据列表（分页） */
export function listCredentials(params?: {
  page?: number
  pageSize?: number
  keyword?: string
}) {
  return request.get<PageList<SshCredential>>('/credentials', { params })
}

/** 单条凭据详情（脱敏） */
export function getCredential(id: number) {
  return request.get<SshCredential>(`/credentials/${id}`)
}

/** 新建凭据 */
export function createCredential(payload: CreateCredentialPayload) {
  return request.post<{ id: number; message: string }>('/credentials', payload)
}

/** 更新凭据（敏感字段留空 = 不修改） */
export function updateCredential(id: number, payload: UpdateCredentialPayload) {
  return request.put<{ message: string }>(`/credentials/${id}`, payload)
}

/** 删除凭据（被作业引用时后端会拒绝） */
export function deleteCredential(id: number) {
  return request.delete<{ message: string }>(`/credentials/${id}`)
}

/** 不分页简易列表（供作业定义下拉选择） */
export function listAllCredentials() {
  return request.get<SshCredentialSimple[]>('/credentials/list-all')
}
