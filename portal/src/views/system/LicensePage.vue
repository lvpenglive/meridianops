<template>
  <div class="license-page" v-loading="loading">
    <!-- ============ Hero 状态卡片 ============ -->
    <div class="hero-card" :class="heroTheme">
      <div class="hero-left">
        <div class="hero-edition-badge">
          <el-icon class="hero-icon"><Medal /></el-icon>
          <span>{{ editionLabel }}</span>
        </div>
        <div class="hero-customer">{{ info?.customer || '未授权客户' }}</div>
        <div class="hero-status-row">
          <el-tag :type="statusTagType" effect="dark" size="small" round>{{ statusLabel }}</el-tag>
          <span class="hero-expire-text">
            {{ info?.expiresAt ? `授权至 ${formatTime(info.expiresAt)}` : '永不到期' }}
          </span>
        </div>
      </div>
      <div class="hero-right">
        <div class="hero-days-label">剩余天数</div>
        <div class="hero-days-value">{{ daysDisplay }}</div>
        <div class="hero-days-hint">{{ daysHint }}</div>
      </div>
      <!-- 装饰水印 -->
      <el-icon class="hero-watermark"><Key /></el-icon>
    </div>

    <!-- ============ 进度条 ============ -->
    <el-card v-if="!isPerpetual && info" shadow="never" class="progress-card">
      <div class="progress-bar-wrap">
        <div class="progress-bar-labels">
          <span class="progress-start">{{ formatTime(info.activatedAt) || '激活' }}</span>
          <span class="progress-end" :class="{ 'text-danger': info.isExpired }">
            {{ formatTime(info.expiresAt) || '永久' }}
          </span>
        </div>
        <el-progress
          :percentage="progressPercent"
          :color="progressColor"
          :stroke-width="14"
          :show-text="false"
          :striped="!info.isExpired"
          striped-flow
        />
        <div class="progress-bar-labels" style="margin-top: 4px">
          <span class="text-muted">已使用 {{ usedDays }} 天</span>
          <span :class="daysClass">{{ info.isExpired ? '已过期' : `剩余 ${info.daysRemaining} 天` }}</span>
        </div>
      </div>
    </el-card>

    <!-- ============ 详细信息 + 机器指纹 ============ -->
    <el-row :gutter="16" style="margin-top: 16px">
      <el-col :xs="24" :md="14">
        <el-card shadow="never">
          <template #header>
            <div class="card-header">
              <span><el-icon><Document /></el-icon> 授权详情</span>
              <div>
                <el-button v-if="canUpdate" type="primary" size="small" :icon="Edit" @click="openDialog">
                  更新授权
                </el-button>
                <el-button size="small" :icon="Refresh" @click="loadData">刷新</el-button>
              </div>
            </div>
          </template>
          <el-descriptions :column="1" border size="default" class="info-desc">
            <el-descriptions-item label="客户名称">
              <span class="info-value">{{ info?.customer || '—' }}</span>
            </el-descriptions-item>
            <el-descriptions-item label="授权版本">
              <el-tag :type="editionTagType" size="small" effect="light">{{ editionLabel }}</el-tag>
            </el-descriptions-item>
            <el-descriptions-item label="激活时间">
              <span class="info-value">{{ formatTime(info?.activatedAt) || '未激活' }}</span>
            </el-descriptions-item>
            <el-descriptions-item label="到期时间">
              <span class="info-value" :class="{ 'text-danger': info?.isExpired, 'text-warning': isUrgent }">
                {{ info?.expiresAt ? formatTime(info?.expiresAt) : '永不到期' }}
              </span>
            </el-descriptions-item>
            <el-descriptions-item label="激活码">
              <div class="license-key-wrap">
                <code class="license-key-text">{{ info?.licenseKey || '—' }}</code>
                <el-button
                  v-if="info?.licenseKey"
                  link
                  size="small"
                  :icon="CopyDocument"
                  @click="copyText(info.licenseKey)"
                />
              </div>
            </el-descriptions-item>
            <el-descriptions-item label="机器指纹">
              <div class="fingerprint-wrap">
                <code class="fingerprint-text">{{ info?.fingerprint || '—' }}</code>
                <el-button
                  v-if="info?.fingerprint"
                  link
                  size="small"
                  :icon="CopyDocument"
                  @click="copyText(info.fingerprint)"
                />
                <el-tooltip content="此指纹绑定当前部署环境（MySQL server_uuid + 主机名），签发激活码时需提供此值" placement="top">
                  <el-icon class="fingerprint-help"><QuestionFilled /></el-icon>
                </el-tooltip>
              </div>
            </el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>

      <el-col :xs="24" :md="10">
        <el-card shadow="never" class="version-compare-card">
          <template #header><span><el-icon><ScaleToOriginal /></el-icon> 版本对比</span></template>
          <div class="version-compare">
            <div
              v-for="v in versionList"
              :key="v.name"
              class="version-item"
              :class="{ active: info?.edition === v.name }"
            >
              <div class="version-item-header">
                <el-tag :type="v.tagType" size="small" effect="dark">{{ v.label }}</el-tag>
                <el-icon v-if="info?.edition === v.name" class="version-active-icon"><CircleCheckFilled /></el-icon>
              </div>
              <div class="version-features">
                <div v-for="f in v.features" :key="f" class="version-feature">
                  <el-icon class="feature-icon"><Check /></el-icon>
                  <span>{{ f }}</span>
                </div>
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- ============ 告警提示 ============ -->
    <el-alert
      v-if="info?.isExpired"
      type="error"
      :closable="false"
      show-icon
      title="产品授权已过期"
      description="业务功能已暂停（业务接口返回 402）。请在「更新授权」中修改到期时间、输入新激活码或联系商务续期。"
      style="margin-top: 16px"
    />
    <el-alert
      v-else-if="isUrgent"
      type="warning"
      :closable="false"
      show-icon
      title="授权即将到期"
      :description="`授权将于 ${info?.daysRemaining} 天后到期，请尽快续期避免业务中断。`"
      style="margin-top: 16px"
    />

    <!-- ============ 编辑对话框 ============ -->
    <el-dialog v-model="dialogVisible" title="更新授权" width="560px" :close-on-click-modal="false">
      <el-alert
        type="info"
        :closable="false"
        show-icon
        :title="dialogMode === 'key' ? '激活码模式' : '手动设置模式'"
        :description="dialogModeHint"
        style="margin-bottom: 16px"
      />

      <el-form ref="formRef" :model="form" :rules="formRules" label-width="100px">
        <el-form-item label="授权版本" prop="edition">
          <el-select v-model="form.edition" placeholder="选择版本" style="width: 100%">
            <el-option label="Community 社区版" value="Community" />
            <el-option label="Enterprise 企业版" value="Enterprise" />
            <el-option label="Ultimate 旗舰版" value="Ultimate" />
          </el-select>
        </el-form-item>
        <el-form-item label="客户名称" prop="customer">
          <el-input v-model="form.customer" placeholder="如：XX银行信息技术部" maxlength="128" show-word-limit />
        </el-form-item>
        <el-form-item label="到期时间">
          <el-switch v-model="form.perpetual" active-text="永不到期" inactive-text="指定日期" />
        </el-form-item>
        <el-form-item v-if="!form.perpetual" label="到期日期" prop="expiresAt">
          <el-date-picker
            v-model="form.expiresAt"
            type="datetime"
            placeholder="选择到期日期时间"
            format="YYYY-MM-DD HH:mm:ss"
            value-format="YYYY-MM-DD HH:mm:ss"
            style="width: 100%"
          />
        </el-form-item>

        <el-divider content-position="left">
          <span class="divider-text">激活码（可选）</span>
        </el-divider>

        <el-form-item label="激活码">
          <el-input
            v-model="form.licenseKey"
            type="textarea"
            :rows="3"
            placeholder="粘贴商务提供的激活码（以 MK- 开头）。输入后将自动验签并覆盖上方版本/客户/到期时间。"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSubmit">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import {
  Edit, Refresh, CopyDocument, QuestionFilled, CircleCheckFilled,
  Check, Document, Medal, Key, ScaleToOriginal,
} from '@element-plus/icons-vue'
import { getLicenseAdmin, updateLicenseAdmin } from '../../api/license'
import type { LicenseAdminInfo } from '../../api/license'
import { useUserStore } from '../../stores/user'
import { PERPETUAL_DAYS } from '../../stores/user'

