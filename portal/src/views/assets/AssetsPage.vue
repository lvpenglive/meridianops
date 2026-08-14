<template>
  <div class="assets-page">
    <!-- 统计卡片 -->
    <div class="stats-row">
      <div class="stat-card" :class="{ active: !selectedModelId }" @click="filterByModel('')">
        <div class="stat-icon" style="background: linear-gradient(135deg, #667eea, #764ba2)">
          <el-icon><Grid /></el-icon>
        </div>
        <div class="stat-info">
          <span class="stat-label">全部资产</span>
          <span class="stat-value">{{ stats?.total ?? 0 }}</span>
        </div>
      </div>
      <div
        v-for="m in stats?.byModel"
        :key="m.modelId"
        class="stat-card"
        :class="{ active: selectedModelId === m.modelId }"
        @click="filterByModel(m.modelId)"
      >
        <div class="stat-icon" :style="iconStyle(m.modelCode)">
          <el-icon><component :is="iconComp(m.icon)" /></el-icon>
        </div>
        <div class="stat-info">
          <span class="stat-label">{{ m.modelName }}</span>
          <span class="stat-value">{{ m.count }}</span>
        </div>
      </div>
    </div>

    <!-- 资产列表 -->
    <el-card shadow="never">
      <template #header>
        <div class="toolbar">
          <div class="toolbar-left">
            <el-select
              v-model="statusFilter"
              placeholder="状态"
              clearable
              style="width: 120px"
              @change="fetchList"
            >
              <el-option label="运行中" value="running" />
              <el-option label="已停止" value="stopped" />
              <el-option label="维护中" value="maintenance" />
              <el-option label="未知" value="unknown" />
            </el-select>
            <el-input
              v-model="keyword"
              placeholder="搜索资产名称..."
              clearable
              style="width: 220px"
              @keyup.enter="onSearch"
              @clear="onSearch"
            >
              <template #prefix><el-icon><Search /></el-icon></template>
            </el-input>
            <el-button :icon="Search" @click="onSearch">搜索</el-button>
          </div>
          <div class="toolbar-right">
            <el-button :icon="Refresh" circle @click="fetchList" />
            <el-dropdown v-if="hasPermission('asset:create')" split-button type="success" :icon="Upload" @click="openImport">
              批量导入
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item @click="openImport">导入 Excel/CSV</el-dropdown-item>
                  <el-dropdown-item @click="downloadTemplate">下载导入模板</el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
            <el-dropdown type="warning" @command="exportData">
              <el-button type="warning" :icon="Download" :loading="exporting">导出<el-icon class="el-icon--right"><ArrowDown /></el-icon></el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="xlsx">导出 Excel</el-dropdown-item>
                  <el-dropdown-item command="csv">导出 CSV</el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
            <el-button
              v-if="hasPermission('asset:create')"
              type="primary"
              :icon="Plus"
              @click="openCreate"
            >
              新增资产
            </el-button>
          </div>
        </div>
      </template>

      <el-table :data="list.items" v-loading="loading" stripe size="default">
        <el-table-column prop="name" label="名称" min-width="180" show-overflow-tooltip>
          <template #default="{ row }">
            <el-link type="primary" :underline="false" @click="goToDetail(row.id)">{{ row.name }}</el-link>
          </template>
        </el-table-column>
        <el-table-column label="类型" width="120">
          <template #default="{ row }">
            <el-tag size="small" :type="modelTagType(row.modelId)">{{ modelName(row.modelId) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="statusTagType(row.status)" size="small">{{ statusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="关键属性" min-width="280">
          <template #default="{ row }">
            <span v-for="(val, key) in keyAttrs(row)" :key="key" class="attr-chip">
              <span class="attr-label">{{ key }}:</span> {{ val }}
            </span>
            <span v-if="!Object.keys(keyAttrs(row)).length" class="text-muted">—</span>
          </template>
        </el-table-column>
        <el-table-column prop="tags" label="标签" min-width="120" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.tags">{{ row.tags }}</span>
            <span v-else class="text-muted">—</span>
          </template>
        </el-table-column>
        <el-table-column label="数据来源" width="160">
          <template #default="{ row }">
            <div class="source-cell">
              <el-tag v-if="row.source" size="small" type="warning" effect="plain">{{ sourceLabel(row.source) }}</el-tag>
              <el-tag v-else size="small" type="info" effect="plain">手工录入</el-tag>
              <span v-if="row.lastSyncedAt" class="sync-time">同步于 {{ formatTime(row.lastSyncedAt) }}</span>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="updatedAt" label="更新时间" width="170">
          <template #default="{ row }">{{ formatTime(row.updatedAt) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link type="primary" :icon="View" @click="goToDetail(row.id)">查看</el-button>
            <el-button v-if="hasPermission('asset:update')" size="small" link type="primary" @click="openEdit(row)">编辑</el-button>
            <el-button v-if="hasPermission('asset:delete')" size="small" link type="danger" @click="onDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-row">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :total="list.total"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next"
          @size-change="fetchList"
          @current-change="fetchList"
        />
      </div>
    </el-card>

    <!-- 新增/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑资产' : '新增资产'"
      width="640px"
      @closed="onDialogClosed"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <div class="form-section">
          <div class="section-title">基本信息</div>
          <el-form-item v-if="!isEdit" label="资产类型" prop="modelId">
            <el-select v-model="formData.modelId" placeholder="选择 CI 模型" style="width: 100%" @change="onModelChange">
              <el-option v-for="m in models" :key="m.id" :label="m.name" :value="m.id">
                <el-icon style="vertical-align: middle"><component :is="iconComp(m.icon)" /></el-icon>
                <span style="margin-left: 6px">{{ m.name }}</span>
              </el-option>
            </el-select>
          </el-form-item>
          <el-form-item v-else label="资产类型">
            <el-tag>{{ modelName(formData.modelId) }}</el-tag>
          </el-form-item>
          <el-form-item label="名称" prop="name">
            <el-input v-model="formData.name" placeholder="资产名称" clearable />
          </el-form-item>
          <el-form-item label="状态" prop="status">
            <el-select v-model="formData.status" style="width: 100%">
              <el-option label="运行中" value="running" />
              <el-option label="已停止" value="stopped" />
              <el-option label="维护中" value="maintenance" />
              <el-option label="未知" value="unknown" />
            </el-select>
          </el-form-item>
          <el-form-item label="标签">
            <el-input v-model="formData.tags" placeholder="多个标签用逗号分隔" clearable />
          </el-form-item>
        </div>

        <div v-if="currentAttrs.length" class="form-section">
          <div class="section-title">扩展属性</div>
          <el-form-item
            v-for="attr in currentAttrs"
            :key="attr.code"
            :label="attr.name"
            :prop="'attributes.' + attr.code"
            :rules="attr.isRequired ? [{ required: true, message: `${attr.name}不能为空`, trigger: 'blur' }] : []"
          >
            <el-select
              v-if="attr.valueType === 'enum'"
              v-model="formData.attributes[attr.code]"
              :placeholder="`选择${attr.name}`"
              clearable
              style="width: 100%"
            >
              <el-option v-for="opt in attr.options || []" :key="opt" :label="opt" :value="opt" />
            </el-select>
            <el-input-number
              v-else-if="attr.valueType === 'number'"
              v-model="formData.attributes[attr.code]"
              :controls="false"
              style="width: 100%"
            />
            <el-switch
              v-else-if="attr.valueType === 'boolean'"
              v-model="formData.attributes[attr.code]"
            />
            <el-input
              v-else
              v-model="formData.attributes[attr.code]"
              :placeholder="`输入${attr.name}`"
              clearable
            />
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onSubmit">确定</el-button>
      </template>
    </el-dialog>

    <!-- 批量导入对话框 -->
    <el-dialog
      v-model="importDialogVisible"
      title="批量导入资产"
      width="860px"
      :close-on-click-modal="false"
      @closed="onImportDialogClosed"
    >
      <div v-if="importStep === 'upload'" class="import-step">
        <el-form label-width="100px">
          <el-form-item label="资产类型" required>
            <el-select v-model="importModelId" placeholder="选择 CI 模型" style="width: 100%" @change="onImportModelChange">
              <el-option v-for="m in models" :key="m.id" :label="m.name" :value="m.id" />
            </el-select>
          </el-form-item>
        </el-form>

        <div class="upload-hint">
          <el-alert type="info" :closable="false" show-icon>
            <template #title>
              支持上传 .xlsx / .xls / .csv 文件，单次最多导入 1000 条。
              请先点击「下载导入模板」获取标准格式，按列填写后上传。
            </template>
          </el-alert>
        </div>

        <el-upload
          ref="uploadRef"
          drag
          accept=".xlsx,.xls,.csv"
          :auto-upload="false"
          :limit="1"
          :on-exceed="onUploadExceed"
          :on-change="onUploadChange"
          :on-remove="onUploadRemove"
          :file-list="fileList"
        >
          <el-icon class="el-icon--upload"><UploadFilled /></el-icon>
          <div class="el-upload__text">拖拽文件到此处 或 <em>点击选择</em></div>
          <template #tip>
            <div class="el-upload__tip">仅支持单个文件，解析后可预览前 10 行数据</div>
          </template>
        </el-upload>

        <div v-if="importAttrs.length" class="attr-mapping">
          <div class="section-title">字段说明（按模型属性自动匹配）</div>
          <div class="attr-mapping-list">
            <el-tag v-for="a in importAttrs" :key="a.code" class="attr-tag">
              {{ a.name }}（{{ a.code }}）
              <span v-if="a.isRequired" class="req-mark">*</span>
            </el-tag>
          </div>
        </div>
      </div>

      <div v-else-if="importStep === 'preview'" class="import-step">
        <div class="preview-header">
          <el-tag type="success">解析成功：共 {{ importItems.length }} 条数据</el-tag>
          <el-button link type="primary" @click="resetUpload">重新选择文件</el-button>
        </div>
        <el-table :data="importItems.slice(0, 10)" border size="small" max-height="360" style="margin-top: 12px">
          <el-table-column type="index" label="#" width="50" fixed />
          <el-table-column prop="name" label="名称" min-width="140" show-overflow-tooltip />
          <el-table-column prop="status" label="状态" width="90" />
          <el-table-column prop="tags" label="标签" min-width="120" show-overflow-tooltip />
          <el-table-column
            v-for="a in importAttrs"
            :key="a.code"
            :label="a.name"
            min-width="110"
            show-overflow-tooltip
          >
            <template #default="{ row }">
              {{ row.attributes?.[a.code] ?? '—' }}
            </template>
          </el-table-column>
        </el-table>
        <div v-if="importItems.length > 10" class="preview-more">
          仅显示前 10 行，共 {{ importItems.length }} 条
        </div>
      </div>

      <div v-else-if="importStep === 'result'" class="import-step">
        <el-result
          :icon="importResult?.status === 'success' ? 'success' : (importResult?.status === 'failed' ? 'error' : 'warning')"
          :title="importResultTitle"
          :sub-title="`共 ${importResult?.total} 条，成功 ${importResult?.success} 条，失败 ${importResult?.failed} 条`"
        />
        <el-table
          v-if="importResult?.errors?.length"
          :data="importResult.errors"
          border
          size="small"
          max-height="280"
          style="margin-top: 8px"
        >
          <el-table-column prop="row" label="行号" width="70" />
          <el-table-column prop="name" label="名称" min-width="140" show-overflow-tooltip />
          <el-table-column prop="message" label="失败原因" min-width="220" show-overflow-tooltip />
        </el-table>
      </div>

      <template #footer>
        <el-button @click="importDialogVisible = false">关闭</el-button>
        <el-button
          v-if="importStep === 'upload'"
          type="primary"
          :disabled="!importModelId || !parsedReady"
          @click="goPreview"
        >
          下一步：预览
        </el-button>
        <el-button
          v-if="importStep === 'preview'"
          type="primary"
          :loading="importing"
          @click="submitImport"
        >
          开始导入（{{ importItems.length }} 条）
        </el-button>
        <el-button
          v-if="importStep === 'result' && importResult?.status !== 'success'"
          type="primary"
          @click="resetUpload"
        >
          重新导入
        </el-button>
      </template>
    </el-dialog>

  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules, type UploadFile, type UploadFiles, type UploadInstance } from 'element-plus'
import { Plus, Search, Refresh, Grid, Monitor, Cpu, Coin, Connection, Upload, UploadFilled, View, Download } from '@element-plus/icons-vue'
import * as XLSX from 'xlsx'
import {
  listCiModels,
  getCiModel,
  listCiInstances,
  createCiInstance,
  updateCiInstance,
  deleteCiInstance,
  batchCreateInstances,
  getCmdbStats,
} from '../../api/cmdb'
import type { CiModel, CiModelAttr, CiInstance, CmdbStats, BatchInstanceItem, BatchImportResult } from '../../api/types'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()
const router = useRouter()
const hasPermission = (code: string) => userStore.hasPermission(code)

const goToDetail = (id: string) => router.push(`/assets/${id}`)

// ---- 统计 ----
const stats = ref<CmdbStats | null>(null)
async function fetchStats() {
  try {
    stats.value = await getCmdbStats()
  } catch { /* ignore */ }
}

// ---- 模型列表 ----
const models = ref<CiModel[]>([])
const modelAttrCache = ref<Record<string, CiModelAttr[]>>({})

async function fetchModels() {
  models.value = await listCiModels()
}

/** 获取某模型的属性定义（带缓存） */
async function getModelAttrs(modelId: string): Promise<CiModelAttr[]> {
  if (modelAttrCache.value[modelId]) return modelAttrCache.value[modelId]
  const detail = await getCiModel(modelId)
  modelAttrCache.value[modelId] = detail.attributes
  return detail.attributes
}

function modelName(id: string): string {
  return models.value.find(m => m.id === id)?.name ?? id
}

function modelTagType(id: string): string {
  const code = models.value.find(m => m.id === id)?.code ?? ''
  const map: Record<string, string> = {
    business_system: 'warning',
    host: '',
    middleware: 'success',
    database: 'danger',
    network_device: 'info',
  }
  return map[code] || 'info'
}

// ---- 列表 ----
const loading = ref(false)
const list = reactive({ items: [] as CiInstance[], total: 0 })
const selectedModelId = ref('')
const statusFilter = ref('')
const keyword = ref('')
const page = ref(1)
const pageSize = ref(20)

async function fetchList() {
  loading.value = true
  try {
    const res = await listCiInstances({
      modelId: selectedModelId.value || undefined,
      status: statusFilter.value || undefined,
      keyword: keyword.value || undefined,
      page: page.value,
      pageSize: pageSize.value,
    })
    list.items = res.items
    list.total = res.total
  } catch (e: any) {
    ElMessage.error(e?.message || '加载失败')
  } finally {
    loading.value = false
  }
}

function filterByModel(modelId: string) {
  selectedModelId.value = modelId
  page.value = 1
  fetchList()
}

function onSearch() {
  page.value = 1
  fetchList()
}

// ---- 表格辅助 ----
function statusLabel(s: string): string {
  const map: Record<string, string> = { running: '运行中', stopped: '已停止', maintenance: '维护中', unknown: '未知' }
  return map[s] ?? s
}
function statusTagType(s: string): string {
  const map: Record<string, string> = { running: 'success', stopped: 'danger', maintenance: 'warning', unknown: 'info' }
  return map[s] ?? 'info'
}

/** 数据来源显示名（source 编码 → 中文标签） */
function sourceLabel(s: string): string {
  const map: Record<string, string> = {
    blueking: '蓝鲸',
    external_cmdb: '外部CMDB',
  }
  return map[s] ?? s
}

/** 从 attributes 中提取关键属性（最多 3 个）展示在表格中 */
function keyAttrs(row: CiInstance): Record<string, string> {
  const attrs = row.attributes || {}
  const result: Record<string, string> = {}
  const labels: Record<string, string> = {
    hostname: '主机名', ip: 'IP', system_code: '编码', system_level: '等级',
    mw_type: '类型', version: '版本', db_type: '类型', instance: '实例',
    device_name: '设备名', mgmt_ip: '管理IP', os: 'OS',
  }
  for (const [k, v] of Object.entries(attrs)) {
    if (v !== null && v !== '' && v !== undefined && labels[k]) {
      result[labels[k]] = String(v)
      if (Object.keys(result).length >= 3) break
    }
  }
  return result
}

function formatTime(t: string): string {
  if (!t) return '—'
  try {
    return new Date(t).toLocaleString('zh-CN', { hour12: false })
  } catch {
    return t
  }
}

// ---- 图标 ----
const iconMap: Record<string, any> = {
  Grid, Monitor, Cpu, Coin, Connection,
}
function iconComp(name: string) {
  return iconMap[name] || Monitor
}
function iconStyle(code: string): Record<string, string> {
  const map: Record<string, string> = {
    business_system: 'linear-gradient(135deg, #f093fb, #f5576c)',
    host: 'linear-gradient(135deg, #4facfe, #00f2fe)',
    middleware: 'linear-gradient(135deg, #43e97b, #38f9d7)',
    database: 'linear-gradient(135deg, #fa709a, #fee140)',
    network_device: 'linear-gradient(135deg, #a8edea, #fed6e3)',
  }
  return { background: map[code] || 'linear-gradient(135deg, #667eea, #764ba2)' }
}

// ---- 新增/编辑对话框 ----
const dialogVisible = ref(false)
const isEdit = ref(false)
const saving = ref(false)
const formRef = ref<FormInstance>()
const currentAttrs = ref<CiModelAttr[]>([])

const formData = reactive({
  id: '',
  modelId: '',
  name: '',
  status: 'running',
  tags: '',
  attributes: {} as Record<string, any>,
})

const formRules: FormRules = {
  modelId: [{ required: true, message: '请选择资产类型', trigger: 'change' }],
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  status: [{ required: true, message: '请选择状态', trigger: 'change' }],
}

async function openCreate() {
  isEdit.value = false
  resetForm()
  if (selectedModelId.value) {
    formData.modelId = selectedModelId.value
    await onModelChange(selectedModelId.value)
  }
  dialogVisible.value = true
}

async function openEdit(row: CiInstance) {
  isEdit.value = true
  resetForm()
  formData.id = row.id
  formData.modelId = row.modelId
  formData.name = row.name
  formData.status = row.status
  formData.tags = row.tags
  formData.attributes = { ...(row.attributes || {}) }
  await onModelChange(row.modelId)
  dialogVisible.value = true
}

function resetForm() {
  formData.id = ''
  formData.modelId = ''
  formData.name = ''
  formData.status = 'running'
  formData.tags = ''
  formData.attributes = {}
  currentAttrs.value = []
}

async function onModelChange(modelId: string) {
  if (!modelId) {
    currentAttrs.value = []
    return
  }
  currentAttrs.value = await getModelAttrs(modelId)
  // 填充默认值
  for (const attr of currentAttrs.value) {
    if (formData.attributes[attr.code] === undefined) {
      if (attr.defaultValue) {
        formData.attributes[attr.code] = attr.valueType === 'number' ? Number(attr.defaultValue) : attr.defaultValue
      } else {
        formData.attributes[attr.code] = attr.valueType === 'number' ? undefined : ''
      }
    }
  }
}

function onDialogClosed() {
  resetForm()
  formRef.value?.resetFields()
}

async function onSubmit() {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    saving.value = true
    try {
      const payload = {
        name: formData.name.trim(),
        status: formData.status,
        tags: formData.tags,
        attributes: formData.attributes,
      }
      if (isEdit.value) {
        await updateCiInstance(formData.id, payload)
        ElMessage.success('更新成功')
      } else {
        await createCiInstance({
          modelId: formData.modelId,
          ...payload,
        })
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      fetchList()
      fetchStats()
    } catch (e: any) {
      ElMessage.error(e?.message || '操作失败')
    } finally {
      saving.value = false
    }
  })
}

async function onDelete(row: CiInstance) {
  try {
    await ElMessageBox.confirm(`确定删除资产「${row.name}」吗？关联的关系也将被删除。`, '删除确认', {
      type: 'warning',
    })
    await deleteCiInstance(row.id)
    ElMessage.success('删除成功')
    fetchList()
    fetchStats()
  } catch { /* cancelled */ }
}

// ---- 批量导入 ----
const importDialogVisible = ref(false)
const importStep = ref<'upload' | 'preview' | 'result'>('upload')
const importModelId = ref('')
const importAttrs = ref<CiModelAttr[]>([])
const fileList = ref<UploadFiles>([])
const uploadRef = ref<UploadInstance>()
const parsedReady = ref(false)
const importItems = ref<BatchInstanceItem[]>([])
const importing = ref(false)
const importResult = ref<BatchImportResult | null>(null)

const importResultTitle = computed(() => {
  const s = importResult.value?.status
  if (s === 'success') return '导入成功'
  if (s === 'failed') return '导入失败'
  return '部分导入成功'
})

function openImport() {
  resetImportState()
  // 如已筛选某模型，预填
  if (selectedModelId.value) {
    importModelId.value = selectedModelId.value
    onImportModelChange(selectedModelId.value)
  }
  importDialogVisible.value = true
}

function resetImportState() {
  importStep.value = 'upload'
  importModelId.value = ''
  importAttrs.value = []
  fileList.value = []
  parsedReady.value = false
  importItems.value = []
  importResult.value = null
}

function onImportDialogClosed() {
  resetImportState()
  uploadRef.value?.clearFiles()
}

async function onImportModelChange(modelId: string) {
  if (!modelId) {
    importAttrs.value = []
    return
  }
  try {
    importAttrs.value = await getModelAttrs(modelId)
  } catch {
    importAttrs.value = []
  }
}

function onUploadExceed() {
  ElMessage.warning('只能上传 1 个文件，请先移除已有文件')
}

async function onUploadChange(file: UploadFile) {
  if (!file.raw) return
  if (!importModelId.value) {
    ElMessage.warning('请先选择资产类型')
    fileList.value = []
    return
  }
  try {
    const buf = await file.raw.arrayBuffer()
    const wb = XLSX.read(buf, { type: 'array' })
    const sheet = wb.Sheets[wb.SheetNames[0]]
    if (!sheet) {
      ElMessage.error('文件没有有效的工作表')
      fileList.value = []
      return
    }
    const rows: Record<string, unknown>[] = XLSX.utils.sheet_to_json(sheet, { defval: '' })
    if (!rows.length) {
      ElMessage.error('文件没有数据行')
      fileList.value = []
      return
    }
    const items: BatchInstanceItem[] = rows.map((row) => {
      const attributes: Record<string, unknown> = {}
      for (const attr of importAttrs.value) {
        // 支持 code 或 name 作为表头
        const v = row[attr.code] ?? row[attr.name]
        if (v !== undefined && v !== null && String(v).trim() !== '') {
          attributes[attr.code] = v
        }
      }
      return {
        name: String(row['name'] ?? row['名称'] ?? '').trim(),
        status: (row['status'] ?? row['状态'] ?? '') ? String(row['status'] ?? row['状态']).trim() : undefined,
        tags: (row['tags'] ?? row['标签'] ?? '') ? String(row['tags'] ?? row['标签']).trim() : undefined,
        attributes,
      }
    })
    importItems.value = items
    parsedReady.value = true
    ElMessage.success(`解析成功，共 ${items.length} 条数据`)
  } catch (e: any) {
    ElMessage.error('文件解析失败：' + (e?.message || '未知错误'))
    fileList.value = []
    parsedReady.value = false
  }
}

function onUploadRemove() {
  parsedReady.value = false
  importItems.value = []
}

function goPreview() {
  if (!importItems.value.length) {
    ElMessage.warning('没有可预览的数据')
    return
  }
  importStep.value = 'preview'
}

function resetUpload() {
  importStep.value = 'upload'
  fileList.value = []
  parsedReady.value = false
  importItems.value = []
  importResult.value = null
  uploadRef.value?.clearFiles()
}

async function submitImport() {
  if (!importModelId.value || !importItems.value.length) return
  importing.value = true
  try {
    const res = await batchCreateInstances({
      modelId: importModelId.value,
      items: importItems.value,
    })
    importResult.value = res
    importStep.value = 'result'
    if (res.status === 'success') {
      ElMessage.success(`全部导入成功（${res.success} 条）`)
    } else if (res.status === 'partial') {
      ElMessage.warning(`部分成功：${res.success} 成功，${res.failed} 失败`)
    } else {
      ElMessage.error(`导入失败：${res.failed} 条全部失败`)
    }
    // 刷新列表与统计
    fetchList()
    fetchStats()
  } catch (e: any) {
    ElMessage.error('导入请求失败：' + (e?.message || '未知错误'))
  } finally {
    importing.value = false
  }
}

/** 下载导入模板：根据当前选中模型（或全部模型第一个）动态生成 Excel */
async function downloadTemplate() {
  // 优先用列表筛选的模型，其次用第一个模型
  let modelId = selectedModelId.value
  let attrs: CiModelAttr[] = []
  if (!modelId && models.value.length) {
    modelId = models.value[0].id
  }
  if (modelId) {
    try {
      attrs = await getModelAttrs(modelId)
    } catch { /* ignore，空属性也能下载 */ }
  }
  const model = models.value.find((m) => m.id === modelId)
  const modelName = model?.name ?? '资产'

  // 表头：name / status / tags + 各属性 code
  const header: string[] = ['name', 'status', 'tags']
  const attrCodes: string[] = []
  for (const a of attrs) {
    header.push(a.code)
    attrCodes.push(a.code)
  }
  // 示例行
  const sample: Record<string, string> = {
    name: `${modelName}示例1`,
    status: 'running',
    tags: '示例',
  }
  for (const a of attrs) {
    if (a.valueType === 'boolean') sample[a.code] = 'true'
    else if (a.valueType === 'number') sample[a.code] = '0'
    else if (a.valueType === 'enum' && a.options?.length) sample[a.code] = a.options[0]
    else sample[a.code] = ''
  }
  const dataSheet = XLSX.utils.json_to_sheet([sample], { header })
  // 列宽
  dataSheet['!cols'] = header.map((h) => ({ wch: Math.max(12, h.length + 4) }))

  // 说明 sheet
  const notes: Array<{ 字段: string; 说明: string }> = [
    { 字段: 'name', 说明: '资产名称（必填）' },
    { 字段: 'status', 说明: '状态：running/stopped/maintenance/unknown，留空默认 running' },
    { 字段: 'tags', 说明: '标签，多个用逗号分隔（可选）' },
  ]
  for (const a of attrs) {
    const req = a.isRequired ? '必填' : '可选'
    let typeDesc = a.valueType
    if (a.valueType === 'enum' && a.options?.length) {
      typeDesc = `枚举(${a.options.join('/')})`
    } else if (a.valueType === 'boolean') {
      typeDesc = '布尔(true/false)'
    } else if (a.valueType === 'number') {
      typeDesc = '数字'
    }
    notes.push({ 字段: a.code, 说明: `${a.name}（${req}，${typeDesc}）` })
  }
  const notesSheet = XLSX.utils.json_to_sheet(notes, { header: ['字段', '说明'] })
  notesSheet['!cols'] = [{ wch: 20 }, { wch: 50 }]

  const wb = XLSX.utils.book_new()
  XLSX.utils.book_append_sheet(wb, dataSheet, '导入数据')
  XLSX.utils.book_append_sheet(wb, notesSheet, '填写说明')

  const fileName = `${modelName}导入模板.xlsx`
  XLSX.writeFile(wb, fileName)
}

// ---- 导出 ----
const exporting = ref(false)

/** 循环分页拉取当前筛选条件下的全部资产 */
async function fetchAllInstances(): Promise<CiInstance[]> {
  const all: CiInstance[] = []
  let p = 1
  while (true) {
    const res = await listCiInstances({
      modelId: selectedModelId.value || undefined,
      status: statusFilter.value || undefined,
      keyword: keyword.value || undefined,
      page: p,
      pageSize: 200,
    })
    all.push(...res.items)
    if (all.length >= res.total) break
    p++
  }
  return all
}

/** 导出当前筛选条件下的全部资产为 Excel/CSV */
async function exportData(format: 'xlsx' | 'csv') {
  exporting.value = true
  try {
    const items = await fetchAllInstances()
    if (!items.length) {
      ElMessage.warning('没有可导出的数据')
      return
    }

    // 属性列：选中模型时用该模型属性定义；否则收集所有实例属性的并集
    let attrCodes: string[] = []
    if (selectedModelId.value) {
      const attrs = await getModelAttrs(selectedModelId.value)
      attrCodes = attrs.map(a => a.code)
    } else {
      attrCodes = [...new Set(items.flatMap(i => Object.keys(i.attributes || {})))]
    }

    // 表头
    const hasModelCol = !selectedModelId.value
    const headers: string[] = ['name', 'status', 'tags']
    if (hasModelCol) headers.push('modelId', 'source')
    headers.push(...attrCodes)

    // 数据行
    const rows = items.map(item => {
      const row: Record<string, any> = {
        name: item.name,
        status: item.status,
        tags: item.tags || '',
      }
      if (hasModelCol) {
        row.modelId = modelName(item.modelId)
        row.source = item.source || '手工录入'
      }
      for (const code of attrCodes) {
        const val = item.attributes?.[code]
        row[code] = val !== undefined && val !== null ? val : ''
      }
      return row
    })

    const ws = XLSX.utils.json_to_sheet(rows, { header: headers })
    ws['!cols'] = headers.map(h => ({ wch: Math.max(12, h.length + 4) }))

    const wb = XLSX.utils.book_new()
    XLSX.utils.book_append_sheet(wb, ws, '资产数据')

    const dateStr = new Date().toISOString().slice(0, 10)
    const fileName = `资产导出_${dateStr}.${format}`
    XLSX.writeFile(wb, fileName, { bookType: format })
    ElMessage.success(`已导出 ${items.length} 条数据`)
  } catch (e: any) {
    ElMessage.error(e?.message || '导出失败')
  } finally {
    exporting.value = false
  }
}

// ---- 初始化 ----
onMounted(async () => {
  // 三个请求无依赖关系，并行加载减少等待时间
  await Promise.all([fetchModels(), fetchList(), fetchStats()])
})
</script>

<style scoped>
.assets-page { padding: 0; }

/* 统计卡片 */
.stats-row {
  display: flex;
  gap: 16px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}
.stat-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 16px 20px;
  background: #fff;
  border-radius: 10px;
  border: 2px solid transparent;
  box-shadow: 0 1px 4px rgba(0,0,0,0.06);
  cursor: pointer;
  transition: all 0.2s;
  min-width: 180px;
  flex: 1;
}
.stat-card:hover { transform: translateY(-2px); box-shadow: 0 4px 12px rgba(0,0,0,0.1); }
.stat-card.active { border-color: #409eff; }
.stat-icon {
  width: 48px; height: 48px;
  border-radius: 10px;
  display: flex; align-items: center; justify-content: center;
  color: #fff;
  font-size: 22px;
}
.stat-info { display: flex; flex-direction: column; }
.stat-label { font-size: 13px; color: #909399; }
.stat-value { font-size: 24px; font-weight: 700; color: #303133; }

/* 工具栏 */
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.toolbar-left { display: flex; gap: 10px; align-items: center; }
.toolbar-right { display: flex; gap: 8px; }

/* 表格属性标签 */
.attr-chip {
  display: inline-block;
  margin-right: 8px;
  font-size: 12px;
  color: #606266;
}
.attr-label { color: #909399; }
.text-muted { color: #c0c4cc; font-size: 12px; }

/* 数据来源列 */
.source-cell { display: flex; flex-direction: column; gap: 2px; }
.sync-time { font-size: 11px; color: #909399; }

/* 分页 */
.pagination-row {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

/* 对话框表单分区 */
.form-section { margin-bottom: 8px; }
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 16px;
  padding-left: 8px;
  border-left: 3px solid #409eff;
}

/* 表单提示文本 */
.form-tip {
  font-size: 12px;
  color: #909399;
  line-height: 1.4;
  margin-top: 4px;
}

/* 批量导入 */
.import-step { min-height: 200px; }
.upload-hint { margin-bottom: 16px; }
.attr-mapping { margin-top: 16px; }
.attr-mapping-list { display: flex; flex-wrap: wrap; gap: 8px; margin-top: 8px; }
.attr-tag { margin: 0; }
.req-mark { color: #f56c6c; margin-left: 2px; font-weight: 700; }
.preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.preview-more {
  text-align: center;
  color: #909399;
  font-size: 12px;
  margin-top: 8px;
}
</style>
