<template>
  <div class="dict-page">
    <el-row :gutter="16">
      <!-- 左侧：字典类型 -->
      <el-col :span="10">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <span>字典类型</span>
              <el-button v-permission="'dict:create'" type="primary" :icon="Plus" size="small" @click="openTypeDialog()">
                新增
              </el-button>
            </div>
          </template>
          <el-table
            :data="dictTypes"
            v-loading="loadingTypes"
            highlight-current-row
            @current-change="handleTypeSelect"
            stripe
            size="small"
          >
            <el-table-column prop="code" label="编码" min-width="140" />
            <el-table-column prop="name" label="名称" min-width="120" />
            <el-table-column label="状态" width="70">
              <template #default="{ row }">
                <el-tag :type="row.enabled ? 'success' : 'danger'" size="small">
                  {{ row.enabled ? '启用' : '禁用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="sortOrder" label="排序" width="60" />
            <el-table-column width="120" label="操作" fixed="right">
              <template #default="{ row }">
                <el-button v-permission="'dict:update'" size="small" link type="primary" @click.stop="openTypeDialog(row)">编辑</el-button>
                <el-button v-permission="'dict:delete'" size="small" link type="danger" @click.stop="handleDeleteType(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>

      <!-- 右侧：字典项 -->
      <el-col :span="14">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <span>
                字典项
                <el-tag v-if="selectedType" type="info" size="small" style="margin-left: 8px">
                  {{ selectedType.name }} ({{ selectedType.code }})
                </el-tag>
              </span>
              <el-button
                v-permission="'dict:create'"
                type="primary"
                :icon="Plus"
                size="small"
                :disabled="!selectedType"
                @click="openItemDialog()"
              >
                新增
              </el-button>
            </div>
          </template>
          <el-table :data="dictItems" v-loading="loadingItems" stripe size="small">
            <el-table-column v-if="!selectedType" label="请先选择左侧字典类型" align="center">
              <template #default>
                <el-empty description="请先选择左侧字典类型" :image-size="60" />
              </template>
            </el-table-column>
            <template v-else>
              <el-table-column prop="value" label="存储值" min-width="140" />
              <el-table-column prop="label" label="显示文本" min-width="140" />
              <el-table-column prop="sortOrder" label="排序" width="60" />
              <el-table-column width="120" label="操作" fixed="right">
                <template #default="{ row }">
                  <el-button v-permission="'dict:update'" size="small" link type="primary" @click="openItemDialog(row)">编辑</el-button>
                  <el-button v-permission="'dict:delete'" size="small" link type="danger" @click="handleDeleteItem(row)">删除</el-button>
                </template>
              </el-table-column>
            </template>
          </el-table>
        </el-card>
      </el-col>
    </el-row>

    <!-- 字典类型对话框 -->
    <el-dialog v-model="typeDialogVisible" width="480px" @closed="resetTypeForm">
      <template #header>{{ editingType ? '编辑字典类型' : '新增字典类型' }}</template>
      <el-form :model="typeForm" label-width="80px">
        <el-form-item label="编码" v-if="!editingType">
          <el-input v-model="typeForm.code" placeholder="如 knowledge_category" />
        </el-form-item>
        <el-form-item label="名称">
          <el-input v-model="typeForm.name" placeholder="如 知识库分类" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="typeForm.description" type="textarea" :rows="2" />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="typeForm.sortOrder" :min="0" />
        </el-form-item>
        <el-form-item label="启用" v-if="editingType">
          <el-switch v-model="typeForm.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="typeDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmitType">确定</el-button>
      </template>
    </el-dialog>

    <!-- 字典项对话框 -->
    <el-dialog v-model="itemDialogVisible" width="480px" @closed="resetItemForm">
      <template #header>{{ editingItem ? '编辑字典项' : '新增字典项' }}</template>
      <el-form :model="itemForm" label-width="80px">
        <el-form-item label="存储值" v-if="!editingItem">
          <el-input v-model="itemForm.itemValue" placeholder="如 database" />
        </el-form-item>
        <el-form-item label="显示文本">
          <el-input v-model="itemForm.itemLabel" placeholder="如 数据库" />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="itemForm.sortOrder" :min="0" />
        </el-form-item>
        <el-form-item label="启用" v-if="editingItem">
          <el-switch v-model="itemForm.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="itemDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmitItem">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listDictTypes, createDictType, updateDictType, deleteDictType,
  listDictItems, createDictItem, updateDictItem, deleteDictItem,
  type DictType, type DictItem,
} from '../../api/dict'

// ---- 字典类型 ----
const dictTypes = ref<DictType[]>([])
const loadingTypes = ref(false)
const selectedType = ref<DictType | null>(null)
const typeDialogVisible = ref(false)
const editingType = ref<DictType | null>(null)
const submitting = ref(false)

const typeForm = ref({
  code: '',
  name: '',
  description: '',
  sortOrder: 0,
  enabled: true,
})

async function loadTypes() {
  loadingTypes.value = true
  try {
    dictTypes.value = await listDictTypes()
  } catch {
    // 错误已由拦截器处理
  } finally {
    loadingTypes.value = false
  }
}

function handleTypeSelect(row: DictType | null) {
  selectedType.value = row
  if (row) loadItems(row.code)
  else dictItems.value = []
}

function openTypeDialog(row?: DictType) {
  if (row) {
    editingType.value = row
    typeForm.value = {
      code: row.code,
      name: row.name,
      description: row.description || '',
      sortOrder: row.sortOrder,
      enabled: row.enabled,
    }
  } else {
    editingType.value = null
    typeForm.value = { code: '', name: '', description: '', sortOrder: 0, enabled: true }
  }
  typeDialogVisible.value = true
}

function resetTypeForm() {
  editingType.value = null
  typeForm.value = { code: '', name: '', description: '', sortOrder: 0, enabled: true }
}

async function handleSubmitType() {
  if (!typeForm.value.name.trim()) {
    ElMessage.warning('名称不能为空')
    return
  }
  submitting.value = true
  try {
    if (editingType.value) {
      await updateDictType(editingType.value.code, {
        name: typeForm.value.name,
        description: typeForm.value.description || undefined,
        enabled: typeForm.value.enabled,
        sortOrder: typeForm.value.sortOrder,
      })
      ElMessage.success('修改成功')
    } else {
      if (!typeForm.value.code.trim()) {
        ElMessage.warning('编码不能为空')
        submitting.value = false
        return
      }
      await createDictType({
        code: typeForm.value.code,
        name: typeForm.value.name,
        description: typeForm.value.description || undefined,
        sortOrder: typeForm.value.sortOrder,
      })
      ElMessage.success('创建成功')
    }
    const preserveCode = editingType.value?.code
    typeDialogVisible.value = false
    await loadTypes()
    // 重新选中刚才编辑的类型（loadTypes 会刷新对象引用导致 el-table 丢失 current-row）
    if (preserveCode) {
      const found = dictTypes.value.find(t => t.code === preserveCode)
      if (found) {
        selectedType.value = found
        await loadItems(found.code)
      }
    }
  } catch {
    // 错误已由拦截器处理
  } finally {
    submitting.value = false
  }
}

async function handleDeleteType(row: DictType) {
  try {
    await ElMessageBox.confirm(
      `确认删除字典类型「${row.name}」？其下所有字典项将一并删除。`,
      '删除确认',
      { type: 'warning' },
    )
    await deleteDictType(row.code)
    ElMessage.success('删除成功')
    if (selectedType.value?.code === row.code) {
      selectedType.value = null
      dictItems.value = []
    }
    await loadTypes()
  } catch {
    // 用户取消或错误已处理
  }
}

// ---- 字典项 ----
const dictItems = ref<DictItem[]>([])
const loadingItems = ref(false)
const itemDialogVisible = ref(false)
const editingItem = ref<DictItem | null>(null)

const itemForm = ref({
  itemValue: '',
  itemLabel: '',
  sortOrder: 0,
  enabled: true,
})

async function loadItems(typeCode: string) {
  loadingItems.value = true
  try {
    dictItems.value = await listDictItems(typeCode)
  } catch {
    // 错误已由拦截器处理
  } finally {
    loadingItems.value = false
  }
}

function openItemDialog(row?: DictItem) {
  if (row) {
    editingItem.value = row
    itemForm.value = {
      itemValue: row.value,
      itemLabel: row.label,
      sortOrder: row.sortOrder,
      enabled: true,
    }
  } else {
    editingItem.value = null
    itemForm.value = { itemValue: '', itemLabel: '', sortOrder: 0, enabled: true }
  }
  itemDialogVisible.value = true
}

function resetItemForm() {
  editingItem.value = null
  itemForm.value = { itemValue: '', itemLabel: '', sortOrder: 0, enabled: true }
}

async function handleSubmitItem() {
  if (!itemForm.value.itemLabel.trim()) {
    ElMessage.warning('显示文本不能为空')
    return
  }
  if (!selectedType.value) return
  submitting.value = true
  try {
    if (editingItem.value) {
      await updateDictItem(editingItem.value.id, {
        itemLabel: itemForm.value.itemLabel,
        enabled: itemForm.value.enabled,
        sortOrder: itemForm.value.sortOrder,
      })
      ElMessage.success('修改成功')
    } else {
      if (!itemForm.value.itemValue.trim()) {
        ElMessage.warning('存储值不能为空')
        submitting.value = false
        return
      }
      await createDictItem(selectedType.value.code, {
        itemValue: itemForm.value.itemValue,
        itemLabel: itemForm.value.itemLabel,
        sortOrder: itemForm.value.sortOrder,
      })
      ElMessage.success('创建成功')
    }
    itemDialogVisible.value = false
    await loadItems(selectedType.value.code)
  } catch {
    // 错误已由拦截器处理
  } finally {
    submitting.value = false
  }
}

async function handleDeleteItem(row: DictItem) {
  try {
    await ElMessageBox.confirm(`确认删除字典项「${row.label}」？`, '删除确认', { type: 'warning' })
    await deleteDictItem(row.id)
    ElMessage.success('删除成功')
    if (selectedType.value) await loadItems(selectedType.value.code)
  } catch {
    // 用户取消或错误已处理
  }
}

onMounted(() => {
  loadTypes()
})
</script>

<style scoped>
.dict-page {
  padding: 0;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
