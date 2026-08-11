<template>
  <div class="system-page" v-loading="loading">
    <el-row :gutter="16">
      <!-- 左侧：安全策略配置 -->
      <el-col :span="16">
        <el-card shadow="never" class="settings-card">
          <template #header>
            <div class="card-header">
              <span class="header-title">🔐 安全策略</span>
              <el-tag v-if="!canUpdate" type="info" size="small">只读模式</el-tag>
            </div>
          </template>

          <!-- 密码策略 -->
          <div class="settings-group">
            <div class="group-title">密码策略</div>
            <el-form label-width="140px" class="settings-form">
              <el-form-item label="最小长度">
                <el-input-number
                  v-model="form.password_min_length"
                  :min="4"
                  :max="64"
                  :disabled="!canUpdate"
                />
                <span class="form-hint">用户密码至少需要的字符数</span>
              </el-form-item>
              <el-form-item label="需要大写字母">
                <el-switch v-model="form.password_require_uppercase" :disabled="!canUpdate" />
              </el-form-item>
              <el-form-item label="需要小写字母">
                <el-switch v-model="form.password_require_lowercase" :disabled="!canUpdate" />
              </el-form-item>
              <el-form-item label="需要数字">
                <el-switch v-model="form.password_require_digit" :disabled="!canUpdate" />
              </el-form-item>
              <el-form-item label="需要特殊字符">
                <el-switch v-model="form.password_require_special" :disabled="!canUpdate" />
              </el-form-item>
            </el-form>
          </div>

          <el-divider />

          <!-- 登录锁定策略 -->
          <div class="settings-group">
            <div class="group-title">登录锁定策略</div>
            <el-form label-width="140px" class="settings-form">
              <el-form-item label="最大失败次数">
                <el-input-number
                  v-model="form.login_max_attempts"
                  :min="3"
                  :max="20"
                  :disabled="!canUpdate"
                />
                <span class="form-hint">连续登录失败达到此次数后锁定账号</span>
              </el-form-item>
              <el-form-item label="锁定时长(分钟)">
                <el-input-number
                  v-model="form.login_lockout_minutes"
                  :min="1"
                  :max="1440"
                  :disabled="!canUpdate"
                />
                <span class="form-hint">账号锁定后需等待的分钟数</span>
              </el-form-item>
            </el-form>
          </div>

          <div class="settings-actions" v-if="canUpdate">
            <el-button type="primary" :loading="saving" @click="handleSave">保存配置</el-button>
            <el-button @click="loadSettings">重置</el-button>
          </div>
        </el-card>
      </el-col>

      <!-- 右侧：系统信息 -->
      <el-col :span="8">
        <el-card shadow="never" class="info-card">
          <template #header><span class="header-title">📊 系统信息</span></template>
          <el-descriptions :column="1" size="small" border>
            <el-descriptions-item label="版本">MeridianOps v0.1.0</el-descriptions-item>
            <el-descriptions-item label="前端框架">Vue 3 + Vite</el-descriptions-item>
            <el-descriptions-item label="网关框架">Rust + Axum</el-descriptions-item>
            <el-descriptions-item label="数据库">MySQL</el-descriptions-item>
            <el-descriptions-item label="鉴权">JWT + Argon2</el-descriptions-item>
            <el-descriptions-item label="权限模型">RBAC</el-descriptions-item>
          </el-descriptions>
        </el-card>

        <el-card shadow="never" class="info-card" style="margin-top: 16px">
          <template #header><span class="header-title">📝 配置变更记录</span></template>
          <div class="update-info" v-if="lastUpdated">
            <div class="update-row">
              <span class="update-label">最后修改人</span>
              <span class="update-value">{{ lastUpdated.updatedBy || 'system' }}</span>
            </div>
            <div class="update-row">
              <span class="update-label">最后修改时间</span>
              <span class="update-value">{{ formatTime(lastUpdated.updatedAt) }}</span>
            </div>
          </div>
          <div v-else class="empty-state">暂无记录</div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { useUserStore } from '../../stores/user'
