<template>
  <div class="profile-page">
    <el-card shadow="never" class="profile-card">
      <template #header>
        <div class="card-header">
          <span class="header-title">个人中心</span>
          <span class="header-sub">{{ userStore.user?.username }}</span>
        </div>
      </template>

      <el-tabs v-model="activeTab" class="profile-tabs">
        <!-- 个人资料 -->
        <el-tab-pane label="个人资料" name="profile">
          <div class="profile-summary">
            <el-avatar :size="72" class="profile-avatar">
              {{ avatarText }}
            </el-avatar>
            <div class="summary-info">
              <div class="summary-name">{{ userStore.user?.displayName || '—' }}</div>
              <div class="summary-meta">
                <el-tag :type="roleTagType" size="small" effect="dark">{{ roleLabel }}</el-tag>
                <span class="summary-username">@{{ userStore.user?.username }}</span>
              </div>
            </div>
          </div>

          <el-descriptions :column="2" border size="default" class="profile-desc">
            <el-descriptions-item label="用户名">{{ userStore.user?.username || '—' }}</el-descriptions-item>
            <el-descriptions-item label="姓名">{{ userStore.user?.displayName || '—' }}</el-descriptions-item>
            <el-descriptions-item label="邮箱">{{ userStore.user?.email || '—' }}</el-descriptions-item>
            <el-descriptions-item label="角色">{{ roleLabel }}</el-descriptions-item>
            <el-descriptions-item label="账号状态">
              <el-tag :type="userStore.user?.enabled ? 'success' : 'danger'" size="small">
                {{ userStore.user?.enabled ? '启用' : '禁用' }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item label="最后登录">{{ formatTime(userStore.user?.lastLoginAt) }}</el-descriptions-item>
            <el-descriptions-item label="创建时间">{{ formatTime(userStore.user?.createdAt) }}</el-descriptions-item>
            <el-descriptions-item label="更新时间">{{ formatTime(userStore.user?.updatedAt) }}</el-descriptions-item>
          </el-descriptions>

          <div class="profile-tip">
            <el-icon><InfoFilled /></el-icon>
            <span>如需修改姓名、邮箱等基本信息，请联系管理员在「用户管理」中更新。</span>
          </div>
        </el-tab-pane>

        <!-- 修改密码 -->
        <el-tab-pane label="修改密码" name="password">
          <div class="pwd-form-wrap">
            <el-alert
              v-if="policy"
              :title="policy.description"
              type="info"
              :closable="false"
              show-icon
              class="policy-alert"
            />
            <el-form
              ref="pwdFormRef"
              :model="pwdForm"
              :rules="pwdRules"
              label-width="100px"
              class="pwd-form"
              @submit.prevent
            >
              <el-form-item label="原密码" prop="oldPassword">
                <el-input
                  v-model="pwdForm.oldPassword"
                  type="password"
                  show-password
                  placeholder="请输入原密码"
                  autocomplete="off"
                />
              </el-form-item>
              <el-form-item label="新密码" prop="newPassword">
                <el-input
                  v-model="pwdForm.newPassword"
                  type="password"
                  show-password
                  placeholder="请输入新密码"
                  autocomplete="off"
                />
              </el-form-item>
              <el-form-item label="确认新密码" prop="confirmPassword">
                <el-input
                  v-model="pwdForm.confirmPassword"
                  type="password"
                  show-password
                  placeholder="请再次输入新密码"
                  autocomplete="off"
                  @keyup.enter="handleChangePassword"
                />
              </el-form-item>
              <el-form-item>
                <el-button type="primary" :loading="pwdLoading" @click="handleChangePassword">
                  确认修改
                </el-button>
                <el-button @click="resetPwdForm">重置</el-button>
              </el-form-item>
            </el-form>
          </div>
        </el-tab-pane>

        <!-- 登录历史（仅 audit:read 权限可见） -->
        <el-tab-pane v-if="canReadAudit" label="登录历史" name="history">
          <div class="history-toolbar">
            <el-button :icon="Refresh" circle @click="loadHistory" />
            <span class="history-tip">展示当前账号最近的登录记录</span>
          </div>
          <el-table :data="loginHistory" v-loading="historyLoading" stripe size="default">
            <el-table-column prop="createdAt" label="时间" min-width="180">
              <template #default="{ row }">{{ formatTime(row.createdAt) }}</template>
            </el-table-column>
            <el-table-column prop="status" label="结果" width="100">
              <template #default="{ row }">
                <el-tag :type="row.status === 'success' ? 'success' : 'danger'" size="small">
                  {{ row.status === 'success' ? '成功' : '失败' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="ip" label="IP 地址" min-width="140" />
            <el-table-column label="详情" min-width="200">
              <template #default="{ row }">
                <span v-if="row.detail">{{ formatDetail(row.detail) }}</span>
                <span v-else class="muted">—</span>
              </template>
            </el-table-column>
            <template #empty>
              <div class="empty-state">暂无登录记录</div>
            </template>
          </el-table>
          <div class="history-pagination">
            <el-pagination
              v-model:current-page="historyPage"
              v-model:page-size="historyPageSize"
              :total="historyTotal"
              :page-sizes="[10, 20, 50]"
              layout="total, sizes, prev, pager, next"
              background
              @current-change="loadHistory"
              @size-change="loadHistory"
            />
          </div>
        </el-tab-pane>
      </el-tabs>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { InfoFilled, Refresh } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import { useUserStore } from '../../stores/user'
import { getPasswordPolicy } from '../../api/system'
import { listAuditLogs } from '../../api/audit'
import type { PasswordPolicy, AuditLog } from '../../api/types'

const userStore = useUserStore()

const activeTab = ref<'profile' | 'password' | 'history'>('profile')

const avatarText = computed(() => {
  const name = userStore.user?.displayName || userStore.user?.username || '?'
  return name.charAt(0).toUpperCase()
})

const roleLabel = computed(() => {
  const map: Record<string, string> = { admin: '管理员', operator: '运维', viewer: '只读' }
  return map[userStore.user?.role || ''] || userStore.user?.role || '—'
})

const roleTagType = computed<'primary' | 'success' | 'info'>(() => {
  const r = userStore.user?.role
  if (r === 'admin') return 'primary'
  if (r === 'operator') return 'success'
  return 'info'
})

const canReadAudit = computed(() => userStore.hasPermission('audit:read'))

// ---- 时间格式化 ----
function formatTime(s?: string | null): string {
  if (!s) return '—'
  try {
    const d = new Date(s)
    if (isNaN(d.getTime())) return s
    return d.toLocaleString('zh-CN', { hour12: false })
  } catch {
    return s
  }
}

// ---- 修改密码 ----
const pwdFormRef = ref<FormInstance>()
const pwdLoading = ref(false)
const policy = ref<PasswordPolicy | null>(null)

const pwdForm = reactive({
  oldPassword: '',
  newPassword: '',
  confirmPassword: '',
})

const pwdRules: FormRules = {
  oldPassword: [{ required: true, message: '请输入原密码', trigger: 'blur' }],
  newPassword: [
    { required: true, message: '请输入新密码', trigger: 'blur' },
    {
      validator: (_rule, value: string, callback) => {
        if (!value) return callback()
        if (value === pwdForm.oldPassword) {
          return callback(new Error('新密码不能与原密码相同'))
        }
        if (policy.value) {
          if (value.length < policy.value.minLength) {
            return callback(new Error(`密码至少 ${policy.value.minLength} 位`))
          }
          if (policy.value.requireUppercase && !/[A-Z]/.test(value)) {
            return callback(new Error('需包含大写字母'))
          }
          if (policy.value.requireLowercase && !/[a-z]/.test(value)) {
            return callback(new Error('需包含小写字母'))
          }
          if (policy.value.requireDigit && !/\d/.test(value)) {
            return callback(new Error('需包含数字'))
          }
          if (policy.value.requireSpecial && !/[^A-Za-z0-9]/.test(value)) {
            return callback(new Error('需包含特殊字符'))
          }
        }
        callback()
      },
      trigger: 'blur',
    },
  ],
  confirmPassword: [
    { required: true, message: '请再次输入新密码', trigger: 'blur' },
    {
      validator: (_rule, value: string, callback) => {
        if (!value) return callback()
        if (value !== pwdForm.newPassword) {
          return callback(new Error('两次输入的密码不一致'))
        }
        callback()
      },
      trigger: 'blur',
    },
  ],
}

async function loadPolicy() {
  try {
    policy.value = await getPasswordPolicy()
  } catch {
    // 拉取策略失败不阻塞，后端会用默认策略兜底校验
    policy.value = null
  }
}

async function handleChangePassword() {
  if (!pwdFormRef.value) return
  await pwdFormRef.value.validate(async (valid) => {
    if (!valid) return
    pwdLoading.value = true
    try {
      await userStore.changePassword({
        oldPassword: pwdForm.oldPassword,
        newPassword: pwdForm.newPassword,
      })
      ElMessage.success('密码修改成功')
      resetPwdForm()
    } catch {
      // 错误提示由 request 拦截器统一处理
    } finally {
      pwdLoading.value = false
    }
  })
}

function resetPwdForm() {
  pwdForm.oldPassword = ''
  pwdForm.newPassword = ''
  pwdForm.confirmPassword = ''
  pwdFormRef.value?.clearValidate()
}

// ---- 登录历史 ----
const historyLoading = ref(false)
const loginHistory = ref<AuditLog[]>([])
const historyTotal = ref(0)
const historyPage = ref(1)
const historyPageSize = ref(10)

async function loadHistory() {
  if (!canReadAudit.value) return
  historyLoading.value = true
  try {
    const res = await listAuditLogs({
      actor: userStore.user?.username,
      action: 'login',
      page: historyPage.value,
      pageSize: historyPageSize.value,
    })
    loginHistory.value = res.items
    historyTotal.value = res.total
  } catch {
    // 错误提示由 request 拦截器统一处理
  } finally {
    historyLoading.value = false
  }
}

function formatDetail(detail: string): string {
  try {
    const obj = JSON.parse(detail) as Record<string, unknown>
    const reason = obj.reason
    const reasonMap: Record<string, string> = {
      bad_password: '密码错误',
      disabled: '账号已禁用',
      locked: '账号已锁定',
    }
    if (typeof reason === 'string' && reasonMap[reason]) {
      return reasonMap[reason]
    }
    return detail
  } catch {
    return detail
  }
}

onMounted(() => {
  loadPolicy()
  if (canReadAudit.value) {
    loadHistory()
  }
})
</script>

<style scoped>
.profile-page {
  max-width: 960px;
  margin: 0 auto;
}

.card-header {
  display: flex;
  align-items: baseline;
  gap: 12px;
}

.header-title {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.header-sub {
  font-size: 13px;
  color: #909399;
}

.profile-tabs {
  margin-top: 4px;
}

/* 个人资料摘要 */
.profile-summary {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 16px 20px;
  background: linear-gradient(135deg, #f0f5ff 0%, #f6faff 100%);
  border-radius: 8px;
  margin-bottom: 24px;
}

.profile-avatar {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  font-size: 28px;
  font-weight: 600;
  flex-shrink: 0;
}

.summary-info {
  flex: 1;
}

.summary-name {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 8px;
}

.summary-meta {
  display: flex;
  align-items: center;
  gap: 10px;
}

.summary-username {
  font-size: 13px;
  color: #909399;
}

.profile-desc {
  margin-bottom: 16px;
}

.profile-tip {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 10px 12px;
  background: #f4f4f5;
  border-radius: 6px;
  font-size: 12px;
  color: #909399;
}

/* 修改密码 */
.pwd-form-wrap {
  max-width: 480px;
}

.policy-alert {
  margin-bottom: 20px;
}

.pwd-form {
  margin-top: 4px;
}

/* 登录历史 */
.history-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.history-tip {
  font-size: 13px;
  color: #909399;
}

.history-pagination {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

.empty-state {
  padding: 32px 0;
  color: #c0c4cc;
  font-size: 13px;
}

.muted {
  color: #c0c4cc;
}
</style>
