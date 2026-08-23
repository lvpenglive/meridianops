<template>
  <div class="ticket-detail-page" v-loading="loading">
    <div class="page-header">
      <el-button :icon="ArrowLeft" link @click="goBack">返回</el-button>
      <span v-if="detail?.ticket" class="page-header__title">
        {{ detail.ticket.ticketNo }} ｜ {{ detail.ticket.title }}
      </span>
      <el-button
        v-if="detail?.ticket"
        class="watch-btn"
        :type="detail.ticket.isWatched ? 'warning' : 'default'"
        :icon="detail.ticket.isWatched ? StarFilled : Star"
        :loading="watchLoading"
        @click="toggleWatch"
      >
        {{ detail.ticket.isWatched ? '已关注' : '关注' }}
      </el-button>
    </div>

    <template v-if="detail?.ticket">
      <el-card shadow="never" class="info-card">
        <div class="info-header">
          <el-tag :type="TICKET_TYPE_META[detail.ticket.ticketType]?.tag || 'info'" size="large">
            {{ TICKET_TYPE_META[detail.ticket.ticketType]?.label || detail.ticket.ticketType }}
          </el-tag>
          <el-tag :type="PRIORITY_META[detail.ticket.priority]?.tag || 'info'" effect="dark" size="large" style="margin-left:8px">
            P{{ detail.ticket.priority }}
          </el-tag>
          <el-tag :type="STATUS_META[detail.ticket.status]?.tag || 'info'" size="large" style="margin-left:8px">
            {{ STATUS_META[detail.ticket.status]?.label || detail.ticket.status }}
          </el-tag>
          <el-tag v-if="isSlaBreach(detail.ticket)" type="danger" effect="dark" size="large" style="margin-left:8px">SLA 违约</el-tag>
          <el-tag v-else-if="isSlaWarn(detail.ticket)" type="warning" size="large" style="margin-left:8px">SLA 临期</el-tag>
          <span class="info-report">
            报告人：{{ detail.ticket.reporterName || '-' }} ｜ 处理人：{{ detail.ticket.assigneeName || '<未分派>' }}
          </span>
        </div>

        <el-descriptions :column="3" size="small" border style="margin-top:12px">
          <el-descriptions-item label="模板">{{ detail.ticket.templateName || '-' }}</el-descriptions-item>
          <el-descriptions-item label="分类">{{ detail.ticket.category || '-' }}</el-descriptions-item>
          <el-descriptions-item label="当前节点">
            <span v-if="detail.ticket.currentNodeKey && detail.ticket.currentNodeKey !== '__end__'" style="color:#409EFF">
              {{ findNode(detail.workflowNodes, detail.ticket.currentNodeKey)?.nodeName || detail.ticket.currentNodeKey }}
            </span>
            <el-tag v-else size="small" type="success" effect="plain">已结束</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="MTTA / MTTR">{{ detail.sla.mttaHours }}h / {{ detail.sla.mttrHours }}h</el-descriptions-item>
          <el-descriptions-item label="SLA 截止">
            <span :style="{ color: isSlaBreach(detail.ticket) ? '#f56c6c' : isSlaWarn(detail.ticket) ? '#e6a23c' : '' }">
              {{ detail.ticket.slaDueAt || '-' }}
            </span>
          </el-descriptions-item>
          <el-descriptions-item label="创建 / 关闭">
            {{ detail.ticket.createdAt }}<br />{{ detail.ticket.closedAt || '未关闭' }}
          </el-descriptions-item>
        </el-descriptions>

        <div v-if="detail.ticket.description" class="text-box">
          <div class="text-box__title">问题描述</div>
          <pre class="text-box__body">{{ detail.ticket.description }}</pre>
        </div>
        <div v-if="detail.ticket.resolution" class="text-box">
          <div class="text-box__title">解决方案</div>
          <pre class="text-box__body">{{ detail.ticket.resolution }}</pre>
        </div>

        <!-- 自定义字段 -->
        <div v-if="hasCustomFields" class="custom-fields-section">
          <div class="custom-fields-header">
            <div class="text-box__title">自定义字段</div>
            <el-button
              v-if="canEditCustomFields"
              link
              size="small"
              type="primary"
              :icon="customFieldsEditing ? Close : EditPen"
              @click="toggleCustomFieldsEdit"
            >
              {{ customFieldsEditing ? '取消' : '编辑' }}
            </el-button>
          </div>
          <el-descriptions :column="2" size="small" border v-if="!customFieldsEditing">
            <el-descriptions-item
              v-for="f in customFieldList"
              :key="f.key"
              :label="f.label"
            >
              {{ formatCustomFieldValue(f.value) }}
            </el-descriptions-item>
          </el-descriptions>
          <el-form v-else :model="customFieldsForm" label-width="120px" size="default" class="custom-fields-form">
            <el-form-item
              v-for="f in customFieldList"
              :key="f.key"
              :label="f.label"
            >
              <el-input
                v-if="f.type === 'text' || !f.type"
                v-model="customFieldsForm[f.key]"
                style="width:100%"
              />
              <el-input
                v-else-if="f.type === 'textarea'"
                v-model="customFieldsForm[f.key]"
                type="textarea"
                :rows="2"
                style="width:100%"
              />
              <el-select
                v-else-if="f.type === 'select' && f.options?.length"
                v-model="customFieldsForm[f.key]"
                style="width:100%"
              >
                <el-option
                  v-for="opt in f.options"
                  :key="opt"
                  :label="opt"
                  :value="opt"
                />
              </el-select>
              <el-switch
                v-else-if="f.type === 'boolean'"
                v-model="customFieldsForm[f.key]"
              />
              <el-input-number
                v-else-if="f.type === 'number'"
                v-model="customFieldsForm[f.key]"
                style="width:200px"
              />
              <el-input v-else v-model="customFieldsForm[f.key]" style="width:100%" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="customFieldsSaving" :icon="Check" @click="saveCustomFields">
                保存
              </el-button>
              <el-button @click="cancelCustomFieldsEdit">取消</el-button>
            </el-form-item>
          </el-form>
        </div>
      </el-card>

      <el-card shadow="never" class="content-card">
        <el-tabs v-model="activeTab">
          <!-- 流程视图 -->
          <el-tab-pane label="流程视图" name="flow">
            <div class="flow-wrap">
              <el-steps direction="vertical" :active="activeStepIndex(detail.workflowNodes)" finish-status="success">
                <el-step
                  v-for="n in displayNodes(detail.workflowNodes)"
                  :key="n.id"
                  :status="stepStatus(n)"
                  :title="n.nodeName"
                  :description="stepDesc(n)"
                >
                  <div class="step-inline">
                    <div class="step-meta">
                      <el-tag size="small" type="info" effect="plain">{{ nodeKindLabel(n.nodeType) }}</el-tag>
                      <el-tag v-if="n.approvers?.length" size="small" type="primary" effect="plain" style="margin-left:6px">
                        审批人: {{ fmtApprovers(n.approvers) }}
                      </el-tag>
                      <span v-if="n.timeoutHours" style="margin-left:8px; color:#909399; font-size:12px">
                        ⏱ {{ n.timeoutHours }}h · {{ n.timeoutAction || '无动作' }}
                      </span>
                    </div>
                    <div v-if="isActable(n) && activeNode?.nodeKey === n.nodeKey" class="action-box">
                      <div class="action-box__title">
                        <b>节点动作：{{ n.nodeName }}</b>
                        <el-tag size="small" type="warning" effect="plain" style="margin-left:6px">当前激活</el-tag>
                      </div>
                      <el-form :model="actionForm" label-width="80px" size="default" inline>
                        <el-form-item label="决策">
                          <el-radio-group v-model="actionForm.decision">
                            <el-radio value="approve">通过</el-radio>
                            <el-radio value="reject">驳回</el-radio>
                            <el-radio value="skip">跳过</el-radio>
                          </el-radio-group>
                        </el-form-item>
                        <el-form-item v-if="actionForm.decision === 'reject' && n.rejectBackTo" label="驳回至">
                          <el-select v-model="actionForm.toNodeKey" style="width:220px">
                            <el-option :label="'默认 → ' + (findNode(detail.workflowNodes, n.rejectBackTo)?.nodeName || n.rejectBackTo)" :value="n.rejectBackTo" />
                            <el-option-group label="回退到前序节点">
                              <el-option v-for="pv in previousKeys(n)" :key="pv"
                                :label="findNode(detail.workflowNodes, pv)?.nodeName || pv" :value="pv" />
                            </el-option-group>
                          </el-select>
                        </el-form-item>
                        <el-form-item label="处理说明" style="width:100%">
                          <el-input v-model="actionForm.comment" type="textarea" :rows="2" placeholder="请填写处理备注…" />
                        </el-form-item>
                        <el-form-item v-if="isResolveNode(n)" label="解决方案" style="width:100%">
                          <el-input v-model="actionForm.resolution" type="textarea" :rows="2" placeholder="填入最终解决方案" />
                        </el-form-item>
                        <el-form-item>
                          <el-button type="primary" :loading="actionLoading" :icon="Check" @click="submitAction(n)">提交决策</el-button>
                          <el-button :icon="SwitchButton" @click="openReassignDialog(n)" :disabled="actionLoading">转派</el-button>
                          <el-button @click="resetActionForm">清空</el-button>
                        </el-form-item>
                      </el-form>
                    </div>
                  </div>
                </el-step>
              </el-steps>
            </div>
          </el-tab-pane>

          <!-- 评论 / 审计 -->
          <el-tab-pane label="评论 / 审计" name="comments">
            <div class="comment-input">
              <div class="comment-toolbar">
                <el-radio-group v-model="commentMode" size="small">
                  <el-radio-button value="edit">编辑</el-radio-button>
                  <el-radio-button value="preview">预览</el-radio-button>
                </el-radio-group>
                <span class="markdown-tip">支持 **加粗**、*斜体*、`代码`、> 引用</span>
              </div>
              <div v-if="commentMode === 'edit'">
                <el-input v-model="commentText" type="textarea" :rows="4" placeholder="写下评论或备注…（支持简单 Markdown）" />
              </div>
              <div v-else class="comment-preview" v-html="renderMarkdown(commentText)"></div>
              <div style="margin-top:8px">
                <el-button :disabled="!commentText.trim()" type="primary" :loading="commentLoading"
                  :icon="ChatDotRound" @click="submitComment">发布评论</el-button>
              </div>
            </div>
            <el-timeline class="comment-timeline">
              <el-timeline-item
                v-for="c in detail.comments"
                :key="c.id"
                :type="timelineType(c.action)"
                :timestamp="c.createdAt"
                :hollow="false"
                :icon="timelineIcon(c.action)"
              >
                <div class="comment-item">
                  <div class="comment-item__head">
                    <b>{{ c.userName || '系统' }}</b>
                    <el-tag size="small" :type="timelineTagType(c.action)" effect="light">{{ actionLabel(c.action) }}</el-tag>
                    <el-tag v-if="c.nodeKey" size="small" type="primary" effect="plain" style="margin-left:6px">
                      节点: {{ findNode(detail.workflowNodes, c.nodeKey)?.nodeName || c.nodeKey }}
                    </el-tag>
                  </div>
                  <div v-if="c.content" class="comment-item__body markdown-body" v-html="renderMarkdown(c.content)"></div>
                </div>
              </el-timeline-item>
              <div v-if="detail.comments.length === 0" style="padding:16px; color:#909399">暂无评论 / 审计记录</div>
            </el-timeline>
          </el-tab-pane>

          <!-- 关联告警 -->
          <el-tab-pane label="关联告警" name="alerts">
            <el-table :data="detail.alertLinks" stripe>
              <el-table-column label="级别" width="90">
                <template #default="{ row }">
                  <el-tag :type="severityTagType(row.alertSeverity)" size="small">{{ row.alertSeverity || '-' }}</el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="alertTitle" label="告警标题" min-width="300" show-overflow-tooltip />
              <el-table-column prop="relation" label="关系" width="110" />
              <el-table-column prop="createdAt" label="关联时间" width="160" />
            </el-table>
            <el-empty v-if="detail.alertLinks.length === 0" description="暂无关联告警" />
          </el-tab-pane>

          <!-- 相关知识 -->
          <el-tab-pane label="相关知识" name="knowledge">
            <div class="knowledge-header">
              <span class="knowledge-tip">基于工单内容智能推荐的知识库文章</span>
              <el-button size="small" :icon="Refresh" :loading="knowledgeLoading" @click="loadKnowledge">刷新推荐</el-button>
            </div>
            <div v-loading="knowledgeLoading" class="knowledge-list">
              <div
                v-for="k in knowledgeList"
                :key="k.id"
                class="knowledge-item"
                @click="openKnowledge(k.id)"
              >
                <div class="knowledge-item__title">
                  <el-icon style="margin-right:6px; color:#409EFF"><Collection /></el-icon>
                  <span>{{ k.title }}</span>
                  <el-tag v-if="k.score" size="small" type="success" effect="plain" style="margin-left:8px">
                    匹配度 {{ Math.round(k.score * 100) }}%
                  </el-tag>
                </div>
                <div v-if="k.summary" class="knowledge-item__summary">{{ k.summary }}</div>
                <div class="knowledge-item__meta">
                  <el-tag v-if="k.category" size="small" type="info" effect="plain">{{ k.category }}</el-tag>
                  <span v-if="k.updatedAt" class="knowledge-item__time">更新于 {{ k.updatedAt }}</span>
                </div>
              </div>
              <el-empty v-if="knowledgeList.length === 0 && !knowledgeLoading" description="暂无相关知识推荐" />
            </div>
          </el-tab-pane>

          <!-- 附件 -->
          <el-tab-pane label="附件" name="attachments">
            <div class="attachment-upload">
              <el-upload
                drag
                :action="uploadUrl"
                :headers="uploadHeaders"
                :show-file-list="false"
                :before-upload="beforeUpload"
                :on-success="onUploadSuccess"
                :on-error="onUploadError"
                name="file"
                multiple
              >
                <el-icon class="el-icon--upload"><UploadFilled /></el-icon>
                <div class="el-upload__text">
                  将文件拖到此处，或<em>点击上传</em>
                </div>
                <template #tip>
                  <div class="el-upload__tip">
                    支持任意格式文件，单文件不超过 50MB
                  </div>
                </template>
              </el-upload>
            </div>
            <el-table :data="detail.attachments" stripe style="margin-top:16px" v-loading="attachmentLoading">
              <el-table-column prop="fileName" label="文件名" min-width="260" show-overflow-tooltip>
                <template #default="{ row }">
                  <el-icon style="margin-right:6px; vertical-align:middle"><Paperclip /></el-icon>
                  <span>{{ row.fileName }}</span>
                </template>
              </el-table-column>
              <el-table-column label="大小" width="100">
                <template #default="{ row }">{{ formatFileSize(row.fileSize) }}</template>
              </el-table-column>
              <el-table-column prop="uploaderName" label="上传人" width="120" show-overflow-tooltip>
                <template #default="{ row }">{{ row.uploaderName || '-' }}</template>
              </el-table-column>
              <el-table-column prop="createdAt" label="上传时间" width="160" />
              <el-table-column label="操作" width="160" align="center">
                <template #default="{ row }">
                  <el-button link size="small" type="primary" :icon="Download" @click="downloadAttachment(row)">下载</el-button>
                  <el-button
                    v-if="canDeleteAttachment(row)"
                    link size="small" type="danger" :icon="Delete"
                    @click="deleteAttachmentItem(row)"
                  >删除</el-button>
                </template>
              </el-table-column>
            </el-table>
            <el-empty v-if="detail.attachments.length === 0 && !attachmentLoading" description="暂无附件" />
          </el-tab-pane>
        </el-tabs>
      </el-card>
    </template>

    <el-empty v-else-if="!loading" description="工单不存在或无权访问" />

    <!-- 转派对话框 -->
    <el-dialog v-model="reassignDialogVisible" title="工单转派" width="480px" :close-on-click-modal="false">
      <el-form :model="reassignForm" label-width="80px">
        <el-form-item label="目标用户">
          <el-select v-model="reassignForm.userId" filterable placeholder="请选择要转派给的用户" style="width:100%">
            <el-option v-for="u in reassignUsers" :key="u.id" :label="u.displayName || u.username" :value="u.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="转派原因">
          <el-input v-model="reassignForm.reason" type="textarea" :rows="3" placeholder="请填写转派原因…" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="reassignDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="reassignLoading" :icon="Check" @click="submitReassign">确认转派</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  ArrowLeft, Check, ChatDotRound, Star, StarFilled,
  SwitchButton, UploadFilled, Paperclip, Download, Delete,
  Refresh, Collection, EditPen, Close, Plus, User,
  CircleCheck, CircleClose, SuccessFilled, Warning, Link, Memo,
} from '@element-plus/icons-vue'
import {
  getTicketDetail, executeNodeAction, addComment,
  uploadAttachment, deleteAttachment, getAttachmentDownloadUrl,
  watchTicket, getKnowledgeSuggestions, updateCustomFields,
  type TicketDetail, type TicketNode, type WorkflowActionReq, type CommentAction,
  type TicketAttachment, type KnowledgeSuggestion,
} from '../../api/ticket'
import { useUserStore } from '../../stores/user'
import { useTicketDicts, labelOf } from '../../composables/useTicketDicts'

