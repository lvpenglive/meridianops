<template>
  <div class="sync-page">
    <!-- 顶部：数据源卡片 -->
    <div class="page-header">
      <div class="header-title">
        <h2>数据源同步</h2>
        <span class="header-desc">管理外部数据源拉取配置，支持手动触发与定时拉取</span>
      </div>
      <div class="header-actions">
        <el-button v-if="hasPermission('system:update')" type="primary" :icon="Plus" @click="openCreate">
          新增数据源
        </el-button>
        <el-button :icon="Refresh" circle @click="fetchSources" />
      </div>
    </div>

    <div v-loading="sourcesLoading" class="sources-row">
      <div v-for="src in sources" :key="src.code" class="source-card">
        <div class="source-head">
          <div class="source-name">
            <el-icon :size="22" :color="src.enabled ? '#67c23a' : '#909399'">
              <component :is="sourceIcon(src)" />
            </el-icon>
            <span>{{ src.name }}</span>
          </div>
          <div class="source-tags">
            <el-tag size="small" :type="src.sourceType === 'pull' ? 'warning' : 'success'" effect="plain">
              {{ src.sourceType === 'pull' ? '拉取' : '推送' }}
            </el-tag>
            <el-tag v-if="!src.enabled" size="small" type="info">已禁用</el-tag>
            <el-tag v-if="src.pullEnabled" size="small" type="primary">定时</el-tag>
          </div>
        </div>

        <div class="source-meta">
          <div class="meta-row">
            <span class="meta-label">API 地址</span>
            <span class="meta-value">{{ src.apiUrl || '—' }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">拉取路径</span>
            <span class="meta-value">{{ pullPath(src) || '—' }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">定时 Cron</span>
            <span class="meta-value">{{ src.pullCron || '—' }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">最后同步</span>
            <span class="meta-value">
              <el-tag v-if="src.lastSyncStatus" size="small" :type="syncStatusType(src.lastSyncStatus)">
                {{ syncStatusLabel(src.lastSyncStatus) }}
              </el-tag>
              <span v-if="src.lastSyncAt" class="sync-time">{{ formatTime(src.lastSyncAt) }}（{{ src.lastSyncCount }} 条）</span>
              <span v-else class="text-muted">从未同步</span>
            </span>
          </div>
        </div>

        <div class="source-actions">
          <el-button
            v-if="hasPermission('asset:create')"
            type="primary"
            size="small"
            :loading="pullingCode === src.code"
            :disabled="!src.enabled || !src.apiUrl"
            @click="onPull(src)"
          >
            <el-icon><Download /></el-icon>&nbsp;手动拉取
          </el-button>
          <el-button
            v-if="hasPermission('system:update')"
            size="small"
            @click="openConfig(src)"
          >
            <el-icon><Setting /></el-icon>&nbsp;配置
          </el-button>
          <el-button size="small" link @click="viewLogs(src.code)">
            <el-icon><Document /></el-icon>&nbsp;日志
          </el-button>
          <el-button
            v-if="hasPermission('system:update')"
            size="small"
            type="danger"
            link
            @click="onDelete(src)"
          >
            <el-icon><Delete /></el-icon>&nbsp;删除
          </el-button>
        </div>
      </div>
      <el-empty v-if="!sourcesLoading && !sources.length" description="暂无数据源" />
    </div>

    <!-- 下方：同步日志 -->
    <el-card shadow="never" class="logs-card">
      <template #header>
        <div class="logs-header">
          <span class="logs-title">同步日志</span>
          <div class="logs-filter">
            <el-select v-model="logQuery.sourceCode" placeholder="数据源" clearable style="width: 160px" @change="fetchLogs">
              <el-option v-for="s in sources" :key="s.code" :label="s.name" :value="s.code" />
            </el-select>
            <el-select v-model="logQuery.status" placeholder="状态" clearable style="width: 120px" @change="fetchLogs">
              <el-option label="成功" value="success" />
              <el-option label="失败" value="failed" />
              <el-option label="跳过" value="skipped" />
            </el-select>
            <el-button :icon="Refresh" circle @click="fetchLogs" />
          </div>
        </div>
      </template>

      <el-table :data="logs.items" v-loading="logsLoading" stripe size="default">
        <el-table-column prop="createdAt" label="时间" width="170">
          <template #default="{ row }">{{ formatTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="数据源" width="110">
          <template #default="{ row }">{{ sourceName(row.sourceCode) }}</template>
        </el-table-column>
        <el-table-column prop="action" label="动作" width="90">
          <template #default="{ row }">
            <el-tag size="small" :type="row.action === 'pull' ? 'warning' : ''">{{ row.action }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="modelCode" label="模型" width="100" />
        <el-table-column prop="externalId" label="外部ID" width="130" show-overflow-tooltip />
        <el-table-column prop="instanceName" label="实例名" min-width="150" show-overflow-tooltip />
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag size="small" :type="logStatusType(row.status)">{{ row.status }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="message" label="消息" min-width="200" show-overflow-tooltip />
      </el-table>

      <div class="pagination-row">
        <el-pagination
          v-model:current-page="logQuery.page"
          v-model:page-size="logQuery.pageSize"
          :total="logs.total"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next"
          @size-change="fetchLogs"
          @current-change="fetchLogs"
        />
      </div>
    </el-card>

    <!-- 数据源配置对话框 -->
    <el-dialog v-model="configVisible" title="数据源拉取配置" width="640px">
      <el-form :model="configForm" label-width="120px">
        <el-form-item label="数据源">
          <el-input :model-value="configForm.name" disabled />
        </el-form-item>
        <el-form-item label="API 地址">
          <el-input v-model="configForm.apiUrl" placeholder="https://bk.example.com" />
        </el-form-item>
        <el-form-item label="API Token">
          <el-input v-model="configForm.apiToken" placeholder="外部系统访问令牌" show-password />
        </el-form-item>
        <el-form-item label="拉取路径">
          <el-input v-model="configForm.path" placeholder="/api/v3/host/list" />
        </el-form-item>
        <el-form-item label="请求方法">
          <el-radio-group v-model="configForm.method">
            <el-radio value="GET">GET</el-radio>
            <el-radio value="POST">POST</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="响应数据路径">
          <el-input v-model="configForm.responsePath" placeholder="data.info" />
          <div class="form-tip">JSON path 到数据项数组，如 data.info / data.list，留空表示响应根为数组</div>
        </el-form-item>
        <el-form-item label="CI 模型编码">
          <el-input v-model="configForm.modelCode" placeholder="host" />
        </el-form-item>
        <el-form-item label="定时拉取">
          <el-switch v-model="configForm.pullEnabled" />
        </el-form-item>
        <el-form-item v-if="configForm.pullEnabled" label="Cron 表达式">
          <el-input v-model="configForm.pullCron" placeholder="0 */2 * * *（每2小时）" />
          <div class="form-tip">5 字段 cron：分 时 日 月 周（标准 Unix cron）</div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="configVisible = false">取消</el-button>
        <el-button type="primary" :loading="configSaving" @click="onSaveConfig">保存配置</el-button>
      </template>
    </el-dialog>

    <!-- 新增数据源对话框 -->
    <el-dialog v-model="createVisible" title="新增数据源" width="640px">
      <el-form :model="createForm" label-width="120px">
        <el-form-item label="编码 code" required>
          <el-input v-model="createForm.code" placeholder="如 zabbix / prometheus / custom_cmdb" />
          <div class="form-tip">唯一标识，仅小写字母/数字/下划线，最长 32 字符，创建后不可修改</div>
        </el-form-item>
        <el-form-item label="名称" required>
          <el-input v-model="createForm.name" placeholder="如 Zabbix 监控" />
        </el-form-item>
        <el-form-item label="接入方式">
          <el-radio-group v-model="createForm.sourceType">
            <el-radio value="webhook">Webhook 推送</el-radio>
            <el-radio value="pull">主动拉取</el-radio>
          </el-radio-group>
          <div class="form-tip">webhook：外部系统主动推送；pull：本系统定时或手动拉取</div>
        </el-form-item>
        <el-form-item label="API 地址">
          <el-input v-model="createForm.apiUrl" placeholder="https://example.com（pull 模式必填）" />
        </el-form-item>
        <el-form-item label="API Token">
          <el-input v-model="createForm.apiToken" placeholder="外部系统访问令牌" show-password />
        </el-form-item>
        <el-form-item label="Webhook 密钥">
          <el-input v-model="createForm.webhookSecret" placeholder="webhook 签名校验密钥（可选）" show-password />
        </el-form-item>
        <el-divider content-position="left">拉取配置（pull 模式）</el-divider>
        <el-form-item label="拉取路径">
          <el-input v-model="createForm.path" placeholder="/api/v3/host/list" />
        </el-form-item>
        <el-form-item label="请求方法">
          <el-radio-group v-model="createForm.method">
            <el-radio value="GET">GET</el-radio>
            <el-radio value="POST">POST</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="响应数据路径">
          <el-input v-model="createForm.responsePath" placeholder="data.info" />
          <div class="form-tip">JSON path 到数据项数组，留空表示响应根为数组</div>
        </el-form-item>
        <el-form-item label="CI 模型编码">
          <el-input v-model="createForm.modelCode" placeholder="host" />
        </el-form-item>
        <el-form-item label="定时拉取">
          <el-switch v-model="createForm.pullEnabled" />
        </el-form-item>
        <el-form-item v-if="createForm.pullEnabled" label="Cron 表达式">
          <el-input v-model="createForm.pullCron" placeholder="0 */2 * * *（每2小时）" />
          <div class="form-tip">5 字段 cron：分 时 日 月 周</div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="createSaving" @click="onCreate">创建</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Download, Setting, Document, Connection, Promotion, Plus, Delete } from '@element-plus/icons-vue'
import {
  listSyncSources,
  createSyncSource,
  deleteSyncSource,
  pullInstances,
  updateSyncSource,
  listSyncLogs,
} from '../../api/cmdb'
import type { SyncSource, SyncLog, SyncLogPage } from '../../api/types'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()
const hasPermission = (code: string) => userStore.hasPermission(code)

// ---- 数据源列表 ----
const sources = ref<SyncSource[]>([])
const sourcesLoading = ref(false)

async function fetchSources() {
  sourcesLoading.value = true
  try {
    sources.value = await listSyncSources()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '加载数据源失败')
  } finally {
    sourcesLoading.value = false
  }
}

function sourceName(code: string): string {
  return sources.value.find(s => s.code === code)?.name ?? code
}

function sourceIcon(src: SyncSource) {
  // 推送模式用 Promotion 图标，拉取模式用 Connection
  return src.sourceType === 'pull' ? Connection : Promotion
}

/** 从 pullConfig 中提取拉取路径（兼容对象/字符串） */
function pullPath(src: SyncSource): string {
  if (!src.pullConfig) return ''
  const cfg = typeof src.pullConfig === 'string'
    ? safeParse(src.pullConfig)
    : src.pullConfig as Record<string, unknown>
  return (cfg.path as string) || ''
}

function safeParse(s: string): Record<string, unknown> {
  try { return JSON.parse(s) } catch { return {} }
}

// ---- 状态辅助 ----
function syncStatusType(s: string): string {
  if (s === 'success') return 'success'
  if (s === 'partial') return 'warning'
  if (s === 'failed') return 'danger'
  return 'info'
}
function syncStatusLabel(s: string): string {
  const map: Record<string, string> = { success: '成功', partial: '部分', failed: '失败' }
  return map[s] ?? s
}
function logStatusType(s: string): string {
  if (s === 'success') return 'success'
  if (s === 'failed') return 'danger'
  if (s === 'skipped') return 'info'
  return 'info'
}
function formatTime(t: string): string {
  if (!t) return '—'
  try { return new Date(t).toLocaleString('zh-CN', { hour12: false }) } catch { return t }
}

// ---- 手动拉取 ----
const pullingCode = ref('')

async function onPull(src: SyncSource) {
  const modelCode = pullModelCode(src)
  if (!modelCode) {
    ElMessage.warning('数据源未配置 modelCode，无法拉取')
    return
  }
  try {
    pullingCode.value = src.code
    const result = await pullInstances({ source: src.code, modelCode })
    ElMessage.success(`拉取完成：共 ${result.total} 条，成功 ${result.success} 条，失败 ${result.failed} 条`)
    fetchSources()
    fetchLogs()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '拉取失败')
  } finally {
    pullingCode.value = ''
  }
}

function pullModelCode(src: SyncSource): string {
  if (!src.pullConfig) return ''
  const cfg = typeof src.pullConfig === 'string'
    ? safeParse(src.pullConfig)
    : src.pullConfig as Record<string, unknown>
  return (cfg.modelCode as string) || ''
}

// ---- 配置对话框 ----
const configVisible = ref(false)
const configSaving = ref(false)
const configForm = reactive({
  code: '',
  name: '',
  apiUrl: '',
  apiToken: '',
  path: '',
  method: 'GET',
  responsePath: 'data.info',
  modelCode: 'host',
  pullEnabled: false,
  pullCron: '0 */2 * * *',
})

function openConfig(src: SyncSource) {
  configForm.code = src.code
  configForm.name = src.name
  configForm.apiUrl = src.apiUrl
  configForm.apiToken = src.apiToken
  configForm.pullEnabled = src.pullEnabled
  configForm.pullCron = src.pullCron || '0 */2 * * *'
  // 解析 pullConfig（兼容对象/字符串）
  let cfg: Record<string, unknown> = {}
  if (src.pullConfig) {
    cfg = typeof src.pullConfig === 'string'
      ? safeParse(src.pullConfig)
      : src.pullConfig as Record<string, unknown>
  }
  configForm.path = (cfg.path as string) || ''
  configForm.method = (cfg.method as string) || 'GET'
  configForm.responsePath = (cfg.responsePath as string) || 'data.info'
  configForm.modelCode = (cfg.modelCode as string) || 'host'
  configVisible.value = true
}

async function onSaveConfig() {
  const pullConfig = JSON.stringify({
    method: configForm.method,
    path: configForm.path,
    responsePath: configForm.responsePath,
    modelCode: configForm.modelCode,
  })
  try {
    configSaving.value = true
    await updateSyncSource(configForm.code, {
      apiUrl: configForm.apiUrl,
      apiToken: configForm.apiToken,
      pullConfig,
      pullCron: configForm.pullCron,
      pullEnabled: configForm.pullEnabled,
    })
    ElMessage.success('配置已保存')
    configVisible.value = false
    await fetchSources()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '保存失败')
  } finally {
    configSaving.value = false
  }
}

// ---- 同步日志 ----
const logs = reactive<SyncLogPage>({ total: 0, page: 1, pageSize: 20, items: [] as SyncLog[] })
const logsLoading = ref(false)
const logQuery = reactive({
  sourceCode: '' as string,
  status: '' as string,
  page: 1,
  pageSize: 20,
})

async function fetchLogs() {
  logsLoading.value = true
  try {
    const params = {
      sourceCode: logQuery.sourceCode || undefined,
      status: logQuery.status || undefined,
      page: logQuery.page,
      pageSize: logQuery.pageSize,
    }
    const res = await listSyncLogs(params)
    logs.items = res.items
    logs.total = res.total
    logs.page = res.page
    logs.pageSize = res.pageSize
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '加载日志失败')
  } finally {
    logsLoading.value = false
  }
}

