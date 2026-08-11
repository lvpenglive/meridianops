import request from './request'
import type { AuditLog, AuditPageResponse, AuditQueryParams } from './types'

/** 分页查询审计日志（仅 admin） */
export function listAuditLogs(params?: AuditQueryParams): Promise<AuditPageResponse> {
  return request.get('/audit-logs', { params })
}

/** 获取审计日志详情 */
export function getAuditLog(id: number): Promise<AuditLog> {
  return request.get(`/audit-logs/${id}`)
}