const route = useRoute()
const router = useRouter()
const userStore = useUserStore()
const dicts = useTicketDicts()

const loading = ref(false)
const detail = ref<TicketDetail | null>(null)
const activeTab = ref<'flow' | 'comments' | 'alerts' | 'knowledge' | 'attachments'>('flow')

// ---- 关注 ----
const watchLoading = ref(false)

// ---- 相关知识 ----
const knowledgeList = ref<KnowledgeSuggestion[]>([])
const knowledgeLoading = ref(false)

// ---- 自定义字段 ----
const customFieldsEditing = ref(false)
const customFieldsSaving = ref(false)
const customFieldsForm = reactive<Record<string, any>>({})

interface CustomFieldDef {
  key: string
  label: string
  value: any
  type?: 'text' | 'textarea' | 'select' | 'boolean' | 'number' | string
  options?: string[]
}

const hasCustomFields = computed(() => {
  const cf = detail.value?.ticket?.customFields
  return cf && Object.keys(cf).length > 0
})

const canEditCustomFields = computed(() => {
  return userStore.hasPermission('ticket:update') && detail.value?.ticket?.status !== 'closed' && detail.value?.ticket?.status !== 'cancelled'
})

const customFieldList = computed<CustomFieldDef[]>(() => {
  const cf = detail.value?.ticket?.customFields
  if (!cf) return []
  return Object.entries(cf).map(([key, val]) => {
    // 约定：如果值是对象形式 { label, value, type, options }，则解析；否则视为简单值
    if (val && typeof val === 'object' && 'value' in val) {
      return {
        key,
        label: val.label || key,
        value: val.value,
        type: val.type || 'text',
        options: val.options,
      }
    }
    return { key, label: key, value: val, type: 'text' }
  })
})

