import request from './request'
import type {
  CreateRoleRequest,
  UpdateRoleRequest,
  Role,
  Permission,
} from './types'

/** 列出所有角色 */
export function listRoles(): Promise<Role[]> {
  return request.get('/roles')
}

/** 角色详情 */
export function getRole(id: string): Promise<Role> {
  return request.get(`/roles/${id}`)
}

/** 创建角色 */
export function createRole(data: CreateRoleRequest): Promise<Role> {
  return request.post('/roles', data)
}

/** 更新角色可变字段（名称不可改） */
export function updateRole(id: string, data: UpdateRoleRequest): Promise<Role> {
  return request.put(`/roles/${id}`, data)
}

/** 删除角色（内置角色不可删，有用户引用时不可删） */
export function deleteRole(id: string): Promise<void> {
  return request.delete(`/roles/${id}`)
}

/** 查看角色已分配的权限点列表 */
export function listRolePermissions(id: string): Promise<Permission[]> {
  return request.get(`/roles/${id}/permissions`)
}

/** 批量设置角色权限（全量覆盖） */
export function setRolePermissions(id: string, permissionIds: string[]): Promise<void> {
  return request.put(`/roles/${id}/permissions`, { permissionIds })
}

/** 列出所有权限点（按模块分组排序） */
export function listPermissions(): Promise<Permission[]> {
  return request.get('/permissions')
}
