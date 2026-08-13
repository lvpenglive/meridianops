import request from './request'
import type {
  ApiToken,
  CreateApiTokenRequest,
  CreateApiTokenResponse,
  MyPermissionsResponse,
  UpdateExpiryRequest,
} from './types'

/** 当前用户可授予的权限范围（新建对话框用） */
export function fetchMyPermissions(): Promise<MyPermissionsResponse> {
  return request.get('/api-tokens/permissions')
}

/** API 令牌列表（管理员=全部，普通用户=自己） */
export function fetchApiTokens(): Promise<ApiToken[]> {
  return request.get('/api-tokens')
}

/** 新建令牌，返回明文 token（仅此次！） */
export function createApiToken(
  payload: CreateApiTokenRequest,
): Promise<CreateApiTokenResponse> {
  return request.post('/api-tokens', payload)
}

/** 吊销令牌 */
export function revokeApiToken(id: string): Promise<boolean> {
  return request.post(`/api-tokens/${id}/revoke`)
}

/** 更新有效期 */
export function updateApiTokenExpiry(
  id: string,
  payload: UpdateExpiryRequest,
): Promise<boolean> {
  return request.put(`/api-tokens/${id}/expiry`, payload)
}

/** 彻底删除（需 system:update，admin） */
export function deleteApiToken(id: string): Promise<boolean> {
  return request.delete(`/api-tokens/${id}`)
}