function formatCustomFieldValue(v: any): string {
  if (v === null || v === undefined || v === '') return '-'
  if (typeof v === 'boolean') return v ? '是' : '否'
  return String(v)
}

function toggleCustomFieldsEdit() {
  if (customFieldsEditing.value) {
    cancelCustomFieldsEdit()
  } else {
    // 填充表单
    const cf = detail.value?.ticket?.customFields || {}
    for (const [k, v] of Object.entries(cf)) {
      if (v && typeof v === 'object' && 'value' in v) {
        customFieldsForm[k] = (v as any).value
      } else {
        customFieldsForm[k] = v
      }
    }
    customFieldsEditing.value = true
  }
}

function cancelCustomFieldsEdit() {
  customFieldsEditing.value = false
  for (const k of Object.keys(customFieldsForm)) {
    delete customFieldsForm[k]
  }
}

async function saveCustomFields() {
  if (!detail.value) return
  customFieldsSaving.value = true
  try {
    await updateCustomFields(detail.value.ticket.id, { ...customFieldsForm })
    ElMessage.success('自定义字段已保存')
    customFieldsEditing.value = false
    // 刷新详情
    await loadDetail(detail.value.ticket.id)
  } catch (e) { ElMessage.error(errMsg(e)) } finally { customFieldsSaving.value = false }
}

