<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  listNotificationLogs,
  listChannels,
  getNotificationCleanerConfig,
  runNotificationCleaner,
  type NotificationLog,
  type NotificationChannel,
  type NotificationCleanerConfig,
} from '@/api/notification'

const loading = ref(false)
const logs = ref<NotificationLog[]>([])
const total = ref(0)
const channels = ref<NotificationChannel[]>([])
const cleanerConfig = ref<NotificationCleanerConfig | null>(null)

const cleanerDialogVisible = ref(false)
const cleanerForm = reactive({
  retentionDays: 30,
  batchSize: 1000,
})
const cleanerRunning = ref(false)

async function loadCleanerConfig() {
  try {
    cleanerConfig.value = await getNotificationCleanerConfig()
    cleanerForm.retentionDays = cleanerConfig.value.retentionDays
    cleanerForm.batchSize = cleanerConfig.value.batchSize
  } catch (e) {
    // 权限或老版本：静默忽略
  }
}

function openCleaner() {
  cleanerForm.retentionDays = cleanerConfig.value?.retentionDays ?? 30
  cleanerForm.batchSize = cleanerConfig.value?.batchSize ?? 1000
  cleanerDialogVisible.value = true
}

async function confirmCleaner() {
  if (cleanerForm.retentionDays <= 0) return ElMessage.warning('保留天数必须大于 0')
  cleanerRunning.value = true
  try {
    const r = await runNotificationCleaner({
      retentionDays: cleanerForm.retentionDays,
      batchSize: cleanerForm.batchSize,
    })
    ElMessage.success(
      `清理完成：删除 ${r.deleted} 行 / ${r.batches} 批 / 耗时 ${r.durationMs} ms`
    )
    cleanerDialogVisible.value = false
    await load()
  } catch (e: any) {
    ElMessage.error(e?.message ?? '清理失败')
  } finally {
    cleanerRunning.value = false
  }
}

const form = reactive({
  page: 1,
  pageSize: 20,
  status: '' as '' | 'success' | 'failed',
  channelType: '' as '' | 'email' | 'feishu' | 'webhook',
  eventType: '' as string,
  channelId: '' as string,
  triggeredBy: '' as '' | 'rule' | 'manual_test',
  keyword: '' as string,
  sentSince: '' as string,
  sentUntil: '' as string,
})

const eventTypeOptions = [
  { label: '全部事件', value: '' },
  { label: '新告警触发', value: 'alert_firing' },
  { label: '告警认领', value: 'alert_acknowledged' },
  { label: '告警解决', value: 'alert_resolved' },
  { label: '通道测试', value: 'test' },
  { label: '工单分派', value: 'ticket_assigned' },
  { label: '工单通过', value: 'ticket_approved' },
  { label: '工单驳回', value: 'ticket_rejected' },
  { label: '工单关闭', value: 'ticket_closed' },
  { label: '作业失败', value: 'job_failed' },
  { label: '系统通知', value: 'system' },
]

const statusTagType = (s: string) => (s === 'success' ? 'success' : s === 'failed' ? 'danger' : 'info')
const channelTypeLabel = (t: string) =>
  ({ email: '邮件', feishu: '飞书', webhook: '通用 Webhook' } as Record<string, string>)[t] ?? t
const triggeredByLabel = (t?: string | null) =>
  ({ rule: '规则触发', manual_test: '手动测试' } as Record<string, string>)[t ?? ''] ?? '-'

const severityTagType = (s?: string | null) =>
  ({
    disaster: 'danger',
    critical: 'danger',
    warning: 'warning',
    info: 'info',
  } as Record<string, string>)[s ?? ''] ?? ''

async function loadChannels() {
  try {
    channels.value = await listChannels()
  } catch (e) {
    // 权限不足可忽略
  }
}

async function search() {
  form.page = 1
  await load()
}

async function reset() {
  form.status = ''
  form.channelType = ''
  form.eventType = ''
  form.channelId = ''
  form.triggeredBy = ''
  form.keyword = ''
  form.sentSince = ''
  form.sentUntil = ''
  form.page = 1
  await load()
}

async function load() {
  loading.value = true
  try {
    const params: Record<string, any> = {
      page: form.page,
      pageSize: form.pageSize,
    }
    if (form.status) params.status = form.status
    if (form.channelType) params.channelType = form.channelType
    if (form.eventType) params.eventType = form.eventType
    if (form.channelId) params.channelId = form.channelId
    if (form.triggeredBy) params.triggeredBy = form.triggeredBy
    if (form.keyword.trim()) params.keyword = form.keyword.trim()
    if (form.sentSince) params.sentSince = form.sentSince
    if (form.sentUntil) params.sentUntil = form.sentUntil
    const r = await listNotificationLogs(params)
    logs.value = r.list
    total.value = r.total
  } catch (e: any) {
    ElMessage.error(e?.message ?? '加载通知发送日志失败')
  } finally {
    loading.value = false
  }
}

