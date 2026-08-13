<template>
  <div class="models-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><Files /></el-icon>
        <span>CI 模型管理</span>
        <span class="page-sub">动态建模：定义资产类型与字段，银行场景 5 种内置模型 + 自定义扩展</span>
      </div>
      <div class="header-actions">
        <el-button
          v-if="hasPermission('system:update')"
          type="primary"
          :icon="Plus"
          @click="openCreateModel"
        >
          新增模型
        </el-button>
      </div>
    </div>

    <div class="models-body">
      <!-- 左侧：模型列表 -->
      <el-card shadow="never" class="model-list-card">
        <template #header>
          <div class="card-header">
            <span>CI 模型（{{ models.length }}）</span>
          </div>
        </template>
        <div v-loading="loadingModels" class="model-list">
          <div
            v-for="m in models"
            :key="m.id"
            class="model-item"
            :class="{ active: selectedModelId === m.id }"
            @click="selectModel(m.id)"
          >
            <div class="model-icon" :style="iconStyle(m.code)">
              <el-icon><component :is="iconComp(m.icon)" /></el-icon>
            </div>
            <div class="model-info">
              <div class="model-name">
                {{ m.name }}
                <el-tag v-if="!m.enabled" size="small" type="info">已禁用</el-tag>
              </div>
              <div class="model-code">{{ m.code }}</div>
            </div>
            <div class="model-attr-count">{{ attrCountMap[m.id] ?? 0 }} 字段</div>
          </div>
          <el-empty v-if="!loadingModels && models.length === 0" description="暂无模型" />
        </div>
      </el-card>

      <!-- 右侧：属性管理 -->
      <el-card shadow="never" class="attr-card">
        <template #header>
          <div v-if="selectedModel" class="card-header">
            <div class="header-title">
              <el-icon><component :is="iconComp(selectedModel.icon)" /></el-icon>
              <span>{{ selectedModel.name }} · 属性定义</span>
              <el-tag size="small" type="info">{{ selectedModel.code }}</el-tag>
            </div>
            <div class="header-actions">
              <el-button
                v-if="hasPermission('system:update')"
                size="small"
                :icon="Edit"
                @click="openEditModel(selectedModel)"
              >编辑模型</el-button>
              <el-button
                v-if="hasPermission('system:update')"
                size="small"
                type="danger"
                :icon="Delete"
                @click="onDeleteModel(selectedModel)"
              >删除模型</el-button>
              <el-button
                v-if="hasPermission('system:update')"
                size="small"
                type="primary"
                :icon="Plus"
                @click="openCreateAttr"
              >新增字段</el-button>
            </div>
          </div>
          <div v-else class="card-header">
            <span>属性定义</span>
          </div>
        </template>

        <div v-if="selectedModel">
          <el-table :data="attrs" v-loading="loadingAttrs" stripe size="default">
            <el-table-column prop="code" label="字段编码" width="140" show-overflow-tooltip />
            <el-table-column prop="name" label="字段名称" width="140" />
            <el-table-column label="类型" width="100">
              <template #default="{ row }">
                <el-tag size="small" :type="valueTypeTag(row.valueType)">{{ row.valueType }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="defaultValue" label="默认值" width="120" show-overflow-tooltip>
              <template #default="{ row }">
                <span v-if="row.defaultValue">{{ row.defaultValue }}</span>
                <span v-else class="text-muted">—</span>
              </template>
            </el-table-column>
            <el-table-column label="枚举选项" min-width="180">
              <template #default="{ row }">
                <template v-if="row.options && row.options.length">
                  <el-tag v-for="opt in row.options" :key="opt" size="small" style="margin-right: 4px">{{ opt }}</el-tag>
                </template>
                <span v-else class="text-muted">—</span>
              </template>
            </el-table-column>
            <el-table-column label="约束" width="160">
              <template #default="{ row }">
                <el-tag v-if="row.isRequired" size="small" type="danger">必填</el-tag>
                <el-tag v-if="row.isUnique" size="small" type="warning" style="margin-left: 4px">唯一</el-tag>
                <el-tag v-if="row.isSearchable" size="small" type="success" style="margin-left: 4px">可搜</el-tag>
                <span v-if="!row.isRequired && !row.isUnique && !row.isSearchable" class="text-muted">—</span>
              </template>
            </el-table-column>
            <el-table-column prop="sortOrder" label="排序" width="80" />
            <el-table-column label="操作" width="140" fixed="right">
              <template #default="{ row }">
                <el-button
                  v-if="hasPermission('system:update')"
                  size="small" link type="primary"
                  @click="openEditAttr(row)"
                >编辑</el-button>
                <el-button
                  v-if="hasPermission('system:update')"
                  size="small" link type="danger"
                  @click="onDeleteAttr(row)"
                >删除</el-button>
              </template>
            </el-table-column>
          </el-table>
          <div v-if="!loadingAttrs && attrs.length === 0" class="empty-tip">
            <el-empty description="该模型还没有字段定义" />
          </div>
        </div>
        <el-empty v-else description="请从左侧选择一个 CI 模型" />
      </el-card>
    </div>

    <!-- 模型对话框 -->
    <el-dialog
      v-model="modelDialogVisible"
      :title="modelIsEdit ? '编辑模型' : '新增模型'"
      width="520px"
      @closed="resetModelForm"
    >
      <el-form ref="modelFormRef" :model="modelForm" :rules="modelRules" label-width="90px">
        <el-form-item label="编码" prop="code">
          <el-input
            v-model="modelForm.code"
            :disabled="modelIsEdit"
            placeholder="小写字母/数字/下划线，如 storage_device"
          />
        </el-form-item>
        <el-form-item label="名称" prop="name">
          <el-input v-model="modelForm.name" placeholder="如 存储设备" />
        </el-form-item>
        <el-form-item label="图标">
          <el-select v-model="modelForm.icon" style="width: 100%">
            <el-option v-for="name in iconOptions" :key="name" :label="name" :value="name">
              <el-icon style="vertical-align: middle"><component :is="name" /></el-icon>
              <span style="margin-left: 6px">{{ name }}</span>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="modelForm.description" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="modelForm.sortOrder" :min="0" :max="999" />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="modelForm.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="modelDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="savingModel" @click="onSubmitModel">确定</el-button>
      </template>
    </el-dialog>

    <!-- 属性对话框 -->
    <el-dialog
      v-model="attrDialogVisible"
      :title="attrIsEdit ? '编辑字段' : '新增字段'"
      width="560px"
      @closed="resetAttrForm"
    >
      <el-form ref="attrFormRef" :model="attrForm" :rules="attrRules" label-width="90px">
        <el-form-item label="编码" prop="code">
          <el-input
            v-model="attrForm.code"
            :disabled="attrIsEdit"
            placeholder="小写字母/数字/下划线，如 capacity"
          />
        </el-form-item>
        <el-form-item label="名称" prop="name">
          <el-input v-model="attrForm.name" placeholder="如 容量" />
        </el-form-item>
        <el-form-item label="类型" prop="valueType">
          <el-select v-model="attrForm.valueType" style="width: 100%">
            <el-option label="字符串" value="string" />
            <el-option label="数字" value="number" />
            <el-option label="布尔" value="boolean" />
            <el-option label="枚举" value="enum" />
            <el-option label="日期" value="date" />
            <el-option label="JSON" value="json" />
          </el-select>
        </el-form-item>
        <el-form-item label="默认值">
          <el-input v-model="attrForm.defaultValue" placeholder="留空则无默认值" />
        </el-form-item>
        <el-form-item v-if="attrForm.valueType === 'enum'" label="枚举选项">
          <div class="options-editor">
            <div v-for="(opt, idx) in attrForm.optionsArr" :key="idx" class="option-row">
              <el-input v-model="attrForm.optionsArr[idx]" placeholder="选项值" />
              <el-button :icon="Delete" circle size="small" @click="attrForm.optionsArr.splice(idx, 1)" />
            </div>
            <el-button :icon="Plus" size="small" @click="attrForm.optionsArr.push('')">添加选项</el-button>
          </div>
        </el-form-item>
        <el-form-item label="约束">
          <div class="constraints">
            <el-checkbox v-model="attrForm.isRequired">必填</el-checkbox>
            <el-checkbox v-model="attrForm.isUnique">唯一</el-checkbox>
            <el-checkbox v-model="attrForm.isSearchable">可搜索</el-checkbox>
          </div>
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="attrForm.sortOrder" :min="0" :max="999" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="attrDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="savingAttr" @click="onSubmitAttr">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Edit, Delete, Files, Monitor, Cpu, Coin, Connection, Grid, Box, List, Service, Platform, Document, Folder, Setting, Cloudy } from '@element-plus/icons-vue'
import {
  listCiModels,
  listCiModelAttrs,
  createCiModel,
  updateCiModel,
  deleteCiModel,
  createCiModelAttr,
  updateCiModelAttr,
  deleteCiModelAttr,
} from '../../api/cmdb'
import type { CiModel, CiModelAttr, CreateCiModelAttrRequest } from '../../api/types'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()
const hasPermission = (code: string) => userStore.hasPermission(code)

// ---- 模型列表 ----
const loadingModels = ref(false)
const models = ref<CiModel[]>([])
const selectedModelId = ref('')
const attrCountMap = ref<Record<string, number>>({})
const selectedModel = computed(() => models.value.find(m => m.id === selectedModelId.value))

async function fetchModels() {
  loadingModels.value = true
  try {
    models.value = await listCiModels()
    // 预加载每个模型的字段数
    const counts: Record<string, number> = {}
    await Promise.all(models.value.map(async (m) => {
      try {
        const attrs = await listCiModelAttrs(m.id)
        counts[m.id] = attrs.length
      } catch {
        counts[m.id] = 0
      }
    }))
    attrCountMap.value = counts
    // 默认选中第一个
    if (!selectedModelId.value && models.value.length) {
      await selectModel(models.value[0].id)
    }
  } catch (e: any) {
    ElMessage.error(e?.message || '加载模型失败')
  } finally {
    loadingModels.value = false
  }
}

async function selectModel(id: string) {
  selectedModelId.value = id
  await fetchAttrs()
}

// ---- 属性列表 ----
const loadingAttrs = ref(false)
const attrs = ref<CiModelAttr[]>([])

async function fetchAttrs() {
  if (!selectedModelId.value) return
  loadingAttrs.value = true
  try {
    attrs.value = await listCiModelAttrs(selectedModelId.value)
    attrCountMap.value = { ...attrCountMap.value, [selectedModelId.value]: attrs.value.length }
  } catch (e: any) {
    ElMessage.error(e?.message || '加载字段失败')
  } finally {
    loadingAttrs.value = false
  }
}

// ---- 图标 ----
const iconOptions = ['Monitor', 'Cpu', 'Coin', 'Connection', 'Grid', 'Box', 'Files', 'List', 'Service', 'Platform', 'Document', 'Folder', 'Setting', 'Cloudy']
const iconMap: Record<string, any> = { Monitor, Cpu, Coin, Connection, Grid, Box, Files, List, Service, Platform, Document, Folder, Setting, Cloudy }
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

function valueTypeTag(t: string): string {
  const map: Record<string, string> = { string: '', number: 'success', boolean: 'warning', enum: 'danger', date: 'info', json: 'info' }
  return map[t] || 'info'
}

// ---- 模型对话框 ----
const modelDialogVisible = ref(false)
const modelIsEdit = ref(false)
const savingModel = ref(false)
const modelFormRef = ref<FormInstance>()

const modelForm = reactive({
  id: '',
  code: '',
  name: '',
  icon: 'Monitor',
  description: '',
  sortOrder: 99,
  enabled: true,
})

const modelRules: FormRules = {
  code: [
    { required: true, message: '请输入编码', trigger: 'blur' },
    { pattern: /^[a-z0-9_]+$/, message: '只能小写字母、数字、下划线', trigger: 'blur' },
  ],
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
}

function openCreateModel() {
  modelIsEdit.value = false
  modelForm.id = ''
  modelForm.code = ''
  modelForm.name = ''
  modelForm.icon = 'Monitor'
  modelForm.description = ''
  modelForm.sortOrder = 99
  modelForm.enabled = true
  modelDialogVisible.value = true
}

function openEditModel(m: CiModel) {
  modelIsEdit.value = true
  modelForm.id = m.id
  modelForm.code = m.code
  modelForm.name = m.name
  modelForm.icon = m.icon || 'Monitor'
  modelForm.description = m.description
  modelForm.sortOrder = m.sortOrder
  modelForm.enabled = m.enabled
  modelDialogVisible.value = true
}

function resetModelForm() {
  modelFormRef.value?.resetFields()
}

async function onSubmitModel() {
  if (!modelFormRef.value) return
  await modelFormRef.value.validate(async (valid) => {
    if (!valid) return
    savingModel.value = true
    try {
      const payload = {
        name: modelForm.name.trim(),
        icon: modelForm.icon,
        description: modelForm.description,
        sortOrder: modelForm.sortOrder,
        enabled: modelForm.enabled,
      }
      if (modelIsEdit.value) {
        await updateCiModel(modelForm.id, payload)
        ElMessage.success('更新成功')
      } else {
        await createCiModel({ code: modelForm.code.trim(), ...payload })
        ElMessage.success('创建成功')
      }
      modelDialogVisible.value = false
      await fetchModels()
    } catch (e: any) {
      ElMessage.error(e?.message || '操作失败')
    } finally {
      savingModel.value = false
    }
  })
}

async function onDeleteModel(m: CiModel) {
  try {
    await ElMessageBox.confirm(
      `确定删除模型「${m.name}」吗？该模型下的字段定义也将一并删除。若模型下已有资产实例则拒绝删除。`,
      '删除确认',
      { type: 'warning' },
    )
    await deleteCiModel(m.id)
    ElMessage.success('删除成功')
    if (selectedModelId.value === m.id) {
      selectedModelId.value = ''
      attrs.value = []
    }
    await fetchModels()
  } catch (e: any) {
    if (e !== 'cancel' && e?.message) ElMessage.error(e.message)
  }
}

// ---- 属性对话框 ----
const attrDialogVisible = ref(false)
const attrIsEdit = ref(false)
const savingAttr = ref(false)
const attrFormRef = ref<FormInstance>()

const attrForm = reactive({
  id: '',
  code: '',
  name: '',
  valueType: 'string',
  defaultValue: '',
  optionsArr: [] as string[],
  isRequired: false,
  isUnique: false,
  isSearchable: true,
  sortOrder: 99,
})

const attrRules: FormRules = {
  code: [
    { required: true, message: '请输入编码', trigger: 'blur' },
    { pattern: /^[a-z0-9_]+$/, message: '只能小写字母、数字、下划线', trigger: 'blur' },
  ],
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  valueType: [{ required: true, message: '请选择类型', trigger: 'change' }],
}

function openCreateAttr() {
  attrIsEdit.value = false
  attrForm.id = ''
  attrForm.code = ''
  attrForm.name = ''
  attrForm.valueType = 'string'
  attrForm.defaultValue = ''
  attrForm.optionsArr = []
  attrForm.isRequired = false
  attrForm.isUnique = false
  attrForm.isSearchable = true
  attrForm.sortOrder = (attrs.value.length || 0) + 1
  attrDialogVisible.value = true
}

function openEditAttr(a: CiModelAttr) {
  attrIsEdit.value = true
  attrForm.id = a.id
  attrForm.code = a.code
  attrForm.name = a.name
  attrForm.valueType = a.valueType
  attrForm.defaultValue = a.defaultValue
  attrForm.optionsArr = a.options ? [...a.options] : []
  attrForm.isRequired = a.isRequired
  attrForm.isUnique = a.isUnique
  attrForm.isSearchable = a.isSearchable
  attrForm.sortOrder = a.sortOrder
  attrDialogVisible.value = true
}

function resetAttrForm() {
  attrFormRef.value?.resetFields()
}

async function onSubmitAttr() {
  if (!attrFormRef.value) return
  await attrFormRef.value.validate(async (valid) => {
    if (!valid) return
    savingAttr.value = true
    try {
      const options = attrForm.valueType === 'enum'
        ? attrForm.optionsArr.filter(o => o.trim() !== '')
        : undefined
      const payload: CreateCiModelAttrRequest = {
        name: attrForm.name.trim(),
        valueType: attrForm.valueType,
        defaultValue: attrForm.defaultValue,
        options,
        isRequired: attrForm.isRequired,
        isUnique: attrForm.isUnique,
        isSearchable: attrForm.isSearchable,
        sortOrder: attrForm.sortOrder,
      }
      if (attrIsEdit.value) {
        await updateCiModelAttr(selectedModelId.value, attrForm.id, payload)
        ElMessage.success('更新成功')
      } else {
        await createCiModelAttr(selectedModelId.value, { code: attrForm.code.trim(), ...payload })
        ElMessage.success('创建成功')
      }
      attrDialogVisible.value = false
      await fetchAttrs()
    } catch (e: any) {
      ElMessage.error(e?.message || '操作失败')
    } finally {
      savingAttr.value = false
    }
  })
}

async function onDeleteAttr(a: CiModelAttr) {
  try {
    await ElMessageBox.confirm(`确定删除字段「${a.name}」吗？`, '删除确认', { type: 'warning' })
    await deleteCiModelAttr(selectedModelId.value, a.id)
    ElMessage.success('删除成功')
    await fetchAttrs()
  } catch (e: any) {
    if (e !== 'cancel' && e?.message) ElMessage.error(e.message)
  }
}

onMounted(() => {
  fetchModels()
})
</script>

<style scoped>
.models-page { padding: 0; }

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

.models-body {
  display: flex;
  gap: 16px;
  align-items: flex-start;
}
.model-list-card { width: 320px; flex-shrink: 0; }
.attr-card { flex: 1; min-width: 0; }

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.header-title {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
}
.header-actions {
  display: flex;
  gap: 8px;
}

.model-list {
  max-height: 600px;
  overflow-y: auto;
}
.model-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s;
  border: 1px solid transparent;
  margin-bottom: 6px;
}
.model-item:hover { background: #f5f7fa; }
.model-item.active {
  background: #ecf5ff;
  border-color: #409eff;
}
.model-icon {
  width: 36px; height: 36px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 18px;
  flex-shrink: 0;
}
.model-info { flex: 1; min-width: 0; }
.model-name {
  font-size: 14px;
  font-weight: 500;
  color: #303133;
  display: flex;
  align-items: center;
  gap: 6px;
}
.model-code {
  font-size: 12px;
  color: #909399;
  margin-top: 2px;
}
.model-attr-count {
  font-size: 12px;
  color: #909399;
  flex-shrink: 0;
}

.text-muted { color: #c0c4cc; font-size: 12px; }
.empty-tip { padding: 20px 0; }

.options-editor { width: 100%; }
.option-row {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
  align-items: center;
}
.constraints {
  display: flex;
  gap: 16px;
}
</style>
