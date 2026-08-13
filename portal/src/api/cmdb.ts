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
  SyncLog,
  SyncLogPage,
  SyncLogQuery,
  SyncRequest,
  SyncResult,
  PullRequest,
  PullResult,
  UpdateSyncSourceRequest,
  CreateSyncSourceRequest,
  CreateCiModelRequest,
  UpdateCiModelRequest,
  CreateCiModelAttrRequest,
  UpdateCiModelAttrRequest,
  TopologyData,
  TopologyQuery,
  CiRelationType,
  CreateCiRelationTypeRequest,
  UpdateCiRelationTypeRequest,
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

/** 更新数据源拉取配置 */
export function updateSyncSource(code: string, data: UpdateSyncSourceRequest): Promise<boolean> {
  return request.put(`/cmdb/sync/sources/${code}`, data)
}

/** 查询同步日志 */
export function listSyncLogs(params: SyncLogQuery): Promise<SyncLogPage> {
  return request.get('/cmdb/sync/logs', { params })
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