const userStore = useUserStore()
const loading = ref(false)
const submitting = ref(false)
const info = ref<LicenseAdminInfo | null>(null)

const dialogVisible = ref(false)
const formRef = ref<FormInstance>()
const form = reactive({
  edition: 'Community' as 'Community' | 'Enterprise' | 'Ultimate',
  customer: '',
  perpetual: true,
  expiresAt: '',
  licenseKey: '',
})

const formRules: FormRules = {
  edition: [{ required: true, message: '请选择授权版本', trigger: 'change' }],
  customer: [{ required: true, message: '请输入客户名称', trigger: 'blur' }],
  expiresAt: [
    {
      validator: (_rule, _value, callback) => {
        if (!form.perpetual && !form.expiresAt) {
          callback(new Error('请选择到期日期，或开启「永不到期」'))
        } else {
          callback()
        }
      },
      trigger: 'change',
    },
  ],
}

const canUpdate = computed(() => userStore.hasPermission('system:update'))

// ---- 版本对比数据 ----
const versionList = [
  {
    name: 'Community',
    label: 'Community',
    tagType: 'info' as const,
    features: ['基础运维面板', '资产 CMDB', '监控告警', '永久免费'],
  },
  {
    name: 'Enterprise',
    label: 'Enterprise',
    tagType: 'primary' as const,
    features: ['社区版全部功能', '审计日志', '报表分析', '知识库', '字典管理'],
  },
  {
    name: 'Ultimate',
    label: 'Ultimate',
    tagType: 'warning' as const,
    features: ['企业版全部功能', 'AIOps 智能分析', '高级拓扑可视化', '多租户支持'],
  },
]

