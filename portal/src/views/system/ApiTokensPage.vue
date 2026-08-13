<template>
  <div class="api-tokens-page">
    <!-- 说明提示 -->
    <el-alert
      type="info"
      :closable="false"
      show-icon
      class="intro-alert"
    >
      <template #title>
        API 令牌用于外部系统对接（如蓝鲸 CMDB webhook）。令牌以 <code>mk-</code> 开头，创建时
        <strong style="color: #f56c6c">明文仅显示一次</strong>，之后只展示脱敏值。请妥善保存。
      </template>
    </el-alert>

    <!-- 统计卡片 -->
    <el-row :gutter="16" class="stat-row">
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card stat-card--total">
          <div class="stat-card__body">
            <div class="stat-card__icon"><Key :size="24" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.total }}</div>
              <div class="stat-card__label">令牌总数</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card stat-card--valid">
          <div class="stat-card__body">
            <div class="stat-card__icon"><CircleCheck :size="24" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.valid }}</div>
              <div class="stat-card__label">有效令牌</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card stat-card--expired">
          <div class="stat-card__body">
            <div class="stat-card__icon"><Clock :size="24" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.expired }}</div>
              <div class="stat-card__label">已过期</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card stat-card--revoked">
          <div class="stat-card__body">
            <div class="stat-card__icon"><Lock :size="24" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.revoked }}</div>
              <div class="stat-card__label">已吊销</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="never" class="table-card">
      <template #header>
        <div class="page-header">
          <div class="page-header__left">
            <span class="title">🔑 API 令牌管理</span>
          </div>
          <div class="page-header__right">
            <el-button :icon="RefreshRight" size="default" @click="loadTokens"> 刷新</el-button>
            <el-button type="primary" :icon="Plus" @click="openCreateDialog"> 新建令牌</el-button>
          </div>
        </div>
      </template>

      <el-table :data="tokens" v-loading="loading" stripe style="width: 100%">
        <el-table-column label="名称" prop="name" min-width="160">
          <template #default="{ row }">
            <div class="name-cell">
              <strong>{{ row.name }}</strong>
              <el-tag size="small" :type="roleTagType(row.role)" effect="light" class="role-tag">
                {{ roleLabel(row.role) }}
              </el-tag>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="令牌" prop="token" min-width="220">
          <template #default="{ row }">
            <div class="token-cell">
              <code class="token-display">{{ row.token }}</code>
              <el-button
                v-if="!row.revoked && !isExpired(row)"
                link type="primary" size="small"
                @click="extendExpiry(row)"
              >续期</el-button>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="权限范围" min-width="260">
          <template #default="{ row }">
            <div class="scopes-cell">
              <el-tag
                v-for="s in displayScopes(row.scopes)" :key="s"
                size="small" effect="plain" style="margin: 2px 4px 2px 0"
              >{{ s }}</el-tag>
              <el-tag size="small" effect="plain" type="info" v-if="row.scopes.length > 4">
                +{{ row.scopes.length - 4 }}
              </el-tag>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="110" align="center">
          <template #default="{ row }">
            <div v-if="row.revoked" class="status-badge status--revoked">
              <el-tag type="danger" effect="dark" size="small">已吊销</el-tag>
            </div>
            <div v-else-if="isExpired(row)" class="status-badge status--expired">
              <el-tag type="warning" effect="dark" size="small">已过期</el-tag>
            </div>
            <div v-else class="status-badge status--valid">
              <el-tag type="success" effect="light" size="small">有效</el-tag>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="过期时间" width="200">
          <template #default="{ row }">
            <div v-if="!row.expiresAt" class="expiry-cell">
              <el-icon><CircleCheck /></el-icon>
              <span style="margin-left:4px">永不过期</span>
            </div>
            <div v-else>
              <div>{{ formatDate(row.expiresAt) }}</div>
              <div v-if="!row.revoked" class="countdown" :class="countdownClass(row)">
                {{ countdownText(row) }}
              </div>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="最近使用" width="190">
          <template #default="{ row }">
            <span v-if="row.lastUsedAt">{{ formatDate(row.lastUsedAt) }}</span>
            <span v-else style="color:#909399">— 未使用 —</span>
          </template>
        </el-table-column>
        <el-table-column label="创建时间" width="190">
          <template #default="{ row }">{{ formatDate(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <template v-if="!row.revoked && !isExpired(row)">
              <el-button link type="warning" size="small" @click="revokeToken(row)">
                吊销
              </el-button>
            </template>
            <template v-else-if="isExpired(row) && !row.revoked">
              <el-button link type="primary" size="small" @click="extendExpiry(row)">
                续期
              </el-button>
            </template>
            <el-button
              v-if="isAdmin"
              link type="danger" size="small" style="margin-left: 8px"
              @click="deleteToken(row)"
            >删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- ====== 新建对话框 ====== -->
    <el-dialog v-model="createVisible" title="新建 API 令牌" width="640px" :close-on-click-modal="false">
      <el-form
        :model="createForm"
        label-width="100px"
        :rules="createRules"
        ref="createFormRef"
      >
        <el-form-item label="令牌名称" prop="name">
          <el-input v-model="createForm.name" placeholder="如：蓝鲸 CMDB 对接 / AxleOps 作业同步" maxlength="128" show-word-limit />
        </el-form-item>

        <el-form-item label="角色" prop="role">
          <el-select v-model="createForm.role" style="width: 100%">
            <el-option label="Operator（推荐，运维常用）" value="operator" />
            <el-option label="Viewer（只读）" value="viewer" />
            <el-option label="Admin（仅管理员可选）" value="admin" :disabled="!isAdmin" />
          </el-select>
          <div style="color:#909399; font-size:12px; margin-top:4px">
            角色仅用于显示分类，实际权限由下方权限范围严格约束。
          </div>
        </el-form-item>

        <el-form-item label="权限范围" prop="scopes">
          <div class="scope-groups">
            <el-collapse v-model="activeGroups">
              <el-collapse-item
                v-for="g in permGroups" :key="g.group"
                :name="g.group" :title="`${scopeGroupLabel(g.group)} (${g.items.length})`"
              >
                <el-checkbox-group v-model="createForm.scopes">
                  <el-checkbox
                    v-for="p in g.items" :key="p" :label="p" :border="true"
                    style="margin: 4px 8px 4px 0"
                  >
                    <span style="font-family: monospace">{{ p }}</span>
                  </el-checkbox>
                </el-checkbox-group>
              </el-collapse-item>
            </el-collapse>
          </div>
          <div style="margin-top:8px">
            <el-button size="small" @click="selectAllScopes">全选</el-button>
            <el-button size="small" @click="clearScopes">清空</el-button>
            <el-button size="small" type="primary" plain @click="presetCmdbScopes">
              预设：CMDB 同步（asset:create/read）
            </el-button>
          </div>
        </el-form-item>

        <el-form-item label="有效期" prop="ttlType">
          <el-radio-group v-model="createForm.ttlType" style="width:100%">
            <el-radio-button value="never">永不过期</el-radio-button>
            <el-radio-button value="hours">小时</el-radio-button>
            <el-radio-button value="days">天</el-radio-button>
            <el-radio-button value="custom">自定义时间</el-radio-button>
          </el-radio-group>
          <div style="margin-top: 10px">
            <el-input-number
              v-if="createForm.ttlType === 'hours'"
              v-model="createForm.ttlValue" :min="1" :max="8760" :step="1"
              style="width:140px"
            />
            <el-input-number
              v-if="createForm.ttlType === 'days'"
              v-model="createForm.ttlValue" :min="1" :max="3650" :step="1"
              style="width:140px"
            />
            <el-date-picker
              v-if="createForm.ttlType === 'custom'"
              v-model="createForm.expiresAt"
              type="datetime"
              format="YYYY-MM-DD HH:mm:ss"
              value-format="YYYY-MM-DDTHH:mm:ssZ"
              placeholder="选择过期时间"
              style="width: 300px"
            />
          </div>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="creating" @click="submitCreate">创建令牌</el-button>
      </template>
    </el-dialog>

    <!-- ====== 明文 Token 提示对话框 ====== -->
    <el-dialog v-model="plainVisible" title="⚠️ 令牌明文仅显示一次，请立即保存！" width="560px" :close-on-click-modal="false">
      <el-alert type="error" :closable="false" show-icon class="plain-alert">
        令牌明文<strong>关闭此窗口后将无法再查看</strong>。如果丢失请重新创建。
      </el-alert>
      <div class="plain-token-box">
        <code class="plain-token">{{ plainTokenText }}</code>
        <el-button type="primary" :icon="Document" @click="copyPlain">复制</el-button>
      </div>
      <div class="plain-info-card">
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="令牌名称">{{ createdToken.name }}</el-descriptions-item>
          <el-descriptions-item label="权限数量">{{ createdToken.scopeCount }} 项</el-descriptions-item>
          <el-descriptions-item label="过期时间">
            {{ createdToken.expiresAt ? formatDate(createdToken.expiresAt) : '永不过期' }}
          </el-descriptions-item>
          <el-descriptions-item label="使用方式">
            <code>Authorization: Bearer <span style="color:#f56c6c">mk-xxxx</span></code>
          </el-descriptions-item>
          <el-descriptions-item label="蓝鲸配置参考">
            蓝鲸侧 webhook → URL = <code>http://&lt;网关&gt;/api/cmdb/sync</code><br>
            Header 填：<code>Authorization: Bearer <span style="color:#f56c6c">{上面复制的 token}</span></code>
          </el-descriptions-item>
        </el-descriptions>
      </div>
      <template #footer>
        <el-button type="primary" @click="plainVisible = false">我已保存，关闭</el-button>
      </template>
    </el-dialog>

    <!-- ====== 续期对话框 ====== -->
    <el-dialog v-model="extendVisible" title="续期令牌" width="480px">
      <el-alert type="info" :closable="false" show-icon style="margin-bottom:16px">
        续期将从「现在」重新计算有效期。
      </el-alert>
      <el-form label-width="100px">
        <el-form-item label="目标令牌"><strong>{{ extendingToken?.name }}</strong></el-form-item>
        <el-form-item label="续期方式">
          <el-radio-group v-model="extendForm.ttlType" style="width:100%">
            <el-radio-button value="never">永不过期</el-radio-button>
            <el-radio-button value="days">天</el-radio-button>
            <el-radio-button value="custom">自定义时间</el-radio-button>
          </el-radio-group>
          <div style="margin-top:10px">
            <el-input-number v-if="extendForm.ttlType === 'days'" v-model="extendForm.ttlValue" :min="1" :max="3650" style="width:140px" />
            <el-date-picker
              v-if="extendForm.ttlType === 'custom'"
              v-model="extendForm.expiresAt" type="datetime"
              format="YYYY-MM-DD HH:mm:ss" value-format="YYYY-MM-DDTHH:mm:ssZ"
              placeholder="选择过期时间" style="width:300px"
            />
          </div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="extendVisible = false">取消</el-button>
        <el-button type="primary" @click="submitExtend">确认续期</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { useUserStore } from '../../stores/user'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import {
  Key, Clock, Lock, CircleCheck, Plus, RefreshRight,
  Document,
} from '@element-plus/icons-vue'
import {
  createApiToken, deleteApiToken, fetchApiTokens, fetchMyPermissions,
  revokeApiToken, updateApiTokenExpiry,
} from '../../api/token'
import type { ApiToken, CreateApiTokenRequest, PermissionGroup } from '../../api/types'

