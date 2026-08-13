<template>
  <div class="report-page">
    <div class="report-header">
      <div class="header-left">
        <el-button text @click="router.push('/report')">
          <el-icon><ArrowLeft /></el-icon> 报表中心
        </el-button>
        <h2>合规健康度</h2>
        <el-tag type="success" effect="dark" size="small">合规审计</el-tag>
      </div>
      <div class="header-right">
        <el-button :icon="Refresh" circle @click="loadAll" />
      </div>
    </div>

    <el-alert
      :type="overallHealth === 'healthy' ? 'success' : overallHealth === 'warning' ? 'warning' : 'danger'"
      :closable="false"
      show-icon
      :title="overallTitle"
      :description="overallDesc"
      class="health-alert"
    />

    <!-- 4 个合规指标卡 -->
    <el-row :gutter="16" class="metric-row">
      <el-col :xs="12" :sm="6">
        <div class="metric-card" :class="summary.expiredPasswordCount > 0 ? 'danger' : 'success'">
          <div class="metric-icon"><el-icon><Key /></el-icon></div>
          <div class="metric-body">
            <div class="metric-label">密码已过期用户</div>
            <div class="metric-value">{{ summary.expiredPasswordCount }}</div>
            <div class="metric-hint">策略：{{ summary.passwordExpiryDays }} 天过期</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="metric-card" :class="summary.inactive90dCount > 0 ? 'warning' : 'success'">
          <div class="metric-icon"><el-icon><Timer /></el-icon></div>
          <div class="metric-body">
            <div class="metric-label">90 天未登录用户</div>
            <div class="metric-value">{{ summary.inactive90dCount }}</div>
            <div class="metric-hint">银行合规建议禁用</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="metric-card info">
          <div class="metric-icon"><el-icon><User /></el-icon></div>
          <div class="metric-body">
            <div class="metric-label">用户总数</div>
            <div class="metric-value">{{ summary.totalUsers }}</div>
            <div class="metric-hint">在册账号</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="metric-card warning">
          <div class="metric-icon"><el-icon><Warning /></el-icon></div>
          <div class="metric-body">
            <div class="metric-label">弱密码用户</div>
            <div class="metric-value">{{ summary.weakPasswordCount }}</div>
            <div class="metric-hint">需人工审计（哈希不可逆）</div>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 角色权限分配饼图 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header">
          <span>角色权限分配统计</span>
          <el-button text :icon="Download" @click="exportRoleAssignment">导出</el-button>
        </div>
      </template>
      <div ref="roleChartRef" class="chart" v-loading="loading.role" />
    </el-card>

    <!-- 长期未登录用户表 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header">
          <span>长期未登录用户（90 天，{{ inactiveUsers.length }} 人）</span>
          <el-button text :icon="Download" @click="exportInactiveUsers">导出</el-button>
        </div>
      </template>
      <el-table :data="inactiveUsers" v-loading="loading.inactive" size="default" stripe>
        <el-table-column type="index" label="#" width="50" />
        <el-table-column prop="username" label="用户名" min-width="120" />
        <el-table-column prop="displayName" label="姓名" min-width="120" />
        <el-table-column prop="email" label="邮箱" min-width="180" />
        <el-table-column label="账号状态" width="100">
          <template #default="{ row }">
            <el-tag :type="row.enabled ? 'success' : 'danger'" size="small">
              {{ row.enabled ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="最后登录时间" min-width="180">
          <template #default="{ row }">
            <span :class="{ 'never-login': !row.lastLoginAt }">
              {{ row.lastLoginAt ? formatTime(row.lastLoginAt) : '从未登录' }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="创建时间" min-width="160">
          <template #default="{ row }">{{ formatTime(row.createdAt) }}</template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!loading.inactive && inactiveUsers.length === 0" description="无长期未登录用户" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  ArrowLeft, Refresh, Download, Key, Timer, User, Warning,
} from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import {
  getComplianceSummary, getInactiveUsers, getRoleAssignment,
} from '../../api/report'
import type { ComplianceSummary, RoleAssignmentItem, UserInfo } from '../../api/types'
import { exportToExcel } from '../../utils/export'

const router = useRouter()

const roleChartRef = ref<HTMLElement>()
let roleChart: echarts.ECharts | null = null

const summary = ref<ComplianceSummary>({
  totalUsers: 0, weakPasswordCount: 0, expiredPasswordCount: 0,
  inactive90dCount: 0, passwordExpiryDays: 0,
})
const inactiveUsers = ref<UserInfo[]>([])
const roleAssignment = ref<RoleAssignmentItem[]>([])

const loading = reactive({ role: false, inactive: false })

const overallHealth = computed<'healthy' | 'warning' | 'danger'>(() => {
  if (summary.value.expiredPasswordCount > 0) return 'danger'
  if (summary.value.inactive90dCount > 0) return 'warning'
  return 'healthy'
})
const overallTitle = computed(() => {
  if (overallHealth.value === 'danger') return '合规健康度：不达标'
  if (overallHealth.value === 'warning') return '合规健康度：需关注'
  return '合规健康度：达标'
})
const overallDesc = computed(() => {
  if (overallHealth.value === 'danger') {
    return `检测到 ${summary.value.expiredPasswordCount} 个密码已过期用户，请立即处理（强制改密或禁用账号）。`
  }
  if (overallHealth.value === 'warning') {
    return `检测到 ${summary.value.inactive90dCount} 个 90 天未登录用户，建议按银行合规要求定期清理。`
  }
  return '所有合规指标均达标，无风险项。'
})

function formatTime(s?: string | null) {
  if (!s) return '—'
  try {
    const d = new Date(s)
    if (isNaN(d.getTime())) return s
    return d.toLocaleString('zh-CN', { hour12: false })
  } catch {
    return s
  }
}

async function loadSummary() {
  summary.value = await getComplianceSummary()
}

async function loadInactive() {
  loading.inactive = true
  try {
    inactiveUsers.value = await getInactiveUsers(90)
  } finally {
    loading.inactive = false
  }
}

async function loadRoleAssignment() {
  loading.role = true
  try {
    roleAssignment.value = await getRoleAssignment()
    await nextTick()
    renderRoleChart()
  } finally {
    loading.role = false
  }
}

function renderRoleChart() {
  if (!roleChartRef.value) return
  if (!roleChart) roleChart = echarts.init(roleChartRef.value)
  const colors = ['#409eff', '#67c23a', '#e6a23c', '#f56c6c', '#909399', '#9c27b0', '#00bcd4']
  roleChart.setOption({
    tooltip: { trigger: 'item', formatter: '{b}: {c} 人 ({d}%)' },
    legend: { orient: 'vertical', left: 10, top: 'center' },
    series: [{
      type: 'pie',
      radius: ['45%', '70%'],
      center: ['65%', '50%'],
      avoidLabelOverlap: true,
      itemStyle: { borderRadius: 6, borderColor: '#fff', borderWidth: 2 },
      label: { show: true, formatter: '{b}\n{c} 人' },
      data: roleAssignment.value.map((r, idx) => ({
        name: r.roleName,
        value: r.userCount,
        itemStyle: { color: colors[idx % colors.length] },
      })),
    }],
  })
  roleChart.resize()
}

async function loadAll() {
  await Promise.all([loadSummary(), loadInactive(), loadRoleAssignment()])
}

function exportRoleAssignment() {
  exportToExcel(
    roleAssignment.value.map((r, idx) => ({
      排名: idx + 1, 角色名称: r.roleName, 用户数: r.userCount,
    })),
    '角色权限分配统计',
  )
  ElMessage.success('导出成功')
}

function exportInactiveUsers() {
  exportToExcel(
    inactiveUsers.value.map((u, idx) => ({
      序号: idx + 1, 用户名: u.username, 姓名: u.displayName, 邮箱: u.email,
      账号状态: u.enabled ? '启用' : '禁用',
      最后登录时间: u.lastLoginAt ? formatTime(u.lastLoginAt) : '从未登录',
      创建时间: formatTime(u.createdAt),
    })),
    '长期未登录用户_90天',
  )
  ElMessage.success('导出成功')
}

function handleResize() {
  roleChart?.resize()
}

onMounted(() => {
  loadAll()
  window.addEventListener('resize', handleResize)
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  roleChart?.dispose()
})
</script>

<style scoped>
.report-page { padding: 0; }
.report-header {
  display: flex; justify-content: space-between; align-items: center;
  margin-bottom: 20px; flex-wrap: wrap; gap: 12px;
}
.header-left { display: flex; align-items: center; gap: 12px; }
.header-left h2 { margin: 0; font-size: 20px; font-weight: 600; }
.header-right { display: flex; align-items: center; gap: 8px; }

.health-alert { margin-bottom: 16px; }

.metric-row { margin-bottom: 16px; }
.metric-card {
  display: flex; align-items: center; gap: 14px;
  padding: 18px 20px; border-radius: 10px;
  background: #fff; border: 1px solid #ebeef5;
  margin-bottom: 16px;
}
.metric-icon {
  width: 48px; height: 48px; border-radius: 10px;
  display: flex; align-items: center; justify-content: center;
  font-size: 24px; color: #fff;
}
.metric-card.success .metric-icon { background: linear-gradient(135deg, #67c23a, #95d475); }
.metric-card.danger .metric-icon { background: linear-gradient(135deg, #f56c6c, #f89898); }
.metric-card.warning .metric-icon { background: linear-gradient(135deg, #e6a23c, #f0c78a); }
.metric-card.info .metric-icon { background: linear-gradient(135deg, #409eff, #66b1ff); }
.metric-label { font-size: 12px; color: #909399; }
.metric-value { font-size: 28px; font-weight: 600; color: #303133; margin-top: 4px; }
.metric-hint { font-size: 11px; color: #c0c4cc; margin-top: 4px; }

.chart-card { margin-bottom: 16px; }
.chart { width: 100%; height: 320px; }
.card-header {
  display: flex; justify-content: space-between; align-items: center;
  font-weight: 600;
}
.never-login { color: #f56c6c; font-style: italic; }

@media (max-width: 768px) {
  .header-left h2 { font-size: 16px; }
  .chart { height: 260px; }
}
</style>