// ---- 评论 Markdown ----
const commentMode = ref<'edit' | 'preview'>('edit')

// ---- 转派 ----
const reassignDialogVisible = ref(false)
const reassignLoading = ref(false)
const reassignNodeKey = ref('')
const reassignUsers = ref<Array<{ id: string; username: string; displayName?: string | null }>>([])
const reassignForm = reactive<{ userId: string; reason: string }>({ userId: '', reason: '' })

// ---- 附件 ----
const attachmentLoading = ref(false)

const TICKET_TYPE_META = computed(() => {
  const m: Record<string, { label: string; tag: string }> = {}
  for (const t of dicts.ticketTypes.value) m[t.value] = { label: t.label, tag: (t as any).tag || 'info' }
  return m
})
const PRIORITY_META: Record<number, { label: string; tag: any; warn: number }> = {
  1: { label: 'P1 紧急', tag: 'danger', warn: 1 },
  2: { label: 'P2 高', tag: 'warning', warn: 2 },
  3: { label: 'P3 中', tag: 'primary', warn: 8 },
  4: { label: 'P4 低', tag: 'info', warn: 24 },
}
const STATUS_TAGS: Record<string, string> = {
  open: 'info', assigned: '', in_progress: 'warning',
  pending_review: 'primary', resolved: 'success', closed: 'success', cancelled: 'danger',
}
const STATUS_META = computed(() => {
  const m: Record<string, { label: string; tag: any }> = {}
  for (const s of dicts.statuses.value) m[s.value] = { label: s.label, tag: STATUS_TAGS[s.value] ?? 'info' }
  return m
})

