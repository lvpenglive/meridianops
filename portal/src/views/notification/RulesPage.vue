<template>
  <div class="rules-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><Setting /></el-icon>
        <span>通知规则</span>
        <span class="page-sub">配置事件触发通知规则：事件类型 + 严重级别过滤 + 通道选择 → 自动分发</span>
      </div>
      <div class="header-actions">
        <el-button
          v-if="hasPermission('notification:manage')"
          type="primary"
          :icon="Plus"
          @click="openCreate"
        >
          新增规则
        </el-button>
      </div>
    </div>

    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>规则列表（{{ total }}）</span>
          <el-button :icon="Refresh" size="small" @click="fetchList">刷新</el-button>
        </div>
      </template>

      <!-- 筛选栏 -->
      <div class="filter-bar">
        <el-input
          v-model="filters.keyword"
          placeholder="规则名称关键字"
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
          <el-option
            v-for="opt in eventTypeOptions"
            :key="opt.value"
            :label="opt.label"
            :value="opt.value"
          />
        </el-select>
        <el-select
          v-model="filters.triggerScene"
          placeholder="全部触发场景"
          clearable
          size="default"
          style="width: 160px;"
          @change="onSearch"
        >
          <el-option
            v-for="opt in triggerSceneOptions"
            :key="opt.value"
            :label="opt.label"
            :value="opt.value"
          />
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
          <el-option label="禁用" :value="false" />
        </el-select>
        <el-button type="primary" size="default" :icon="Search" @click="onSearch">查询</el-button>
        <el-button size="default" @click="onReset">重置</el-button>
      </div>

      <el-table :data="list" v-loading="loading" stripe size="default">
        <el-table-column prop="name" label="规则名称" min-width="160" show-overflow-tooltip />
        <el-table-column label="事件类型" width="140">
          <template #default="{ row }">
            <el-tag v-if="row.eventType" size="small" type="primary" effect="plain">
              {{ eventTypeLabel(row.eventType) }}
            </el-tag>
            <span v-else class="text-muted">全部类型</span>
          </template>
        </el-table-column>
        <el-table-column label="触发场景" width="140">
          <template #default="{ row }">
            <el-tag v-if="row.triggerScene" size="small" type="warning" effect="plain">
              {{ triggerSceneLabel(row.triggerScene) }}
            </el-tag>
            <span v-else class="text-muted">全部场景</span>
          </template>
        </el-table-column>
        <el-table-column label="级别条件" width="200">
          <template #default="{ row }">
            <el-tag
              v-if="row.severityFilter && row.severityFilter.length"
              size="small"
              type="info"
              effect="plain"
            >{{ severityDisplay(row) }}</el-tag>
            <span v-else class="text-muted">全部级别</span>
          </template>
        </el-table-column>
        <el-table-column label="通道" min-width="220">
          <template #default="{ row }">
            <template v-if="row.channelIds && row.channelIds.length">
              <el-tag
                v-for="cid in row.channelIds"
                :key="cid"
                size="small"
                type="info"
                effect="plain"
                style="margin-right: 4px;"
              >{{ channelName(cid) }}</el-tag>
            </template>
            <span v-else class="text-muted">—</span>
          </template>
        </el-table-column>
        <el-table-column label="收件人" min-width="180" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.recipientList">{{ row.recipientList }}</span>
            <span v-else class="text-muted">—</span>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.enabled" size="small" type="success">启用</el-tag>
            <el-tag v-else size="small" type="danger">禁用</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="updatedAt" label="更新时间" width="180" show-overflow-tooltip />
        <el-table-column label="操作" width="160" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="hasPermission('notification:manage')"
              size="small" link type="primary"
              @click="openEdit(row)"
            >编辑</el-button>
            <el-button
              v-if="hasPermission('notification:manage')"
              size="small" link type="danger"
              @click="onDelete(row)"
            >删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <div v-if="!loading && list.length === 0" class="empty-tip">
        <el-empty description="暂无通知规则" />
      </div>

      <!-- 分页 -->
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

    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑规则' : '新增规则'"
      width="640px"
      @closed="resetForm"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="110px">
        <el-form-item label="规则名称" prop="name">
          <el-input v-model="form.name" placeholder="如 P0告警邮件通知" />
        </el-form-item>
        <el-form-item label="事件类型">
          <el-select v-model="form.eventType" placeholder="留空 = 匹配所有事件类型" clearable style="width: 100%;">
            <el-option
              v-for="opt in eventTypeOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
          <div class="text-xs text-gray-500 mt-1">
            事件类型由字典「event_type」维护；留空表示匹配所有事件类型
          </div>
        </el-form-item>
        <el-form-item label="触发场景">
          <el-select v-model="form.triggerScene" placeholder="留空 = 匹配所有触发场景" clearable style="width: 100%;">
            <el-option
              v-for="opt in triggerSceneOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
          <div class="text-xs text-gray-500 mt-1">
            6 个内置生命周期：告警触发/认领/恢复、工单分派/关闭、作业失败；留空表示匹配所有场景
          </div>
        </el-form-item>
        <el-form-item label="级别条件">
          <el-select
            v-model="form.severityOp"
            placeholder="选择级别匹配方式"
            style="width: 100%; margin-bottom: 8px;"
            @change="onSeverityOpChange"
          >
            <el-option
              v-for="opt in severityOps"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            >
              <span>{{ opt.label }}</span>
              <span class="text-xs text-gray-400 ml-2">{{ opt.desc }}</span>
            </el-option>
          </el-select>
          <!-- 根据级别条件动态渲染表单 -->
          <el-select
            v-if="form.severityOp === 'in'"
            v-model="form.severityFilter"
            multiple
            collapse-tags
            collapse-tags-tooltip
            placeholder="留空表示全部级别"
            style="width: 100%;"
          >
            <el-option
              v-for="opt in severityOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
          <div v-else-if="form.severityOp === 'between'" style="display: flex; gap: 4%;">
            <el-select
              v-model="form.severityMin"
              placeholder="最低级别（含）"
              style="width: 48%;"
            >
              <el-option
                v-for="opt in severityOptions"
                :key="opt.value"
                :label="opt.label"
                :value="opt.value"
              />
            </el-select>
            <el-select
              v-model="form.severityMax"
              placeholder="最高级别（含）"
              style="width: 48%;"
            >
              <el-option
                v-for="opt in severityOptions"
                :key="opt.value"
                :label="opt.label"
                :value="opt.value"
              />
            </el-select>
          </div>
          <el-select
            v-else
            v-model="form.severitySingle"
            placeholder="选择级别"
            clearable
            style="width: 100%;"
          >
            <el-option
              v-for="opt in severityOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
          <div class="text-xs text-gray-500 mt-1">
            级别由低到高：信息 &lt; 警告 &lt; 重要 &lt; 灾难；留空 = 匹配所有级别
          </div>
        </el-form-item>
        <el-form-item label="设备/IP 过滤（可选）">
          <el-input
            v-model="form.hostFilter"
            placeholder="逗号分隔多值；支持 % 通配，例 10.0.5.1, 10.0.5.%"
          />
          <div class="text-xs text-gray-500 mt-1">
            仅当告警归属的设备 IP/主机名匹配时才触发；留空 = 所有设备都匹配
          </div>
        </el-form-item>
        <el-form-item label="事件名关键字（可选）">
          <el-input
            v-model="form.nameKeyword"
            placeholder="告警标题包含此关键字才命中，例 CPU / 磁盘 / MySQL"
          />
          <div class="text-xs text-gray-500 mt-1">大小写不敏感；留空 = 所有告警都匹配</div>
        </el-form-item>
        <el-form-item label="通道" prop="channelIds">
          <el-select
            v-model="form.channelIds"
            multiple
            filterable
            placeholder="选择要分发的通道"
            style="width: 100%;"
          >
            <el-option
              v-for="c in channels"
              :key="c.id"
              :label="`${c.name}（${channelTypeLabel(c.channelType)}）`"
              :value="c.id"
              :disabled="!c.enabled"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="收件人">
          <el-input
            v-model="form.recipientList"
            type="textarea"
            :rows="2"
            placeholder="可选，多个以逗号分隔，主要用作邮件通道的收件邮箱；飞书/Webhook 无需填"
          />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="form.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Refresh, Search, Setting } from '@element-plus/icons-vue'
