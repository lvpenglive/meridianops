<template>
  <div class="report-page">
    <div class="report-header">
      <div class="header-left">
        <el-button text @click="router.push('/report')">
          <el-icon><ArrowLeft /></el-icon> 报表中心
        </el-button>
        <h2>登录安全分析</h2>
        <el-tag type="danger" effect="dark" size="small">安全审计</el-tag>
      </div>
      <div class="header-right">
        <el-radio-group v-model="daysRange" @change="loadAll">
          <el-radio-button :value="7">近 7 天</el-radio-button>
          <el-radio-button :value="30">近 30 天</el-radio-button>
          <el-radio-button :value="90">近 90 天</el-radio-button>
        </el-radio-group>
        <el-button :icon="Refresh" circle @click="loadAll" />
      </div>
    </div>

    <!-- 摘要卡片 -->
    <el-row :gutter="16" class="summary-row">
      <el-col :xs="12" :sm="6">
        <div class="summary-card success">
          <div class="summary-icon"><el-icon><CircleCheck /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">成功登录次数</div>
            <div class="summary-value">{{ totalSuccess }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="summary-card danger">
          <div class="summary-icon"><el-icon><CircleClose /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">失败登录次数</div>
            <div class="summary-value">{{ totalFailed }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="summary-card warning">
          <div class="summary-icon"><el-icon><Warning /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">失败率</div>
            <div class="summary-value">{{ failureRate }}%</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="summary-card info">
          <div class="summary-icon"><el-icon><Lock /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">当前锁定账号</div>
            <div class="summary-value">{{ lockedUsers.length }}</div>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 登录趋势图 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header">
          <span>登录趋势（成功 vs 失败）</span>
          <el-button text :icon="Download" @click="exportTrend">导出 Excel</el-button>
        </div>
      </template>
      <div ref="trendChartRef" class="chart" v-loading="loading.trend" />
    </el-card>

    <!-- 失败登录 TOP -->
    <el-row :gutter="16">
      <el-col :xs="24" :lg="12">
        <el-card shadow="never" class="chart-card">
          <template #header>
            <div class="card-header">
              <span>失败登录 TOP 10 用户</span>
              <el-button text :icon="Download" @click="exportFailedTop">导出</el-button>
            </div>
          </template>
          <div ref="failedTopChartRef" class="chart" v-loading="loading.failedTop" />
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="12">
        <el-card shadow="never" class="chart-card">
          <template #header>
            <div class="card-header">
              <span>失败登录 TOP 明细</span>
            </div>
          </template>
          <el-table :data="failedTop" v-loading="loading.failedTop" size="small" stripe>
            <el-table-column type="index" label="#" width="50" />
            <el-table-column prop="username" label="用户名" min-width="120" />
            <el-table-column prop="failedCount" label="失败次数" width="100" sortable>
              <template #default="{ row }">
                <el-tag type="danger" size="small">{{ row.failedCount }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="最近失败时间" min-width="160">
              <template #default="{ row }">{{ formatTime(row.lastFailedAt) }}</template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
    </el-row>

    <!-- 锁定账号列表 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header">
          <span>当前锁定账号（{{ lockedUsers.length }}）</span>
        </div>
      </template>
      <el-table :data="lockedUsers" v-loading="loading.locked" size="default" stripe>
        <el-table-column type="index" label="#" width="50" />
        <el-table-column prop="username" label="用户名" min-width="120" />
        <el-table-column prop="displayName" label="姓名" min-width="120" />
        <el-table-column prop="email" label="邮箱" min-width="180" />
        <el-table-column label="失败次数" width="100">
          <template #default="{ row }">
            <el-tag type="danger" size="small">{{ row.failedLoginAttempts }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="锁定截止时间" min-width="180">
          <template #default="{ row }">{{ formatTime(row.lockedUntil) }}</template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!loading.locked && lockedUsers.length === 0" description="当前无锁定账号" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  ArrowLeft, Refresh, Download, CircleCheck, CircleClose, Warning, Lock,
} from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import {
  getLoginTrend, getLoginFailedTop, getLockedUsers,
} from '../../api/report'
import type { LoginTrendItem, FailedTopItem, UserInfo } from '../../api/types'
import { exportToExcel } from '../../utils/export'

const router = useRouter()
const daysRange = ref(30)

const trendChartRef = ref<HTMLElement>()
const failedTopChartRef = ref<HTMLElement>()
let trendChart: echarts.ECharts | null = null
let failedTopChart: echarts.ECharts | null = null

const trend = ref<LoginTrendItem[]>([])
const failedTop = ref<FailedTopItem[]>([])
const lockedUsers = ref<UserInfo[]>([])

const loading = reactive({ trend: false, failedTop: false, locked: false })

const totalSuccess = ref(0)
const totalFailed = ref(0)
const failureRate = ref(0)

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

async function loadTrend() {
  loading.trend = true
  try {
    trend.value = await getLoginTrend(daysRange.value)
    totalSuccess.value = trend.value.reduce((s, i) => s + i.success, 0)
    totalFailed.value = trend.value.reduce((s, i) => s + i.failed, 0)
    const total = totalSuccess.value + totalFailed.value
    failureRate.value = total === 0 ? 0 : Number(((totalFailed.value / total) * 100).toFixed(1))
    await nextTick()
    renderTrendChart()
  } finally {
    loading.trend = false
  }
}

function renderTrendChart() {
  if (!trendChartRef.value) return
  if (!trendChart) trendChart = echarts.init(trendChartRef.value)
  trendChart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['成功', '失败'], top: 5 },
    grid: { left: 40, right: 20, top: 40, bottom: 30 },
    xAxis: {
      type: 'category',
      data: trend.value.map(i => i.date),
      axisLabel: { fontSize: 11 },
    },
    yAxis: { type: 'value', name: '次数' },
    series: [
      { name: '成功', type: 'line', smooth: true, areaStyle: { opacity: 0.15 }, itemStyle: { color: '#67c23a' }, data: trend.value.map(i => i.success) },
      { name: '失败', type: 'line', smooth: true, areaStyle: { opacity: 0.15 }, itemStyle: { color: '#f56c6c' }, data: trend.value.map(i => i.failed) },
    ],
  })
  trendChart.resize()
}

