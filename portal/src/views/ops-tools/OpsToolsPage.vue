<template>
  <div class="ops-tools-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><SetUp /></el-icon>
        <span>运维工具</span>
        <span class="page-sub">外链打开第三方平台；单点登录后续接入</span>
      </div>
      <div class="header-actions">
        <el-button :icon="Refresh" :loading="loading" @click="loadList">刷新</el-button>
        <el-button v-if="canManage" type="primary" :icon="Plus" @click="openCreate">新增</el-button>
      </div>
    </div>

    <el-alert type="info" :closable="false" show-icon class="hint">
      点击卡片将在新标签页打开对应系统。当前不会携带 MeridianOps 登录态，需在目标系统自行登录。
    </el-alert>

    <el-empty v-if="!loading && visibleTools.length === 0" description="暂无可用工具，请管理员配置外链" />

    <div v-else class="tool-grid">
      <div
        v-for="tool in visibleTools"
        :key="tool.id"
        class="tool-card"
        :class="{ disabled: !tool.enabled }"
        @click="openTool(tool)"
      >
        <div class="tool-top">
          <div class="tool-icon">
            <el-icon :size="28"><component :is="iconOf(tool.icon)" /></el-icon>
          </div>
          <div class="tool-meta">
            <div class="tool-name">
              {{ tool.name }}
              <el-tag v-if="canManage && !tool.enabled" size="small" type="info">已停用</el-tag>
            </div>
            <div class="tool-desc">{{ tool.description || tool.url }}</div>
          </div>
        </div>
        <div class="tool-footer">
          <span class="tool-url" :title="tool.url">{{ tool.url }}</span>
          <div v-if="canManage" class="tool-actions" @click.stop>
            <el-button link type="primary" @click="openEdit(tool)">编辑</el-button>
            <el-button link type="danger" @click="removeTool(tool)">删除</el-button>
          </div>
        </div>
      </div>
    </div>

    <el-dialog v-model="dialogVisible" :title="editing ? '编辑工具' : '新增工具'" width="520px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="rules" label-width="88px">
        <el-form-item label="名称" prop="name">
          <el-input v-model="form.name" maxlength="128" placeholder="如 Zabbix" />
        </el-form-item>
        <el-form-item label="地址" prop="url">
          <el-input v-model="form.url" maxlength="1024" placeholder="https://..." />
        </el-form-item>
        <el-form-item label="说明" prop="description">
          <el-input v-model="form.description" maxlength="512" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item label="图标" prop="icon">
          <el-select v-model="form.icon" style="width: 100%">
            <el-option v-for="opt in iconOptions" :key="opt" :label="opt" :value="opt" />
          </el-select>
        </el-form-item>
        <el-form-item label="排序" prop="sortOrder">
          <el-input-number v-model="form.sortOrder" :min="0" :max="9999" />
        </el-form-item>
        <el-form-item label="启用" prop="enabled">
          <el-switch v-model="form.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="submit">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import {
  Cloudy,
  Link,
  Monitor,
  Plus,
  Refresh,
  SetUp,
  TrendCharts,
} from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import {
  createOpsTool,
  deleteOpsTool,
  listOpsTools,
  updateOpsTool,
  type OpsTool,
} from '../../api/opsTools'
import { useUserStore } from '../../stores/user'

const user = useUserStore()
const canManage = computed(() => user.hasPermission('ops_tool:manage'))

const loading = ref(false)
const saving = ref(false)
const tools = ref<OpsTool[]>([])
const dialogVisible = ref(false)
const editing = ref<OpsTool | null>(null)
const formRef = ref<FormInstance>()

const iconOptions = ['Link', 'Monitor', 'TrendCharts', 'Cloudy', 'SetUp']
const iconMap: Record<string, unknown> = {
  Link,
  Monitor,
  TrendCharts,
  Cloudy,
  SetUp,
}

function iconOf(name: string) {
  return iconMap[name] || Link
}

const form = reactive({
  name: '',
  url: '',
  description: '',
  icon: 'Link',
  sortOrder: 100,
  enabled: true,
})

const rules: FormRules = {
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  url: [
    { required: true, message: '请输入地址', trigger: 'blur' },
    {
      validator: (_r, v, cb) => {
        const s = String(v || '').trim().toLowerCase()
        if (!s.startsWith('http://') && !s.startsWith('https://')) {
          cb(new Error('须以 http:// 或 https:// 开头'))
        } else cb()
      },
      trigger: 'blur',
    },
  ],
}

const visibleTools = computed(() => {
  if (canManage.value) return tools.value
  return tools.value.filter((t) => t.enabled)
})

async function loadList() {
  loading.value = true
  try {
    tools.value = await listOpsTools()
  } catch {
    tools.value = []
  } finally {
    loading.value = false
  }
}

function openTool(tool: OpsTool) {
  if (!tool.enabled) {
    ElMessage.warning('该工具已停用')
    return
  }
  window.open(tool.url, '_blank', 'noopener,noreferrer')
}

function resetForm() {
  form.name = ''
  form.url = ''
  form.description = ''
  form.icon = 'Link'
  form.sortOrder = 100
  form.enabled = true
}

function openCreate() {
  editing.value = null
  resetForm()
  dialogVisible.value = true
}

function openEdit(tool: OpsTool) {
  editing.value = tool
  form.name = tool.name
  form.url = tool.url
  form.description = tool.description || ''
  form.icon = tool.icon || 'Link'
  form.sortOrder = tool.sortOrder ?? 100
  form.enabled = tool.enabled
  dialogVisible.value = true
}

async function submit() {
  await formRef.value?.validate()
  saving.value = true
  try {
    const body = {
      name: form.name.trim(),
      url: form.url.trim(),
      description: form.description.trim(),
      icon: form.icon,
      sortOrder: form.sortOrder,
      enabled: form.enabled,
    }
    if (editing.value) {
      await updateOpsTool(editing.value.id, body)
      ElMessage.success('已更新')
    } else {
      await createOpsTool(body)
      ElMessage.success('已创建')
    }
    dialogVisible.value = false
    await loadList()
  } finally {
    saving.value = false
  }
}

async function removeTool(tool: OpsTool) {
  await ElMessageBox.confirm(`确定删除「${tool.name}」？`, '删除确认', { type: 'warning' })
  await deleteOpsTool(tool.id)
  ElMessage.success('已删除')
  await loadList()
}

onMounted(loadList)
</script>

<style scoped>
.ops-tools-page {
  padding: 4px;
}
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}
.page-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
  font-weight: 600;
}
.page-sub {
  margin-left: 4px;
  font-size: 13px;
  font-weight: 400;
  color: var(--el-text-color-secondary);
}
.hint {
  margin-bottom: 16px;
}
.tool-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 14px;
}
.tool-card {
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 10px;
  padding: 16px;
  background: var(--el-bg-color);
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.tool-card:hover {
  border-color: var(--el-color-primary-light-5);
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.06);
}
.tool-card.disabled {
  opacity: 0.55;
}
.tool-top {
  display: flex;
  gap: 12px;
}
.tool-icon {
  width: 48px;
  height: 48px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--el-fill-color-light);
  color: var(--el-color-primary);
  flex-shrink: 0;
}
.tool-name {
  font-size: 16px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 8px;
}
.tool-desc {
  margin-top: 4px;
  font-size: 13px;
  color: var(--el-text-color-secondary);
  line-height: 1.4;
}
.tool-footer {
  margin-top: 14px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.tool-url {
  font-size: 12px;
  color: var(--el-text-color-placeholder);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}
.tool-actions {
  flex-shrink: 0;
}
</style>