import {
  listRules,
  createRule,
  updateRule,
  deleteRule,
  listAllChannels,
  listEventTypes,
  TRIGGER_SCENES,
  SEVERITY_OPS,
  type NotificationRule,
  type NotificationChannel,
} from '../../api/notification'
import type { DictItem } from '../../api/dict'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()
const hasPermission = (code: string) => userStore.hasPermission(code)

const loading = ref(false)
const list = ref<NotificationRule[]>([])
const total = ref(0)
const channels = ref<NotificationChannel[]>([])
const eventTypeOptions = ref<DictItem[]>([])   // 来自字典 event_type
const triggerSceneOptions = TRIGGER_SCENES       // 系统内置 6 个生命周期

// 筛选 + 分页
const filters = reactive({
  keyword: '',
  eventType: '' as string,
  triggerScene: '' as string,
  enabled: null as boolean | null,
})
const pagination = reactive({
  page: 1,
  pageSize: 20,
})

function buildParams() {
  const params: Record<string, any> = {
    page: pagination.page,
    pageSize: pagination.pageSize,
  }
  if (filters.keyword.trim()) params.keyword = filters.keyword.trim()
  if (filters.eventType) params.eventType = filters.eventType
  if (filters.triggerScene) params.triggerScene = filters.triggerScene
  if (filters.enabled !== null) params.enabled = filters.enabled
  return params
}

