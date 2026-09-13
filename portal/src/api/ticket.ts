import request from './request'

/* ============================================================
 * 工单后端 API 对接层
 * 后端端口：POST /api/tickets 等
 * request.ts axios 拦截器已解包 {code:0,data:T} → T
 * ============================================================ */

export type TicketType =
  | 'incident'
  | 'problem'
  | 'change'
  | 'change_emergency'
  | 'task'
  | string

export type TicketPriority = 1 | 2 | 3 | 4

export type TicketStatus =
  | 'open'
  | 'assigned'
  | 'in_progress'
  | 'pending_review'
  | 'resolved'
  | 'closed'
  | 'cancelled'
  | string

/** 工单列表项 */
export interface TicketSummary {
  id: string
  ticketNo: string
  ticketType: TicketType
  title: string
  description?: string | null
  priority: TicketPriority
  category?: string | null
  status: TicketStatus
  assigneeId?: string | null
  assigneeName?: string | null
  reporterId: string
  reporterName?: string | null
  currentNodeKey?: string | null
  templateId?: string | null
  slaDueAt?: string | null
  resolution?: string | null
  createdAt: string
  updatedAt: string
  closedAt?: string | null
}

export interface PageTickets {
  total: number
  page: number
  pageSize: number
  list: TicketSummary[]
}

export type NodeKind =
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
  | string

export interface OutEdge {
  to?: string
  priority?: number
  /** 条件表达式（后端会 eval，前端显示可读描述） */
  condition?:
    | null
    | { field?: string; op?: string; value?: any }
    | Record<string, any>
  /** 前端可读描述："P1/P2 分支" 之类 */
  label?: string
}

export interface TicketNode {
  id: string
  nodeKey: string
  nodeName: string
  nodeType: NodeKind
  approvers: Array<{ id?: string; name?: string }>
  outs: OutEdge[]
  status: 'pending' | 'active' | 'done' | 'rejected' | 'skipped' | string
  decision?: 'approve' | 'reject' | 'skip' | string | null
  deciderId?: string | null
  enteredAt?: string | null
  doneAt?: string | null
  timeoutHours?: number | null
  timeoutAction?: string | null
  rejectBackTo?: string | null
  extra?: any
  updatedAt?: string | null
}

export type CommentAction =
  | 'create'
  | 'comment'
  | 'assign'
  | 'approve'
  | 'reject'
  | 'reassign'
  | 'close'
  | 'cancel'
  | 'link_alert'
  | 'unlink_alert'
  | string

export interface TicketComment {
  id: string
  action: CommentAction
  nodeKey?: string | null
  content?: string | null
  extra?: any
  createdAt: string
  userId?: string | null
  userName?: string | null
}

export interface TicketAlertLink {
  alertId: string
  relation?: string | null
  createdAt: string
  alertTitle?: string | null
  alertSeverity?: string | null
}

export interface TicketAttachment {
  id: string
  fileName: string
  fileSize: number
  contentType?: string | null
  uploaderId: string
  uploaderName?: string | null
  createdAt: string
}

export interface TicketDetail {
  ticket: TicketSummary & {
    templateName?: string | null
    templateDefinition?: any
    isWatched?: boolean
    customFields?: Record<string, any> | null
  }
  workflowNodes: TicketNode[]
  comments: TicketComment[]
  alertLinks: TicketAlertLink[]
  attachments: TicketAttachment[]
  sla: { mttaHours: number; mttrHours: number }
}

/* ---------------- 查询参数 ---------------- */

export interface TicketListQuery {
  page?: number
  pageSize?: number
  keyword?: string
  ticketType?: TicketType
  priority?: TicketPriority
  priorityLeq?: TicketPriority
  priorityGeq?: TicketPriority
  status?: TicketStatus
  category?: string
  assigneeId?: string
  reporterId?: string
  templateId?: string
  slaState?: 'ok' | 'warn' | 'breached'
  createdAtFrom?: string
  createdAtTo?: string
}

export interface CreateTicketReq {
  ticketType: TicketType
  title: string
  description?: string
  priority: TicketPriority
  category?: string
  assigneeId?: string
  templateId?: string
  alertIds?: string[]
  extra?: Record<string, any>
}

export interface UpdateTicketReq {
  title?: string
  description?: string
  priority?: TicketPriority
  category?: string
  status?: TicketStatus
  assigneeId?: string
  resolution?: string
}

export interface WorkflowActionReq {
  decision?: 'approve' | 'reject' | 'skip' | string
  comment?: string
  resolution?: string
  toNodeKey?: string
  userId?: string
}

export interface TicketKpis {
  total: number
  open: number
  pendingReview: number
  closed: number
  slaBreached: number
  byPriority: Record<string, number>
  byType: Record<string, number>
}

/** 工单统计看板数据 */
export interface TicketStats {
  total: number
  pending: number
  closed: number
  newThisMonth: number
  avgResolveHours: number
  byType: Array<{ name: string; value: number }>
  byStatus: Array<{ name: string; value: number }>
  trend30d: Array<{ date: string; count: number }>
}

/* ============= 快捷 HTTP 封装 ============= */

