<template>
  <div class="depts-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <span>🏢 部门管理</span>
          <el-button v-permission="'dept:create'" type="primary" :icon="Plus" @click="openCreateDialog()">
            新增根部门
          </el-button>
        </div>
      </template>
      <el-table
        :data="deptTree"
        row-key="id"
        :tree-props="{ children: 'children' }"
        stripe
        default-expand-all
        v-loading="loading"
      >
        <el-table-column prop="name" label="部门名称" min-width="220" />
        <el-table-column label="排序" width="80" prop="sortOrder" />
        <el-table-column width="80" label="状态">
          <template #default="{ row }">
            <el-tag :type="row.enabled ? 'success' : 'danger'" size="small">
              {{ row.enabled ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="更新时间" width="170">
          <template #default="{ row }">{{ formatTime(row.updatedAt) }}</template>
        </el-table-column>
        <el-table-column width="260" label="操作" fixed="right">
          <template #default="{ row }">
            <el-button v-permission="'dept:create'" size="small" link type="primary" @click="openCreateDialog(row.id)">
              新增子部门
            </el-button>
            <el-button v-permission="'dept:update'" size="small" link type="primary" @click="openEditDialog(row)">
              编辑
            </el-button>
            <el-button v-permission="'dept:delete'" size="small" link type="danger" @click="handleDelete(row)">
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 新增/编辑部门对话框 -->
    <el-dialog
      v-model="deptDialogVisible"
      width="520px"
      class="dept-dialog"
      @closed="resetDeptForm"
    >
      <template #header>
        <div class="dialog-header">
          <div class="dialog-header__icon">
            <component :is="editingId ? EditPen : Plus" :size="20" />
          </div>
          <div class="dialog-header__text">
            <div class="dialog-header__title">{{ editingId ? '编辑部门' : '新增部门' }}</div>
            <div class="dialog-header__desc">
              {{ editingId ? '修改部门信息与层级关系' : '创建一个新的部门节点' }}
            </div>
          </div>
        </div>
      </template>
      <el-form ref="deptFormRef" :model="deptForm" :rules="deptRules" label-width="100px" class="dept-form">
        <div class="form-section">
          <div class="form-section__title">基本信息</div>
          <el-form-item label="部门名称" prop="name">
            <el-input v-model="deptForm.name" placeholder="部门名称" clearable />
          </el-form-item>
          <el-form-item label="上级部门" prop="parentId">
            <el-tree-select
              v-model="deptForm.parentId"
              :data="parentTreeData"
              :props="{ label: 'name', value: 'id', children: 'children' }"
              placeholder="留空表示根部门"
              clearable
              check-strictly
              style="width: 100%"
            />
          </el-form-item>
        </div>
        <div class="form-section">
          <div class="form-section__title">排序与状态</div>
          <el-form-item label="排序" prop="sortOrder">
            <el-input-number v-model="deptForm.sortOrder" :min="0" :max="9999" />
            <span class="form-item-hint">数值越小越靠前</span>
          </el-form-item>
          <el-form-item label="启用部门">
            <div class="switch-with-desc">
              <el-switch v-model="deptForm.enabled" />
              <span class="switch-desc">{{ deptForm.enabled ? '启用后该部门可正常使用' : '禁用后该部门不可分配用户' }}</span>
            </div>
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="deptDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmitDept">
          {{ editingId ? '保存修改' : '创建部门' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, EditPen } from '@element-plus/icons-vue'
import * as deptsApi from '../../api/departments'
import type { Department, DepartmentNode } from '../../api/types'

const departments = ref<Department[]>([])
const loading = ref(false)
const submitting = ref(false)

async function loadDepartments() {
  loading.value = true
  try {
    departments.value = await deptsApi.listDepartments()
  } catch {
    // 拦截器已提示
  } finally {
    loading.value = false
  }
}

onMounted(loadDepartments)

/** 扁平列表 → 树（el-table 需要 children 结构） */
const deptTree = computed<DepartmentNode[]>(() => buildTree(departments.value))
/** 上级部门选择器的数据：与 deptTree 同结构 */
const parentTreeData = computed(() => deptTree.value)

function buildTree(list: Department[]): DepartmentNode[] {
  const map: Record<string, DepartmentNode> = {}
  const roots: DepartmentNode[] = []
  list.forEach((d) => (map[d.id] = { ...d, children: [] }))
  list.forEach((d) => {
    const node = map[d.id]
    if (d.parentId && map[d.parentId]) {
      map[d.parentId].children.push(node)
    } else {
      roots.push(node)
    }
  })
  const sortRec = (nodes: DepartmentNode[]) => {
    nodes.sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name))
    nodes.forEach((n) => sortRec(n.children))
  }
  sortRec(roots)
  return roots
}

/** RFC3339 → 'YYYY-MM-DD HH:mm' */
function formatTime(s: string | null | undefined): string {
  if (!s) return '-'
  return s.replace('T', ' ').slice(0, 16)
}

// ---- 新增/编辑部门 ----
const deptDialogVisible = ref(false)
const deptFormRef = ref<FormInstance>()
const editingId = ref('')
const deptForm = reactive({
  name: '',
  parentId: '',
  sortOrder: 0,
  enabled: true,
})
const deptRules: FormRules = {
  name: [{ required: true, message: '请输入部门名称', trigger: 'blur' }],
}

function resetDeptForm() {
  deptForm.name = ''
  deptForm.parentId = ''
  deptForm.sortOrder = 0
  deptForm.enabled = true
  editingId.value = ''
  deptFormRef.value?.clearValidate()
}

/** parentId 留空表示根部门 */
function openCreateDialog(parentId?: string) {
  resetDeptForm()
  deptForm.parentId = parentId || ''
  deptDialogVisible.value = true
}

function openEditDialog(row: Department) {
  editingId.value = row.id
  deptForm.name = row.name
  deptForm.parentId = row.parentId || ''
  deptForm.sortOrder = row.sortOrder
  deptForm.enabled = row.enabled
  deptDialogVisible.value = true
}

async function handleSubmitDept() {
  if (!deptFormRef.value) return
  await deptFormRef.value.validate(async (valid) => {
    if (!valid) return
    submitting.value = true
    try {
      const payload = {
        name: deptForm.name,
        parentId: deptForm.parentId || undefined,
        sortOrder: deptForm.sortOrder,
        enabled: deptForm.enabled,
      }
      if (editingId.value) {
        await deptsApi.updateDepartment(editingId.value, payload)
        ElMessage.success('保存成功')
      } else {
        await deptsApi.createDepartment(payload)
        ElMessage.success('部门创建成功')
      }
      deptDialogVisible.value = false
      await loadDepartments()
    } catch {
      // 拦截器已提示
    } finally {
      submitting.value = false
    }
  })
}

async function handleDelete(row: Department) {
  await ElMessageBox.confirm(
    `确定删除部门 "${row.name}" 吗？有子部门或用户时将无法删除。`,
    '提示',
    { type: 'warning' },
  )
  try {
    await deptsApi.deleteDepartment(row.id)
    ElMessage.success('删除成功')
    await loadDepartments()
  } catch {
    // 拦截器已提示
  }
}
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  font-weight: 600;
}