// ============ 基础 ============
const loading = ref(false)
const tokens = ref<ApiToken[]>([])
const permGroups = ref<PermissionGroup[]>([])
const myRole = ref<string>('viewer')
const userStore = useUserStore()
const isAdmin = computed(() => userStore.role === 'admin')

const stats = computed(() => {
  const total = tokens.value.length
  let valid = 0, expired = 0, revoked = 0
  for (const t of tokens.value) {
    if (t.revoked) revoked++
    else if (isExpired(t)) expired++
    else valid++
  }
  return { total, valid, expired, revoked }
})

async function loadTokens() {
  loading.value = true
  try {
    tokens.value = await fetchApiTokens()
  } finally {
    loading.value = false
  }
}

async function loadPerms() {
  const r = await fetchMyPermissions()
  permGroups.value = r.groups
  myRole.value = r.role
}

onMounted(async () => {
  await Promise.all([loadTokens(), loadPerms()])
})

// ============ 日期工具 ============
function formatDate(s: string | null) {
  if (!s) return ''
  try {
    const d = new Date(s)
    const pad = (n: number) => String(n).padStart(2, '0')
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
  } catch {
    return s
  }
}

function isExpired(t: ApiToken) {
  if (!t.expiresAt) return false
  return new Date(t.expiresAt).getTime() < Date.now()
}