function viewLogs(code: string) {
  logQuery.sourceCode = code
  logQuery.page = 1
  fetchLogs()
}

// ---- 新增数据源 ----
const createVisible = ref(false)
const createSaving = ref(false)
const createForm = reactive({
  code: '',
  name: '',
  sourceType: 'pull' as 'webhook' | 'pull',
  apiUrl: '',
  apiToken: '',
  webhookSecret: '',
  path: '',
  method: 'GET',
  responsePath: 'data.info',
  modelCode: 'host',
  pullEnabled: false,
  pullCron: '0 */2 * * *',
})

function openCreate() {
  // 重置表单到默认值
  createForm.code = ''
  createForm.name = ''
  createForm.sourceType = 'pull'
  createForm.apiUrl = ''
  createForm.apiToken = ''
  createForm.webhookSecret = ''
  createForm.path = ''
  createForm.method = 'GET'
  createForm.responsePath = 'data.info'
  createForm.modelCode = 'host'
  createForm.pullEnabled = false
  createForm.pullCron = '0 */2 * * *'
  createVisible.value = true
}

async function onCreate() {
  // 前端基础校验
  if (!createForm.code.trim() || !createForm.name.trim()) {
    ElMessage.warning('编码和名称不能为空')
    return
  }
  if (!/^[a-z0-9_]{1,32}$/.test(createForm.code)) {
    ElMessage.warning('编码只能包含小写字母、数字、下划线，最长 32 字符')
    return
  }
  // pull 模式且启用定时，必须有 cron
  if (createForm.sourceType === 'pull' && createForm.pullEnabled && !createForm.pullCron.trim()) {
    ElMessage.warning('启用定时拉取时必须填写 Cron 表达式')
    return
  }
  // pull 模式必须有 apiUrl
  if (createForm.sourceType === 'pull' && !createForm.apiUrl.trim()) {
    ElMessage.warning('拉取模式必须填写 API 地址')
    return
  }

  const pullConfig = JSON.stringify({
    method: createForm.method,
    path: createForm.path,
    responsePath: createForm.responsePath,
    modelCode: createForm.modelCode,
  })
  try {
    createSaving.value = true
    await createSyncSource({
      code: createForm.code.trim(),
      name: createForm.name.trim(),
      sourceType: createForm.sourceType,
      apiUrl: createForm.apiUrl,
      apiToken: createForm.apiToken,
      webhookSecret: createForm.webhookSecret,
      pullConfig,
      pullCron: createForm.pullCron,
      pullEnabled: createForm.pullEnabled,
    })
    ElMessage.success('数据源已创建')
    createVisible.value = false
    await fetchSources()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '创建失败')
  } finally {
    createSaving.value = false
  }
}

