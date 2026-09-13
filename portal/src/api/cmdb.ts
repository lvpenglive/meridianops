import request from './request'
import type {
  CiModel,
  CiModelDetail,
  CiModelAttr,
  CiInstance,
  CiInstancePage,
  CiInstanceQuery,
  CiRelation,
  CmdbStats,
  CreateCiInstanceRequest,
  UpdateCiInstanceRequest,
  CreateCiRelationRequest,
  BatchCreateInstancesRequest,
  BatchImportResult,
  SyncSource,
  SyncLogPage,
  SyncLogQuery,
  SyncRequest,
  SyncResult,
  PullRequest,
  PullResult,
  UpdateSyncSourceRequest,
  CreateSyncSourceRequest,
  HttpPushReport,
  HttpPushPreview,
  CreateCiModelRequest,
  UpdateCiModelRequest,
  CreateCiModelAttrRequest,
  UpdateCiModelAttrRequest,
  TopologyData,
  TopologyQuery,
  CiRelationType,
  CreateCiRelationTypeRequest,
  UpdateCiRelationTypeRequest,
  CmdbUserOption,
} from './types'

// ---- CI 模型 ----

/** 列出所有 CI 模型 */
export function listCiModels(): Promise<CiModel[]> {
  return request.get('/cmdb/models')
}

/** 获取模型详情（含属性定义） */
export function getCiModel(id: string): Promise<CiModelDetail> {
  return request.get(`/cmdb/models/${id}`)
}

/** 创建 CI 模型（动态建模，需 system:update） */
export function createCiModel(data: CreateCiModelRequest): Promise<CiModel> {
  return request.post('/cmdb/models', data)
}

/** 更新 CI 模型（code 不可改，需 system:update） */
export function updateCiModel(id: string, data: UpdateCiModelRequest): Promise<CiModel> {
  return request.put(`/cmdb/models/${id}`, data)
}

/** 删除 CI 模型（有实例时拒绝，需 system:update） */
export function deleteCiModel(id: string): Promise<void> {
  return request.delete(`/cmdb/models/${id}`)
}

/** 列出某模型的属性定义 */
export function listCiModelAttrs(modelId: string): Promise<CiModelAttr[]> {
  return request.get(`/cmdb/models/${modelId}/attrs`)
}

/** 创建模型属性（需 system:update） */
export function createCiModelAttr(modelId: string, data: CreateCiModelAttrRequest): Promise<CiModelAttr> {
  return request.post(`/cmdb/models/${modelId}/attrs`, data)
}

/** 更新模型属性（需 system:update） */
export function updateCiModelAttr(modelId: string, attrId: string, data: UpdateCiModelAttrRequest): Promise<CiModelAttr> {
  return request.put(`/cmdb/models/${modelId}/attrs/${attrId}`, data)
}

/** 删除模型属性（需 system:update） */
export function deleteCiModelAttr(modelId: string, attrId: string): Promise<void> {
  return request.delete(`/cmdb/models/${modelId}/attrs/${attrId}`)
}

// ---- CI 实例 ----

/** 分页查询 CI 实例 */
export function listCiInstances(params: CiInstanceQuery): Promise<CiInstancePage> {
  return request.get('/cmdb/instances', { params })
}

/** 获取实例详情 */
export function getCiInstance(id: string): Promise<CiInstance> {
  return request.get(`/cmdb/instances/${id}`)
}

/** 资产负责人候选（在职启用用户，需 asset:read） */
export function listCmdbUserOptions(): Promise<CmdbUserOption[]> {
  return request.get('/cmdb/user-options')
}

/** 创建 CI 实例 */
export function createCiInstance(data: CreateCiInstanceRequest): Promise<CiInstance> {
  return request.post('/cmdb/instances', data)
}

/** 更新 CI 实例 */
export function updateCiInstance(id: string, data: UpdateCiInstanceRequest): Promise<CiInstance> {
  return request.put(`/cmdb/instances/${id}`, data)
}

/** 删除 CI 实例 */
export function deleteCiInstance(id: string): Promise<void> {
  return request.delete(`/cmdb/instances/${id}`)
}

/** 批量导入 CI 实例（Excel/CSV 解析后提交，需 asset:create） */
export function batchCreateInstances(data: BatchCreateInstancesRequest): Promise<BatchImportResult> {
  return request.post('/cmdb/instances/batch', data)
}

// ---- CI 关系 ----

/** 查询某实例的关系列表 */
export function listCiRelations(ciId: string): Promise<CiRelation[]> {
  return request.get(`/cmdb/instances/${ciId}/relations`)
}

/** 创建 CI 关系 */
export function createCiRelation(data: CreateCiRelationRequest): Promise<{ id: string }> {
  return request.post('/cmdb/relations', data)
}

/** 删除 CI 关系 */
export function deleteCiRelation(id: string): Promise<void> {
  return request.delete(`/cmdb/relations/${id}`)
}

// ---- 统计 ----

/** CMDB 统计（各模型实例数） */
export function getCmdbStats(): Promise<CmdbStats> {
  return request.get('/cmdb/stats')
}

// ---- CMDB 同步：外部系统（蓝鲸等）数据接入 ----

/** 列出所有同步数据源 */
export function listSyncSources(): Promise<SyncSource[]> {
  return request.get('/cmdb/sync/sources')
}

