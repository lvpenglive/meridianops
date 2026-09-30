import request from './request'

export interface MonitorInstance {
  code: string
  name: string
  tokenConfigured: boolean
}

export interface MonitorHost {
  hostId: string
  hostname: string
  displayName: string
  ip: string
  agentAvailable: boolean | null
  agentVersion: string | null
  ciId?: string | null
  cmdbLinked?: boolean
}

export interface MonitorProblem {
  eventId: string
  name: string
  severity: string
  clock: string
}

export interface MetricPoint {
  clock: string
  value: string
}

export interface MetricSeries {
  name: string
  key: string | null
  points: MetricPoint[]
}

export type MonitorAction = 'start' | 'stop' | 'restart' | 'upgrade' | 'rollback'

export interface MonitorTaskHost {
  hostId: string
  ciId?: string | null
  jobRunId?: number | null
  versionBefore?: string | null
  versionAfter?: string | null
  status: string
  error?: string | null
}

export interface MonitorTask {
  id: string
  instanceCode: string
  action: string
  targetVersion?: string | null
  concurrency: number
  maintenanceId?: string | null
  status: string
  requestedBy: string
  error?: string | null
  hosts: MonitorTaskHost[]
}

export function listInstances(): Promise<MonitorInstance[]> {
  return request.get('/monitor/instances')
}

export function listHosts(code: string, search?: string): Promise<MonitorHost[]> {
  return request.get(`/monitor/instances/${code}/hosts`, {
    params: { search: search || undefined },
  })
}

export function hostProblems(code: string, hostId: string): Promise<MonitorProblem[]> {
  return request.get(`/monitor/instances/${code}/hosts/${hostId}/problems`)
}

export function hostMetrics(code: string, hostId: string, hours = 24): Promise<MetricSeries[]> {
  return request.get(`/monitor/instances/${code}/hosts/${hostId}/metrics`, { params: { hours } })
}

export function syncHosts(code: string): Promise<{ synced: number }> {
  return request.post(`/monitor/instances/${code}/sync`)
}

export interface GovernanceProxy {
  proxyId: string
  name: string
  lastAccess: number
  ageSecs: number | null
  online: boolean
}

export interface GovernanceDrift {
  hostId: string
  hostname: string
  displayName: string
  templates: string[]
  missing: string[]
  extra: string[]
  drifted: boolean
}

export interface GovernanceQueue {
  available: boolean
  count: number
  warn: number
  backedUp: boolean
  source?: string
  message?: string
}

export interface GovernanceData {
  proxies: GovernanceProxy[]
  offlineProxyCount: number
  templateDrift: GovernanceDrift[]
  driftCount: number
  queue: GovernanceQueue
  standardTemplates: string[]
}

export function getGovernance(code: string): Promise<GovernanceData> {
  return request.get(`/monitor/instances/${code}/governance`)
}

export function createTask(data: {
  instanceCode: string
  action: MonitorAction
  hostIds: string[]
  targetVersion?: string
  concurrency?: number
}): Promise<{ id: string }> {
  return request.post('/monitor/tasks', data)
}

export function getTask(id: string): Promise<MonitorTask> {
  return request.get(`/monitor/tasks/${id}`)
}