// ---- 删除数据源 ----
async function onDelete(src: SyncSource) {
  try {
    await ElMessageBox.confirm(
      `确定要删除数据源「${src.name}」(${src.code}) 吗？\n若有关联的 CI 实例或同步日志将拒绝删除。`,
      '删除确认',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' },
    )
  } catch {
    return // 用户取消
  }
  try {
    await deleteSyncSource(src.code)
    ElMessage.success('数据源已删除')
    fetchSources()
    fetchLogs()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '删除失败')
  }
}

// ---- 初始化 ----
onMounted(async () => {
  await fetchSources()
  await fetchLogs()
})
</script>

<style scoped>
.sync-page { padding: 0; }

/* 页头 */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
.header-title h2 {
  margin: 0;
  font-size: 20px;
  color: #303133;
}
.header-desc {
  display: block;
  font-size: 13px;
  color: #909399;
  margin-top: 4px;
}

/* 数据源卡片 */
.sources-row {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
  gap: 16px;
  margin-bottom: 20px;
}
.source-card {
  background: #fff;
  border-radius: 10px;
  box-shadow: 0 1px 4px rgba(0,0,0,0.06);
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.source-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.source-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}
.source-tags {
  display: flex;
  gap: 6px;
}
.source-meta {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.meta-row {
  display: flex;
  font-size: 13px;
  line-height: 1.5;
}
.meta-label {
  width: 80px;
  color: #909399;
  flex-shrink: 0;
}
.meta-value {
  color: #606266;
  word-break: break-all;
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.sync-time { font-size: 12px; color: #909399; }
.text-muted { color: #c0c4cc; font-size: 12px; }

.source-actions {
  display: flex;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px solid #f0f0f0;
}

/* 日志卡片 */
.logs-card { margin-top: 4px; }
.logs-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.logs-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
}
.logs-filter {
  display: flex;
  gap: 8px;
}

/* 分页 */
.pagination-row {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

/* 表单提示 */
.form-tip {
  font-size: 12px;
  color: #909399;
  line-height: 1.4;
  margin-top: 4px;
}
</style>
