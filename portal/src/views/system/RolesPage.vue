<template>
  <div class="roles-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <span>🛡️ 角色管理</span>
          <el-button v-permission="'role:create'" type="primary" :icon="Plus" @click="openCreateDialog">
            新增角色
          </el-button>
        </div>
      </template>
      <el-table :data="roles" stripe v-loading="loading">
        <el-table-column prop="name" label="角色标识" width="140" />
        <el-table-column prop="displayName" label="显示名" width="140">
          <template #default="{ row }">{{ row.displayName || '-' }}</template>
        </el-table-column>
        <el-table-column label="描述" min-width="200">
          <template #default="{ row }">{{ row.description || '-' }}</template>
        </el-table-column>
        <el-table-column width="100" label="类型">
          <template #default="{ row }">
            <el-tag v-if="row.builtIn" type="warning" size="small">内置</el-tag>
            <el-tag v-else type="info" size="small">自定义</el-tag>
          </template>
        </el-table-column>
        <el-table-column width="80" label="状态">
          <template #default="{ row }">
            <el-tag :type="row.enabled ? 'success' : 'danger'" size="small">
              {{ row.enabled ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column width="280" label="操作" fixed="right">
          <template #default="{ row }">
            <el-button v-permission="'role:assign_permission'" size="small" link type="primary" @click="openPermissionDialog(row)">
              分配权限
            </el-button>
            <el-button v-permission="'role:update'" size="small" link type="primary" @click="openEditDialog(row)">
              编辑
            </el-button>
            <el-button
              v-permission="'role:delete'"
              size="small"
              link
              type="danger"
              :disabled="row.builtIn"
              @click="handleDelete(row)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 新增/编辑角色对话框 -->
    <el-dialog
      v-model="roleDialogVisible"
      width="520px"
      class="role-dialog"
      @closed="resetRoleForm"
    >
      <template #header>
        <div class="dialog-header">
          <div class="dialog-header__icon">
            <component :is="editingId ? EditPen : Plus" :size="20" />
          </div>
          <div class="dialog-header__text">
            <div class="dialog-header__title">{{ editingId ? '编辑角色' : '新增角色' }}</div>
            <div class="dialog-header__desc">
              {{ editingId ? '修改角色信息与权限属性' : '创建一个新的角色，用于权限分配' }}
            </div>
          </div>
        </div>
      </template>
      <el-form ref="roleFormRef" :model="roleForm" :rules="roleRules" label-width="90px" class="role-form">
        <div class="form-section">
          <div class="form-section__title">基本信息</div>
          <el-form-item v-if="!editingId" label="标识" prop="name">
            <el-input v-model="roleForm.name" placeholder="英文标识，如 devops" />
          </el-form-item>
          <el-form-item v-else label="标识">
            <el-input :model-value="roleForm.name" disabled />
          </el-form-item>
          <el-form-item label="显示名" prop="displayName">
            <el-input v-model="roleForm.displayName" placeholder="如：运维开发" clearable />
          </el-form-item>
          <el-form-item label="描述" prop="description">
            <el-input v-model="roleForm.description" type="textarea" :rows="2" placeholder="角色职责说明（可选）" />
          </el-form-item>
        </div>
        <div class="form-section">
          <div class="form-section__title">角色属性</div>
          <el-form-item label="启用角色">
            <div class="switch-with-desc">
              <el-switch v-model="roleForm.enabled" />
              <span class="switch-desc">{{ roleForm.enabled ? '启用后该角色可分配给用户' : '禁用后该角色不可分配给用户' }}</span>
            </div>
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="roleDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmitRole">
          {{ editingId ? '保存修改' : '创建角色' }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 分配权限对话框 -->
    <el-dialog v-model="permDialogVisible" width="680px" class="perm-dialog" @closed="permChecked = []">
      <template #header>
        <div class="dialog-header">
          <div class="dialog-header__icon dialog-header__icon--perm"><Setting :size="20" /></div>
          <div class="dialog-header__text">
            <div class="dialog-header__title">分配权限</div>
            <div class="dialog-header__desc">为角色设置可访问的功能权限</div>
          </div>
        </div>
      </template>
      <template v-if="permRole">
        <div class="perm-role-info">
          <el-tag size="small" type="primary">{{ permRole.displayName || permRole.name }}</el-tag>
          <span class="perm-role-info__label">当前角色</span>
        </div>
        <div v-loading="permLoading" class="perm-groups">
          <div v-for="g in permGroups" :key="g.module" class="perm-group">
            <div class="perm-group__title">{{ g.module }}</div>
            <el-checkbox-group v-model="permChecked">
              <el-checkbox
                v-for="p in g.items"
                :key="p.id"
                :value="p.id"
              >
                {{ p.name }}
                <span class="perm-code">{{ p.code }}</span>
              </el-checkbox>
            </el-checkbox-group>
          </div>
        </div>
        <div class="perm-actions">
          <el-button size="small" @click="permSelectAll(true)">全选</el-button>
          <el-button size="small" @click="permSelectAll(false)">清空</el-button>
        </div>
      </template>
      <template #footer>
        <el-button @click="permDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmitPermissions">保存权限</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, EditPen, Setting } from '@element-plus/icons-vue'
import * as rolesApi from '../../api/roles'
import type { Role, Permission } from '../../api/types'

const roles = ref<Role[]>([])
const allPermissions = ref<Permission[]>([])
const loading = ref(false)
const submitting = ref(false)

async function loadRoles() {
  loading.value = true
  try {
    roles.value = await rolesApi.listRoles()
  } catch {
    // 拦截器已提示
  } finally {
    loading.value = false
  }
}

async function loadPermissions() {
  try {
    allPermissions.value = await rolesApi.listPermissions()
  } catch {
    // 拦截器已提示
  }
}

onMounted(() => {
  loadRoles()
  loadPermissions()
})

// ---- 新增/编辑角色 ----
const roleDialogVisible = ref(false)
const roleFormRef = ref<FormInstance>()
const editingId = ref('')
const roleForm = reactive({
  name: '',
  displayName: '',
  description: '',
  enabled: true,
})
const roleRules: FormRules = {
  name: [
    { required: true, message: '请输入角色标识', trigger: 'blur' },
    { pattern: /^[a-zA-Z][a-zA-Z0-9_]*$/, message: '字母开头，仅含字母数字下划线', trigger: 'blur' },
  ],
}

function resetRoleForm() {
  roleForm.name = ''
  roleForm.displayName = ''
  roleForm.description = ''
  roleForm.enabled = true
  editingId.value = ''
  roleFormRef.value?.clearValidate()
}

function openCreateDialog() {
  resetRoleForm()
  roleDialogVisible.value = true
}

function openEditDialog(row: Role) {
  editingId.value = row.id
  roleForm.name = row.name
  roleForm.displayName = row.displayName
  roleForm.description = row.description
  roleForm.enabled = row.enabled
  roleDialogVisible.value = true
}

async function handleSubmitRole() {
  if (!roleFormRef.value) return
  await roleFormRef.value.validate(async (valid) => {
    if (!valid) return
    submitting.value = true
    try {
      if (editingId.value) {
        await rolesApi.updateRole(editingId.value, {
          displayName: roleForm.displayName,
          description: roleForm.description,
          enabled: roleForm.enabled,
        })
        ElMessage.success('保存成功')
      } else {
        await rolesApi.createRole({
          name: roleForm.name,
          displayName: roleForm.displayName || undefined,
          description: roleForm.description || undefined,
          enabled: roleForm.enabled,
        })
        ElMessage.success('角色创建成功')
      }
      roleDialogVisible.value = false
      await loadRoles()
    } catch {
      // 拦截器已提示
    } finally {
      submitting.value = false
    }
  })
}

async function handleDelete(row: Role) {
  await ElMessageBox.confirm(`确定删除角色 "${row.displayName || row.name}" 吗？`, '提示', { type: 'warning' })
  try {
    await rolesApi.deleteRole(row.id)
    ElMessage.success('删除成功')
    await loadRoles()
  } catch {
    // 拦截器已提示
  }
}

// ---- 分配权限 ----
const permDialogVisible = ref(false)
const permLoading = ref(false)
const permRole = ref<Role | null>(null)
const permChecked = ref<string[]>([])

/** 权限按 module 分组 */
const permGroups = computed(() => {
  const map: Record<string, Permission[]> = {}
  allPermissions.value.forEach((p) => {
    if (!map[p.module]) map[p.module] = []
    map[p.module].push(p)
  })
  return Object.entries(map).map(([module, items]) => ({ module, items }))
})

async function openPermissionDialog(row: Role) {
  permRole.value = row
  permDialogVisible.value = true
  permLoading.value = true
  permChecked.value = []
  try {
    const assigned = await rolesApi.listRolePermissions(row.id)
    permChecked.value = assigned.map((p) => p.id)
  } catch {
    // 拦截器已提示
  } finally {
    permLoading.value = false
  }
}

function permSelectAll(select: boolean) {
  permChecked.value = select ? allPermissions.value.map((p) => p.id) : []
}

async function handleSubmitPermissions() {
  if (!permRole.value) return
  submitting.value = true
  try {
    await rolesApi.setRolePermissions(permRole.value.id, permChecked.value)
    ElMessage.success('权限已更新')
    permDialogVisible.value = false
  } catch {
    // 拦截器已提示
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  font-weight: 600;
}

/* ---- 对话框通用 ---- */
.role-dialog :deep(.el-dialog__header),
.perm-dialog :deep(.el-dialog__header) {
  padding: 20px 24px 16px;
  margin-right: 0;
  border-bottom: 1px solid #f0f0f0;
}
.role-dialog :deep(.el-dialog__body),
.perm-dialog :deep(.el-dialog__body) {
  padding: 20px 24px;
}
.role-dialog :deep(.el-dialog__footer),
.perm-dialog :deep(.el-dialog__footer) {
  padding: 12px 24px 20px;
  border-top: 1px solid #f0f0f0;
}
.role-dialog :deep(.el-dialog__headerbtn),
.perm-dialog :deep(.el-dialog__headerbtn) {
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
.dialog-header__icon--perm {
  background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);
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
.role-form .form-section {
  margin-bottom: 8px;
}
.role-form .form-section:last-child {
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
.role-form :deep(.el-form-item) {
  margin-bottom: 18px;
}
.role-form :deep(.el-form-item__label) {
  font-weight: 500;
  color: #4b5563;
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

/* ---- 分配权限 ---- */
.perm-role-info {
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  gap: 8px;
  color: #606266;
}
.perm-role-info__label {
  font-size: 12px;
  color: #909399;
}
.perm-groups {
  max-height: 380px;
  overflow-y: auto;
  padding-right: 4px;
}
.perm-group {
  margin-bottom: 14px;
  padding: 14px 16px;
  border: 1px solid #ebeef5;
  border-radius: 8px;
  background: #fafbfc;
  transition: border-color 0.2s;
}
.perm-group:hover {
  border-color: #c6e2ff;
}
.perm-group:last-child {
  margin-bottom: 0;
}
.perm-group__title {
  font-weight: 600;
  margin-bottom: 10px;
  color: #303133;
  font-size: 14px;
}
.perm-group :deep(.el-checkbox) {
  margin-right: 18px;
  margin-bottom: 8px;
}
.perm-code {
  color: #909399;
  font-size: 12px;
  margin-left: 4px;
  font-family: monospace;
}
.perm-actions {
  margin-top: 12px;
  text-align: right;
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