// ---- 计算属性 ----
const editionLabel = computed(() => {
  const v = info.value?.edition
  if (v === 'Community') return 'Community 社区版'
  if (v === 'Enterprise') return 'Enterprise 企业版'
  if (v === 'Ultimate') return 'Ultimate 旗舰版'
  return v || '—'
})
const editionTagType = computed(() => {
  const v = info.value?.edition
  if (v === 'Ultimate') return 'warning'
  if (v === 'Enterprise') return 'primary'
  return 'info'
})

const isPerpetual = computed(() => {
  const d = info.value?.daysRemaining
  return d === undefined || d === null || d >= PERPETUAL_DAYS
})

const statusLabel = computed(() => {
  if (!info.value) return '—'
  if (info.value.isExpired) return '已过期'
  if (isPerpetual.value) return '永久授权'
  const w = info.value.warnLevel
  if (w === 'urgent') return '即将到期'
  if (w === 'soon') return '即将到期'
  return '正常'
})
const statusTagType = computed<'success' | 'warning' | 'danger' | 'info'>(() => {
  if (!info.value) return 'info'
  if (info.value.isExpired) return 'danger'
  if (isPerpetual.value) return 'success'
  const w = info.value.warnLevel
  if (w === 'urgent') return 'danger'
  if (w === 'soon') return 'warning'
  return 'success'
})

const isUrgent = computed(() => info.value?.warnLevel === 'urgent' || info.value?.warnLevel === 'soon')

const daysDisplay = computed(() => {
  if (!info.value) return '—'
  if (isPerpetual.value) return '∞'
  const d = info.value.daysRemaining
  if (d < 0) return `${Math.abs(d)}`
  return `${d}`
})

const daysClass = computed(() => {
  if (!info.value || isPerpetual.value) return 'text-success'
  if (info.value.isExpired) return 'text-danger'
  if (info.value.daysRemaining <= 7) return 'text-danger'
  if (info.value.daysRemaining <= 30) return 'text-warning'
  return 'text-success'
})

const daysHint = computed(() => {
  if (!info.value) return ''
  if (isPerpetual.value) return '永不到期'
  if (info.value.isExpired) return '业务已暂停'
  if (info.value.daysRemaining <= 7) return '请立即续期'
  if (info.value.daysRemaining <= 30) return '建议提前续期'
  return '授权有效'
})

