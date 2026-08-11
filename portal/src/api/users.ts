import request from './request'
import type { CreateUserRequest, UpdateUserRequest, UserInfo, UserRole } from './types'

/** 列出所有用户（仅 admin） */
export function listUsers(): Promise<UserInfo[]> {
  return request.get('/users')
}

/** 创建用户（仅 admin） */
export function createUser(data: CreateUserRequest): Promise<UserInfo> {
  return request.post('/users', data)
}

/** 编辑用户可变字段（显示名/邮箱/角色/启用）。用户名不可改。 */
export function updateUser(id: string, data: UpdateUserRequest): Promise<UserInfo> {
  return request.put(`/users/${id}`, data)
}

/** 启用/禁用用户 */
export function toggleUserEnable(id: string, enabled: boolean): Promise<void> {
  return request.patch(`/users/${id}/enable`, { enabled })
}

/** 管理员重置用户密码 */
export function resetUserPassword(id: string, password: string): Promise<void> {
  return request.post(`/users/${id}/password-reset`, { password })
}