import { listSettings, updateSettings } from '../../api/system'
import type { SystemSetting } from '../../api/types'

const userStore = useUserStore()
const canUpdate = computed(() => userStore.hasPermission('system:update'))

const loading = ref(false)
const saving = ref(false)
const lastUpdated = ref<SystemSetting | null>(null)

// 表单：布尔项用 v-model 绑定 boolean，提交时转字符串
const form = reactive({
  password_min_length: 8,
  password_require_uppercase: true,
  password_require_lowercase: true,
  password_require_digit: true,
  password_require_special: false,
  login_max_attempts: 5,
  login_lockout_minutes: 15,
})

// 配置项元数据：key → 类型，用于把后端字符串还原为表单值
const SETTING_KEYS: { key: keyof typeof form; type: 'number' | 'bool' }[] = [
  { key: 'password_min_length', type: 'number' },
  { key: 'password_require_uppercase', type: 'bool' },
  { key: 'password_require_lowercase', type: 'bool' },
  { key: 'password_require_digit', type: 'bool' },
  { key: 'password_require_special', type: 'bool' },
  { key: 'login_max_attempts', type: 'number' },
  { key: 'login_lockout_minutes', type: 'number' },
]

async function loadSettings() {
  loading.value = true
  try {
    const settings = await listSettings()
    applySettings(settings)
    // 取最新 updated_at 的项作为「最后修改」记录
    const sorted = [...settings].sort((a, b) => (a.updatedAt < b.updatedAt ? 1 : -1))
    lastUpdated.value = sorted[0] || null
  } catch {
    // 错误提示由 request 拦截器统一处理
  } finally {
    loading.value = false
  }
}

function applySettings(settings: SystemSetting[]) {
  const map = new Map(settings.map((s) => [s.settingKey, s.settingValue]))
  for (const meta of SETTING_KEYS) {
    const raw = map.get(meta.key)
    if (raw === undefined) continue
    if (meta.type === 'number') {
      const n = parseInt(raw, 10)
      if (!isNaN(n)) (form[meta.key] as number) = n
    } else {
      (form[meta.key] as boolean) = raw === 'true'
    }
  }
}

async function handleSave() {
  saving.value = true
  try {
    // 把表单值序列化为字符串 map
    const payload: Record<string, string> = {}
    for (const meta of SETTING_KEYS) {
      const v = form[meta.key]
      payload[meta.key] = meta.type === 'number' ? String(v) : v ? 'true' : 'false'
    }
    await updateSettings(payload)
    ElMessage.success('配置已保存')
    await loadSettings()
  } catch {
    // 错误提示由 request 拦截器统一处理
  } finally {
    saving.value = false
  }
}

function formatTime(s?: string): string {
  if (!s) return '—'
  try {
    const d = new Date(s)
    if (isNaN(d.getTime())) return s
    return d.toLocaleString('zh-CN', { hour12: false })
  } catch {
    return s
  }
}

onMounted(() => {
  loadSettings()
})
</script>

<style scoped>
.system-page {
  min-height: 400px;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.header-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
}

.settings-group {
  margin-bottom: 8px;
}

.group-title {
  font-size: 14px;
  font-weight: 600;
  color: #409eff;
  margin-bottom: 16px;
  padding-left: 8px;
  border-left: 3px solid #409eff;
}

.settings-form .el-form-item {
  margin-bottom: 18px;
}

.form-hint {
  margin-left: 12px;
  font-size: 12px;
  color: #909399;
}

.settings-actions {
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid #ebeef5;
}

.update-info {
  font-size: 13px;
}

.update-row {
  display: flex;
  justify-content: space-between;
  padding: 6px 0;
}

.update-label {
  color: #909399;
}

.update-value {
  color: #303133;
  font-weight: 500;
}

.empty-state {
  padding: 16px 0;
  text-align: center;
  color: #c0c4cc;
  font-size: 13px;
}
</style>
