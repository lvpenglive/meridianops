import request from './request'

// ===== 类型定义 =====

export type ScriptType = 'shell' | 'python' | 'powershell'
export type TargetScope = 'static' | 'cmdb_query' | 'manual'
/** V1.5: 执行器类型 mock=模拟 / ssh=真实 SSH */
export type ExecutorType = 'mock' | 'ssh'

/** 作业定义 */
export interface JobDefinition {
  id: number
  name: string
  description: string
  scriptType: ScriptType
  scriptContent: string
  timeoutSecs: number
  targetScope: TargetScope
  targetAssetIds: number[]
  targetCmdbQuery: string
  runAs: string
  port: number
  enabled: boolean
  /** V1.5: 执行器类型 */
  executorType: ExecutorType
  /** V1.5: SSH 凭据 ID（executorType=ssh 时必填） */
  credentialId: number | null
  /** V1.5: 凭据名称（列表接口回显用） */
  credentialName?: string
  createdBy: string
  createdAt: string
  updatedAt: string
}

/** 执行状态 */
export type JobStatus =
  | 'pending'
  | 'running'
  | 'success'
  | 'failed'
  | 'partial'
  | 'timeout'
  | 'cancelled'
  | 'skipped'

/** 执行历史（run） */
export interface JobRun {
  id: number
  jobId: number
  jobName: string
  scriptType: ScriptType
  scriptContent?: string
  triggerMode: 'manual' | 'cron' | 'api'
  targetCount: number
  successCount: number
  failedCount: number
  overallStatus: JobStatus
  startedBy: string
  startedAt: string
  finishedAt?: string
  durationMs: number
}

/** 单个资产执行结果 */
export interface JobRunTarget {
  id: number
  assetId: number
  assetName: string
  assetIp: string
  status: JobStatus
  exitCode?: number
  stdout: string
  stderr: string
  durationMs: number
  startedAt?: string
  finishedAt?: string
}

/** 分页结果 */
export interface PageList<T> {
  list: T[]
  total: number
  page: number
  pageSize: number
}

// ===== API =====

/** 作业定义列表 */
export function listJobDefinitions(params?: {
  page?: number
  pageSize?: number
  keyword?: string
  status?: 'enabled' | 'disabled' | ''
}) {
  return request.get<PageList<JobDefinition>>('/jobs/definitions', { params })
}

/** 单条作业定义 */
export function getJobDefinition(id: number) {
  return request.get<JobDefinition>(`/jobs/definitions/${id}`)
}

/** 创建作业定义 */
export function createJobDefinition(payload: Partial<JobDefinition>) {
  return request.post<{ id: number; message: string }>('/jobs/definitions', payload)
}

/** 更新作业定义 */
export function updateJobDefinition(id: number, payload: Partial<JobDefinition>) {
  return request.put<{ message: string }>(`/jobs/definitions/${id}`, payload)
}

/** 删除作业定义 */
export function deleteJobDefinition(id: number) {
  return request.delete<{ message: string }>(`/jobs/definitions/${id}`)
}

/** 执行作业 */
export function executeJob(id: number, assetIds: number[]) {
  return request.post<{ jobRunId: number; targetCount: number; message: string }>(
    `/jobs/definitions/${id}/execute`,
    { assetIds },
  )
}

/** 执行历史列表 */
export function listJobRuns(params?: {
  page?: number
  pageSize?: number
  keyword?: string
  status?: string
}) {
  return request.get<PageList<JobRun>>('/jobs/runs', { params })
}

/** 单条执行历史（含 targets 列表） */
export function getJobRun(id: number) {
  return request.get<{ run: JobRun; targets: JobRunTarget[] }>(`/jobs/runs/${id}`)
}

/** 单个 target 完整 stdout/stderr（列表接口 stdout 会截断） */
export function getJobRunTargetOutput(runId: number, targetId: number) {
  return request.get<{ stdout: string; stderr: string; exitCode?: number; status: JobStatus }>(
    `/jobs/runs/${runId}/targets/${targetId}`,
  )
}

/** 作业执行对话框资产选择列表（来自 ci_assets） */
export function listJobAssets(params?: { page?: number; pageSize?: number; keyword?: string }) {
  return request.get<PageList<{
    id: number
    assetName: string
    primaryIp: string
    assetType: string
    status: string
  }>>('/jobs/assets', { params })
}
