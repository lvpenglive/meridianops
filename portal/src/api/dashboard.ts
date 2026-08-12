import request from './request'
import type { DashboardData } from './types'

/** 计算今日 00:00（本地时区）的 ISO 字符串，作为 dashboard "今日" 起始时间。 */
function todayStartIso(): string {
  const d = new Date()
  d.setHours(0, 0, 0, 0)
  return d.toISOString()
}

/** 拉取个人工作台聚合数据（任意已登录用户）。 */
export function getDashboard(): Promise<DashboardData> {
  return request.get('/dashboard', { params: { since: todayStartIso() } })
}
