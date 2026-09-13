<template>
  <div class="sms-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><BellFilled /></el-icon>
        <span>通知策略</span>
        <span class="page-sub">按条件匹配后多选渠道发给选定人员；可同时通知资产责任人</span>
      </div>
      <div class="header-actions">
        <el-button
          v-if="hasPermission('sms_strategy:manage')"
          type="primary"
          :icon="Plus"
          @click="openCreate"
        >
          录入
        </el-button>
      </div>
    </div>

    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>策略列表（{{ total }}）</span>
          <el-button :icon="Refresh" size="small" @click="fetchList">刷新</el-button>
        </div>
      </template>

      <!-- 筛选栏 -->
      <div class="filter-bar">
        <el-input
          v-model="filters.keyword"
          placeholder="事件名称 / 描述关键字"
          clearable
          size="default"
          style="width: 220px;"
          @keyup.enter="onSearch"
          @clear="onSearch"
        />
        <el-select
          v-model="filters.eventType"
          placeholder="全部事件类型"
          clearable
          size="default"
          style="width: 160px;"
          @change="onSearch"
        >
          <el-option v-for="o in eventTypeOptions" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <el-select
          v-model="filters.enabled"
          placeholder="全部状态"
          clearable
          size="default"
          style="width: 140px;"
          @change="onSearch"
        >
          <el-option label="启用" :value="true" />
          <el-option label="停用" :value="false" />
        </el-select>
        <el-button type="primary" size="default" :icon="Search" @click="onSearch">查询</el-button>
        <el-button size="default" @click="onReset">重置</el-button>
      </div>

      <el-table :data="list" v-loading="loading" stripe size="default">
        <el-table-column label="事件类型" width="180">
          <template #default="{ row }">
            <el-tag v-if="row.eventType" size="small" type="primary" effect="plain">
              {{ eventTypeLabel(row.eventType) }}
            </el-tag>
            <el-tag v-else size="small" type="info" effect="plain">全部类型</el-tag>
            <el-tag
              v-if="row.eventSubType"
              size="small"
              type="warning"
              effect="plain"
              style="margin-left: 4px;"
            >{{ subTypeLabel(row.eventSubType) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="触发级别" width="100">
          <template #default="{ row }">{{ triggerOpLabel(row.triggerOp) }}</template>
        </el-table-column>
        <el-table-column label="事件级别" width="110">
          <template #default="{ row }">
            <span v-if="row.severityFilter">{{ severityLabel(row.severityFilter) }}</span>
            <span v-else class="text-muted">全部级别</span>
          </template>
        </el-table-column>
        <el-table-column label="设备IP" width="130" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.hostFilter">{{ row.hostFilter }}</span>
            <span v-else class="text-muted">全部设备</span>
          </template>
        </el-table-column>
        <el-table-column label="事件名称" min-width="120" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.nameKeyword">{{ row.nameKeyword }}</span>
            <span v-else class="text-muted">—</span>
          </template>
        </el-table-column>
        <el-table-column label="触发场景" width="110">
          <template #default="{ row }">{{ triggerSceneLabel(row.triggerScene) }}</template>
        </el-table-column>
        <el-table-column label="通知渠道" min-width="200">
          <template #default="{ row }">
            <el-tag
              v-for="k in channelKindsOf(row)"
              :key="k"
              size="small"
              effect="plain"
              style="margin-right: 4px; margin-bottom: 2px;"
            >{{ channelKindLabel(k) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="告警接收组" min-width="220" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.recipientUserIds && row.recipientUserIds.length">
              {{ recipientNames(row.recipientUserIds) }}
            </span>
            <span v-else class="text-muted">—</span>
            <el-tag
              v-if="row.notifyOwner"
              size="small"
              type="warning"
              effect="plain"
              style="margin-left: 6px;"
            >含责任人</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="createdAt" label="创建时间" width="180" show-overflow-tooltip />
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag v-if="row.enabled" size="small" type="success">启用</el-tag>
            <el-tag v-else size="small" type="danger">停用</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="description" label="描述" min-width="120" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.description">{{ row.description }}</span>
            <span v-else class="text-muted">—</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="hasPermission('sms_strategy:manage')"
              size="small" link type="primary"
              @click="openEdit(row)"
            >编辑</el-button>
            <el-button
              v-if="hasPermission('sms_strategy:manage')"
              size="small" link :type="row.enabled ? 'warning' : 'success'"
              @click="onToggle(row)"
            >{{ row.enabled ? '停用' : '启用' }}</el-button>
            <el-button
              v-if="hasPermission('sms_strategy:manage')"
              size="small" link type="danger"
              @click="onDelete(row)"
            >删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <div v-if="!loading && list.length === 0" class="empty-tip">
        <el-empty description="暂无通知策略" />
      </div>

      <div class="pagination-wrap">
        <el-pagination
          v-model:current-page="pagination.page"
          v-model:page-size="pagination.pageSize"
          :total="total"
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          background
          @size-change="onSizeChange"
          @current-change="fetchList"
        />
      </div>
    </el-card>

    <!-- 录入 / 编辑弹窗 -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑策略' : '录入策略'"
      width="780px"
      top="6vh"
      @closed="resetForm"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item label="触发场景" prop="triggerScene">
          <el-select v-model="form.triggerScene" style="width: 100%;">
            <el-option v-for="o in TRIGGER_SCENES" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="事件类型" prop="eventType">
              <el-select v-model="form.eventType" placeholder="全部类型" clearable style="width: 100%;">
                <el-option v-for="o in eventTypeOptions" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="事件子类">
              <el-select
                v-model="form.eventSubType"
                placeholder="全部子类"
                clearable
                style="width: 100%;"
                :disabled="!canPickSubType"
              >
                <el-option v-for="o in subTypeOptions" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="触发级别">
              <el-select v-model="form.triggerOp" style="width: 100%;">
                <el-option v-for="o in SMS_TRIGGER_OPS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="事件级别">
              <el-select v-model="form.severityFilter" placeholder="全部级别" clearable style="width: 100%;">
                <el-option v-for="o in SMS_SEVERITIES" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="设备IP">
              <el-input v-model="form.hostFilter" placeholder="逗号分隔多值，支持 % 通配" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="事件名称">
              <el-input v-model="form.nameKeyword" placeholder="标题包含此关键字才命中" />
            </el-form-item>
          </el-col>
        </el-row>

        <el-form-item label="选择组">
          <el-tree-select
            v-model="form.alertGroupId"
            :data="alertGroupTree"
            :props="{ label: 'name', children: 'children' }"
            value-key="id"
            placeholder="选择父组，自动带出该组及所有子孙组成员（按用户 ID 去重）"
            clearable
            style="width: 100%;"
            @change="onGroupChange"
          />
        </el-form-item>

        <!-- 候选 / 已选人员双列表 -->
        <el-form-item label="接收人员" prop="recipientUserIds">
          <div class="staff-transfer">
            <div class="staff-panel">
              <div class="panel-title">候选人员</div>
              <el-input
                v-model="candidateKeyword"
                placeholder="搜索姓名 / 账号"
                size="small"
                clearable
                style="margin-bottom: 6px;"
              />
              <div class="staff-list">
                <div
                  v-for="u in candidateUsers"
                  :key="u.id"
                  class="staff-item"
                  @dblclick="addUser(u)"
                >
                  <span class="staff-name">{{ u.displayName || u.username }}</span>
                  <span class="staff-dept">{{ deptName(u.departmentId) }}</span>
                </div>
                <div v-if="candidateUsers.length === 0" class="list-empty">无可选人员</div>
              </div>
            </div>

            <div class="transfer-actions">
              <el-button size="small" :icon="ArrowRight" @click="addSelected">添加</el-button>
              <el-button size="small" :icon="ArrowLeft" @click="removeSelected">删除</el-button>
            </div>

            <div class="staff-panel">
              <div class="panel-title">已选人员（{{ form.recipientUserIds.length }}）</div>
              <div class="staff-list">
                <div
                  v-for="u in selectedUsers"
                  :key="u.id"
                  class="staff-item"
                  @dblclick="removeUser(u)"
                >
                  <span class="staff-name">{{ u.displayName || u.username }}</span>
                  <span class="staff-dept">{{ deptName(u.departmentId) }}</span>
                </div>
                <div v-if="selectedUsers.length === 0" class="list-empty">未选择人员</div>
              </div>
            </div>
          </div>
        </el-form-item>

        <el-form-item label="资产责任人">
          <el-switch v-model="form.notifyOwner" @change="onNotifyOwnerChange" />
          <span class="form-hint">同时通知告警 IP 关联资产的负责人（告警中心列表「联系人」）。对不上资产或无手机号则跳过。</span>
        </el-form-item>

        <el-form-item label="通知渠道" prop="channelKinds">
          <el-checkbox-group v-model="form.channelKinds">
            <el-checkbox v-for="o in CHANNEL_KIND_OPTIONS" :key="o.value" :value="o.value">
              {{ o.label }}
            </el-checkbox>
          </el-checkbox-group>
          <div class="form-hint" style="margin-left: 0; margin-top: 4px;">勾选本次要发的渠道。短信走短信平台通道，邮件走 SMTP 通道。</div>
        </el-form-item>

        <el-form-item v-if="showExtraChannels" label="指定通道">
          <el-select
            v-model="form.extraChannelIds"
            multiple
            clearable
            collapse-tags
            collapse-tags-tooltip
            placeholder="不指定则用该类型第一条启用通道"
            style="width: 100%;"
          >
            <el-option-group v-if="feishuChannelOptions.length" label="飞书">
              <el-option
                v-for="c in feishuChannelOptions"
                :key="c.id"
                :label="c.name"
                :value="c.id"
              />
            </el-option-group>
            <el-option-group v-if="webhookChannelOptions.length" label="Webhook">
              <el-option
                v-for="c in webhookChannelOptions"
                :key="c.id"
                :label="c.name"
                :value="c.id"
              />
            </el-option-group>
          </el-select>
        </el-form-item>

        <el-form-item label="描述">
          <el-input v-model="form.description" type="textarea" :rows="2" placeholder="策略说明（可选）" />
        </el-form-item>

        <el-form-item label="是否启用">
          <el-radio-group v-model="form.enabled">
            <el-radio :value="true">启用</el-radio>
            <el-radio :value="false">停用</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">关闭</el-button>
        <el-button type="primary" :loading="saving" @click="onSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Refresh, Search, BellFilled, ArrowRight, ArrowLeft } from '@element-plus/icons-vue'
import {
  listSmsStrategies,
  createSmsStrategy,
  updateSmsStrategy,
  toggleSmsStrategy,
  deleteSmsStrategy,
  listSmsEventTypes,
  listSmsSubTypes,
  listNotifyChannelOptions,
  SMS_TRIGGER_OPS,
  SMS_SEVERITIES,
  CHANNEL_KIND_OPTIONS,
  DEFAULT_CHANNEL_KINDS,
  type SmsStrategy,
  type NotifyChannelKind,
  type NotifyChannelOption,
} from '../../api/smsStrategy'
import { TRIGGER_SCENES } from '../../api/notification'
import type { DictItem } from '../../api/dict'
import { listUsers } from '../../api/users'
import { listDepartments } from '../../api/departments'
import {
  getAlertGroupTree,
  listAlertGroupMembers,
  type AlertGroupTreeNode,
} from '../../api/alertGroup'
import type { UserInfo, Department } from '../../api/types'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()
const hasPermission = (code: string) => userStore.hasPermission(code)

const loading = ref(false)
const list = ref<SmsStrategy[]>([])
const total = ref(0)
const eventTypeOptions = ref<DictItem[]>([])
const subTypeOptions = ref<DictItem[]>([])
const allUsers = ref<UserInfo[]>([])
const departments = ref<Department[]>([])
const alertGroupTree = ref<AlertGroupTreeNode[]>([])
const channelOptions = ref<NotifyChannelOption[]>([])

const filters = reactive({
  keyword: '',
  eventType: '' as string,
  enabled: null as boolean | null,
})
const pagination = reactive({ page: 1, pageSize: 20 })

function buildParams() {
  const params: Record<string, any> = { page: pagination.page, pageSize: pagination.pageSize }
  if (filters.keyword.trim()) params.keyword = filters.keyword.trim()
  if (filters.eventType) params.eventType = filters.eventType
  if (filters.enabled !== null) params.enabled = filters.enabled
  return params
}

async function fetchList() {
  loading.value = true
  try {
    const res = await listSmsStrategies(buildParams())
    list.value = res.list
    total.value = res.total
  } catch (e: any) {
    ElMessage.error(e?.message || '加载策略失败')
  } finally {
    loading.value = false
  }
}

function onSearch() { pagination.page = 1; fetchList() }
function onReset() {
  filters.keyword = ''
  filters.eventType = ''
  filters.enabled = null
  pagination.page = 1
  fetchList()
}
function onSizeChange(size: number) { pagination.pageSize = size; pagination.page = 1; fetchList() }

async function fetchMeta() {
  try { eventTypeOptions.value = await listSmsEventTypes() } catch { /* 忽略 */ }
  try { subTypeOptions.value = await listSmsSubTypes() } catch { /* 忽略 */ }
  try { allUsers.value = await listUsers() } catch { /* 忽略，选人依赖此数据 */ }
  try { departments.value = await listDepartments() } catch { /* 忽略（候选行展示仍用部门名） */ }
  try { alertGroupTree.value = await getAlertGroupTree() } catch { /* 忽略 */ }
  try {
    const r = await listNotifyChannelOptions()
    channelOptions.value = r.list || []
  } catch { /* 忽略 */ }
}

function eventTypeLabel(t: string) {
  const o = eventTypeOptions.value.find((x) => x.value === t)
  return o ? o.label : t
}
function subTypeLabel(t: string) {
  const o = subTypeOptions.value.find((x) => x.value === t)
  return o ? o.label : t
}
function triggerOpLabel(t: string) {
  const o = SMS_TRIGGER_OPS.find((x) => x.value === t)
  return o ? o.label : (t || '等于')
}
function severityLabel(s: string) {
  const o = SMS_SEVERITIES.find((x) => x.value === s)
  return o ? o.label : s
}
function deptName(id?: string | null) {
  if (!id) return ''
  const d = departments.value.find((x) => x.id === id)
  return d ? d.name : ''
}
function triggerSceneLabel(t?: string) {
  const o = TRIGGER_SCENES.find((x) => x.value === (t || 'alert_firing'))
  return o ? o.label : (t || '告警触发')
}
function channelKindsOf(row: SmsStrategy): NotifyChannelKind[] {
  return row.channelKinds?.length ? row.channelKinds : DEFAULT_CHANNEL_KINDS
}
function channelKindLabel(k: string) {
  return CHANNEL_KIND_OPTIONS.find((x) => x.value === k)?.label || k
}
function recipientNames(ids: string[]) {
  const names = ids
    .map((id) => {
      const u = allUsers.value.find((x) => x.id === id)
      return u ? (u.displayName || u.username) : id.slice(0, 8)
    })
    .filter(Boolean)
  return names.join('、')
}

// ---- 选人双列表 ----
const dialogVisible = ref(false)
const isEdit = ref(false)
const saving = ref(false)
const formRef = ref<FormInstance>()
const candidateKeyword = ref('')

const form = reactive({
  id: '',
  eventType: '' as string,
  eventSubType: '' as string,
  triggerOp: 'eq' as string,
  severityFilter: '' as string,
  hostFilter: '',
  nameKeyword: '',
  alertGroupId: '' as string,
  recipientUserIds: [] as string[],
  notifyOwner: false,
  channelKinds: [...DEFAULT_CHANNEL_KINDS] as NotifyChannelKind[],
  extraChannelIds: [] as string[],
  triggerScene: 'alert_firing',
  description: '',
  enabled: true,
})

const showExtraChannels = computed(() =>
  form.channelKinds.includes('feishu') || form.channelKinds.includes('webhook'),
)
const feishuChannelOptions = computed(() =>
  form.channelKinds.includes('feishu')
    ? channelOptions.value.filter((c) => c.channelType === 'feishu')
    : [],
)
const webhookChannelOptions = computed(() =>
  form.channelKinds.includes('webhook')
    ? channelOptions.value.filter((c) => c.channelType === 'webhook')
    : [],
)

/// 仅当一级类型为 database/middleware 时才允许选子类
const canPickSubType = computed(() => ['database', 'middleware'].includes(form.eventType))

// 选中父组后，由后端返回该组及所有子孙组的成员（按真实 user_id 去重，停用组已排除）。
// 候选列表 = 这些真实成员；切换父组仅刷新候选，已选接收人保留。
const groupMemberIds = ref<Set<string>>(new Set())

const candidateUsers = computed<UserInfo[]>(() => {
  let pool: UserInfo[]
  if (form.alertGroupId && groupMemberIds.value.size > 0) {
    pool = allUsers.value.filter((u) => u.enabled && groupMemberIds.value.has(u.id))
  } else {
    pool = allUsers.value.filter((u) => u.enabled)
  }
  // 排除已选人员
  pool = pool.filter((u) => !form.recipientUserIds.includes(u.id))
  const kw = candidateKeyword.value.trim().toLowerCase()
  if (kw) {
    pool = pool.filter(
      (u) =>
        (u.displayName || '').toLowerCase().includes(kw) ||
        (u.username || '').toLowerCase().includes(kw),
    )
  }
  return pool
})

const selectedUsers = computed<UserInfo[]>(() => {
  return form.recipientUserIds
    .map((id) => allUsers.value.find((u) => u.id === id))
    .filter((u): u is UserInfo => !!u)
})

const rules: FormRules = {
  eventType: [{ required: true, message: '请选择事件类型', trigger: 'change' }],
  recipientUserIds: [
    {
      validator: (_rule, value, callback) => {
        const ids = Array.isArray(value) ? value : []
        if (ids.length === 0 && !form.notifyOwner) {
          callback(new Error('请至少选择一名接收人员，或打开「同时通知资产责任人」'))
        } else {
          callback()
        }
      },
      trigger: 'change',
    },
  ],
  channelKinds: [
    {
      validator: (_rule, value, callback) => {
        if (!Array.isArray(value) || value.length === 0) {
          callback(new Error('请至少勾选一个通知渠道'))
        } else {
          callback()
        }
      },
      trigger: 'change',
    },
  ],
}

function onNotifyOwnerChange() {
  formRef.value?.validateField('recipientUserIds')
}

function onGroupChange() {
  // 切换父组：拉取该组及所有子孙组成员（真实 ID，已去重、已排除停用组），
  // 仅刷新候选池，已选接收人保留（可能跨组）。
  candidateKeyword.value = ''
  groupMemberIds.value = new Set()
  if (!form.alertGroupId) return
  listAlertGroupMembers(form.alertGroupId, true)
    .then((members) => {
      groupMemberIds.value = new Set(members.map((m) => m.id))
    })
    .catch((e: unknown) => {
      ElMessage.error((e as Error).message || '加载组成员失败')
    })
}

function addUser(u: UserInfo) {
  if (!form.recipientUserIds.includes(u.id)) {
    form.recipientUserIds.push(u.id)
  }
}
function removeUser(u: UserInfo) {
  form.recipientUserIds = form.recipientUserIds.filter((id) => id !== u.id)
}
function addSelected() {
  // 添加当前候选列表中的所有人员（按关键词过滤后）
  candidateUsers.value.forEach((u) => addUser(u))
}
function removeSelected() {
  // 删除当前候选列表过滤下已选中的人员
  const removeIds = new Set(candidateUsers.value.map((u) => u.id))
  form.recipientUserIds = form.recipientUserIds.filter((id) => !removeIds.has(id))
}

function openCreate() {
  isEdit.value = false
  form.id = ''
  form.eventType = ''
  form.eventSubType = ''
  form.triggerOp = 'eq'
  form.severityFilter = ''
  form.hostFilter = ''
  form.nameKeyword = ''
  form.alertGroupId = ''
  form.recipientUserIds = []
  form.notifyOwner = false
  form.channelKinds = [...DEFAULT_CHANNEL_KINDS]
  form.extraChannelIds = []
  form.triggerScene = 'alert_firing'
  form.description = ''
  form.enabled = true
  candidateKeyword.value = ''
  groupMemberIds.value = new Set()
  dialogVisible.value = true
}

function openEdit(row: SmsStrategy) {
  isEdit.value = true
  form.id = row.id
  form.eventType = row.eventType || ''
  form.eventSubType = row.eventSubType || ''
  form.triggerOp = row.triggerOp || 'eq'
  form.severityFilter = row.severityFilter || ''
  form.hostFilter = row.hostFilter || ''
  form.nameKeyword = row.nameKeyword || ''
  form.alertGroupId = row.alertGroupId || ''
  form.recipientUserIds = Array.isArray(row.recipientUserIds) ? [...row.recipientUserIds] : []
  form.notifyOwner = !!row.notifyOwner
  form.channelKinds = row.channelKinds?.length ? [...row.channelKinds] : [...DEFAULT_CHANNEL_KINDS]
  form.extraChannelIds = Array.isArray(row.extraChannelIds) ? [...row.extraChannelIds] : []
  form.triggerScene = row.triggerScene || 'alert_firing'
  form.description = row.description || ''
  form.enabled = row.enabled
  candidateKeyword.value = ''
  groupMemberIds.value = new Set()
  dialogVisible.value = true
  // 预载所选父组的真实成员（用于候选池），已选接收人保留
  onGroupChange()
}

function resetForm() {
  formRef.value?.resetFields()
}

async function onSubmit() {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    saving.value = true
    try {
      const payload = {
        eventType: form.eventType || undefined,
        eventSubType: form.eventSubType || undefined,
        triggerOp: form.triggerOp,
        severityFilter: form.severityFilter || undefined,
        hostFilter: form.hostFilter.trim() || undefined,
        nameKeyword: form.nameKeyword.trim() || undefined,
        alertGroupId: form.alertGroupId || undefined,
        recipientUserIds: form.recipientUserIds,
        notifyOwner: form.notifyOwner,
        channelKinds: form.channelKinds,
        extraChannelIds: form.extraChannelIds.filter((id) =>
          [...feishuChannelOptions.value, ...webhookChannelOptions.value].some((c) => c.id === id),
        ),
        triggerScene: form.triggerScene,
        description: form.description || undefined,
        enabled: form.enabled,
      }
      if (isEdit.value) {
        await updateSmsStrategy(form.id, payload)
        ElMessage.success('更新成功')
      } else {
        await createSmsStrategy(payload)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      await fetchList()
    } catch (e: any) {
      ElMessage.error(e?.message || '操作失败')
    } finally {
      saving.value = false
    }
  })
}

async function onToggle(row: SmsStrategy) {
  try {
    await toggleSmsStrategy(row.id, !row.enabled)
    ElMessage.success(row.enabled ? '已停用' : '已启用')
    await fetchList()
  } catch (e: any) {
    ElMessage.error(e?.message || '操作失败')
  }
}

async function onDelete(row: SmsStrategy) {
  try {
    await ElMessageBox.confirm('确定删除该策略吗？', '删除确认', { type: 'warning' })
    await deleteSmsStrategy(row.id)
    ElMessage.success('删除成功')
    await fetchList()
  } catch (e: any) {
    if (e !== 'cancel' && e?.message) ElMessage.error(e.message)
  }
}

onMounted(() => {
  fetchList()
  fetchMeta()
})
</script>

<style scoped>
.sms-page { padding: 0; }
.page-header {
  display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;
}
.page-title { display: flex; align-items: center; gap: 8px; font-size: 18px; font-weight: 600; color: #303133; }
.page-sub { font-size: 12px; font-weight: normal; color: #909399; margin-left: 8px; }
.form-hint { margin-left: 10px; font-size: 12px; color: #909399; line-height: 1.4; }
.header-actions { display: flex; gap: 8px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
.text-muted { color: #c0c4cc; font-size: 12px; }
.empty-tip { padding: 20px 0; }
.filter-bar { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 12px; }
.pagination-wrap { display: flex; justify-content: flex-end; padding: 12px 0 0; }

.staff-transfer { display: flex; align-items: center; gap: 12px; width: 100%; }
.staff-panel {
  flex: 1; border: 1px solid #e4e7ed; border-radius: 4px; padding: 8px; min-width: 0;
}
.panel-title { font-size: 13px; font-weight: 600; color: #606266; margin-bottom: 6px; }
.staff-list { height: 180px; overflow-y: auto; border: 1px solid #ebeef5; border-radius: 4px; padding: 4px; }
.staff-item {
  display: flex; justify-content: space-between; align-items: center;
  padding: 5px 8px; border-radius: 4px; cursor: pointer; font-size: 13px;
}
.staff-item:hover { background: #f5f7fa; }
.staff-name { color: #303133; }
.staff-dept { color: #909399; font-size: 12px; margin-left: 8px; }
.list-empty { color: #c0c4cc; font-size: 12px; text-align: center; padding: 40px 0; }
.transfer-actions { display: flex; flex-direction: column; gap: 8px; flex-shrink: 0; }
</style>
