<template>
  <div class="report-page">
    <div class="report-header">
      <div class="header-left">
        <el-button text @click="router.push('/report')">
          <el-icon><ArrowLeft /></el-icon> 报表中心
        </el-button>
        <h2>敏感操作审计</h2>
        <el-tag type="warning" effect="dark" size="small">安全审计</el-tag>
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

    <el-alert
      type="warning"
      :closable="false"
      show-icon
      title="敏感操作定义"
      description="包含删除用户 / 禁用用户 / 重置密码 / 角色增改删 / 权限分配 / 系统设置变更 / 密码修改等高风险操作。"
      class="def-alert"
    />

    <!-- 摘要卡片 -->
    <el-row :gutter="16" class="summary-row">
      <el-col :xs="12" :sm="8">
        <div class="summary-card danger">
          <div class="summary-icon"><el-icon><Warning /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">敏感操作总数</div>
            <div class="summary-value">{{ totalOps }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8">
        <div class="summary-card warning">
          <div class="summary-icon"><el-icon><User /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">涉及操作人</div>
            <div class="summary-value">{{ topOps.length }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8">
        <div class="summary-card info">
          <div class="summary-icon"><el-icon><Calendar /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">日均操作次数</div>
            <div class="summary-value">{{ dailyAvg }}</div>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 敏感操作趋势 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header">
          <span>敏感操作趋势</span>
          <el-button text :icon="Download" @click="exportTrend">导出 Excel</el-button>
        </div>
      </template>
      <div ref="trendChartRef" class="chart" v-loading="loading.trend" />
    </el-card>

    <!-- TOP 操作人 -->
    <el-row :gutter="16">
      <el-col :xs="24" :lg="12">
        <el-card shadow="never" class="chart-card">
          <template #header>
            <div class="card-header">
              <span>敏感操作 TOP 10 操作人</span>
              <el-button text :icon="Download" @click="exportTop">导出</el-button>
            </div>
          </template>
          <div ref="topChartRef" class="chart" v-loading="loading.top" />
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="12">
        <el-card shadow="never" class="chart-card">
          <template #header>
            <div class="card-header">
              <span>TOP 操作人明细</span>
            </div>
          </template>
          <el-table :data="topOps" v-loading="loading.top" size="small" stripe>
            <el-table-column type="index" label="#" width="50" />
            <el-table-column prop="username" label="用户名" min-width="120" />
            <el-table-column prop="count" label="操作次数" width="100" sortable>
              <template #default="{ row }">
                <el-tag type="warning" size="small">{{ row.count }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="最近操作时间" min-width="160">
              <template #default="{ row }">{{ formatTime(row.lastActionAt) }}</template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
    </el-row>

    <!-- 明细分页表 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header">
          <span>敏感操作明细</span>
          <el-button text :icon="Download" @click="exportList">导出当前页</el-button>
        </div>
      </template>
      <el-table :data="list.items" v-loading="loading.list" size="default" stripe>
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="actorUsername" label="操作人" min-width="120" />
        <el-table-column prop="action" label="操作类型" min-width="140">
          <template #default="{ row }">
            <el-tag :type="actionTagType(row.action)" size="small">{{ actionLabel(row.action) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="targetType" label="对象类型" min-width="100" />
        <el-table-column prop="targetId" label="对象 ID" min-width="200" show-overflow-tooltip />
        <el-table-column prop="ip" label="IP" min-width="130" />
        <el-table-column prop="status" label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="row.status === 'success' ? 'success' : 'danger'" size="small">
              {{ row.status === 'success' ? '成功' : '失败' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="时间" min-width="160">
          <template #default="{ row }">{{ formatTime(row.createdAt) }}</template>
        </el-table-column>
      </el-table>
      <div class="pager">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :total="list.total"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next"
          @size-change="loadList"
          @current-change="loadList"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  ArrowLeft, Refresh, Download, Warning, User, Calendar,
} from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import {
  getSensitiveOpsTrend, getSensitiveOpsTop, getSensitiveOpsList,
} from '../../api/report'
import type { SensitiveTrendItem, SensitiveTopItem, SensitiveListResponse, AuditLog } from '../../api/types'
import { exportToExcel } from '../../utils/export'

const router = useRouter()
const daysRange = ref(30)

const trendChartRef = ref<HTMLElement>()
const topChartRef = ref<HTMLElement>()
let trendChart: echarts.ECharts | null = null
let topChart: echarts.ECharts | null = null

const trend = ref<SensitiveTrendItem[]>([])
const topOps = ref<SensitiveTopItem[]>([])
const list = ref<SensitiveListResponse>({ total: 0, page: 1, pageSize: 20, items: [] as AuditLog[] })

const loading = reactive({ trend: false, top: false, list: false })
const page = ref(1)
const pageSize = ref(20)

const totalOps = computed(() => trend.value.reduce((s, i) => s + i.count, 0))
const dailyAvg = computed(() => {
  if (trend.value.length === 0) return 0
  return (totalOps.value / trend.value.length).toFixed(1)
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

const ACTION_LABELS: Record<string, string> = {
  delete_user: '删除用户',
  disable_user: '禁用用户',
  reset_password: '重置密码',
  create_role: '创建角色',
  update_role: '更新角色',
  delete_role: '删除角色',
  assign_permission: '权限分配',
  update_settings: '系统设置变更',
  change_password: '密码修改',
}
function actionLabel(a: string) {
  return ACTION_LABELS[a] || a
}
function actionTagType(a: string): 'danger' | 'warning' | 'info' {
  if (['delete_user', 'delete_role', 'reset_password'].includes(a)) return 'danger'
  if (['disable_user', 'update_role', 'assign_permission', 'update_settings'].includes(a)) return 'warning'
  return 'info'
}

async function loadTrend() {
  loading.trend = true
  try {
    trend.value = await getSensitiveOpsTrend(daysRange.value)
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
    grid: { left: 40, right: 20, top: 30, bottom: 30 },
    xAxis: { type: 'category', data: trend.value.map(i => i.date), axisLabel: { fontSize: 11 } },
    yAxis: { type: 'value', name: '次数' },
    series: [{
      type: 'bar',
      data: trend.value.map(i => i.count),
      itemStyle: { color: '#e6a23c', borderRadius: [4, 4, 0, 0] },
      label: { show: true, position: 'top', fontSize: 11 },
    }],
  })
  trendChart.resize()
}

async function loadTop() {
  loading.top = true
  try {
    topOps.value = await getSensitiveOpsTop(daysRange.value, 10)
    await nextTick()
    renderTopChart()
  } finally {
    loading.top = false
  }
}

function renderTopChart() {
  if (!topChartRef.value) return
  if (!topChart) topChart = echarts.init(topChartRef.value)
  const reversed = [...topOps.value].reverse()
  topChart.setOption({
    tooltip: { trigger: 'axis' },
    grid: { left: 100, right: 20, top: 20, bottom: 30 },
    xAxis: { type: 'value', name: '操作次数' },
    yAxis: { type: 'category', data: reversed.map(i => i.username) },
    series: [{
      type: 'bar',
      data: reversed.map(i => i.count),
      itemStyle: { color: '#e6a23c', borderRadius: [0, 4, 4, 0] },
      label: { show: true, position: 'right', fontSize: 11 },
    }],
  })
  topChart.resize()
}

async function loadList() {
  loading.list = true
  try {
    list.value = await getSensitiveOpsList({
      days: daysRange.value,
      page: page.value,
      pageSize: pageSize.value,
    })
  } finally {
    loading.list = false
  }
}

async function loadAll() {
  page.value = 1
  await Promise.all([loadTrend(), loadTop(), loadList()])
}

function exportTrend() {
  exportToExcel(
    trend.value.map(i => ({ 日期: i.date, 操作次数: i.count })),
    `敏感操作趋势_${daysRange.value}天`,
  )
  ElMessage.success('导出成功')
}
function exportTop() {
  exportToExcel(
    topOps.value.map((i, idx) => ({
      排名: idx + 1, 用户名: i.username, 操作次数: i.count,
      最近操作时间: formatTime(i.lastActionAt),
    })),
    `敏感操作TOP10_${daysRange.value}天`,
  )
  ElMessage.success('导出成功')
}
function exportList() {
  exportToExcel(
    list.value.items.map(i => ({
      ID: i.id, 操作人: i.actorUsername, 操作类型: actionLabel(i.action),
      对象类型: i.targetType, 对象ID: i.targetId, IP: i.ip,
      状态: i.status === 'success' ? '成功' : '失败', 时间: formatTime(i.createdAt),
    })),
    `敏感操作明细_${daysRange.value}天_第${page.value}页`,
  )
  ElMessage.success('导出成功')
}

function handleResize() {
  trendChart?.resize()
  topChart?.resize()
}

onMounted(() => {
  loadAll()
  window.addEventListener('resize', handleResize)
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  trendChart?.dispose()
  topChart?.dispose()
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

.def-alert { margin-bottom: 16px; }

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
.pager {
  margin-top: 16px; display: flex; justify-content: flex-end;
}

@media (max-width: 768px) {
  .header-left h2 { font-size: 16px; }
  .chart { height: 260px; }
}
</style>
