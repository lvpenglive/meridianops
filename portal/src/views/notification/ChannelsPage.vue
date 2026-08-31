<template>
  <div class="channels-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><Message /></el-icon>
        <span>通知通道</span>
        <span class="page-sub">配置邮件 / 飞书 / 通用 Webhook 通道，供通知规则调用分发消息</span>
      </div>
      <div class="header-actions">
        <el-button
          v-if="hasPermission('notification:manage')"
          type="primary"
          :icon="Plus"
          @click="openCreate"
        >
          新增通道
        </el-button>
      </div>
    </div>

    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>通道列表（{{ list.length }}）</span>
          <el-button :icon="Refresh" size="small" @click="fetchList">刷新</el-button>
        </div>
      </template>

      <el-table :data="list" v-loading="loading" stripe size="default">
        <el-table-column prop="name" label="通道名称" min-width="160" show-overflow-tooltip />
        <el-table-column label="类型" width="120">
          <template #default="{ row }">
            <el-tag :type="channelTypeTagType(row.channelType)" size="small" effect="plain">
              {{ channelTypeLabel(row.channelType) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="配置摘要" min-width="240" show-overflow-tooltip>
          <template #default="{ row }">
            <span class="config-summary">{{ configSummary(row) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.enabled" size="small" type="success">启用</el-tag>
            <el-tag v-else size="small" type="danger">禁用</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="createdBy" label="创建人" width="120" show-overflow-tooltip />
        <el-table-column prop="updatedAt" label="更新时间" width="180" show-overflow-tooltip />
        <el-table-column label="操作" width="220" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="hasPermission('notification:manage')"
              size="small" link type="success"
              :loading="testingId === row.id"
              @click="onTest(row)"
            >测试</el-button>
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
        <el-empty description="暂无通知通道" />
      </div>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑通道' : '新增通道'"
      width="600px"
      @closed="resetForm"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-form-item label="通道名称" prop="name">
          <el-input v-model="form.name" placeholder="如 银行运维邮箱组" />
        </el-form-item>
        <el-form-item label="通道类型" prop="channelType">
          <el-radio-group v-model="form.channelType" :disabled="isEdit" @change="onTypeChange">
            <el-radio-button label="email">邮件</el-radio-button>
            <el-radio-button label="feishu">飞书</el-radio-button>
            <el-radio-button label="webhook">Webhook</el-radio-button>
          </el-radio-group>
        </el-form-item>

        <!-- 邮件配置 -->
        <template v-if="form.channelType === 'email'">
          <el-form-item label="SMTP 主机" prop="email.smtpHost">
            <el-input v-model="form.email.smtpHost" placeholder="smtp.example.com" />
          </el-form-item>
          <el-form-item label="SMTP 端口" prop="email.smtpPort">
            <el-input-number v-model="form.email.smtpPort" :min="1" :max="65535" controls-position="right" />
          </el-form-item>
          <el-form-item label="用户名" prop="email.username">
            <el-input v-model="form.email.username" placeholder="发件账号" />
          </el-form-item>
          <el-form-item label="密码" prop="email.password">
            <el-input v-model="form.email.password" type="password" show-password placeholder="发件账号密码" />
          </el-form-item>
          <el-form-item label="发件地址" prop="email.fromAddr">
            <el-input v-model="form.email.fromAddr" placeholder="noreply@example.com" />
          </el-form-item>
          <el-form-item label="发件人名">
            <el-input v-model="form.email.fromName" placeholder="MeridianOps 告警通知" />
          </el-form-item>
          <el-form-item label="启用 TLS">
            <el-switch v-model="form.email.useTls" />
          </el-form-item>
        </template>

        <!-- 飞书配置 -->
        <template v-if="form.channelType === 'feishu'">
          <el-form-item label="Webhook URL" prop="feishu.webhookUrl">
            <el-input v-model="form.feishu.webhookUrl" placeholder="https://open.feishu.cn/open-apis/bot/v2/hook/xxx" />
          </el-form-item>
          <el-form-item label="签名密钥">
            <el-input v-model="form.feishu.secret" type="password" show-password placeholder="可选，飞书机器人安全设置中的签名校验" />
          </el-form-item>
        </template>

        <!-- 通用 Webhook 配置 -->
        <template v-if="form.channelType === 'webhook'">
          <el-form-item label="URL" prop="webhook.url">
            <el-input v-model="form.webhook.url" placeholder="https://your-system/hook" />
          </el-form-item>
          <el-form-item label="请求头">
            <el-input
              v-model="webhookHeadersText"
              type="textarea"
              :rows="4"
              placeholder='JSON 格式，如 {"Authorization": "Bearer xxx"}'
            />
          </el-form-item>
        </template>

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
import { Plus, Refresh, Message } from '@element-plus/icons-vue'
import {
  listChannels,
  createChannel,
  updateChannel,
  deleteChannel,
  testChannel,
  type NotificationChannel,
} from '../../api/notification'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()
const hasPermission = (code: string) => userStore.hasPermission(code)

const loading = ref(false)
const list = ref<NotificationChannel[]>([])
const testingId = ref<string | null>(null)

async function fetchList() {
  loading.value = true
  try {
    list.value = await listChannels()
  } catch (e: any) {
    ElMessage.error(e?.message || '加载通道失败')
  } finally {
    loading.value = false
  }
}

// ---- 通道类型映射 ----
function channelTypeLabel(t: string): string {
  return { email: '邮件', feishu: '飞书', webhook: 'Webhook' }[t] || t
}
function channelTypeTagType(t: string) {
  return ({ email: 'primary', feishu: 'success', webhook: 'warning' } as Record<string, any>)[t] || 'info'
}
function configSummary(c: NotificationChannel): string {
  if (!c.config || typeof c.config !== 'object') return '—'
  const cfg = c.config as Record<string, any>
  if (c.channelType === 'email') return `${cfg.smtpHost || '?'}:${cfg.smtpPort || '?'} · ${cfg.fromAddr || '?'}`
  if (c.channelType === 'feishu') {
    const u = cfg.webhookUrl || ''
    return u.length > 40 ? u.slice(0, 40) + '...' : u || '—'
  }
  if (c.channelType === 'webhook') return cfg.url || '—'
  return '—'
}

// ---- 对话框 ----
const dialogVisible = ref(false)
const isEdit = ref(false)
const saving = ref(false)
const formRef = ref<FormInstance>()

const form = reactive({
  id: '',
  name: '',
  channelType: 'email' as 'email' | 'feishu' | 'webhook',
  email: {
    smtpHost: '',
    smtpPort: 465,
    username: '',
    password: '',
    fromAddr: '',
    fromName: '',
    useTls: true,
  },
  feishu: {
    webhookUrl: '',
    secret: '',
  },
  webhook: {
    url: '',
    headers: {} as Record<string, string>,
  },
  enabled: true,
})

const webhookHeadersText = ref('')

const rules: FormRules = {
  name: [{ required: true, message: '请输入通道名称', trigger: 'blur' }],
  channelType: [{ required: true, message: '请选择通道类型', trigger: 'change' }],
}

function onTypeChange() {
  // 切换类型时无需特殊处理，模板已按类型显隐
}

function openCreate() {
  isEdit.value = false
  form.id = ''
  form.name = ''
  form.channelType = 'email'
  form.email = { smtpHost: '', smtpPort: 465, username: '', password: '', fromAddr: '', fromName: '', useTls: true }
  form.feishu = { webhookUrl: '', secret: '' }
  form.webhook = { url: '', headers: {} }
  form.enabled = true
  webhookHeadersText.value = ''
  dialogVisible.value = true
}

function openEdit(row: NotificationChannel) {
  isEdit.value = true
  form.id = row.id
  form.name = row.name
  form.channelType = row.channelType as any
  form.enabled = row.enabled
  const cfg = (row.config || {}) as Record<string, any>
  if (row.channelType === 'email') {
    form.email = {
      smtpHost: cfg.smtpHost || '',
      smtpPort: Number(cfg.smtpPort) || 465,
      username: cfg.username || '',
      // 编辑时密码不返回，留空表示不改
      password: '',
      fromAddr: cfg.fromAddr || '',
      fromName: cfg.fromName || '',
      useTls: cfg.useTls !== false,
    }
  } else if (row.channelType === 'feishu') {
    form.feishu = { webhookUrl: cfg.webhookUrl || '', secret: cfg.secret || '' }
  } else if (row.channelType === 'webhook') {
    form.webhook = { url: cfg.url || '', headers: (cfg.headers as any) || {} }
    webhookHeadersText.value = form.webhook.headers && Object.keys(form.webhook.headers).length
      ? JSON.stringify(form.webhook.headers, null, 2)
      : ''
  }
  dialogVisible.value = true
}

function resetForm() {
  formRef.value?.resetFields()
}

function buildConfig(): Record<string, any> {
  if (form.channelType === 'email') {
    // 编辑时空密码表示不修改，但通道配置必须完整，故仍下发原字段占位
    return { ...form.email }
  }
  if (form.channelType === 'feishu') return { ...form.feishu }
  if (form.channelType === 'webhook') {
    let headers = {}
    try {
      headers = webhookHeadersText.value.trim() ? JSON.parse(webhookHeadersText.value) : {}
    } catch {
      throw new Error('请求头 JSON 格式错误')
    }
    return { url: form.webhook.url, headers }
  }
  return {}
}

async function onSubmit() {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    let configObj: Record<string, any>
    try {
      configObj = buildConfig()
    } catch (e: any) {
      ElMessage.error(e?.message || '配置格式错误')
      return
    }
    saving.value = true
    try {
      if (isEdit.value) {
        await updateChannel(form.id, {
          name: form.name.trim(),
          channelType: form.channelType,
          config: configObj,
          enabled: form.enabled,
        })
        ElMessage.success('更新成功')
      } else {
        await createChannel({
          name: form.name.trim(),
          channelType: form.channelType,
          config: configObj,
          enabled: form.enabled,
        })
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

async function onTest(row: NotificationChannel) {
  testingId.value = row.id
  try {
    const r = await testChannel(row.id)
    ElMessage.success(r?.message || '测试成功')
  } catch (e: any) {
    ElMessage.error(e?.message || '测试失败')
  } finally {
    testingId.value = null
  }
}

async function onDelete(row: NotificationChannel) {
  try {
    await ElMessageBox.confirm(
      `确定删除通道「${row.name}」吗？若通知规则引用该通道将无法分发。`,
      '删除确认',
      { type: 'warning' },
    )
    await deleteChannel(row.id)
    ElMessage.success('删除成功')
    await fetchList()
  } catch (e: any) {
    if (e !== 'cancel' && e?.message) ElMessage.error(e.message)
  }
}

onMounted(() => {
  fetchList()
})
</script>

<style scoped>
.channels-page { padding: 0; }

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

.config-summary {
  color: #606266;
  font-size: 12px;
}

.empty-tip { padding: 20px 0; }
</style>