const actionLoading = ref(false)
const actionForm = reactive<WorkflowActionReq>({ decision: 'approve', comment: '' })
function resetActionForm() {
  actionForm.decision = 'approve'
  actionForm.comment = ''
  actionForm.resolution = undefined
  actionForm.toNodeKey = undefined
}

const commentLoading = ref(false)
const commentText = ref('')

const activeNode = computed<TicketNode | null>(() => {
  if (!detail.value) return null
  return findNode(detail.value.workflowNodes, detail.value.ticket.currentNodeKey) || null
})

function goBack() {
  if (window.history.length > 1) router.back()
  else router.push('/tickets')
}

function displayNodes(nodes: TicketNode[]) {
  return (nodes || []).filter(n => n.nodeKey !== '__start__' && n.nodeKey !== '__end__')
}
function findNode(nodes: TicketNode[], key?: string | null) {
  return (nodes || []).find(n => n.nodeKey === key)
}
function stepStatus(n: TicketNode): any {
  switch (n.status) {
    case 'done': return 'success'
    case 'active': return 'process'
    case 'rejected': return 'error'
    case 'skipped': return 'wait'
    default: return 'wait'
  }
}
function stepDesc(n: TicketNode) {
  const who = fmtApprovers(n.approvers)
  const when = n.enteredAt ? `进入：${n.enteredAt.replace('T', ' ').slice(0, 19)}` : '未进入'
  const done = n.doneAt ? `｜完成：${n.doneAt.replace('T', ' ').slice(0, 19)}` : ''
  const dec = n.decision ? `｜决策：${n.decision}` : ''
  return `审批人：${who}  ｜  ${when}${done}${dec}`
}
function activeStepIndex(nodes: TicketNode[]) {
  const list = displayNodes(nodes)
  const idx = list.findIndex(n => n.status === 'active' || n.status === 'pending')
  return idx < 0 ? list.length : idx + 1
}
function nodeKindLabel(k: string) { return labelOf(dicts.nodeKinds.value, k) }
function fmtApprovers(list?: any[] | null): string {
  if (!Array.isArray(list)) return '-'
  return list.map(a => String(a?.name || a?.id || '')).filter(Boolean).join(', ') || '-'
}
function isActable(n: TicketNode) {
  if (n.status !== 'active') return false
  return !['start', 'end', 'auto_pass', 'condition_gateway', 'parallel_split', 'parallel_join'].includes(n.nodeType)
}
function previousKeys(n: TicketNode) {
  if (!detail.value) return []
  return detail.value.workflowNodes
    .filter(x => (x.outs || []).some(o => o.to === n.nodeKey))
    .map(x => x.nodeKey)
    .filter(k => k !== '__start__' && k !== '__end__' && k !== n.nodeKey)
}
function isResolveNode(n: TicketNode) {
  return n.nodeKey === 'resolve' || n.nodeKey === 'verify' || n.nodeKey === 'closure' || n.nodeName.includes('解决')
}
function isSlaBreach(t?: { slaDueAt?: string | null; status?: string } | null) {
  if (!t || !t.slaDueAt) return false
  if (['closed', 'cancelled', 'resolved'].includes(t.status || '')) return false
  return new Date(t.slaDueAt).getTime() < Date.now()
}
function isSlaWarn(t: { slaDueAt?: string | null; status?: string; priority?: number }) {
  if (!t.slaDueAt) return false
  if (['closed', 'cancelled'].includes(t.status || '')) return false
  const left = (new Date(t.slaDueAt).getTime() - Date.now()) / 3600000
  const warn = PRIORITY_META[t.priority || 3]?.warn ?? 24
  return left >= 0 && left < warn
}
function actionLabel(a: CommentAction | string) {
  const m: Record<string, string> = {
    create: '创建工单', comment: '评论', assign: '分派', approve: '审批通过', reject: '审批驳回',
    reassign: '改派', close: '关闭工单', cancel: '取消工单', link_alert: '关联告警', unlink_alert: '解除关联',
  }
  return m[a] || a
}
function timelineType(a: CommentAction | string) {
  switch (a) {
    case 'approve': case 'close': return 'success'
    case 'reject': case 'cancel': return 'danger'
    case 'assign': case 'reassign': return 'warning'
    default: return 'primary'
  }
}
function timelineTagType(a: CommentAction | string) {
  switch (a) {
    case 'create': return 'primary'
    case 'comment': return 'info'
    case 'assign': case 'reassign': return 'warning'
    case 'approve': return 'success'
    case 'reject': case 'cancel': return 'danger'
    case 'close': return 'success'
    case 'link_alert': case 'unlink_alert': return ''
    default: return 'info'
  }
}
function timelineIcon(a: CommentAction | string): any {
  const icons: Record<string, any> = {
    create: Plus,
    comment: ChatDotRound,
    assign: User,
    reassign: SwitchButton,
    approve: CircleCheck,
    reject: CircleClose,
    close: SuccessFilled,
    cancel: Warning,
    link_alert: Link,
    unlink_alert: Close,
  }
  return icons[a] || Memo
}
function severityTagType(s?: string | null) {
  const m: Record<string, any> = { critical: 'danger', high: 'danger', medium: 'warning', low: 'info', info: 'info' }
  return m[s || ''] || 'info'
}
function errMsg(e: unknown): string {
  return ((e as any)?.message || String(e || '请求失败')).slice(0, 300)
}

