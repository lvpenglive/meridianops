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

export function getLoginTrend(days = 30) {
  return request.get<unknown, LoginTrendItem[]>('/reports/login-trend', { params: { days } })
}

export function getLoginFailedTop(days = 30, limit = 10) {
  return request.get<unknown, FailedTopItem[]>('/reports/login-failed-top', {
    params: { days, limit },
  })
}

export function getLockedUsers() {
  return request.get<unknown, UserInfo[]>('/reports/locked-users')
}

export function getSensitiveOpsTrend(days = 30) {
  return request.get<unknown, SensitiveTrendItem[]>('/reports/sensitive-ops-trend', {
    params: { days },
  })
}

export function getSensitiveOpsTop(days = 30, limit = 10) {
  return request.get<unknown, SensitiveTopItem[]>('/reports/sensitive-ops-top', {
    params: { days, limit },
  })
}

export function getSensitiveOpsList(params: {
  days?: number
  page?: number
  pageSize?: number
}) {
  return request.get<unknown, SensitiveListResponse>('/reports/sensitive-ops-list', { params })
}

export function getComplianceSummary() {
  return request.get<unknown, ComplianceSummary>('/reports/compliance-summary')
}

export function getInactiveUsers(days = 90) {
  return request.get<unknown, UserInfo[]>('/reports/inactive-users', { params: { days } })
}

export function getRoleAssignment() {
  return request.get<unknown, RoleAssignmentItem[]>('/reports/role-assignment')
}
