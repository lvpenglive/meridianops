import request from './request'
import type { SystemSetting, PasswordPolicy, UpdateSettingsRequest } from './types'

/** 查询全部系统配置（需 system:read）。 */
export function listSettings(): Promise<SystemSetting[]> {
  return request.get('/system/settings')
}

/** 批量更新系统配置（需 system:update）。 */
export function updateSettings(data: UpdateSettingsRequest): Promise<void> {
  return request.put('/system/settings', { settings: data })
}

/** 查询当前密码策略（任意已登录用户）。个人中心改密页用。 */
export function getPasswordPolicy(): Promise<PasswordPolicy> {
  return request.get('/system/password-policy')
}
