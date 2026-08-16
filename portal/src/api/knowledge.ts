import request from './request'

export interface KnowledgeItem {
  id: string
  title: string
  category: string
  tags: string[]
  summary: string | null
  status: string
  viewCount: number
  helpfulCount: number
  version: number
  createdByName: string | null
  createdAt: string
  updatedAt: string
}

export interface KnowledgeDetail extends KnowledgeItem {
  content: string
  createdBy: string
}

export interface KnowledgePage {
  items: KnowledgeItem[]
  total: number
  page: number
  pageSize: number
}

export interface KnowledgeCategory {
  category: string
  count: number
}

export interface KnowledgeTag {
  tag: string
  count: number
}

export interface KnowledgeVersion {
  id: string
  version: number
  title: string
  tags: string[]
  editedByName: string | null
  createdAt: string
}

export interface CreateKnowledgeRequest {
  title: string
  category: string
  tags: string[]
  content: string
  summary?: string
  status?: string
}

export interface UpdateKnowledgeRequest extends CreateKnowledgeRequest {}

export interface KnowledgeQuery {
  page?: number
  page_size?: number
  category?: string
  tag?: string
  status?: string
  q?: string
}

/** 分页查询知识条目 */
export function listKnowledge(params: KnowledgeQuery): Promise<KnowledgePage> {
  return request.get('/knowledge', { params })
}

/** 获取知识详情 */
export function getKnowledge(id: string): Promise<KnowledgeDetail> {
  return request.get(`/knowledge/${id}`)
}

/** 创建知识条目 */
export function createKnowledge(data: CreateKnowledgeRequest): Promise<{ id: string }> {
  return request.post('/knowledge', data)
}

/** 更新知识条目 */
export function updateKnowledge(id: string, data: UpdateKnowledgeRequest): Promise<{ id: string; version: number }> {
  return request.put(`/knowledge/${id}`, data)
}

/** 删除知识条目 */
export function deleteKnowledge(id: string): Promise<void> {
  return request.delete(`/knowledge/${id}`)
}

/** 全文检索 */
export function searchKnowledge(q: string, pageSize?: number): Promise<{ items: KnowledgeItem[]; total: number; query: string }> {
  return request.get('/knowledge/search', { params: { q, page_size: pageSize } })
}

/** 列出所有分类 */
export function listCategories(): Promise<KnowledgeCategory[]> {
  return request.get('/knowledge/categories')
}

/** 列出所有标签 */
export function listTags(): Promise<KnowledgeTag[]> {
  return request.get('/knowledge/tags')
}

/** 查看版本历史 */
export function listVersions(id: string): Promise<KnowledgeVersion[]> {
  return request.get(`/knowledge/${id}/versions`)
}

/** 标记有帮助 */
export function markHelpful(id: string): Promise<{ helpfulCount: number }> {
  return request.post(`/knowledge/${id}/helpful`)
}
