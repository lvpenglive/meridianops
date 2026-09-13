import request from './request'
import type {
  LoginTrendItem,
  FailedTopItem,
  SensitiveTrendItem,
  SensitiveTopItem,
  SensitiveListResponse,
  ComplianceSummary,
  RoleAssignmentItem,
  UserInfo,
} from './types'

export function getLoginTrend(days = 30): Promise<LoginTrendItem[]> {
  return request.get<LoginTrendItem[]>('/reports/login-trend', { params: { days } })
}

export function getLoginFailedTop(days = 30, limit = 10): Promise<FailedTopItem[]> {
  return request.get<FailedTopItem[]>('/reports/login-failed-top', {
    params: { days, limit },
  })
}

export function getLockedUsers(): Promise<UserInfo[]> {
  return request.get<UserInfo[]>('/reports/locked-users')
}

export function getSensitiveOpsTrend(days = 30): Promise<SensitiveTrendItem[]> {
  return request.get<SensitiveTrendItem[]>('/reports/sensitive-ops-trend', {
    params: { days },
  })
}

export function getSensitiveOpsTop(days = 30, limit = 10): Promise<SensitiveTopItem[]> {
  return request.get<SensitiveTopItem[]>('/reports/sensitive-ops-top', {
    params: { days, limit },
  })
}

export function getSensitiveOpsList(params: {
  days?: number
  page?: number
  pageSize?: number
}): Promise<SensitiveListResponse> {
  return request.get<SensitiveListResponse>('/reports/sensitive-ops-list', { params })
}

export function getComplianceSummary(): Promise<ComplianceSummary> {
  return request.get<ComplianceSummary>('/reports/compliance-summary')
}

export function getInactiveUsers(days = 90): Promise<UserInfo[]> {
  return request.get<UserInfo[]>('/reports/inactive-users', { params: { days } })
}

export function getRoleAssignment(): Promise<RoleAssignmentItem[]> {
  return request.get<RoleAssignmentItem[]>('/reports/role-assignment')
}