async function loadFailedTop() {
  loading.failedTop = true
  try {
    failedTop.value = await getLoginFailedTop(daysRange.value, 10)
    await nextTick()
    renderFailedTopChart()
  } finally {
    loading.failedTop = false
  }
}

function renderFailedTopChart() {
  if (!failedTopChartRef.value) return
  if (!failedTopChart) failedTopChart = echarts.init(failedTopChartRef.value)
  const reversed = [...failedTop.value].reverse()
  failedTopChart.setOption({
    tooltip: { trigger: 'axis' },
    grid: { left: 100, right: 20, top: 20, bottom: 30 },
    xAxis: { type: 'value', name: '失败次数' },
    yAxis: { type: 'category', data: reversed.map(i => i.username) },
    series: [{
      type: 'bar',
      data: reversed.map(i => i.failedCount),
      itemStyle: { color: '#f56c6c', borderRadius: [0, 4, 4, 0] },
      label: { show: true, position: 'right', fontSize: 11 },
    }],
  })
  failedTopChart.resize()
}

async function loadLocked() {
  loading.locked = true
  try {
    lockedUsers.value = await getLockedUsers()
  } finally {
    loading.locked = false
  }
}

async function loadAll() {
  await Promise.all([loadTrend(), loadFailedTop(), loadLocked()])
}

function exportTrend() {
  exportToExcel(
    trend.value.map(i => ({ 日期: i.date, 成功次数: i.success, 失败次数: i.failed })),
    `登录趋势_${daysRange.value}天`,
  )
  ElMessage.success('导出成功')
}

function exportFailedTop() {
  exportToExcel(
    failedTop.value.map((i, idx) => ({
      排名: idx + 1, 用户名: i.username, 失败次数: i.failedCount,
      最近失败时间: formatTime(i.lastFailedAt),
    })),
    `失败登录TOP10_${daysRange.value}天`,
  )
  ElMessage.success('导出成功')
}

function handleResize() {
  trendChart?.resize()
  failedTopChart?.resize()
}

onMounted(() => {
  loadAll()
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  trendChart?.dispose()
  failedTopChart?.dispose()
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

.summary-row { margin-bottom: 16px; }
.summary-card {
  display: flex; align-items: center; gap: 14px;
  padding: 18px 20px; border-radius: 10px;
  background: #fff; border: 1px solid #ebeef5;
  margin-bottom: 16px;
}
.summary-icon {
  width: 48px; height: 48px; border-radius: 10px;
  display: flex; align-items: center; justify-content: center;
  font-size: 24px; color: #fff;
}
.summary-card.success .summary-icon { background: linear-gradient(135deg, #67c23a, #95d475); }
.summary-card.danger .summary-icon { background: linear-gradient(135deg, #f56c6c, #f89898); }
.summary-card.warning .summary-icon { background: linear-gradient(135deg, #e6a23c, #f0c78a); }
.summary-card.info .summary-icon { background: linear-gradient(135deg, #909399, #b1b3b8); }
.summary-label { font-size: 12px; color: #909399; }
.summary-value { font-size: 26px; font-weight: 600; color: #303133; margin-top: 4px; }

.chart-card { margin-bottom: 16px; }
.chart { width: 100%; height: 320px; }
.card-header {
  display: flex; justify-content: space-between; align-items: center;
  font-weight: 600;
}

@media (max-width: 768px) {
  .header-left h2 { font-size: 16px; }
  .chart { height: 260px; }
}
</style>