async function loadDetail(id: string) {
  try {
    loading.value = true
    detail.value = await getTicketDetail(id)
    // 兼容后端可能未返回 attachments 字段
    if (!detail.value.attachments) detail.value.attachments = []
    resetActionForm()
  } catch (e) { ElMessage.error(errMsg(e)) } finally { loading.value = false }
}

async function submitAction(n: TicketNode) {
  if (!detail.value) return
  actionLoading.value = true
  try {
    const ticketId = detail.value.ticket.id
    const res = await executeNodeAction(ticketId, n.nodeKey, { ...actionForm })
    ElMessage.success(res.done ? '流程已全部走完，工单关闭' : `动作提交成功，当前节点: ${res.currentNodeKey || '-'}`)
    await loadDetail(ticketId)
  } catch (e) { ElMessage.error(errMsg(e)) } finally { actionLoading.value = false }
}

async function submitComment() {
  if (!detail.value || !commentText.value.trim()) return
  try {
    commentLoading.value = true
    await addComment(detail.value.ticket.id, { content: commentText.value.trim() })
    commentText.value = ''
    commentMode.value = 'edit'
    await loadDetail(detail.value.ticket.id)
    ElMessage.success('评论已发布')
  } catch (e) { ElMessage.error(errMsg(e)) } finally { commentLoading.value = false }
}

/* ===================== 关注 ===================== */
async function toggleWatch() {
  if (!detail.value?.ticket) return
  const currentWatched = !!detail.value.ticket.isWatched
  const newWatched = !currentWatched
  try {
    watchLoading.value = true
    await watchTicket(detail.value.ticket.id, newWatched)
    detail.value.ticket.isWatched = newWatched
    ElMessage.success(newWatched ? '已关注该工单' : '已取消关注')
  } catch (e) { ElMessage.error(errMsg(e)) } finally { watchLoading.value = false }
}

/* ===================== Markdown 渲染 ===================== */
function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