function countdownClass(t: ApiToken) {
  if (!t.expiresAt) return ''
  const diff = new Date(t.expiresAt).getTime() - Date.now()
  if (diff < 0) return 'countdown--expired'
  if (diff < 3 * 24 * 3600 * 1000) return 'countdown--soon'
  return 'countdown--ok'
}

function countdownText(t: ApiToken) {
  if (!t.expiresAt) return ''
  const diff = new Date(t.expiresAt).getTime() - Date.now()
  if (diff < 0) return '已过期'
  const day = Math.floor(diff / (24 * 3600 * 1000))
  const hr = Math.floor((diff % (24 * 3600 * 1000)) / 3600000)
  if (day > 0) return `剩余 ${day} 天 ${hr} 小时`
  const min = Math.floor((diff % 3600000) / 60000)
  if (hr > 0) return `剩余 ${hr} 小时 ${min} 分`
  return `剩余 ${min} 分钟`
}

// ============ 显示辅助 ============
function displayScopes(scopes: string[]) {
  return scopes.slice(0, 4)
}
function roleLabel(r: string) {
  return r === 'admin' ? '管理员' : r === 'operator' ? '运维' : '只读'
}
function roleTagType(r: string) {
  return r === 'admin' ? 'danger' : r === 'operator' ? 'primary' : 'info'
}
function scopeGroupLabel(g: string) {
  const map: Record<string, string> = {
    user: '用户管理', role: '角色权限', dept: '部门管理',
    audit: '审计中心', system: '系统设置', asset: '资产管理',
    ticket: '工单管理', job: '作业平台', report: '报表中心',
    alert: '告警中心', dashboard: '运营态势',
  }
  return map[g] ?? g
}

