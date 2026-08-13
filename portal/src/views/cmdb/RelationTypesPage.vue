<template>
  <div class="relation-types-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><Connection /></el-icon>
        <span>关系类型管理</span>
        <span class="page-sub">配置 CI 之间的关系字典：有向/无向、启用状态、排序，资产详情页关系下拉自动同步</span>
      </div>
      <div class="header-actions">
        <el-button
          v-if="hasPermission('system:update')"
          type="primary"
          :icon="Plus"
          @click="openCreate"
        >
          新增关系类型
        </el-button>
      </div>
    </div>

    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>关系类型（{{ list.length }}）</span>
          <div class="header-actions">
            <el-button :icon="Refresh" size="small" @click="fetchList">刷新</el-button>
          </div>
        </div>
      </template>

      <el-table :data="list" v-loading="loading" stripe size="default">
        <el-table-column prop="code" label="编码" width="160" show-overflow-tooltip>
          <template #default="{ row }">
            <span class="code-text">{{ row.code }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="name" label="名称" width="140" />
        <el-table-column prop="description" label="描述" min-width="200" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.description">{{ row.description }}</span>
            <span v-else class="text-muted">—</span>
          </template>
        </el-table-column>
        <el-table-column label="方向" width="110">
          <template #default="{ row }">
            <el-tag v-if="row.directional" size="small" type="primary">有向</el-tag>
            <el-tag v-else size="small" type="info">无向</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.enabled" size="small" type="success">启用</el-tag>
            <el-tag v-else size="small" type="danger">禁用</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="sortOrder" label="排序" width="80" />
        <el-table-column prop="updatedAt" label="更新时间" width="180" show-overflow-tooltip />
        <el-table-column label="操作" width="140" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="hasPermission('system:update')"
              size="small" link type="primary"
              @click="openEdit(row)"
            >编辑</el-button>
            <el-button
              v-if="hasPermission('system:update')"
              size="small" link type="danger"
              @click="onDelete(row)"
            >删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <div v-if="!loading && list.length === 0" class="empty-tip">
        <el-empty description="暂无关系类型" />
      </div>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="isEdit ? '编辑关系类型' : '新增关系类型'"
      width="520px"
      @closed="resetForm"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="90px">
        <el-form-item label="编码" prop="code">
          <el-input
            v-model="form.code"
            :disabled="isEdit"
            placeholder="小写字母/数字/下划线，长度 2-32，如 backup_of"
          />
        </el-form-item>
        <el-form-item label="名称" prop="name">
          <el-input v-model="form.name" placeholder="如 备份于" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="form.description" type="textarea" :rows="2" placeholder="可选" />
        </el-form-item>
        <el-form-item label="方向">
          <el-switch v-model="form.directional" active-text="有向（源→目标）" inactive-text="无向" />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="form.enabled" />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="form.sortOrder" :min="0" :max="999" />
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
import { Plus, Refresh, Connection } from '@element-plus/icons-vue'
import {
  listCiRelationTypes,
  createCiRelationType,
  updateCiRelationType,
  deleteCiRelationType,
} from '../../api/cmdb'
import type { CiRelationType } from '../../api/types'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()
const hasPermission = (code: string) => userStore.hasPermission(code)

const loading = ref(false)
const list = ref<CiRelationType[]>([])

async function fetchList() {
  loading.value = true
  try {
    list.value = await listCiRelationTypes()
  } catch (e: any) {
    ElMessage.error(e?.message || '加载关系类型失败')
  } finally {
    loading.value = false
  }
}

// ---- 对话框 ----
const dialogVisible = ref(false)
const isEdit = ref(false)
const saving = ref(false)
const formRef = ref<FormInstance>()

const form = reactive({
  id: '',
  code: '',
  name: '',
  description: '',
  directional: true,
  enabled: true,
  sortOrder: 0,
})

const rules: FormRules = {
  code: [
    { required: true, message: '请输入编码', trigger: 'blur' },
    { pattern: /^[a-z0-9_]{2,32}$/, message: '只能小写字母、数字、下划线，长度 2-32', trigger: 'blur' },
  ],
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
}

function openCreate() {
  isEdit.value = false
  form.id = ''
  form.code = ''
  form.name = ''
  form.description = ''
  form.directional = true
  form.enabled = true
  form.sortOrder = (list.value.length || 0) + 1
  dialogVisible.value = true
}

function openEdit(row: CiRelationType) {
  isEdit.value = true
  form.id = row.id
  form.code = row.code
  form.name = row.name
  form.description = row.description
  form.directional = row.directional
  form.enabled = row.enabled
  form.sortOrder = row.sortOrder
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
        description: form.description,
        directional: form.directional,
        enabled: form.enabled,
        sortOrder: form.sortOrder,
      }
      if (isEdit.value) {
        await updateCiRelationType(form.id, payload)
        ElMessage.success('更新成功')
      } else {
        await createCiRelationType({ code: form.code.trim(), ...payload })
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

async function onDelete(row: CiRelationType) {
  try {
    await ElMessageBox.confirm(
      `确定删除关系类型「${row.name}」吗？若已有 CI 关系使用该类型则拒绝删除。`,
      '删除确认',
      { type: 'warning' },
    )
    await deleteCiRelationType(row.id)
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
.relation-types-page { padding: 0; }

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

.code-text {
  font-family: 'Courier New', Courier, monospace;
  color: #409eff;
}

.text-muted { color: #c0c4cc; font-size: 12px; }
.empty-tip { padding: 20px 0; }
</style>