/** 新增同步数据源 */
export function createSyncSource(data: CreateSyncSourceRequest): Promise<{ id: string; code: string }> {
  return request.post('/cmdb/sync/sources', data)
}

/** 删除同步数据源 */
export function deleteSyncSource(code: string): Promise<boolean> {
  return request.delete(`/cmdb/sync/sources/${code}`)
}

/** 批量同步（外部 CMDB webhook 推送入口） */
export function syncInstances(data: SyncRequest): Promise<SyncResult> {
  return request.post('/cmdb/sync', data)
}

/** 手动拉取（从外部 API 拉取数据） */
export function pullInstances(data: PullRequest): Promise<PullResult> {
  return request.post('/cmdb/sync/pull', data)
}

/** 手动 HTTP 出站推送（优云 / AxleOps / 理想自动化等） */
export function pushOutInstances(source: string): Promise<HttpPushReport> {
  return request.post('/cmdb/sync/push-out', { source })
}

/** 出站推送内容预览（不发请求） */
export function previewHttpPush(source: string): Promise<HttpPushPreview> {
  return request.get('/cmdb/sync/push-out/preview', { params: { source } })
}

/** 更新数据源拉取配置 */
export function updateSyncSource(code: string, data: UpdateSyncSourceRequest): Promise<boolean> {
  return request.put(`/cmdb/sync/sources/${code}`, data)
}

/** 查询同步日志 */
export function listSyncLogs(params: SyncLogQuery): Promise<SyncLogPage> {
  return request.get('/cmdb/sync/logs', { params })
}

export interface EventideLookupTargetStatus {
  lookupId: string
  name: string
  rowCount?: number
  httpStatus?: number | null
  latencyMs?: number
  syncedAt?: string | null
  error?: string | null
}

export interface EventideLookupLast {
  trigger: string
  startedAt: string
  finishedAt: string
  ok: boolean
  skipped: boolean
  message: string
  targets: EventideLookupTargetStatus[]
}

export interface EventideLookupStatus {
  enabled: boolean
  configured: boolean
  baseUrl: string
  intervalSecs: number
  debounceSecs: number
  tokenSet: boolean
  tokenMasked: string
  targets: EventideLookupTargetConfig[]
  last: EventideLookupLast | null
}

export interface EventideLookupTargetConfig {
  lookupId: string
  name: string
  source?: 'hosts' | 'sql'
  sql?: string
  keyColumn?: string
}

export interface UpdateEventideLookupConfig {
  enabled?: boolean
  baseUrl?: string
  lookupSyncToken?: string
  intervalSecs?: number
  debounceSecs?: number
  lookupId?: string
  name?: string
  targets?: EventideLookupTargetConfig[]
}

/** Eventide 外表同步状态 */
export function getEventideLookupStatus(): Promise<EventideLookupStatus> {
  return request.get('/cmdb/sync/eventide-lookups')
}

/** 手动推一轮 CMDB → Eventide 外表 */
export function runEventideLookupSync(): Promise<EventideLookupLast> {
  return request.post('/cmdb/sync/eventide-lookups')
}

/** 更新 Eventide 外表同步配置（立即生效，写入 system_settings） */
export function updateEventideLookupConfig(
  data: UpdateEventideLookupConfig,
): Promise<EventideLookupStatus> {
  return request.put('/cmdb/sync/eventide-lookups/config', data)
}

export interface EventideLookupPreviewRow {
  ip: string
  [col: string]: string
}

export interface EventideLookupPreviewTarget {
  lookupId: string
  name: string
  source?: string
  rowCount: number
  rows: EventideLookupPreviewRow[]
}

export function previewEventideLookupSql(data: {
  sql: string
  keyColumn?: string
}): Promise<{ rowCount: number; columns: string[]; rows: EventideLookupPreviewRow[] }> {
  return request.post('/cmdb/sync/eventide-lookups/preview-sql', data)
}

export interface EventideLookupPreview {
  rowCount: number
  targets: EventideLookupPreviewTarget[]
}

/** 预览当前将推到 Eventide 的行（不写对端） */
export function previewEventideLookup(): Promise<EventideLookupPreview> {
  return request.get('/cmdb/sync/eventide-lookups/preview')
}

// ---- 拓扑视图 ----

/** 查询拓扑（节点 + 边） */
export function getTopology(params?: TopologyQuery): Promise<TopologyData> {
  return request.get('/cmdb/topology', { params })
}

// ---- CI 关系类型 ----

/** 列出所有关系类型（按 sortOrder 排序，需 asset:read） */
export function listCiRelationTypes(): Promise<CiRelationType[]> {
  return request.get('/cmdb/relation-types')
}

/** 创建关系类型（需 system:update） */
export function createCiRelationType(data: CreateCiRelationTypeRequest): Promise<{ id: string }> {
  return request.post('/cmdb/relation-types', data)
}

/** 更新关系类型（code 不可改，需 system:update） */
export function updateCiRelationType(id: string, data: UpdateCiRelationTypeRequest): Promise<void> {
  return request.put(`/cmdb/relation-types/${id}`, data)
}

/** 删除关系类型（有关联关系时拒绝，需 system:update） */
export function deleteCiRelationType(id: string): Promise<void> {
  return request.delete(`/cmdb/relation-types/${id}`)
}