// ============ 新建对话框 ============
const createVisible = ref(false)
const creating = ref(false)
const createFormRef = ref<FormInstance>()
const activeGroups = ref<string[]>([])
const createForm = reactive<CreateApiTokenRequest>({
  name: '',
  scopes: [],
  ttlType: 'days',
  ttlValue: 30,
  expiresAt: '',
  role: 'operator',
})
const createRules: FormRules = {
  name: [{ required: true, message: '请输入令牌名称', trigger: 'blur' }],
  scopes: [
    { type: 'array', required: true, min: 1, message: '请至少选择一项权限', trigger: 'change' },
  ],
  ttlType: [{ required: true, message: '请选择有效期', trigger: 'change' }],
}

function openCreateDialog() {
  Object.assign(createForm, {
    name: '',
    scopes: [],
    ttlType: 'days',
    ttlValue: 30,
    expiresAt: '',
    role: isAdmin.value ? 'operator' : 'operator',
  })
  // 默认展开第 1 组
  activeGroups.value = permGroups.value.slice(0, 3).map((g) => g.group)
  createVisible.value = true
}
function selectAllScopes() {
  createForm.scopes = permGroups.value.flatMap((g) => g.items)
}
function clearScopes() { createForm.scopes = [] }
function presetCmdbScopes() {
  createForm.scopes = ['asset:create', 'asset:read']
  // 展开 asset 组
  if (!activeGroups.value.includes('asset')) activeGroups.value.push('asset')
}

async function submitCreate() {
  if (!createFormRef.value) return
  await createFormRef.value.validate(async (ok) => {
    if (!ok) return
    creating.value = true
    try {
      const r = await createApiToken({
        name: createForm.name,
        scopes: createForm.scopes,
        ttlType: createForm.ttlType,
        ttlValue: createForm.ttlType === 'custom' || createForm.ttlType === 'never'
          ? undefined : createForm.ttlValue,
        expiresAt: createForm.ttlType === 'custom' ? createForm.expiresAt : undefined,
        role: createForm.role,
      })
      ElMessage.success('创建成功！')
      createVisible.value = false
      showPlainToken(r.token, {
        name: createForm.name,
        scopeCount: createForm.scopes.length,
        expiresAt: r.expiresAt,
      })
      await loadTokens()
    } finally {
      creating.value = false
    }
  })
}

// ============ 明文显示对话框 ============
const plainVisible = ref(false)
const plainTokenText = ref('')
const createdToken = reactive<{ name: string; scopeCount: number; expiresAt: string | null }>({
  name: '', scopeCount: 0, expiresAt: null,
})
function showPlainToken(token: string, meta: { name: string; scopeCount: number; expiresAt: string | null }) {
  plainTokenText.value = token
  Object.assign(createdToken, meta)
  plainVisible.value = true
}
async function copyPlain() {
  try {
    await navigator.clipboard.writeText(plainTokenText.value)
    ElMessage.success('已复制到剪贴板')
  } catch {
    // Fallback
    const ta = document.createElement('textarea')
    ta.value = plainTokenText.value
    document.body.appendChild(ta)
    ta.select()
    document.execCommand('copy')
    document.body.removeChild(ta)
    ElMessage.success('已复制到剪贴板')
  }
}

