import request from './request'

/* ============================================================
 * 工作流模板 API
 *   - GET    /api/workflow-templates          列表（内置 + 自定义）
 *   - POST   /api/workflow-templates          创建（workflow:admin 权限）
 *   - GET    /api/workflow-templates/:id      详情 + 编译预览
 *   - PUT    /api/workflow-templates/:id      更新
 *   - DELETE /api/workflow-templates/:id      软删
 *   - POST   /api/workflow-templates/:id/enable  启停（body: {enabled: bool}）
 * ============================================================ */

export type TemplateScope = 'builtin' | 'custom'

/** 定义：LogicFlow Graph 结构（前端画布编辑 / 保存 / 下发） */
export interface LfDefinition {
  nodes: Array<LfNode>
  edges: Array<LfEdge>
}

export interface LfNodeProps {
  /** 唯一业务 key，用于推进决策路由（不与 id 耦合） */
  key?: string
  name?: string
  approverSelector?: string[]
  /** 单人/会签/任一/或签 等超时时间 */
  timeoutHours?: number
  timeoutAction?: 'escalate' | 'auto_pass' | 'auto_reject' | 'auto_close' | string
  rejectBackTo?: string
}

export interface LfNode {
  id: string
  type:
    | 'start'
    | 'end'
    | 'auto_pass'
    | 'single_approval'
    | 'all_approval'
    | 'any_approval'
    | 'countersign'
    | 'condition_gateway'
    | 'parallel_split'
    | 'parallel_join'
  x: number
  y: number
  properties: LfNodeProps & Record<string, any>
}

export interface LfEdgeProps {
  condition?: { field?: string; op?: string; value?: any } | null
  priority?: number
  label?: string
}

export interface LfEdge {
  id: string
  sourceNodeId: string
  targetNodeId: string
  properties?: LfEdgeProps
}

export interface WorkflowTemplate {
  id: string
  /** 模板代码（英文短名唯一） */
  name: string
  /** 模板展示名 */
  displayName?: string | null
  ticketType: string
  category?: string | null
  scope: TemplateScope
  enabled: boolean
  description?: string | null
  version: number
  definition: LfDefinition
  createdBy?: string | null
  creatorName?: string | null
  createdAt: string
  updatedAt: string
}

/** 列表/卡片通用；与 WorkflowTemplate 共用字段 */
export type WorkflowTemplateSummary = WorkflowTemplate

export interface WorkflowTemplateDetail extends WorkflowTemplate {
  /** 预览时后端 compile 输出：编译后的运行时节点 */
  compiledNodes?: Array<{
    key: string
    name: string
    kind: string
    outs: Array<{ to: string; priority: number; conditionRaw?: string }>
    errors: string[]
  }>
  compileErrors?: string[]
}

export interface TemplateQuery {
  ticketType?: string
  scope?: TemplateScope
  keyword?: string
  enabled?: boolean
}

/** 兼容旧调用：列表接口实际返回数组（后端暂不做分页） */
export interface PageTemplates {
  total: number
  page: number
  pageSize: number
  list: WorkflowTemplate[]
}

export interface CreateTemplateReq {
  name: string
  displayName?: string
  ticketType: string
  category?: string
  description?: string
  definition: LfDefinition
  enabled?: boolean
}

export interface UpdateTemplateReq {
  name?: string
  displayName?: string
  ticketType?: string
  category?: string
  description?: string
  definition?: LfDefinition
  enabled?: boolean
}

export function listTemplates(q: TemplateQuery): Promise<WorkflowTemplate[]> {
  return request.get('/workflow-templates', { params: q })
}
/** 无分页的简易列表，用于下拉框 */
export function listAllTemplates(): Promise<WorkflowTemplate[]> {
  return request.get('/workflow-templates')
}
export async function getTemplate(id: string): Promise<WorkflowTemplateDetail> {
  // 后端返回 {template: WorkflowTemplate, compileErrors: string[]} 包装结构，
  // 这里拍平成 WorkflowTemplateDetail（template 字段提到顶层，保留 compileErrors），
  // 让调用方可以直接访问 detail.definition / detail.name 等字段。
  const raw = await request.get<{ template: WorkflowTemplate; compileErrors?: string[] }>(
    `/workflow-templates/${id}`
  )
  const tmpl = raw?.template ?? (raw as any)
  return { ...tmpl, compileErrors: raw?.compileErrors } as WorkflowTemplateDetail
}
export function createTemplate(
  data: CreateTemplateReq
): Promise<{ id: string }> {
  return request.post('/workflow-templates', data)
}
export function updateTemplate(
  id: string,
  data: UpdateTemplateReq
): Promise<{ version: number }> {
  return request.put(`/workflow-templates/${id}`, data)
}
export function deleteTemplate(id: string): Promise<void> {
  return request.delete(`/workflow-templates/${id}`)
}
/** 启用/禁用模板（后端仅注册 `/enable` 端点，通过 body 的 enabled 字段切换） */
export function enableTemplate(id: string): Promise<void> {
  return request.post(`/workflow-templates/${id}/enable`, { enabled: true })
}
export function disableTemplate(id: string): Promise<void> {
  return request.post(`/workflow-templates/${id}/enable`, { enabled: false })
}
