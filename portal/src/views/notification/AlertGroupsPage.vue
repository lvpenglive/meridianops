<template>
  <div class="alert-groups-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><UserFilled /></el-icon>
        <span>告警组维护</span>
        <span class="page-sub">维护告警组层级与成员，短信策略「选择组」可引用父组以包含所有子孙成员</span>
      </div>
      <div class="header-actions">
        <el-button
          v-if="hasPermission('alert_group:manage')"
          type="primary"
          :icon="Plus"
          @click="openCreateTop"
        >
          新增顶级组
        </el-button>
      </div>
    </div>

    <el-row :gutter="16">
      <!-- 左：树 -->
      <el-col :span="8">
        <el-card shadow="never" class="tree-card">
          <template #header>
            <div class="card-header">
              <span>告警组树（{{ totalCount }}）</span>
              <el-button :icon="Refresh" size="small" @click="fetchTree">刷新</el-button>
            </div>
          </template>
          <el-tree
            v-loading="treeLoading"
            :data="tree"
            node-key="id"
            :props="{ label: 'name', children: 'children' }"
            default-expand-all
            highlight-current
            :current-node-key="selectedId || undefined"
            @node-click="onNodeClick"
            class="group-tree"
          >
            <template #default="{ data }">
              <span class="tree-node" :class="{ active: data.id === selectedId }">
                <span class="tree-name">{{ data.name }}</span>
                <el-tag v-if="!data.enabled" size="small" type="info">停用</el-tag>
                <span class="tree-count">{{ data.memberCount }} 人</span>
              </span>
            </template>
          </el-tree>
          <el-empty v-if="!treeLoading && tree.length === 0" description="暂无告警组" />
        </el-card>
      </el-col>

      <!-- 右：组详情 + 成员 -->
      <el-col :span="16">
        <el-card v-if="selectedGroup" shadow="never">
          <template #header>
            <div class="card-header">
              <span>组详情 — {{ selectedGroup.name }}</span>
              <div class="detail-actions">
                <el-button
                  v-if="hasPermission('alert_group:manage')"
                  size="small"
                  :icon="Plus"
                  @click="openCreateChild(selectedGroup.id)"
                >新增子组</el-button>
                <el-button
                  v-if="hasPermission('alert_group:manage')"
                  size="small"
                  :icon="Plus"
                  @click="openCreateSibling"
                >新增同级</el-button>
                <el-button
                  v-if="hasPermission('alert_group:manage')"
                  size="small"
                  :icon="UserFilled"
                  @click="openMembers(selectedGroup)"
                >成员维护</el-button>
                <el-button
                  v-if="hasPermission('alert_group:manage')"
                  size="small"
                  :icon="Edit"
                  @click="openEdit(selectedGroup)"
                >编辑</el-button>
                <el-button
                  v-if="hasPermission('alert_group:manage')"
                  size="small"
                  :type="selectedGroup.enabled ? 'warning' : 'success'"
                  :icon="selectedGroup.enabled ? 'Close' : 'Check'"
                  @click="onToggle(selectedGroup, !selectedGroup.enabled)"
                >{{ selectedGroup.enabled ? '停用' : '启用' }}</el-button>
                <el-popconfirm
                  :title="`确认删除告警组「${selectedGroup.name}」？`"
                  @confirm="onDelete(selectedGroup)"
                  confirm-button-text="删除"
                  cancel-button-text="取消"
                >
                  <template #reference>
                    <el-button v-if="hasPermission('alert_group:manage')" size="small" type="danger" :icon="Delete">删除</el-button>
                  </template>
                </el-popconfirm>
              </div>
            </div>
          </template>

          <el-descriptions :column="2" border size="small">
            <el-descriptions-item label="编码">{{ selectedGroup.code }}</el-descriptions-item>
            <el-descriptions-item label="名称">{{ selectedGroup.name }}</el-descriptions-item>
            <el-descriptions-item label="上级">
              {{ parentName(selectedGroup.parentId) || '（顶级组）' }}
            </el-descriptions-item>
            <el-descriptions-item label="状态">
              <el-tag :type="selectedGroup.enabled ? 'success' : 'danger'" size="small">
                {{ selectedGroup.enabled ? '启用' : '停用' }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item label="成员数">{{ selectedGroup.memberCount }} 人</el-descriptions-item>
            <el-descriptions-item label="创建人">{{ selectedGroup.createdBy }}</el-descriptions-item>
            <el-descriptions-item label="描述" :span="2">{{ selectedGroup.description || '—' }}</el-descriptions-item>
          </el-descriptions>

          <div class="members-block">
            <div class="block-title">
              <span>直属成员（{{ directMembers.length }}）</span>
              <el-button size="small" text type="primary" :icon="Refresh" @click="loadDirectMembers">刷新</el-button>
            </div>
            <div v-loading="membersLoading" class="members-wrap">
              <el-tag
                v-for="m in directMembers"
                :key="m.id"
                size="small"
                :type="m.enabled ? 'primary' : 'info'"
                class="member-tag"
              >{{ m.displayName || m.username }}</el-tag>
              <span v-if="!membersLoading && directMembers.length === 0" class="list-empty">无直属成员</span>
            </div>
            <div class="block-tip">
              提示：短信策略「选择组」若选中本组的父组，将自动包含本组及所有子孙组成员（按用户 ID 去重，停用组除外）。
            </div>
          </div>
        </el-card>

        <el-empty v-else description="请选择左侧告警组查看详情" />
      </el-col>
    </el-row>

    <!-- 编辑 / 新增弹窗 -->
    <el-dialog
      v-model="editVisible"
      :title="isEdit ? '编辑告警组' : '新增告警组'"
      width="560"
      :close-on-click-modal="false"
      @closed="resetEditForm"
    >
      <el-form ref="editFormRef" :model="editForm" :rules="editRules" label-width="80">
        <el-form-item label="编码" prop="code">
          <el-input v-model="editForm.code" placeholder="业务可读编码，全局唯一" />
        </el-form-item>
        <el-form-item label="名称" prop="name">
          <el-input v-model="editForm.name" placeholder="告警组名称" />
        </el-form-item>
        <el-form-item label="上级">
          <el-select
            v-model="editForm.parentId"
            placeholder="顶级组（无上级）"
            clearable
            style="width: 100%;"
          >
            <el-option
              v-for="o in parentOptions"
              :key="o.id"
              :label="o.label"
              :value="o.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="editForm.description" type="textarea" :rows="3" placeholder="可选" />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="editForm.enabled" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editVisible = false">取消</el-button>
        <el-button type="primary" :loading="editSaving" @click="submitEdit">保存</el-button>
      </template>
    </el-dialog>

    <!-- 成员维护弹窗 -->
    <el-dialog
      v-model="memberVisible"
      :title="`成员维护 — ${currentGroup?.name || ''}`"
      width="780"
      :close-on-click-modal="false"
      @closed="resetMemberForm"
    >
      <div class="member-layout">
        <div class="member-panel">
          <div class="panel-title">候选人员</div>
          <el-input v-model="memberKeyword" placeholder="搜索用户名/姓名" clearable size="small" />
          <div class="member-list">
              <div v-if="filteredCandidates.length === 0" class="list-empty">无可选人员</div>
              <div
                v-for="u in filteredCandidates"
                :key="u.id"
                class="member-row"
                @dblclick="addMember(u)"
              >
                <span class="member-name">{{ u.displayName || u.username }}</span>
                <el-button size="small" text type="primary" @click="addMember(u)">添加</el-button>
              </div>
            </div>
          </div>

        <div class="member-arrow">
          <el-button :icon="ArrowLeft" @click="removeAll" plain :disabled="memberSelectedIds.length === 0">全部移除</el-button>
          <el-button :icon="ArrowRight" type="primary" plain :disabled="memberFilteredSelected.length === 0" @click="addAll">全部添加</el-button>
        </div>

        <div class="member-panel">
          <div class="panel-title">已选人员（{{ memberSelectedIds.length }}）</div>
          <el-input v-model="memberSelectedKeyword" placeholder="搜索已选" clearable size="small" />
          <div class="member-list">
            <div v-if="memberFilteredSelected.length === 0" class="list-empty">未选</div>
            <div
              v-for="u in memberFilteredSelected"
              :key="u.id"
              class="member-row"
              @dblclick="removeMember(u)"
            >
              <span class="member-name">{{ u.displayName || u.username }}</span>
              <el-button size="small" text type="danger" @click="removeMember(u)">移除</el-button>
            </div>
          </div>
        </div>
      </div>
      <template #footer>
        <el-button @click="memberVisible = false">取消</el-button>
        <el-button type="primary" :loading="memberSaving" @click="saveMembers">保存成员</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import {
  Plus,
  Refresh,
  Edit,
  Delete,
  UserFilled,
  ArrowLeft,
  ArrowRight,
} from '@element-plus/icons-vue'
import {
  createAlertGroup,
  deleteAlertGroup,
  getAlertGroupTree,
  listAlertGroupMembers,
  replaceAlertGroupMembers,
  toggleAlertGroup,
  updateAlertGroup,
  type AlertGroup,
  type AlertGroupMember,
  type AlertGroupTreeNode,
} from '../../api/alertGroup'
import { listUsers } from '../../api/users'
import type { UserInfo } from '../../api/types'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()
const hasPermission = (code: string) => userStore.hasPermission(code)

const tree = ref<AlertGroupTreeNode[]>([])
const treeLoading = ref(false)
const selectedId = ref<string | null>(null)
const totalCount = ref(0)

/// 扁平化树（用于详情回填 / 上级下拉 / 名称反查）
const flatGroups = ref<AlertGroup[]>([])
function flatten(nodes: AlertGroupTreeNode[], depth: number, acc: AlertGroup[]) {
  for (const n of nodes) {
    acc.push({
      id: n.id,
      code: n.code,
      name: n.name,
      description: n.description,
      enabled: n.enabled,
      parentId: n.parentId ?? null,
      createdBy: '',
      createdAt: '',
      updatedAt: '',
      memberCount: n.memberCount,
      memberNames: [],
    })
    flatten(n.children, depth + 1, acc)
  }
}

async function fetchTree() {
  treeLoading.value = true
  try {
    const data = await getAlertGroupTree()
    tree.value = data
    const acc: AlertGroup[] = []
    flatten(data, 0, acc)
    flatGroups.value = acc
    totalCount.value = acc.length
    if (selectedId.value && !acc.find((g) => g.id === selectedId.value)) {
      selectedId.value = null
    }
  } catch (e: unknown) {
    ElMessage.error((e as Error).message || '加载树失败')
  } finally {
    treeLoading.value = false
  }
}

const selectedGroup = computed<AlertGroup | null>(() => {
  if (!selectedId.value) return null
  return flatGroups.value.find((g) => g.id === selectedId.value) || null
})

function parentName(pid?: string | null) {
  if (!pid) return ''
  return flatGroups.value.find((g) => g.id === pid)?.name || '(已删除的上级)'
}

function onNodeClick(data: AlertGroupTreeNode) {
  selectedId.value = data.id
  loadDirectMembers()
}

// 直属成员
const directMembers = ref<AlertGroupMember[]>([])
const membersLoading = ref(false)
async function loadDirectMembers() {
  if (!selectedId.value) {
    directMembers.value = []
    return
  }
  membersLoading.value = true
  try {
    directMembers.value = await listAlertGroupMembers(selectedId.value, false)
  } catch (e: unknown) {
    ElMessage.error((e as Error).message || '加载成员失败')
  } finally {
    membersLoading.value = false
  }
}

async function onDelete(row: AlertGroup) {
  try {
    await deleteAlertGroup(row.id)
    ElMessage.success('已删除')
    if (selectedId.value === row.id) selectedId.value = null
    fetchTree()
  } catch (e: unknown) {
    ElMessage.error((e as Error).message || '删除失败')
  }
}

async function onToggle(row: AlertGroup, enabled: boolean) {
  try {
    await toggleAlertGroup(row.id, enabled)
    ElMessage.success(enabled ? '已启用' : '已停用')
    fetchTree()
  } catch (e: unknown) {
    ElMessage.error((e as Error).message || '操作失败')
  }
}

// 上级下拉：排除自身及其所有子孙（防循环）
const parentOptions = computed(() => {
  if (!selectedId.value || !isEdit.value) {
    // 新增时不排除任何组
    return flatGroups.value.map((g) => ({ id: g.id, label: g.name }))
  }
  const forbidden = new Set<string>([selectedId.value])
  // 收集 descendants
  const childrenMap = new Map<string, AlertGroup[]>()
  for (const g of flatGroups.value) {
    if (g.parentId) childrenMap.set(g.parentId, [...(childrenMap.get(g.parentId) || []), g])
  }
  const stack = [selectedId.value]
  while (stack.length) {
    const cur = stack.pop()!
    for (const c of childrenMap.get(cur) || []) {
      forbidden.add(c.id)
      stack.push(c.id)
    }
  }
  return flatGroups.value
    .filter((g) => !forbidden.has(g.id))
    .map((g) => ({ id: g.id, label: g.name }))
})

// ---- 编辑 / 新增弹窗 ----
const editVisible = ref(false)
const isEdit = ref(false)
const editSaving = ref(false)
const editFormRef = ref<FormInstance | null>(null)
const editForm = reactive({
  id: '',
  code: '',
  name: '',
  description: '',
  parentId: '' as string,
  enabled: true,
})
const editRules: FormRules = {
  code: [{ required: true, message: '请输入编码', trigger: 'blur' }],
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
}

function resetEditForm() {
  editForm.id = ''
  editForm.code = ''
  editForm.name = ''
  editForm.description = ''
  editForm.parentId = ''
  editForm.enabled = true
}

function openCreateTop() {
  resetEditForm()
  isEdit.value = false
  editVisible.value = true
}
function openCreateSibling() {
  if (!selectedGroup.value) return
  resetEditForm()
  isEdit.value = false
  editForm.parentId = selectedGroup.value.parentId || ''
  editVisible.value = true
}
function openCreateChild(parentId: string) {
  resetEditForm()
  isEdit.value = false
  editForm.parentId = parentId
  editVisible.value = true
}
function openEdit(row: AlertGroup) {
  resetEditForm()
  isEdit.value = true
  editForm.id = row.id
  editForm.code = row.code
  editForm.name = row.name
  editForm.description = row.description || ''
  editForm.parentId = row.parentId || ''
  editForm.enabled = row.enabled
  editVisible.value = true
}

async function submitEdit() {
  const ok = await editFormRef.value?.validate().catch(() => false)
  if (!ok) return
  editSaving.value = true
  const payload = {
    code: editForm.code.trim(),
    name: editForm.name.trim(),
    description: editForm.description || undefined,
    enabled: editForm.enabled,
    parentId: editForm.parentId || undefined,
  }
  try {
    if (isEdit.value) {
      await updateAlertGroup(editForm.id, payload)
      ElMessage.success('已保存')
    } else {
      await createAlertGroup(payload)
      ElMessage.success('已创建')
    }
    editVisible.value = false
    fetchTree()
  } catch (e: unknown) {
    ElMessage.error((e as Error).message || '保存失败')
  } finally {
    editSaving.value = false
  }
}

// ---- 成员维护弹窗 ----
const memberVisible = ref(false)
const memberSaving = ref(false)
const currentGroup = ref<AlertGroup | null>(null)
const memberKeyword = ref('')
const memberSelectedKeyword = ref('')

const allUsers = ref<UserInfo[]>([])
const memberSelectedIds = ref<string[]>([])

const enabledUsers = computed(() => allUsers.value.filter((u) => u.enabled))

const filteredCandidates = computed<UserInfo[]>(() => {
  const kw = memberKeyword.value.trim().toLowerCase()
  const sel = new Set(memberSelectedIds.value)
  let pool = enabledUsers.value.filter((u) => !sel.has(u.id))
  if (kw) {
    pool = pool.filter(
      (u) =>
        (u.displayName || '').toLowerCase().includes(kw) ||
        (u.username || '').toLowerCase().includes(kw),
    )
  }
  return pool
})

const memberFilteredSelected = computed<UserInfo[]>(() => {
  const ids = new Set(memberSelectedIds.value)
  const list: UserInfo[] = []
  for (const id of ids) {
    const u = allUsers.value.find((x) => x.id === id)
    if (u && u.enabled) list.push(u)
  }
  const kw = memberSelectedKeyword.value.trim().toLowerCase()
  if (!kw) return list
  return list.filter(
    (u) =>
      (u.displayName || '').toLowerCase().includes(kw) ||
      (u.username || '').toLowerCase().includes(kw),
  )
})

function addMember(u: UserInfo) {
  if (!memberSelectedIds.value.includes(u.id)) {
    memberSelectedIds.value.push(u.id)
  }
}
function removeMember(u: UserInfo) {
  memberSelectedIds.value = memberSelectedIds.value.filter((id) => id !== u.id)
}
function addAll() {
  filteredCandidates.value.forEach(addMember)
}
function removeAll() {
  const ids = new Set(memberFilteredSelected.value.map((u) => u.id))
  memberSelectedIds.value = memberSelectedIds.value.filter((id) => !ids.has(id))
}

function resetMemberForm() {
  memberSelectedIds.value = []
  memberKeyword.value = ''
  memberSelectedKeyword.value = ''
  currentGroup.value = null
}

async function openMembers(row: AlertGroup) {
  resetMemberForm()
  currentGroup.value = row
  if (allUsers.value.length === 0) {
    allUsers.value = await listUsers()
  }
  memberVisible.value = true
  try {
    const members: AlertGroupMember[] = await listAlertGroupMembers(row.id, false)
    memberSelectedIds.value = members.map((m) => m.id)
  } catch (e: unknown) {
    ElMessage.error((e as Error).message || '加载成员失败')
  }
}

async function saveMembers() {
  if (!currentGroup.value) return
  memberSaving.value = true
  try {
    await replaceAlertGroupMembers(currentGroup.value.id, memberSelectedIds.value)
    ElMessage.success('已保存')
    memberVisible.value = false
    fetchTree()
  } catch (e: unknown) {
    ElMessage.error((e as Error).message || '保存失败')
  } finally {
    memberSaving.value = false
  }
}

onMounted(() => {
  fetchTree()
})
</script>

<style scoped>
.alert-groups-page {
  padding: 16px;
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
  margin-left: 12px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
  font-weight: normal;
}
.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.detail-actions {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.tree-card {
  height: calc(100vh - 160px);
  overflow: auto;
}
.group-tree {
  min-height: 120px;
}
.tree-node {
  display: flex;
  align-items: center;
  gap: 6px;
}
.tree-node.active .tree-name {
  font-weight: 600;
  color: var(--el-color-primary);
}
.tree-count {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.members-block {
  margin-top: 16px;
}
.block-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-weight: 600;
  margin-bottom: 8px;
}
.members-wrap {
  min-height: 60px;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 4px;
  padding: 8px;
}
.member-tag {
  margin: 2px 4px;
}
.block-tip {
  margin-top: 8px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}

/* 成员维护 */
.member-layout {
  display: grid;
  grid-template-columns: 1fr auto 1fr;
  gap: 12px;
  align-items: stretch;
}
.member-panel {
  border: 1px solid var(--el-border-color);
  border-radius: 4px;
  padding: 8px;
  min-height: 360px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.panel-title {
  font-weight: 600;
  color: var(--el-text-color-regular);
}
.member-list {
  flex: 1;
  max-height: 320px;
  overflow-y: auto;
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 4px;
  padding: 4px;
}
.member-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px;
  border-radius: 4px;
}
.member-row:hover {
  background: var(--el-fill-color-light);
}
.member-name {
  font-size: 13px;
}
.list-empty {
  padding: 12px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
  text-align: center;
}
.member-arrow {
  display: flex;
  flex-direction: column;
  gap: 8px;
  align-self: center;
}
</style>