// ============ 吊销 / 删除 ============
async function revokeToken(t: ApiToken) {
  await ElMessageBox.confirm(`确定吊销令牌「${t.name}」？吊销后立即失效，不可恢复。`, '确认吊销', {
    type: 'warning', confirmButtonText: '吊销', cancelButtonText: '取消',
  })
  try {
    await revokeApiToken(t.id)
    ElMessage.success('已吊销')
    await loadTokens()
  } catch {}
}
async function deleteToken(t: ApiToken) {
  await ElMessageBox.confirm(`确定彻底删除令牌「${t.name}」？此操作不可恢复。`, '确认删除', {
    type: 'error', confirmButtonText: '删除', cancelButtonText: '取消',
  })
  try {
    await deleteApiToken(t.id)
    ElMessage.success('已删除')
    await loadTokens()
  } catch {}
}

// ============ 续期 ============
const extendVisible = ref(false)
const extendingToken = ref<ApiToken | null>(null)
const extendForm = reactive<{ ttlType: string; ttlValue: number; expiresAt: string }>({
  ttlType: 'days', ttlValue: 30, expiresAt: '',
})
function extendExpiry(t: ApiToken) {
  extendingToken.value = t
  Object.assign(extendForm, { ttlType: 'days', ttlValue: 30, expiresAt: '' })
  extendVisible.value = true
}
async function submitExtend() {
  if (!extendingToken.value) return
  const payload: any = { ttlType: extendForm.ttlType as any }
  if (extendForm.ttlType === 'days') payload.ttlValue = extendForm.ttlValue
  else if (extendForm.ttlType === 'custom') payload.expiresAt = extendForm.expiresAt
  try {
    await updateApiTokenExpiry(extendingToken.value.id, payload)
    ElMessage.success('续期成功')
    extendVisible.value = false
    await loadTokens()
  } catch {}
}
</script>

<style scoped>
.api-tokens-page {
  padding: 16px 24px;
}
.api-tokens-page .intro-alert { margin-bottom: 16px; }
.api-tokens-page code {
  background: #f4f4f5;
  padding: 1px 6px;
  border-radius: 4px;
  color: #606266;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
}
.stat-row { margin-bottom: 16px; }
.stat-card {
  border-radius: 8px;
}
.stat-card .stat-card__body { display: flex; align-items: center; }
.stat-card .stat-card__icon {
  width: 42px; height: 42px; border-radius: 10px;
  display: flex; align-items: center; justify-content: center;
  margin-right: 14px; color: #fff;
}
.stat-card .stat-card__value { font-size: 24px; font-weight: 700; line-height: 1.1; }
.stat-card .stat-card__label { font-size: 12px; color: #909399; margin-top: 4px; }
.stat-card--total .stat-card__icon { background: linear-gradient(135deg, #409eff, #66b1ff); }
.stat-card--valid .stat-card__icon { background: linear-gradient(135deg, #67c23a, #85ce61); }
.stat-card--expired .stat-card__icon { background: linear-gradient(135deg, #e6a23c, #ebb563); }
.stat-card--revoked .stat-card__icon { background: linear-gradient(135deg, #f56c6c, #f78989); }
.table-card { border-radius: 8px; }
.page-header { display: flex; justify-content: space-between; align-items: center; }
.page-header .title { font-size: 16px; font-weight: 600; }
.name-cell {
  display: flex; align-items: center; gap: 8px;
}
.name-cell .role-tag { margin-left: 6px; }
.token-cell {
  display: flex; align-items: center; gap: 8px;
}
.token-cell .token-display {
  background: #f0f2f5; padding: 3px 10px; border-radius: 4px;
  letter-spacing: 0.3px;
}
.scopes-cell { display: flex; flex-wrap: wrap; align-items: center; }
.expiry-cell { display: flex; align-items: center; color: #67c23a; }
.countdown {
  font-size: 12px;
}
.countdown--ok { color: #909399; }
.countdown--soon { color: #e6a23c; font-weight: 600; }
.countdown--expired { color: #f56c6c; }

.plain-alert { margin-bottom: 18px; }
.plain-token-box {
  display: flex; align-items: center; gap: 12px; padding: 18px;
  background: linear-gradient(135deg, #fff6ec, #fef0f0);
  border: 1px dashed #f56c6c;
  border-radius: 8px; margin-bottom: 16px;
}
.plain-token-box .plain-token {
  flex: 1; background: #fff; padding: 10px 14px;
  border-radius: 6px; font-size: 15px;
  letter-spacing: 0.5px; word-break: break-all;
  color: #303133; font-weight: 600;
}
.plain-info-card {
  background: #fafbfc; border-radius: 6px;
}
.plain-info-card :deep(.el-descriptions) { padding: 10px 14px; }
.scope-groups {
  border: 1px solid #ebeef5; border-radius: 6px;
}
.scope-groups :deep(.el-collapse-item__wrap) { border-bottom: none; }
</style>