function renderMarkdown(text?: string | null): string {
  if (!text) return ''
  let html = escapeHtml(text)
  // 代码块 `code`
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>')
  // 加粗 **bold**
  html = html.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>')
  // 斜体 *italic*
  html = html.replace(/(^|[^*])\*([^*\n]+)\*/g, '$1<em>$2</em>')
  // 引用 > quote
  const lines = html.split('\n')
  const result: string[] = []
  let inQuote = false
  for (const line of lines) {
    if (line.startsWith('&gt; ') || line.startsWith('&gt;')) {
      if (!inQuote) {
        result.push('<blockquote>')
        inQuote = true
      }
      result.push(line.replace(/^&gt;\s?/, ''))
    } else {
      if (inQuote) {
        result.push('</blockquote>')
        inQuote = false
      }
      result.push(line)
    }
  }
  if (inQuote) result.push('</blockquote>')
  html = result.join('\n')
  // 换行
  html = html.replace(/\n/g, '<br>')
  return html
}

/* ===================== 转派 ===================== */
async function loadReassignUsers() {
  if (reassignUsers.value.length > 0) return
  try {
    const res = await fetch('/api/users', {
      headers: { Authorization: `Bearer ${localStorage.getItem('meridianops_token')}` }
    })
    const data = await res.json()
    const arr = (data.data || data) as any[]
    reassignUsers.value = (arr || []).map((u: any) => ({
      id: u.id, username: u.username, displayName: u.displayName || u.username,
    }))
  } catch { reassignUsers.value = [] }
}

function openReassignDialog(n: TicketNode) {
  reassignNodeKey.value = n.nodeKey
  reassignForm.userId = ''
  reassignForm.reason = ''
  void loadReassignUsers()
  reassignDialogVisible.value = true
}

async function submitReassign() {
  if (!detail.value || !reassignForm.userId) {
    ElMessage.warning('请选择目标用户')
    return
  }
  reassignLoading.value = true
  try {
    const ticketId = detail.value.ticket.id
    await executeNodeAction(ticketId, reassignNodeKey.value, {
      decision: 'reassign',
      userId: reassignForm.userId,
      comment: reassignForm.reason,
    })
    ElMessage.success('转派成功')
    reassignDialogVisible.value = false
    await loadDetail(ticketId)
  } catch (e) { ElMessage.error(errMsg(e)) } finally { reassignLoading.value = false }
}

/* ===================== 附件 ===================== */
const uploadUrl = computed(() => {
  return detail.value ? `/api/tickets/${detail.value.ticket.id}/attachments` : ''
})

const uploadHeaders = computed(() => ({
  Authorization: `Bearer ${localStorage.getItem('meridianops_token')}`
}))

function beforeUpload(file: File) {
  const isLt50M = file.size / 1024 / 1024 < 50
  if (!isLt50M) {
    ElMessage.error('文件大小不能超过 50MB')
    return false
  }
  return true
}

function onUploadSuccess(response: any) {
  // 后端统一响应格式 { code:0, data: attachment }
  const attachment = response?.data || response
  if (detail.value) {
    detail.value.attachments = [...detail.value.attachments, attachment]
  }
  ElMessage.success('上传成功')
}