async function fetchList() {
  loading.value = true
  try {
    const res = await listRules(buildParams())
    list.value = res.list
    total.value = res.total
  } catch (e: any) {
    ElMessage.error(e?.message || '加载规则失败')
  } finally {
    loading.value = false
  }
}

function onSearch() {
  pagination.page = 1
  fetchList()
}
function onReset() {
  filters.keyword = ''
  filters.eventType = ''
  filters.triggerScene = ''
  filters.enabled = null
  pagination.page = 1
  fetchList()
}
function onSizeChange(size: number) {
  pagination.pageSize = size
  pagination.page = 1
  fetchList()
}

async function fetchChannels() {
  try {
    channels.value = await listAllChannels()
  } catch {
    // 静默失败，仅影响通道名展示
  }
}

async function fetchEventTypes() {
  try {
    eventTypeOptions.value = await listEventTypes()
  } catch {
    // 字典读取失败时静默
  }
}

// ---- 选项映射 ----
function eventTypeLabel(t: string): string {
  if (!t) return '全部类型'
  const o = eventTypeOptions.value.find((x) => x.value === t)
  return o ? o.label : t
}
function triggerSceneLabel(t: string): string {
  if (!t) return '全部场景'
  const o = TRIGGER_SCENES.find((x) => x.value === t)
  return o ? o.label : t
}

const severityOptions = [
  { value: '5', label: '5级（灾难）' },
  { value: '4', label: '4级（重要）' },
  { value: '3', label: '3级（一般）' },
  { value: '2', label: '2级（警告）' },
  { value: '1', label: '1级（信息）' },
]
const severityOps = SEVERITY_OPS  // 级别比较运算符选项

/// 切换级别条件时清空无关字段，避免遗留值污染 payload
function onSeverityOpChange() {
  if (form.severityOp !== 'in') {
    form.severityFilter = []
  }
  if (form.severityOp !== 'between') {
    form.severityMin = ''
    form.severityMax = ''
  }
  if (!['gte', 'gt', 'lte', 'lt', 'eq'].includes(form.severityOp)) {
    form.severitySingle = ''
  }
}

function severityLabel(s: string): string {
  // 兼容历史值 disaster/critical/warning/info
  const legacy: Record<string, string> = {
    disaster: '5级', critical: '4级', warning: '2级', info: '1级',
  }
  if (legacy[s]) return legacy[s]
  const o = severityOptions.find((o) => o.value === s)
  return o ? o.label : `${s}级`
}
function severityTagType(s: string) {
  const n = Number(s)
  if (s === 'disaster' || n === 5) return 'danger'
  if (s === 'critical' || n === 4) return 'warning'
  if (s === 'warning' || n === 2 || n === 3) return 'primary'
  return 'info'
}

function channelTypeLabel(t: string): string {
  return { email: '邮件', feishu: '飞书', webhook: 'Webhook' }[t] || t
}
function channelName(id: string): string {
  const c = channels.value.find((x) => x.id === id)
  return c ? c.name : id.slice(0, 8)
}

// ---- 对话框 ----
const dialogVisible = ref(false)
const isEdit = ref(false)
const saving = ref(false)
const formRef = ref<FormInstance>()

const form = reactive({
  id: '',
  name: '',
  eventType: '' as string,           // 空字符串 = 匹配所有事件类型
  triggerScene: '' as string,        // 空字符串 = 匹配所有触发场景
  severityOp: 'in' as string,        // 默认包含匹配
  severityFilter: [] as string[],    // op=in 时为多值数组
  severitySingle: '' as string,      // op=gte/gt/lte/lt/eq 时为单值
  severityMin: '' as string,         // op=between 时为下界
  severityMax: '' as string,          // op=between 时为上界
  hostFilter: '',
  nameKeyword: '',
  channelIds: [] as string[],
  recipientList: '',
  enabled: true,
})

