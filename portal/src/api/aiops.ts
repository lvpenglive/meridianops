import request from './request'

/** AIOps 概览数据 */
export interface AiopsOverview {
  activeAlerts: number
  todayNew: number
  bySeverity: Record<string, number>
  bySource: Record<string, number>
  knowledgeCount: number
  ticketCount: number
  trend24h: { hour: string; count: number }[]
  llmEnabled: boolean
}

/** 相似案例推荐 */
export interface RecommendResult {
  alert: { title: string; message: string | null; ciName: string | null } | null
  keyword: string
  knowledge: {
    id: string
    title: string
    category: string
    summary: string | null
    score: number
  }[]
  tickets: {
    id: string
    ticketNo: string
    title: string
    status: string
    priority: string
    resolution: string | null
    createdAt: string
    closedAt: string | null
    score: number
  }[]
}

/** 异常检测 */
export interface AnomalyItem {
  type: string
  severity: 'high' | 'medium' | 'critical' | string
  title: string
  message: string
  data?: Record<string, unknown>
}

export interface AnomaliesResult {
  anomalies: AnomalyItem[]
  trend: { hour: string; count: number }[]
  stats: { totalEvents: number; resolvedCount: number; anomalyCount: number }
}

/** 根因分析 */
export interface RcaResult {
  rootAlert: {
    id: string
    title: string
    severity: string
    source: string
    ciId: string | null
    ciName: string | null
    firedAt: string
  }
  relatedAlerts: {
    id: string
    source: string
    severity: string
    status: string
    title: string
    message: string | null
    ciId: string | null
    ciName: string | null
    fireCount: number
    firedAt: string
  }[]
  topology: {
    nodes: { id: string; name: string; type: string; level: string }[]
    links: { source: string; target: string; relationType: string }[]
  }
  rootCause: { ciId: string; ciName: string; confidence: number } | null
  diagnosisTree: {
    id: string
    label: string
    level: string
    confidence: number
    children?: any[]
  }[]
  knowledge: {
    id: string
    title: string
    category: string
    summary: string | null
  }[]
  suggestions: {
    time: string
    type: string
    title: string
    desc: string
  }[]
}

/** LLM 诊断结果 */
export interface LlmDiagnoseResult {
  answer: string
  model: string
  context: Record<string, unknown>
}

/** 获取 AIOps 概览 */
export function getAiopsOverview(): Promise<AiopsOverview> {
  return request.get('/aiops/overview')
}

/** 相似案例推荐 */
export function getRecommend(params: { alertId?: string; q?: string }): Promise<RecommendResult> {
  return request.get('/aiops/recommend', { params })
}

/** 异常趋势检测 */
export function getAnomalies(): Promise<AnomaliesResult> {
  return request.get('/aiops/anomalies')
}

/** 根因分析 */
export function getRca(alertId: string): Promise<RcaResult> {
  return request.get(`/aiops/rca/${alertId}`)
}

/** LLM 诊断 */
export function llmDiagnose(data: { alertId?: string; question?: string }): Promise<LlmDiagnoseResult> {
  return request.post('/aiops/llm-diagnose', data)
}