function onUploadError(err: any) {
  ElMessage.error(err?.message || '上传失败')
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return bytes + ' B'
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB'
  if (bytes < 1024 * 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + ' MB'
  return (bytes / 1024 / 1024 / 1024).toFixed(2) + ' GB'
}

function downloadAttachment(att: TicketAttachment) {
  const url = getAttachmentDownloadUrl(att.id)
  const token = localStorage.getItem('meridianops_token')
  // 用 fetch 下载，带 token
  fetch(url, { headers: { Authorization: `Bearer ${token}` } })
    .then(res => {
      if (!res.ok) throw new Error('下载失败')
      return res.blob()
    })
    .then(blob => {
      const a = document.createElement('a')
      a.href = URL.createObjectURL(blob)
      a.download = att.fileName
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(a.href)
    })
    .catch(e => ElMessage.error(e.message || '下载失败'))
}

function canDeleteAttachment(att: TicketAttachment): boolean {
  if (!detail.value) return false
  // 上传者或管理员可删除
  const currentUserId = userStore.user?.id
  if (att.uploaderId === currentUserId) return true
  if (userStore.hasPermission('ticket:update')) return true
  return false
}

async function deleteAttachmentItem(att: TicketAttachment) {
  if (!detail.value) return
  try {
    await ElMessageBox.confirm(`确认删除附件「${att.fileName}」？此操作不可撤销。`, '删除附件', { type: 'warning' })
    await deleteAttachment(att.id)
    detail.value.attachments = detail.value.attachments.filter(a => a.id !== att.id)
    ElMessage.success('已删除')
  } catch {}
}

/* ===================== 相关知识 ===================== */
async function loadKnowledge() {
  if (!detail.value) return
  try {
    knowledgeLoading.value = true
    knowledgeList.value = await getKnowledgeSuggestions(detail.value.ticket.id)
  } catch (e) { ElMessage.error(errMsg(e)) } finally { knowledgeLoading.value = false }
}

function openKnowledge(id: string) {
  router.push(`/knowledge/${id}`)
}

// 切换到相关知识 tab 时加载数据
watch(activeTab, (tab) => {
  if (tab === 'knowledge' && knowledgeList.value.length === 0 && detail.value) {
    void loadKnowledge()
  }
})

onMounted(async () => {
  await dicts.load()
  const id = route.params.id as string
  if (route.query.action) activeTab.value = 'flow'
  if (id) await loadDetail(id)
})
</script>

<style scoped>
.ticket-detail-page {
  padding: 8px 0 40px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.page-header {
  display: flex;
  align-items: center;
  gap: 12px;
}
.page-header__title {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}
.info-card :deep(.el-card__body) {
  padding: 16px 20px;
}
.info-header {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}
.info-report {
  margin-left: 12px;
  font-size: 13px;
  color: #606266;
}
.text-box {
  margin-top: 14px;
}
.text-box__title {
  font-weight: 600;
  font-size: 14px;
  color: #303133;
  margin-bottom: 6px;
}
.text-box__body {
  background: #f5f7fa;
  padding: 10px 14px;
  border-radius: 6px;
  font-size: 13px;
  color: #606266;
  white-space: pre-wrap;
  word-break: break-word;
  margin: 0;
}
.content-card :deep(.el-card__body) {
  padding: 0 20px 16px;
}
.content-card :deep(.el-tabs__header) {
  margin: 0 0 12px;
}
.flow-wrap {
  padding: 8px 0;
}
.step-inline {
  padding: 8px 0 16px 0;
}
.step-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
  margin: 6px 0;
}
.action-box {
  background: #f0f9ff;
  border: 1px solid #d0e9fd;
  border-radius: 8px;
  padding: 14px 16px;
  margin-top: 8px;
}
.action-box__title {
  margin-bottom: 10px;
}
.comment-input {
  margin-bottom: 16px;
}
.comment-timeline {
  padding: 8px 0 0 4px;
}
.comment-item__head {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
  margin-bottom: 4px;
}
.comment-item__body {
  font-size: 13px;
  color: #606266;
  white-space: pre-wrap;
  word-break: break-word;
}
.comment-item__body.markdown-body {
  white-space: normal;
}
.markdown-body code {
  background: #f5f7fa;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
  color: #e83e8c;
  font-family: 'Consolas', 'Monaco', monospace;
}
.markdown-body strong {
  font-weight: 600;
  color: #303133;
}
.markdown-body em {
  font-style: italic;
  color: #606266;
}
.markdown-body blockquote {
  margin: 8px 0;
  padding: 8px 12px;
  background: #f4f4f5;
  border-left: 4px solid #dcdfe6;
  color: #606266;
  border-radius: 0 4px 4px 0;
}

/* 评论工具栏 */
.comment-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}
.markdown-tip {
  font-size: 12px;
  color: #909399;
}
.comment-preview {
  min-height: 80px;
  padding: 12px;
  background: #fafafa;
  border: 1px solid #dcdfe6;
  border-radius: 4px;
  font-size: 13px;
  color: #303133;
  white-space: pre-wrap;
  word-break: break-word;
}

/* 关注按钮 */
.watch-btn {
  margin-left: 12px;
}

/* 附件上传区 */
.attachment-upload :deep(.el-upload-dragger) {
  padding: 24px;
}
.attachment-upload :deep(.el-icon--upload) {
  font-size: 48px;
  color: #409EFF;
}
.attachment-upload :deep(.el-upload__text) {
  font-size: 14px;
  color: #606266;
  margin-top: 8px;
}
.attachment-upload :deep(.el-upload__text em) {
  color: #409EFF;
  font-style: normal;
}
.attachment-upload :deep(.el-upload__tip) {
  font-size: 12px;
  color: #909399;
  margin-top: 6px;
}

/* 自定义字段 */
.custom-fields-section {
  margin-top: 14px;
}
.custom-fields-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}
.custom-fields-form {
  background: #f5f7fa;
  padding: 12px 16px;
  border-radius: 8px;
}

/* 相关知识 */
.knowledge-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
.knowledge-tip {
  font-size: 13px;
  color: #909399;
}
.knowledge-list {
  min-height: 120px;
}
.knowledge-item {
  padding: 14px 16px;
  border: 1px solid #ebeef5;
  border-radius: 8px;
  margin-bottom: 10px;
  cursor: pointer;
  transition: all 0.2s;
  background: #fff;
}
.knowledge-item:hover {
  border-color: #409EFF;
  box-shadow: 0 2px 8px rgba(64, 158, 255, 0.15);
  transform: translateY(-1px);
}
.knowledge-item__title {
  font-size: 14px;
  font-weight: 500;
  color: #303133;
  display: flex;
  align-items: center;
  margin-bottom: 6px;
}
.knowledge-item__summary {
  font-size: 13px;
  color: #606266;
  line-height: 1.6;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.knowledge-item__meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
}
.knowledge-item__time {
  font-size: 12px;
  color: #c0c4cc;
}
</style>