const rules: FormRules = {
  name: [{ required: true, message: '请输入规则名称', trigger: 'blur' }],
  channelIds: [{ required: true, type: 'array', message: '至少选择一个通道', trigger: 'change' }],
}

/// 根据 severityOp 把表单中的 severitySingle/severityMin/severityMax/severityFilter
/// 组装为后端期望的 severityFilter 数组（保持后端 JSON 结构简单）
function buildSeverityFilter(): string[] {
  switch (form.severityOp) {
    case 'in':
      return form.severityFilter
    case 'between':
      return [form.severityMin, form.severityMax].filter((s) => !!s)
    default:
      // gte/gt/lte/lt/eq
      return form.severitySingle ? [form.severitySingle] : []
  }
}

/// 根据 severityOp 和 severityFilter，在表格中展示级别条件描述
function severityDisplay(row: NotificationRule): string {
  const op = row.severityOp || 'in'
  const arr = Array.isArray(row.severityFilter) ? row.severityFilter : []
  if (arr.length === 0) return '全部级别'
  const labels = arr.map(severityLabel)
  switch (op) {
    case 'in':      return `包含: ${labels.join('、')}`
    case 'gte':     return `≥ ${labels[0] || ''}`
    case 'gt':      return `> ${labels[0] || ''}`
    case 'eq':      return `= ${labels[0] || ''}`
    case 'lte':     return `≤ ${labels[0] || ''}`
    case 'lt':      return `< ${labels[0] || ''}`
    case 'between': return `区间 [${labels[0] || ''}, ${labels[1] || ''}]`
    default:        return labels.join('、')
  }
}

function openCreate() {
  isEdit.value = false
  form.id = ''
  form.name = ''
  form.eventType = ''
  form.triggerScene = ''
  form.severityOp = 'in'
  form.severityFilter = []
  form.severitySingle = ''
  form.severityMin = ''
  form.severityMax = ''
  form.hostFilter = ''
  form.nameKeyword = ''
  form.channelIds = []
  form.recipientList = ''
  form.enabled = true
  dialogVisible.value = true
}

function openEdit(row: NotificationRule) {
  isEdit.value = true
  form.id = row.id
  form.name = row.name
  form.eventType = row.eventType || ''
  form.triggerScene = row.triggerScene || ''
  form.severityOp = row.severityOp || 'in'
  // 根据原 op 回填表单字段
  const arr = Array.isArray(row.severityFilter) ? [...row.severityFilter] : []
  form.severityFilter = arr
  form.severitySingle = arr[0] || ''
  form.severityMin = arr[0] || ''
  form.severityMax = arr[1] || arr[0] || ''
  form.hostFilter = row.hostFilter || ''
  form.nameKeyword = row.nameKeyword || ''
  form.channelIds = Array.isArray(row.channelIds) ? [...row.channelIds] : []
  form.recipientList = row.recipientList || ''
  form.enabled = row.enabled
  dialogVisible.value = true
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
      const sevFilter = buildSeverityFilter()
      const payload = {
        name: form.name.trim(),
        eventType: form.eventType || undefined,
        triggerScene: form.triggerScene || undefined,
        severityFilter: sevFilter,
        severityOp: form.severityOp,
        hostFilter: form.hostFilter.trim(),
        nameKeyword: form.nameKeyword.trim(),
        channelIds: form.channelIds,
        recipientList: form.recipientList,
        enabled: form.enabled,
      }
      if (isEdit.value) {
        await updateRule(form.id, payload)
        ElMessage.success('更新成功')
      } else {
        await createRule(payload)
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

async function onDelete(row: NotificationRule) {
  try {
    await ElMessageBox.confirm(
      `确定删除规则「${row.name}」吗？`,
      '删除确认',
      { type: 'warning' },
    )
    await deleteRule(row.id)
    ElMessage.success('删除成功')
    await fetchList()
  } catch (e: any) {
    if (e !== 'cancel' && e?.message) ElMessage.error(e.message)
  }
}

onMounted(() => {
  fetchList()
  fetchChannels()
  fetchEventTypes()
})
</script>

<style scoped>
.rules-page { padding: 0; }

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.page-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}
.page-sub {
  font-size: 12px;
  font-weight: normal;
  color: #909399;
  margin-left: 8px;
}
.header-actions {
  display: flex;
  gap: 8px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.text-muted { color: #c0c4cc; font-size: 12px; }
.empty-tip { padding: 20px 0; }

.filter-bar {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}

.pagination-wrap {
  display: flex;
  justify-content: flex-end;
  padding: 12px 0 0;
}
</style>
