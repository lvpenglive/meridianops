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
    <el-dialog
      v-model="permDialogVisible"
      width="820px"
      class="perm-dialog"
      @closed="onPermDialogClosed"
    >
      <template #header>
        <div class="dialog-header">
          <div class="dialog-header__icon dialog-header__icon--perm"><Setting :size="20" /></div>
          <div class="dialog-header__text">
            <div class="dialog-header__title">分配权限</div>
            <div class="dialog-header__desc">
              按侧栏菜单勾选 · 已选 {{ permChecked.length }} / {{ allPermissions.length }}
            </div>
          </div>
        </div>
      </template>
      <template v-if="permRole">
        <div class="perm-toolbar">
          <div class="perm-role-info">
            <el-tag size="small" type="primary">{{ permRole.displayName || permRole.name }}</el-tag>
            <span class="perm-role-info__label">当前角色</span>
          </div>
          <div class="perm-toolbar__filters">
            <el-input
              v-model="permKeyword"
              clearable
              placeholder="搜索菜单 / 权限名 / code"
              style="width: 240px"
            />
            <el-checkbox v-model="permOnlyChecked">仅看已选</el-checkbox>
            <el-button size="small" @click="permSelectAll(true)">全选</el-button>
            <el-button size="small" @click="permSelectAll(false)">清空</el-button>
          </div>
        </div>
        <div v-loading="permLoading" class="perm-groups">
          <el-empty
            v-if="!permLoading && filteredPermGroups.length === 0"
            description="没有匹配的权限"
            :image-size="72"
          />
          <el-collapse v-else v-model="permActiveParents">
            <el-collapse-item
              v-for="g in filteredPermGroups"
              :key="g.parent"
              :name="g.parent"
            >
              <template #title>
                <div class="perm-collapse-title">
                  <el-checkbox
                    :model-value="isGroupAllChecked(g)"
                    :indeterminate="isGroupIndeterminate(g)"
                    @change="(v: boolean | string | number) => toggleGroup(g, !!v)"
                    @click.stop
                  />
                  <span class="perm-collapse-title__name">{{ g.parent }}</span>
                  <span class="perm-collapse-title__meta">
                    {{ countGroupChecked(g) }}/{{ countGroupTotal(g) }}
                  </span>
                </div>
              </template>
              <div v-for="c in g.children" :key="c.title" class="perm-child">
                <div class="perm-child__head">
                  <el-checkbox
                    :model-value="isChildAllChecked(c.items)"
                    :indeterminate="isChildIndeterminate(c.items)"
                    @change="(v: boolean | string | number) => toggleChild(c.items, !!v)"
                  >
                    <span class="perm-child__title">{{ c.title }}</span>
                  </el-checkbox>
                </div>
                <el-checkbox-group v-model="permChecked" class="perm-child__boxes">
                  <el-checkbox
                    v-for="p in c.items"
                    :key="p.id"
                    :value="p.id"
                    border
                  >
                    {{ permissionActionLabel(p.code, p.name) }}
                    <span class="perm-code">{{ p.code }}</span>
                  </el-checkbox>
                </el-checkbox-group>
              </div>
            </el-collapse-item>
          </el-collapse>
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
import { ref, reactive, computed, watch, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, EditPen, Setting } from '@element-plus/icons-vue'
import * as rolesApi from '../../api/roles'
import type { Role, Permission } from '../../api/types'
import {
  groupByMenu,
  permissionActionLabel,
  type MenuPermGroup,
} from '../../utils/permissionDisplay'

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
const permKeyword = ref('')
const permOnlyChecked = ref(false)
const permActiveParents = ref<string[]>([])

/** 权限按侧栏一级 / 二级菜单分组 */
const permMenuGroups = computed(() =>
  groupByMenu(allPermissions.value, (p) => p.code, (p) => p.module),
)

const filteredPermGroups = computed(() => {
  const kw = permKeyword.value.trim().toLowerCase()
  const only = permOnlyChecked.value
  const checked = new Set(permChecked.value)
  return permMenuGroups.value
    .map((g) => ({
      parent: g.parent,
      children: g.children
        .map((c) => ({
          title: c.title,
          items: c.items.filter((p) => {
            if (only && !checked.has(p.id)) return false
            if (!kw) return true
            const hay = [
              g.parent,
              c.title,
              p.code,
              p.name,
              permissionActionLabel(p.code, p.name),
            ]
              .join(' ')
              .toLowerCase()
            return hay.includes(kw)
          }),
        }))
        .filter((c) => c.items.length > 0),
    }))
    .filter((g) => g.children.length > 0)
})

