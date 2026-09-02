import request from './request'

// ============================================================
// 日志中心 API（对接后端 log_routes.rs，底层 HTTP POST SQL 给 ClickHouse）
// 三层检索能力：L1 基础检索 / L2 聚合统计 / L3 模式挖掘
// ============================================================

/** 单条日志记录（对应 ClickHouse meridianops_logs.all_logs 表） */
export interface LogRow {
  timestamp: string
  hostname: string
  service: string
  source: string
  level: string
  message: string
  traceId?: string | null
  ip?: string | null
  extra?: Record<string, any> | null
  rawJson?: string
}

/** 日志列表分页响应 */
export interface LogListResponse {
  total: number
  limit: number
  offset: number
  items: LogRow[]
}

/** 单条聚合统计行 */
export interface StatRow {
  bucket: string
  level: string
  count: number
}

/** 聚合统计响应 */
export interface StatsResponse {
  groupBy: string
  items: StatRow[]
}

/** 单条模式挖掘结果 */
export interface PatternRow {
  template: string
  count: number
  sample: string
}

/** 模式挖掘响应 */
export interface PatternsResponse {
  items: PatternRow[]
}

/** Grafana 跳转链接响应 */
export interface GrafanaLinkResponse {
  url: string
  expr: string
  from: string
  to: string
}

// ===== L1 基础检索 =====

export function listLogs(params?: {
  hostname?: string
  service?: string
  level?: string // 逗号分隔：error,warn,info
  keyword?: string
  startTime?: string
  endTime?: string
  limit?: number
  offset?: number
}): Promise<LogListResponse> {
  return request.get('/logs', { params })
}

// ===== L2 聚合统计 =====

export function getLogStats(params?: {
  hostname?: string
  service?: string
  level?: string
  startTime?: string
  endTime?: string
  groupBy?: 'service' | 'level' | 'hostname'
  top?: number
}): Promise<StatsResponse> {
  return request.get('/logs/stats', { params })
}

// ===== L3 模式挖掘 =====

export function getLogPatterns(params?: {
  hostname?: string
  service?: string
  level?: string
  startTime?: string
  endTime?: string
  top?: number
}): Promise<PatternsResponse> {
  return request.get('/logs/patterns', { params })
}

// ===== 生成 Grafana Explore 跳转 URL =====

export function getGrafanaLink(params?: {
  hostname?: string
  level?: string
  keyword?: string
  startTime?: string
  endTime?: string
}): Promise<GrafanaLinkResponse> {
  return request.get('/logs/grafana-link', { params })
}

// ===== 常量选项 =====

/** 日志级别选项（与 ClickHouse all_logs.level 字段值对应） */
export const LOG_LEVELS: { value: string; label: string; tagType: string }[] = [
  { value: 'fatal',     label: 'FATAL',    tagType: 'danger'  },
  { value: 'critical',  label: 'CRITICAL', tagType: 'danger'  },
  { value: 'error',     label: 'ERROR',    tagType: 'danger'  },
  { value: 'warn',      label: 'WARN',     tagType: 'warning' },
  { value: 'info',      label: 'INFO',     tagType: 'info'    },
  { value: 'debug',     label: 'DEBUG',    tagType: ''        },
]

/** 把 level 字符串映射为 el-tag type（用于行内 tag 颜色） */
export function levelTagType(level: string): string {
  return LOG_LEVELS.find((l) => l.value === level?.toLowerCase())?.tagType ?? ''
}

/** 把 level 字符串映射为大写标签文本 */
export function levelLabel(level: string): string {
  return LOG_LEVELS.find((l) => l.value === level?.toLowerCase())?.label ?? (level || '').toUpperCase()
}
