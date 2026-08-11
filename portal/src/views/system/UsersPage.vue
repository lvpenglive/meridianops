<template>
  <div class="users-page">
    <!-- 统计卡片 -->
    <el-row :gutter="16" class="stat-row">
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card stat-card--total">
          <div class="stat-card__body">
            <div class="stat-card__icon"><User :size="24" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.total }}</div>
              <div class="stat-card__label">用户总数</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card stat-card--enabled">
          <div class="stat-card__body">
            <div class="stat-card__icon"><CircleCheck :size="24" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.enabled }}</div>
              <div class="stat-card__label">已启用</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card stat-card--disabled">
          <div class="stat-card__body">
            <div class="stat-card__icon"><CircleClose :size="24" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.disabled }}</div>
              <div class="stat-card__label">已禁用</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card stat-card--admin">
          <div class="stat-card__body">
            <div class="stat-card__icon"><GoldMedal :size="24" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.admins }}</div>
              <div class="stat-card__label">管理员</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="never" class="table-card">
      <template #header>
        <div class="page-header">
          <div class="page-header__left">
            <span class="title">👥 用户管理</span>
          </div>
          <div class="page-header__right">
            <div class="filters">
              <el-input
                v-model="filters.keyword"
                placeholder="搜索用户名 / 姓名 / 邮箱"
                clearable
                style="width: 240px"
                :prefix-icon="Search"
                @keyup.enter="page = 1"
              />
              <el-select v-model="filters.role" placeholder="角色" clearable style="width: 130px">
                <el-option
                  v-for="r in roleOptions"
                  :key="r.id"
                  :label="r.displayName || r.name"
                  :value="r.name"
                />
              </el-select>
              <el-select v-model="filters.status" placeholder="状态" clearable style="width: 110px">
                <el-option label="启用" :value="true" />
                <el-option label="禁用" :value="false" />
              </el-select>
              <el-button :icon="Refresh" circle title="刷新" @click="loadUsers" />
              <el-button @click="resetFilters">重置</el-button>
            </div>
            <el-button v-permission="'user:create'" type="primary" :icon="Plus" @click="openCreateDialog">
              新增用户
            </el-button>
          </div>
        </div>
      </template>

      <!-- 表格 -->
      <el-table :data="pagedUsers" stripe v-loading="loading" style="width: 100%">
        <el-table-column label="用户" min-width="200">
          <template #default="{ row }">
            <div class="user-cell">
              <el-avatar :size="36" :class="avatarClass(row.role)">
                {{ getInitial(row) }}
              </el-avatar>
              <div class="user-info">
                <div class="user-name">{{ row.username }}</div>
                <div class="user-display">{{ row.displayName || '—' }}</div>
              </div>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="邮箱" width="200">
          <template #default="{ row }">
            <el-tooltip v-if="row.email" :content="row.email" placement="top" :show-after="400">
              <span class="text-ellipsis">{{ row.email }}</span>
            </el-tooltip>
            <span v-else class="text-muted">—</span>
          </template>
        </el-table-column>
        <el-table-column label="角色" width="120">
          <template #default="{ row }">
            <el-tag :type="getRoleColor(row.role)" size="small" effect="light">
              {{ getRoleLabel(row) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="部门" width="150">
          <template #default="{ row }">
            <el-tooltip v-if="deptNameMap[row.departmentId]" :content="deptNameMap[row.departmentId]">
              <span class="text-ellipsis">{{ deptNameMap[row.departmentId] }}</span>
            </el-tooltip>
            <span v-else class="text-muted">—</span>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="row.enabled ? 'success' : 'danger'" size="small" effect="dark" round>
              {{ row.enabled ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="最后登录" width="160">
          <template #default="{ row }">
            <span class="text-muted">{{ formatTime(row.lastLoginAt) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="140" fixed="right" align="center">
          <template #default="{ row }">
            <el-dropdown trigger="click" @command="(cmd: string) => handleAction(cmd, row)">
              <el-button size="small" link type="primary">
                操作<ArrowDown class="el-icon--right" />
              </el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item v-permission="'user:update'" command="edit">
                    编辑
                  </el-dropdown-item>
                  <el-dropdown-item
                    v-permission="'user:toggle_enable'"
                    :command="row.enabled ? 'disable' : 'enable'"
                  >
                    {{ row.enabled ? '禁用' : '启用' }}
                  </el-dropdown-item>
                  <el-dropdown-item v-permission="'user:reset_password'" command="reset" divided>
                    重置密码
                  </el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </template>
        </el-table-column>
      </el-table>

      <!-- 分页 -->
      <div class="pagination-wrap">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :total="filteredUsers.length"
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          background
        />
      </div>
    </el-card>

    <!-- 新增用户对话框 -->
    <el-dialog v-model="createDialogVisible" width="560px" class="user-dialog" @closed="resetCreateForm">
      <template #header>
        <div class="dialog-header">
          <div class="dialog-header__icon"><Plus :size="20" /></div>
          <div class="dialog-header__text">
            <div class="dialog-header__title">新增用户</div>
            <div class="dialog-header__desc">创建一个新的系统账号，标 <span class="required-mark">*</span> 为必填项</div>
          </div>
        </div>
      </template>
      <el-form ref="createFormRef" :model="createForm" :rules="createRules" label-width="90px" class="user-form">
        <div class="form-section">
          <div class="form-section__title">基本信息</div>
          <el-row :gutter="16">
            <el-col :span="12">
              <el-form-item label="用户名" prop="username">
                <el-input v-model="createForm.username" placeholder="登录用户名" clearable />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="密码" prop="password">
                <el-input v-model="createForm.password" type="password" show-password placeholder="至少 6 位" />
              </el-form-item>
            </el-col>
          </el-row>
          <el-row :gutter="16">
            <el-col :span="12">
              <el-form-item label="姓名" prop="displayName">
                <el-input v-model="createForm.displayName" placeholder="显示名（可选）" clearable />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="邮箱" prop="email">
                <el-input v-model="createForm.email" placeholder="邮箱（可选）" clearable />
              </el-form-item>
            </el-col>
          </el-row>
        </div>
        <div class="form-section">
          <div class="form-section__title">权限分配</div>
          <el-row :gutter="16">
            <el-col :span="12">
              <el-form-item label="角色" prop="roleId">
                <el-select v-model="createForm.roleId" placeholder="选择角色" style="width: 100%">
                  <el-option
                    v-for="r in roleOptions"
                    :key="r.id"
                    :label="r.displayName || r.name"
                    :value="r.id"
                    :disabled="!r.enabled"
                  />
                </el-select>
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="部门" prop="departmentId">
                <el-tree-select
                  v-model="createForm.departmentId"
                  :data="deptTreeData"
                  :props="{ label: 'name', value: 'id', children: 'children' }"
                  placeholder="选择部门（可选）"
                  clearable
                  check-strictly
                  style="width: 100%"
                />
              </el-form-item>
            </el-col>
          </el-row>
          <el-form-item label="启用账号">
            <div class="switch-with-desc">
              <el-switch v-model="createForm.enabled" />
              <span class="switch-desc">{{ createForm.enabled ? '开启后用户可立即登录' : '关闭后用户无法登录' }}</span>
            </div>
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="createDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleCreate">创建用户</el-button>
      </template>
    </el-dialog>

    <!-- 编辑用户对话框 -->
    <el-dialog v-model="editDialogVisible" width="560px" class="user-dialog">
      <template #header>
        <div class="dialog-header">
          <div class="dialog-header__icon dialog-header__icon--edit"><EditPen :size="20" /></div>
          <div class="dialog-header__text">
            <div class="dialog-header__title">编辑用户</div>
            <div class="dialog-header__desc">修改用户信息与权限分配</div>
          </div>
        </div>
      </template>
      <el-form ref="editFormRef" :model="editForm" :rules="editRules" label-width="90px" class="user-form">
        <div class="form-section">
          <div class="form-section__title">基本信息</div>
          <el-row :gutter="16">
            <el-col :span="12">
              <el-form-item label="用户名">
                <el-input :model-value="editForm.username" disabled />
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="姓名" prop="displayName">
                <el-input v-model="editForm.displayName" placeholder="显示名" clearable />
              </el-form-item>
            </el-col>
          </el-row>
          <el-row :gutter="16">
            <el-col :span="12">
              <el-form-item label="邮箱" prop="email">
                <el-input v-model="editForm.email" placeholder="邮箱" clearable />
              </el-form-item>
            </el-col>
          </el-row>
        </div>
        <div class="form-section">
          <div class="form-section__title">权限分配</div>
          <el-row :gutter="16">
            <el-col :span="12">
              <el-form-item label="角色" prop="roleId">
                <el-select v-model="editForm.roleId" style="width: 100%">
                  <el-option
                    v-for="r in roleOptions"
                    :key="r.id"
                    :label="r.displayName || r.name"
                    :value="r.id"
                    :disabled="!r.enabled"
                  />
                </el-select>
              </el-form-item>
            </el-col>
            <el-col :span="12">
              <el-form-item label="部门" prop="departmentId">
                <el-tree-select
                  v-model="editForm.departmentId"
                  :data="deptTreeData"
                  :props="{ label: 'name', value: 'id', children: 'children' }"
                  placeholder="选择部门（可选）"
                  clearable
                  check-strictly
                  style="width: 100%"
                />
              </el-form-item>
            </el-col>
          </el-row>
          <el-form-item label="启用账号">
            <div class="switch-with-desc">
              <el-switch v-model="editForm.enabled" />
              <span class="switch-desc">{{ editForm.enabled ? '开启后用户可立即登录' : '关闭后用户无法登录' }}</span>
            </div>
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="editDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleEdit">保存修改</el-button>
      </template>
    </el-dialog>

    <!-- 重置密码对话框 -->
    <el-dialog v-model="passwordDialogVisible" width="440px" class="user-dialog">
      <template #header>
        <div class="dialog-header">
          <div class="dialog-header__icon dialog-header__icon--warning"><Key :size="20" /></div>
          <div class="dialog-header__text">
            <div class="dialog-header__title">重置密码</div>
            <div class="dialog-header__desc">为用户设置新的登录密码</div>
          </div>
        </div>
      </template>
      <el-form ref="passwordFormRef" :model="passwordForm" :rules="passwordRules" label-width="90px" class="user-form">
        <el-form-item label="用户名">
          <el-input :model-value="passwordForm.username" disabled />
        </el-form-item>
        <el-form-item label="新密码" prop="password">
          <el-input v-model="passwordForm.password" type="password" show-password placeholder="至少 6 位" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="passwordDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleResetPassword">确认重置</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Search, ArrowDown, User, CircleCheck, CircleClose, GoldMedal, Refresh, EditPen, Key } from '@element-plus/icons-vue'
import * as usersApi from '../../api/users'
import * as rolesApi from '../../api/roles'
import * as deptsApi from '../../api/departments'
import type { UserInfo, Role, Department, DepartmentNode } from '../../api/types'

// ---- 基础数据 ----
const users = ref<UserInfo[]>([])
const roles = ref<Role[]>([])
const departments = ref<Department[]>([])
const loading = ref(false)
const submitting = ref(false)

// ---- 筛选 ----
const filters = reactive({
  keyword: '',
  role: '' as string | boolean,
  status: null as boolean | null,
})

// ---- 分页 ----
const page = ref(1)
const pageSize = ref(10)

// ---- 计算属性 ----
const roleOptions = computed(() => roles.value)
const roleMap = computed<Record<string, Role>>(() => {
  const m: Record<string, Role> = {}
  roles.value.forEach((r) => (m[r.id] = r))
  return m
})
const deptNameMap = computed<Record<string, string>>(() => {
  const m: Record<string, string> = {}
  departments.value.forEach((d) => (m[d.id] = d.name))
  return m
})
const deptTreeData = computed(() => buildDeptTree(departments.value))

function buildDeptTree(list: Department[]): DepartmentNode[] {
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

/** 筛选后的用户列表 */
const filteredUsers = computed(() => {
  return users.value.filter((u) => {
    if (filters.keyword) {
      const kw = filters.keyword.toLowerCase()
      const hit =
        u.username.toLowerCase().includes(kw) ||
        (u.displayName?.toLowerCase().includes(kw) ?? false) ||
        (u.email?.toLowerCase().includes(kw) ?? false)
      if (!hit) return false
    }
    if (filters.role) {
      if (u.role !== filters.role) return false
    }
    if (filters.status !== null && filters.status !== '') {
      if (u.enabled !== filters.status) return false
    }
    return true
  })
})

/** 分页切片 */
const pagedUsers = computed(() => {
  const start = (page.value - 1) * pageSize.value
  return filteredUsers.value.slice(start, start + pageSize.value)
})

/** 统计数据 */
const stats = computed(() => ({
  total: users.value.length,
  enabled: users.value.filter((u) => u.enabled).length,
  disabled: users.value.filter((u) => !u.enabled).length,
  admins: users.value.filter((u) => u.role === 'admin').length,
}))

function resetFilters() {
  filters.keyword = ''
  filters.role = ''
  filters.status = null
  page.value = 1
}

// ---- 辅助函数 ----
function getRoleColor(role: string) {
  return ({ admin: 'danger', operator: 'warning', viewer: 'info' } as Record<string, string>)[role] || 'info'
}
function getRoleLabel(row: UserInfo) {
  if (row.roleId && roleMap.value[row.roleId]) {
    const r = roleMap.value[row.roleId]
    return r.displayName || r.name
  }
  return ({ admin: '管理员', operator: '运维', viewer: '只读' } as Record<string, string>)[row.role] || row.role
}
function avatarClass(role: string) {
  return `avatar--${role}`
}
function getInitial(row: UserInfo) {
  return (row.displayName || row.username || '?').charAt(0).toUpperCase()
}
function formatTime(s: string | null | undefined): string {
  if (!s) return '—'
  return s.replace('T', ' ').slice(0, 16)
}

// ---- 数据加载 ----
async function loadUsers() {
  loading.value = true
  try {
    users.value = await usersApi.listUsers()
  } catch {
    // 拦截器已提示
  } finally {
    loading.value = false
  }
}

async function loadRolesAndDepts() {
  try {
    const [r, d] = await Promise.all([rolesApi.listRoles(), deptsApi.listDepartments()])
    roles.value = r
    departments.value = d
  } catch {
    // 拦截器已提示
  }
}

onMounted(() => {
  loadUsers()
  loadRolesAndDepts()
})

// ---- 下拉操作处理 ----
function handleAction(cmd: string, row: UserInfo) {
  switch (cmd) {
    case 'edit':
      openEditDialog(row)
      break
    case 'enable':
    case 'disable':
      handleToggleEnable(row)
      break
    case 'reset':
      openPasswordDialog(row)
      break
  }
}

// ---- 新增用户 ----
const createDialogVisible = ref(false)
const createFormRef = ref<FormInstance>()
const createForm = reactive({
  username: '',
  password: '',
  displayName: '',
  email: '',
  roleId: '',
  departmentId: '',
  enabled: true,
})
const createRules: FormRules = {
  username: [{ required: true, message: '请输入用户名', trigger: 'blur' }],
  password: [
    { required: true, message: '请输入密码', trigger: 'blur' },
    { min: 6, message: '至少 6 位', trigger: 'blur' },
  ],
  roleId: [{ required: true, message: '请选择角色', trigger: 'change' }],
}

function openCreateDialog() {
  resetCreateForm()
  createDialogVisible.value = true
}
function resetCreateForm() {
  createForm.username = ''
  createForm.password = ''
  createForm.displayName = ''
  createForm.email = ''
  createForm.roleId = ''
  createForm.departmentId = ''
  createForm.enabled = true
  createFormRef.value?.clearValidate()
}

async function handleCreate() {
  if (!createFormRef.value) return
  await createFormRef.value.validate(async (valid) => {
    if (!valid) return
    submitting.value = true
    try {
      await usersApi.createUser({
        username: createForm.username,
        password: createForm.password,
        displayName: createForm.displayName || undefined,
        email: createForm.email || undefined,
        roleId: createForm.roleId || undefined,
        departmentId: createForm.departmentId || undefined,
        enabled: createForm.enabled,
      })
      ElMessage.success('用户创建成功')
      createDialogVisible.value = false
      await loadUsers()
    } catch {
      // 拦截器已提示
    } finally {
      submitting.value = false
    }
  })
}

// ---- 编辑用户 ----
const editDialogVisible = ref(false)
const editFormRef = ref<FormInstance>()
const editForm = reactive({
  id: '',
  username: '',
  displayName: '',
  email: '',
  roleId: '',
  departmentId: '',
  enabled: true,
})
const editRules: FormRules = {
  roleId: [{ required: true, message: '请选择角色', trigger: 'change' }],
}

function openEditDialog(row: UserInfo) {
  editForm.id = row.id
  editForm.username = row.username
  editForm.displayName = row.displayName
  editForm.email = row.email
  editForm.roleId = row.roleId || ''
  editForm.departmentId = row.departmentId || ''
  editForm.enabled = row.enabled
  editDialogVisible.value = true
}

async function handleEdit() {
  if (!editFormRef.value) return
  await editFormRef.value.validate(async (valid) => {
    if (!valid) return
    submitting.value = true
    try {
      await usersApi.updateUser(editForm.id, {
        displayName: editForm.displayName,
        email: editForm.email,
        roleId: editForm.roleId || undefined,
        departmentId: editForm.departmentId || undefined,
        enabled: editForm.enabled,
      })
      ElMessage.success('保存成功')
      editDialogVisible.value = false
      await loadUsers()
    } catch {
      // 拦截器已提示
    } finally {
      submitting.value = false
    }
  })
}

// ---- 重置密码 ----
const passwordDialogVisible = ref(false)
const passwordFormRef = ref<FormInstance>()
const passwordForm = reactive({ id: '', username: '', password: '' })
const passwordRules: FormRules = {
  password: [
    { required: true, message: '请输入新密码', trigger: 'blur' },
    { min: 6, message: '至少 6 位', trigger: 'blur' },
  ],
}

function openPasswordDialog(row: UserInfo) {
  passwordForm.id = row.id
  passwordForm.username = row.username
  passwordForm.password = ''
  passwordDialogVisible.value = true
}

async function handleResetPassword() {
  if (!passwordFormRef.value) return
  await passwordFormRef.value.validate(async (valid) => {
    if (!valid) return
    submitting.value = true
    try {
      await usersApi.resetUserPassword(passwordForm.id, passwordForm.password)
      ElMessage.success('密码已重置')
      passwordDialogVisible.value = false
    } catch {
      // 拦截器已提示
    } finally {
      submitting.value = false
    }
  })
}

// ---- 启停 ----
async function handleToggleEnable(row: UserInfo) {
  const action = row.enabled ? '禁用' : '启用'
  await ElMessageBox.confirm(`确定${action}用户 "${row.username}" 吗？`, '提示', { type: 'warning' })
  try {
    await usersApi.toggleUserEnable(row.id, !row.enabled)
    ElMessage.success(`${action}成功`)
    await loadUsers()
  } catch {
    // 拦截器已提示
  }
}
</script>

<style scoped>
.users-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* ---- 统计卡片 ---- */
.stat-row {
  margin-bottom: 0;
}
.stat-card {
  border: none;
  border-radius: 10px;
  transition: transform 0.25s ease, box-shadow 0.25s ease;
  background: #fff;
}
.stat-card :deep(.el-card__body) {
  padding: 18px 20px;
}
.stat-card:hover {
  transform: translateY(-3px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.08);
}
.stat-card__body {
  display: flex;
  align-items: center;
  gap: 16px;
}
.stat-card__icon {
  width: 52px;
  height: 52px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  flex-shrink: 0;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
}
.stat-card__value {
  font-size: 28px;
  font-weight: 700;
  color: #1f2937;
  line-height: 1.1;
  letter-spacing: -0.5px;
}
.stat-card__label {
  font-size: 13px;
  color: #6b7280;
  margin-top: 4px;
  font-weight: 500;
}
.stat-card--total .stat-card__icon {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
.stat-card--enabled .stat-card__icon {
  background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);
}
.stat-card--disabled .stat-card__icon {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.stat-card--admin .stat-card__icon {
  background: linear-gradient(135deg, #f7971e 0%, #ffd200 100%);
}

/* ---- 表格卡片 ---- */
.table-card {
  border-radius: 10px;
}
.table-card :deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #f0f0f0;
}
.table-card :deep(.el-table__row:hover > td) {
  background-color: #f7f9fc !important;
}
.table-card :deep(.el-table__empty-block) {
  min-height: 180px;
}

/* ---- 页面Header ---- */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 20px;
  flex-wrap: wrap;
}
.page-header__left {
  flex-shrink: 0;
}
.page-header__right {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
.page-header .title {
  font-weight: 600;
  font-size: 16px;
  color: #1f2937;
}

/* ---- 筛选工具栏 ---- */
.filters {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}

/* ---- 用户单元格 ---- */
.user-cell {
  display: flex;
  align-items: center;
  gap: 10px;
}
.user-info {
  min-width: 0;
}
.user-name {
  font-weight: 600;
  color: #1f2937;
  font-size: 14px;
  line-height: 1.2;
}
.user-display {
  font-size: 12px;
  color: #909399;
  margin-top: 2px;
}

/* ---- 头像颜色 ---- */
.avatar--admin {
  background: linear-gradient(135deg, #fef0f0 0%, #fde2e2 100%);
  color: #f56c6c;
  font-weight: 600;
}
.avatar--operator {
  background: linear-gradient(135deg, #fdf6ec 0%, #faecd8 100%);
  color: #e6a23c;
  font-weight: 600;
}
.avatar--viewer {
  background: linear-gradient(135deg, #f4f4f5 0%, #e9e9eb 100%);
  color: #909399;
  font-weight: 600;
}

/* ---- 文本工具 ---- */
.text-ellipsis {
  display: inline-block;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  vertical-align: middle;
}
.text-muted {
  color: #c0c4cc;
}

/* ---- 分页 ---- */
.pagination-wrap {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

/* ---- 对话框 ---- */
.user-dialog :deep(.el-dialog__header) {
  padding: 20px 24px 16px;
  margin-right: 0;
  border-bottom: 1px solid #f0f0f0;
}
.user-dialog :deep(.el-dialog__body) {
  padding: 20px 24px;
}
.user-dialog :deep(.el-dialog__footer) {
  padding: 12px 24px 20px;
  border-top: 1px solid #f0f0f0;
}
.user-dialog :deep(.el-dialog__headerbtn) {
  top: 20px;
  right: 20px;
}

/* 对话框头部 */
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
.dialog-header__icon--edit {
  background: linear-gradient(135deg, #11998e 0%, #38ef7d 100%);
}
.dialog-header__icon--warning {
  background: linear-gradient(135deg, #f7971e 0%, #ffd200 100%);
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
.required-mark {
  color: #f56c6c;
}

/* 表单分区 */
.user-form .form-section {
  margin-bottom: 8px;
}
.user-form .form-section:last-child {
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
.user-form :deep(.el-form-item) {
  margin-bottom: 18px;
}
.user-form :deep(.el-form-item__label) {
  font-weight: 500;
  color: #4b5563;
}

/* 开关带描述 */
.switch-with-desc {
  display: flex;
  align-items: center;
  gap: 10px;
}
.switch-desc {
  font-size: 12px;
  color: #909399;
}

/* ---- 响应式 ---- */
@media (max-width: 1200px) {
  .stat-card__value {
    font-size: 24px;
  }
  .stat-card__icon {
    width: 44px;
    height: 44px;
  }
}
@media (max-width: 992px) {
  .page-header {
    flex-direction: column;
    align-items: stretch;
  }
  .page-header__right {
    justify-content: space-between;
  }
}
</style>