const detailVisible = ref(false)
const detailItem = ref<NotificationLog | null>(null)
function openDetail(item: NotificationLog) {
  detailItem.value = item
  detailVisible.value = true
}

const previewableContent = computed(() => {
  if (!detailItem.value) return ''
  return detailItem.value.content ?? ''
})

const previewableError = computed(() => {
  if (!detailItem.value) return ''
  return detailItem.value.errorMsg ?? ''
})

onMounted(async () => {
  await loadChannels()
  await loadCleanerConfig()
  await load()
})
</script>

<template>
  <div class="p-5">
    <el-card shadow="never">
      <template #header>
        <div class="flex items-center justify-between">
          <div>
            <span class="text-lg font-semibold">通知发送日志</span>
            <div v-if="cleanerConfig" class="mt-1 text-xs text-gray-500">
              自动清理：
              <el-tag :type="cleanerConfig.enabled ? 'success' : 'info'" size="small">
                {{ cleanerConfig.enabled ? '已启用' : '已禁用' }}
              </el-tag>
              <span class="ml-2">保留 {{ cleanerConfig.retentionDays }} 天</span>
              <span class="ml-2">每 {{ Math.round(cleanerConfig.intervalSecs / 3600 * 10) / 10 }} 小时一次</span>
              <span class="ml-2">批大小 {{ cleanerConfig.batchSize }}</span>
            </div>
          </div>
          <div>
            <el-button :icon="Delete" type="danger" plain @click="openCleaner">立即清理</el-button>
            <el-button :icon="Refresh" :loading="loading" @click="load">刷新</el-button>
          </div>
        </div>
      </template>

      <el-form :inline="true" :model="form" class="mb-4">
        <el-form-item label="状态">
          <el-select v-model="form.status" placeholder="全部" clearable style="width: 140px">
            <el-option label="发送成功" value="success" />
            <el-option label="发送失败" value="failed" />
          </el-select>
        </el-form-item>
        <el-form-item label="通道类型">
          <el-select v-model="form.channelType" placeholder="全部" clearable style="width: 150px">
            <el-option label="邮件" value="email" />
            <el-option label="飞书" value="feishu" />
            <el-option label="通用 Webhook" value="webhook" />
          </el-select>
        </el-form-item>
        <el-form-item label="通道">
          <el-select v-model="form.channelId" placeholder="全部通道" clearable filterable style="width: 200px">
            <el-option v-for="c in channels" :key="c.id" :label="c.name" :value="c.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="事件类型">
          <el-select v-model="form.eventType" placeholder="全部事件" clearable filterable style="width: 170px">
            <el-option v-for="o in eventTypeOptions" :key="o.value" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="触发方式">
          <el-select v-model="form.triggeredBy" placeholder="全部" clearable style="width: 140px">
            <el-option label="规则触发" value="rule" />
            <el-option label="手动测试" value="manual_test" />
          </el-select>
        </el-form-item>
        <el-form-item label="发送时间">
          <el-date-picker
            v-model="form.sentSince"
            type="datetime"
            placeholder="起始时间"
            value-format="YYYY-MM-DDTHH:mm:ssZ"
            style="width: 200px"
          />
          <span class="mx-2">-</span>
          <el-date-picker
            v-model="form.sentUntil"
            type="datetime"
            placeholder="结束时间"
            value-format="YYYY-MM-DDTHH:mm:ssZ"
            style="width: 200px"
          />
        </el-form-item>
        <el-form-item label="关键字">
          <el-input
            v-model="form.keyword"
            placeholder="标题 / 收件人 / 错误"
            clearable
            style="width: 260px"
            @keyup.enter="search"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="search">查询</el-button>
          <el-button @click="reset">重置</el-button>
        </el-form-item>
      </el-form>

      <el-table :data="logs" v-loading="loading" stripe border style="width: 100%">
        <el-table-column label="发送时间" prop="sentAt" width="200" sortable="custom" />
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="statusTagType(row.status)" effect="dark">
              {{ row.status === 'success' ? '成功' : row.status === 'failed' ? '失败' : row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="触发方式" width="100">
          <template #default="{ row }">{{ triggeredByLabel(row.triggeredBy) }}</template>
        </el-table-column>
        <el-table-column label="事件类型" width="150">
          <template #default="{ row }">
            <span>{{ row.eventType }}</span>
            <el-tag v-if="row.severity" :type="severityTagType(row.severity)" size="small" effect="plain" class="ml-1">
              {{ row.severity }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="规则" min-width="160">
          <template #default="{ row }">
            <div v-if="row.ruleName" class="truncate" :title="row.ruleName">{{ row.ruleName }}</div>
            <span v-else class="text-gray-400">—</span>
          </template>
        </el-table-column>
        <el-table-column label="通道" width="160">
          <template #default="{ row }">
            <el-tag size="small" class="mr-1">{{ channelTypeLabel(row.channelType) }}</el-tag>
            <span class="truncate" :title="row.channelName">{{ row.channelName }}</span>
          </template>
        </el-table-column>
        <el-table-column label="收件人" min-width="160">
          <template #default="{ row }">
            <div v-if="row.recipients" class="truncate" :title="row.recipients">{{ row.recipients }}</div>
            <span v-else class="text-gray-400">—</span>
          </template>
        </el-table-column>
        <el-table-column label="标题" min-width="260" show-overflow-tooltip prop="title" />
        <el-table-column label="耗时" width="90">
          <template #default="{ row }">{{ row.durationMs ?? '-' }} ms</template>
        </el-table-column>
        <el-table-column label="错误" min-width="200" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.errorMsg" class="text-red-600">{{ row.errorMsg }}</span>
            <span v-else class="text-gray-400">—</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="100" fixed="right">
          <template #default="{ row }">
            <el-button link type="primary" @click="openDetail(row)">详情</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="mt-4 flex justify-end">
        <el-pagination
          v-model:current-page="form.page"
          v-model:page-size="form.pageSize"
          :total="total"
          :page-sizes="[20, 50, 100, 200]"
          background
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="load"
          @current-change="load"
        />
      </div>
    </el-card>

    <el-dialog v-model="detailVisible" title="通知发送详情" width="900px" destroy-on-close top="4vh">
      <div v-if="detailItem" class="space-y-3">
        <el-descriptions :column="2" border>
          <el-descriptions-item label="发送时间" :span="1">{{ detailItem.sentAt }}</el-descriptions-item>
          <el-descriptions-item label="结果" :span="1">
            <el-tag :type="statusTagType(detailItem.status)" effect="dark">
              {{ detailItem.status === 'success' ? '成功' : detailItem.status === 'failed' ? '失败' : detailItem.status }}
            </el-tag>
            <span class="ml-2 text-gray-500">耗时 {{ detailItem.durationMs ?? '-' }} ms</span>
          </el-descriptions-item>
          <el-descriptions-item label="触发方式" :span="1">{{ triggeredByLabel(detailItem.triggeredBy) }}</el-descriptions-item>
          <el-descriptions-item label="事件类型" :span="1">
            {{ detailItem.eventType }}
            <el-tag v-if="detailItem.severity" :type="severityTagType(detailItem.severity)" size="small" effect="plain" class="ml-1">
              {{ detailItem.severity }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="规则" :span="1">{{ detailItem.ruleName ?? '—' }}</el-descriptions-item>
          <el-descriptions-item label="通道" :span="1">
            {{ channelTypeLabel(detailItem.channelType) }} / {{ detailItem.channelName }}
          </el-descriptions-item>
          <el-descriptions-item label="收件人" :span="2">{{ detailItem.recipients ?? '—' }}</el-descriptions-item>
          <el-descriptions-item label="标题" :span="2">{{ detailItem.title }}</el-descriptions-item>
          <el-descriptions-item v-if="detailItem.link" label="跳转链接" :span="2">
            <a :href="detailItem.link" target="_blank" class="text-blue-600 underline">{{ detailItem.link }}</a>
          </el-descriptions-item>
        </el-descriptions>

        <div>
          <div class="mb-1 font-medium">通知正文</div>
          <el-input v-model="previewableContent" type="textarea" :rows="8" readonly />
        </div>

        <div v-if="detailItem.errorMsg">
          <div class="mb-1 font-medium text-red-600">错误详情</div>
          <el-input v-model="previewableError" type="textarea" :rows="6" readonly class="error-textarea" />
        </div>
        <div v-else-if="detailItem.responseSnippet">
          <div class="mb-1 font-medium">响应摘要</div>
          <pre class="bg-gray-50 border rounded p-3 whitespace-pre-wrap break-all">{{ detailItem.responseSnippet }}</pre>
        </div>
      </div>
      <template #footer>
        <el-button @click="detailVisible = false">关闭</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="cleanerDialogVisible" title="立即清理通知发送日志" width="520px" destroy-on-close>
      <el-alert
        type="warning"
        :closable="false"
        class="mb-4"
        title="清理后数据无法恢复，建议先确认保留天数是否正确。"
      />
      <el-form :model="cleanerForm" label-width="110px">
        <el-form-item label="保留最近天数">
          <el-input-number v-model="cleanerForm.retentionDays" :min="1" :max="3650" />
          <span class="ml-2 text-gray-500 text-xs">仅删除该天数之前的记录</span>
        </el-form-item>
        <el-form-item label="每批删除上限">
          <el-input-number v-model="cleanerForm.batchSize" :min="100" :max="10000" :step="100" />
          <span class="ml-2 text-gray-500 text-xs">较大值快，较小值对 DB 更友好</span>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="cleanerDialogVisible = false">取消</el-button>
        <el-button type="danger" :loading="cleanerRunning" @click="confirmCleaner">
          确认清理
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script lang="ts">
import { Refresh, Delete } from '@element-plus/icons-vue'
export default { name: 'NotificationLogsPage' }
</script>

<style scoped>
.error-textarea :deep(.el-textarea__inner) {
  color: #c0392b;
  background-color: #fef5f5;
}
</style>
