import request from './request'
import type {
  CreateDepartmentRequest,
  UpdateDepartmentRequest,
  Department,
} from './types'

/** 列出所有部门（扁平列表，前端按 parentId 构建树） */
export function listDepartments(): Promise<Department[]> {
  return request.get('/departments')
}

/** 部门详情 */
export function getDepartment(id: string): Promise<Department> {
  return request.get(`/departments/${id}`)
}

/** 创建部门。parentId 为空表示根部门 */
export function createDepartment(data: CreateDepartmentRequest): Promise<Department> {
  return request.post('/departments', data)
}

/** 更新部门（parentId 可改，但不允许设为自己或子孙，防环） */
export function updateDepartment(id: string, data: UpdateDepartmentRequest): Promise<Department> {
  return request.put(`/departments/${id}`, data)
}

/** 删除部门（有子部门或用户引用时不可删） */
export function deleteDepartment(id: string): Promise<void> {
  return request.delete(`/departments/${id}`)
}