// Hero 卡片主题色
const heroTheme = computed(() => {
  if (!info.value) return 'theme-default'
  if (info.value.isExpired) return 'theme-danger'
  if (isPerpetual.value) return 'theme-success'
  if (info.value.daysRemaining <= 7) return 'theme-danger'
  if (info.value.daysRemaining <= 30) return 'theme-warning'
  return 'theme-success'
})

// 进度条
const usedDays = computed(() => {
  if (!info.value || !info.value.activatedAt) return 0
  const activated = new Date(info.value.activatedAt).getTime()
  const now = Date.now()
  return Math.max(0, Math.floor((now - activated) / (24 * 60 * 60 * 1000)))
})
const progressPercent = computed(() => {
  if (!info.value || isPerpetual.value) return 100
  const total = usedDays.value + Math.max(0, info.value.daysRemaining)
  if (total <= 0) return 0
  return Math.min(100, Math.round((usedDays.value / total) * 100))
})
const progressColor = computed(() => {
  if (!info.value || isPerpetual.value) return '#67c23a'
  if (info.value.isExpired || info.value.daysRemaining <= 7) return '#f56c6c'
  if (info.value.daysRemaining <= 30) return '#e6a23c'
  return '#67c23a'
})

// 对话框模式提示
const dialogMode = computed(() => (form.licenseKey.trim() ? 'key' : 'manual'))
const dialogModeHint = computed(() =>
  dialogMode.value === 'key'
    ? '检测到已输入激活码，保存时后端将验签。验签通过后，激活码中的版本/客户/到期时间将自动覆盖上方手动填写的值。'
    : '未输入激活码，将使用上方手动填写的版本/客户/到期时间直接保存。如需正式授权，请联系商务获取激活码。',
)

// ---- 方法 ----
function formatTime(s?: string): string {
  if (!s) return ''
  const d = new Date(s)
  if (isNaN(d.getTime())) return s
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

function copyText(text: string) {
  navigator.clipboard.writeText(text).then(
    () => ElMessage.success('已复制到剪贴板'),
    () => ElMessage.warning('复制失败，请手动选择复制'),
  )
}

async function loadData() {
  loading.value = true
  try {
    const data = await getLicenseAdmin()
    info.value = data
    userStore.setLicense({
      edition: data.edition,
      customer: data.customer,
      expiresAt: data.expiresAt,
      activatedAt: data.activatedAt,
      daysRemaining: data.daysRemaining,
      isExpired: data.isExpired,
      warnLevel: data.warnLevel,
    })
  } catch (e: any) {
    ElMessage.error(e?.message || '加载授权信息失败')
  } finally {
    loading.value = false
  }
}

function openDialog() {
  if (!info.value) return
  form.edition = (info.value.edition as any) || 'Community'
  form.customer = info.value.customer || ''
  form.perpetual = isPerpetual.value
  form.expiresAt = info.value.expiresAt ? formatTime(info.value.expiresAt) : ''
  form.licenseKey = ''
  dialogVisible.value = true
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    submitting.value = true
    try {
      const payload: any = {
        edition: form.edition,
        customer: form.customer,
        expiresAt: form.perpetual ? '' : form.expiresAt,
      }
      if (form.licenseKey.trim()) {
        payload.licenseKey = form.licenseKey.trim()
      }
      const data = await updateLicenseAdmin(payload)
      info.value = data
      userStore.setLicense({
        edition: data.edition,
        customer: data.customer,
        expiresAt: data.expiresAt,
        activatedAt: data.activatedAt,
        daysRemaining: data.daysRemaining,
        isExpired: data.isExpired,
        warnLevel: data.warnLevel,
      })
      ElMessage.success(data.message || '授权已更新')
      dialogVisible.value = false
    } catch (e: any) {
      ElMessage.error(e?.message || '保存失败')
    } finally {
      submitting.value = false
    }
  })
}

onMounted(() => {
  loadData()
})
</script>

<style scoped>
.license-page {
  padding: 16px;
}

