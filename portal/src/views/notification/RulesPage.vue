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
          <span>规则列表（{{ list.length }}）</span>
          <el-button :icon="Refresh" size="small" @click="fetchList">刷新</el-button>
        </div>
      </template>

      <el-table :data="list" v-loading="loading" stripe size="default">
        <el-table-column prop="name" label="规则名称" min-width="160" show-overflow-tooltip />
        <el-table-column label="事件类型" width="170">
          <template #default="{ row }">
            <el-tag size="small" type="primary" effect="plain">
              {{ eventTypeLabel(row.eventType) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="严重级别过滤" min-width="200">
          <template #default="{ row }">
            <template v-if="row.severityFilter && row.severityFilter.length">
              <el-tag
                v-for="s in row.severityFilter"
                :key="s"
                :type="severityTagType(s)"
                size="small"
                effect="dark"
                style="margin-right: 4px;"
              >{{ severityLabel(s) }}</el-tag>
            </template>
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
        <el-form-item label="事件类型" prop="eventType">
          <el-select v-model="form.eventType" placeholder="选择事件类型" style="width: 100%;">
            <el-option
              v-for="opt in eventTypeOptions"
              :key="opt.value"
              :label="opt.label"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="严重级别">
          <el-select
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
import { Plus, Refresh, Setting } from '@element-plus/icons-vue'
import {
  listRules,
  createRule,
  updateRule,
  deleteRule,
  listChannels,
  type NotificationRule,
  type NotificationChannel,
} from '../../api/notification'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()
const hasPermission = (code: string) => userStore.hasPermission(code)

const loading = ref(false)
const list = ref<NotificationRule[]>([])
const channels = ref<NotificationChannel[]>([])

async function fetchList() {
  loading.value = true
  try {
    list.value = await listRules()
  } catch (e: any) {
    ElMessage.error(e?.message || '加载规则失败')
  } finally {
    loading.value = false
  }
}

async function fetchChannels() {
  try {
    channels.value = await listChannels()
  } catch {
    // 静默失败，仅影响通道名展示
  }
}

// ---- 选项映射 ----
const eventTypeOptions = [
  { value: 'alert_firing', label: '告警触发' },
  { value: 'alert_acknowledged', label: '告警认领' },
  { value: 'alert_resolved', label: '告警解决' },
  { value: 'ticket_assigned', label: '工单分派' },
  { value: 'ticket_closed', label: '工单关闭' },
  { value: 'job_failed', label: '作业失败' },
]
function eventTypeLabel(t: string): string {
  return eventTypeOptions.find((o) => o.value === t)?.label || t
}

const severityOptions = [
  { value: 'disaster', label: '灾难' },
  { value: 'critical', label: '重要' },
  { value: 'warning', label: '警告' },
  { value: 'info', label: '信息' },
]
function severityLabel(s: string): string {
  return severityOptions.find((o) => o.value === s)?.label || s
}
function severityTagType(s: string) {
  return ({ disaster: 'danger', critical: 'warning', warning: 'primary', info: 'info' } as Record<string, any>)[s] || 'info'
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
  eventType: 'alert_firing',
  severityFilter: [] as string[],
  channelIds: [] as string[],
  recipientList: '',
  enabled: true,
})

const rules: FormRules = {
  name: [{ required: true, message: '请输入规则名称', trigger: 'blur' }],
  eventType: [{ required: true, message: '请选择事件类型', trigger: 'change' }],
  channelIds: [{ required: true, type: 'array', message: '至少选择一个通道', trigger: 'change' }],
}

function openCreate() {
  isEdit.value = false
  form.id = ''
  form.name = ''
  form.eventType = 'alert_firing'
  form.severityFilter = []
  form.channelIds = []
  form.recipientList = ''
  form.enabled = true
  dialogVisible.value = true
}

function openEdit(row: NotificationRule) {
  isEdit.value = true
  form.id = row.id
  form.name = row.name
  form.eventType = row.eventType
  form.severityFilter = Array.isArray(row.severityFilter) ? [...row.severityFilter] : []
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
      const payload = {
        name: form.name.trim(),
        eventType: form.eventType,
        severityFilter: form.severityFilter,
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
</style>