export function listTickets(p: TicketListQuery): Promise<PageTickets> {
  return request.get('/tickets', { params: p })
}
export function getTicketKpis(): Promise<TicketKpis> {
  return request.get('/tickets/kpis')
}
export function getTicketDetail(id: string): Promise<TicketDetail> {
  return request.get(`/tickets/${id}`)
}
export function createTicket(
  data: CreateTicketReq
): Promise<{ id: string; ticketNo: string }> {
  return request.post('/tickets', data)
}
export function updateTicket(id: string, data: UpdateTicketReq): Promise<void> {
  return request.put(`/tickets/${id}`, data)
}
export function deleteTicket(id: string): Promise<void> {
  return request.delete(`/tickets/${id}`)
}
export function assignTicket(
  id: string,
  data: { assigneeId: string }
): Promise<void> {
  return request.post(`/tickets/${id}/assign`, data)
}
export function reassignTicket(
  id: string,
  data: { assigneeId: string }
): Promise<void> {
  return request.post(`/tickets/${id}/reassign`, data)
}
export function executeNodeAction(
  id: string,
  nodeKey: string,
  data: WorkflowActionReq
): Promise<{ currentNodeKey?: string | null; status?: string; done?: boolean }> {
  return request.post(`/tickets/${id}/actions/${nodeKey}`, data)
}
export function addComment(
  id: string,
  data: { action?: CommentAction; content: string; nodeKey?: string; extra?: any }
): Promise<{ id: string }> {
  return request.post(`/tickets/${id}/comments`, data)
}
export function linkAlert(
  id: string,
  data: { alertId: string; relation?: string }
): Promise<void> {
  return request.post(`/tickets/${id}/link-alert`, data)
}
export function unlinkAlert(id: string, alertId: string): Promise<void> {
  return request.delete(`/tickets/${id}/link-alert`, { data: { alertId } })
}
export function closeTicket(
  id: string,
  data?: { resolution?: string; comment?: string }
): Promise<void> {
  return request.post(`/tickets/${id}/close`, data ?? {})
}
export function cancelTicket(
  id: string,
  data?: { comment?: string }
): Promise<void> {
  return request.post(`/tickets/${id}/cancel`, data ?? {})
}

/* ============= 我的待办/已办 ============= */

export interface MyTodoItem extends TicketSummary {
  activeNodeKey?: string | null
  activeNodeName?: string | null
}

export interface MyDoneItem extends TicketSummary {
  doneNodeKey?: string | null
  doneNodeName?: string | null
  doneDecision?: string | null
  doneAt?: string | null
}

export function getMyTodos(pageSize = 5): Promise<MyTodoItem[]> {
  return request.get('/tickets/my-todos', { params: { pageSize } })
}

export function getMyDone(pageSize = 5): Promise<MyDoneItem[]> {
  return request.get('/tickets/my-done', { params: { pageSize } })
}

export interface BatchActionResult {
  ticketId: string
  ok: boolean
  currentNodeKey?: string | null
  done?: boolean
  error?: string
}

/* ============= 附件管理 ============= */

export function listAttachments(ticketId: string): Promise<TicketAttachment[]> {
  return request.get(`/tickets/${ticketId}/attachments`)
}

export function uploadAttachment(ticketId: string, file: File): Promise<TicketAttachment> {
  const formData = new FormData()
  formData.append('file', file)
  return request.post(`/tickets/${ticketId}/attachments`, formData, {
    headers: { 'Content-Type': 'multipart/form-data' },
  })
}

export function deleteAttachment(attachmentId: string): Promise<void> {
  return request.delete(`/tickets/attachments/${attachmentId}`)
}

export function getAttachmentDownloadUrl(attachmentId: string): string {
  return `/api/tickets/attachments/${attachmentId}/download`
}

/* ============= 工单关注 ============= */

export function watchTicket(ticketId: string, watched: boolean): Promise<void> {
  return request.post(`/tickets/${ticketId}/watch`, { watched })
}

/* ============= 统计看板 ============= */

export function getTicketStats(): Promise<TicketStats> {
  return request.get('/tickets/stats/overview')
}

/* ============= 知识推荐 ============= */

export interface KnowledgeSuggestion {
  id: string
  title: string
  summary?: string | null
  category?: string | null
  score?: number
  updatedAt?: string
}

export function getKnowledgeSuggestions(ticketId: string): Promise<KnowledgeSuggestion[]> {
  return request.get(`/tickets/${ticketId}/knowledge-suggestions`)
}

/* ============= 工单导出 ============= */

/** 导出工单 CSV（走 Authorization 头，不把 token 放进 URL） */
export async function exportTicketsCsv(params?: TicketListQuery): Promise<void> {
  const search = new URLSearchParams()
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      if (v !== undefined && v !== null && v !== '') {
        search.append(k, String(v))
      }
    }
  }
  const qs = search.toString()
  const res = await request.get<Blob>(`/tickets/export${qs ? `?${qs}` : ''}`, {
    responseType: 'blob',
  })
  const blob = res instanceof Blob ? res : new Blob([res as unknown as BlobPart], { type: 'text/csv;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `tickets_${new Date().toISOString().slice(0, 10)}.csv`
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
}

/* ============= 自定义字段 ============= */

export function updateCustomFields(ticketId: string, fields: Record<string, any>): Promise<void> {
  return request.put(`/tickets/${ticketId}/custom-fields`, { fields })
}

/* ============= 批量操作（扩展） ============= */

export type BatchDecision = 'approve' | 'reject' | 'assign' | 'close' | 'priority'

export function batchAction(
  ticketIds: string[],
  decision: BatchDecision,
  opts?: {
    comment?: string
    assigneeId?: string
    resolution?: string
    priority?: TicketPriority
  }
): Promise<{ total: number; success: number; results: BatchActionResult[] }> {
  return request.post('/tickets/batch-action', { ticketIds, decision, ...opts })
}