watch(filteredPermGroups, (groups) => {
  if (permKeyword.value.trim() || permOnlyChecked.value) {
    permActiveParents.value = groups.map((g) => g.parent)
  }
})

function syncPermCollapseDefaults() {
  permActiveParents.value = permMenuGroups.value.map((g) => g.parent)
}

function onPermDialogClosed() {
  permChecked.value = []
  permKeyword.value = ''
  permOnlyChecked.value = false
  permActiveParents.value = []
  permRole.value = null
}

async function openPermissionDialog(row: Role) {
  permRole.value = row
  permKeyword.value = ''
  permOnlyChecked.value = false
  permDialogVisible.value = true
  permLoading.value = true
  permChecked.value = []
  try {
    const assigned = await rolesApi.listRolePermissions(row.id)
    permChecked.value = assigned.map((p) => p.id)
    syncPermCollapseDefaults()
  } catch {
    // 拦截器已提示
  } finally {
    permLoading.value = false
  }
}

function permSelectAll(select: boolean) {
  permChecked.value = select ? allPermissions.value.map((p) => p.id) : []
}

function flatGroupItems(g: MenuPermGroup<Permission>): Permission[] {
  return g.children.flatMap((c) => c.items)
}

function countGroupTotal(g: MenuPermGroup<Permission>) {
  return flatGroupItems(g).length
}

function countGroupChecked(g: MenuPermGroup<Permission>) {
  const set = new Set(permChecked.value)
  return flatGroupItems(g).filter((p) => set.has(p.id)).length
}

function isChildAllChecked(items: Permission[]) {
  return items.length > 0 && items.every((p) => permChecked.value.includes(p.id))
}

function isChildIndeterminate(items: Permission[]) {
  const n = items.filter((p) => permChecked.value.includes(p.id)).length
  return n > 0 && n < items.length
}

function isGroupAllChecked(g: MenuPermGroup<Permission>) {
  return isChildAllChecked(flatGroupItems(g))
}

function isGroupIndeterminate(g: MenuPermGroup<Permission>) {
  return isChildIndeterminate(flatGroupItems(g))
}

function toggleChild(items: Permission[], checked: boolean) {
  const set = new Set(permChecked.value)
  for (const p of items) {
    if (checked) set.add(p.id)
    else set.delete(p.id)
  }
  permChecked.value = [...set]
}

function toggleGroup(g: MenuPermGroup<Permission>, checked: boolean) {
  toggleChild(flatGroupItems(g), checked)
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
.perm-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 12px;
}
.perm-role-info {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #606266;
}
.perm-role-info__label {
  font-size: 12px;
  color: #909399;
}
.perm-toolbar__filters {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
}
.perm-groups {
  max-height: 460px;
  overflow-y: auto;
  padding-right: 4px;
}
.perm-groups :deep(.el-collapse) {
  border: none;
}
.perm-groups :deep(.el-collapse-item) {
  margin-bottom: 8px;
  border: 1px solid #ebeef5;
  border-radius: 8px;
  overflow: hidden;
  background: #fafbfc;
}
.perm-groups :deep(.el-collapse-item__header) {
  height: auto;
  min-height: 44px;
  padding: 0 12px;
  background: #f5f7fa;
  border-bottom: none;
  line-height: 1.4;
}
.perm-groups :deep(.el-collapse-item__wrap) {
  border-bottom: none;
  background: #fff;
}
.perm-groups :deep(.el-collapse-item__content) {
  padding: 8px 14px 12px;
}
.perm-collapse-title {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding-right: 8px;
}
.perm-collapse-title__name {
  font-weight: 600;
  color: #303133;
}
.perm-collapse-title__meta {
  margin-left: auto;
  font-size: 12px;
  color: #909399;
  font-variant-numeric: tabular-nums;
}
.perm-child {
  padding: 8px 0 8px 4px;
  border-left: 2px solid #dbeafe;
  margin: 0 0 8px 6px;
  padding-left: 12px;
}
.perm-child:last-child {
  margin-bottom: 0;
}
.perm-child__head {
  margin-bottom: 6px;
}
.perm-child__title {
  font-size: 13px;
  font-weight: 600;
  color: #409eff;
}
.perm-child__boxes {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 0;
}
.perm-child__boxes :deep(.el-checkbox) {
  margin: 4px 8px 4px 0;
}
.perm-code {
  color: #909399;
  font-size: 12px;
  margin-left: 4px;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
</style>