/* ===== Hero 卡片 ===== */
.hero-card {
  position: relative;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 28px 32px;
  border-radius: 12px;
  overflow: hidden;
  color: #fff;
  transition: all 0.3s;
}
.hero-card::before {
  content: '';
  position: absolute;
  inset: 0;
  opacity: 0.12;
  background-image: radial-gradient(circle at 20% 50%, #fff 1px, transparent 1px);
  background-size: 24px 24px;
}
.theme-success { background: linear-gradient(135deg, #67c23a, #4e9e2f); }
.theme-warning { background: linear-gradient(135deg, #e6a23c, #c68a2e); }
.theme-danger  { background: linear-gradient(135deg, #f56c6c, #d94a4a); }
.theme-default { background: linear-gradient(135deg, #409eff, #2b7cd6); }

.hero-left { z-index: 1; }
.hero-edition-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 18px;
  font-weight: 600;
  background: rgba(255, 255, 255, 0.2);
  padding: 4px 14px;
  border-radius: 20px;
  margin-bottom: 12px;
}
.hero-icon { font-size: 20px; }
.hero-customer {
  font-size: 24px;
  font-weight: 700;
  margin-bottom: 10px;
  text-shadow: 0 1px 2px rgba(0, 0, 0, 0.15);
}
.hero-status-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.hero-expire-text { font-size: 14px; opacity: 0.92; }

.hero-right {
  text-align: center;
  z-index: 1;
  min-width: 140px;
}
.hero-days-label { font-size: 13px; opacity: 0.85; margin-bottom: 2px; }
.hero-days-value { font-size: 56px; font-weight: 800; line-height: 1.1; }
.hero-days-hint { font-size: 12px; opacity: 0.85; margin-top: 4px; }

.hero-watermark {
  position: absolute;
  right: -20px;
  bottom: -20px;
  font-size: 140px;
  opacity: 0.08;
  z-index: 0;
}

/* ===== 进度条卡片 ===== */
.progress-card { margin-top: 16px; }
.progress-bar-wrap { padding: 4px 0; }
.progress-bar-labels {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
  color: #909399;
  margin-bottom: 6px;
}

/* ===== 卡片通用 ===== */
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.card-header .el-icon { vertical-align: -2px; margin-right: 4px; }

/* ===== 信息表格 ===== */
.info-desc :deep(.el-descriptions__label) { width: 100px; }
.info-value { font-weight: 500; }
.license-key-wrap, .fingerprint-wrap {
  display: flex;
  align-items: center;
  gap: 6px;
}
.license-key-text {
  font-family: 'Courier New', monospace;
  font-size: 13px;
  color: #606266;
  word-break: break-all;
}
.fingerprint-text {
  font-family: 'Courier New', monospace;
  font-size: 13px;
  color: #409eff;
  font-weight: 600;
  letter-spacing: 1px;
}
.fingerprint-help { color: #c0c4cc; cursor: help; }

/* ===== 版本对比 ===== */
.version-compare-card { height: 100%; }
.version-compare { display: flex; flex-direction: column; gap: 12px; }
.version-item {
  border: 1px solid #ebeef5;
  border-radius: 8px;
  padding: 12px 14px;
  transition: all 0.2s;
}
.version-item.active {
  border-color: #409eff;
  background: linear-gradient(135deg, #ecf5ff, #f0f7ff);
  box-shadow: 0 2px 8px rgba(64, 158, 255, 0.1);
}
.version-item-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.version-active-icon { color: #409eff; font-size: 16px; }
.version-features { display: flex; flex-direction: column; gap: 4px; }
.version-feature {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: #606266;
}
.feature-icon { color: #67c23a; font-size: 14px; flex-shrink: 0; }

/* ===== 对话框 ===== */
.divider-text { font-size: 13px; color: #909399; }

/* ===== 通用文字色 ===== */
.text-success { color: #67c23a; }
.text-warning { color: #e6a23c; }
.text-danger { color: #f56c6c; }
.text-muted { color: #c0c4cc; }

/* ===== 响应式 ===== */
@media (max-width: 768px) {
  .hero-card { flex-direction: column; text-align: center; gap: 16px; padding: 20px; }
  .hero-right { min-width: auto; }
  .hero-days-value { font-size: 42px; }
}
</style>