/* ---- 对话框 ---- */
.dept-dialog :deep(.el-dialog__header) {
  padding: 20px 24px 16px;
  margin-right: 0;
  border-bottom: 1px solid #f0f0f0;
}
.dept-dialog :deep(.el-dialog__body) {
  padding: 20px 24px;
}
.dept-dialog :deep(.el-dialog__footer) {
  padding: 12px 24px 20px;
  border-top: 1px solid #f0f0f0;
}
.dept-dialog :deep(.el-dialog__headerbtn) {
  top: 20px;
  right: 20px;
}

.dialog-header {
  display: flex;
  align-items: center;
  gap: 12px;
}
.dialog-header__icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  flex-shrink: 0;
  background: linear-gradient(135deg, #409EFF 0%, #667eea 100%);
}
.dialog-header__text {
  flex: 1;
}
.dialog-header__title {
  font-size: 17px;
  font-weight: 600;
  color: #1f2937;
  line-height: 1.3;
}
.dialog-header__desc {
  font-size: 12px;
  color: #909399;
  margin-top: 3px;
}

/* ---- 表单分区 ---- */
.dept-form .form-section {
  margin-bottom: 8px;
}
.dept-form .form-section:last-child {
  margin-bottom: 0;
}
.form-section__title {
  font-size: 13px;
  font-weight: 600;
  color: #409EFF;
  margin-bottom: 16px;
  padding-left: 8px;
  border-left: 3px solid #409EFF;
  line-height: 1;
}
.dept-form :deep(.el-form-item) {
  margin-bottom: 18px;
}
.dept-form :deep(.el-form-item__label) {
  font-weight: 500;
  color: #4b5563;
}

/* ---- 表单提示 ---- */
.form-item-hint {
  margin-left: 12px;
  font-size: 12px;
  color: #909399;
}

/* ---- 开关带描述 ---- */
.switch-with-desc {
  display: flex;
  align-items: center;
  gap: 10px;
}
.switch-desc {
  font-size: 12px;
  color: #909399;
}
</style>
