<template>
  <div class="report-page">
    <div class="report-header">
      <div class="header-left">
        <el-button text @click="router.push('/report')">
          <el-icon><ArrowLeft /></el-icon> 报表中心
        </el-button>
        <h2>作业执行统计</h2>
        <el-tag type="success" effect="dark" size="small">系统健康</el-tag>
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

    <!-- 成功率柱状图 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header"><span>作业成功率排序</span></div>
      </template>
      <div ref="barChartRef" class="chart chart-tall" v-loading="loading.summary" />
    </el-card>

    <!-- 作业明细表格 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header"><span>作业执行明细（{{ summary.length }}）</span></div>
      </template>
      <el-table :data="summary" v-loading="loading.summary" size="default" stripe>
        <el-table-column type="index" label="#" width="50" />
        <el-table-column prop="jobName" label="作业名称" min-width="180" />
        <el-table-column prop="total" label="总执行" width="100" sortable />
        <el-table-column prop="success" label="成功" width="100" sortable>
          <template #default="{ row }">
            <el-tag type="success" size="small">{{ row.success }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="failed" label="失败" width="100" sortable>
          <template #default="{ row }">
            <el-tag type="danger" size="small" v-if="row.failed > 0">{{ row.failed }}</el-tag>
            <span v-else>0</span>
          </template>
        </el-table-column>
        <el-table-column label="平均耗时" width="130" sortable :sort-method="sortByDuration">
          <template #default="{ row }">{{ formatDuration(row.avgDurationSec) }}</template>
        </el-table-column>
        <el-table-column label="成功率" width="160" sortable :sort-method="sortByRate">
          <template #default="{ row }">
            <el-progress :percentage="Number(row.successRate) || 0" :status="rateStatus(row.successRate)" :stroke-width="14" />
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!loading.summary && summary.length === 0" description="暂无数据" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowLeft, Refresh } from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import { useUserStore } from '../../stores/user'

const router = useRouter()
const daysRange = ref(30)

interface SummaryItem {
  jobName: string
  total: number
  success: number
  failed: number
  avgDurationSec: number
  successRate: number
}
const summary = ref<SummaryItem[]>([])
const loading = reactive({ summary: false })

const barChartRef = ref<HTMLElement>()
let barChart: echarts.ECharts | null = null

function formatDuration(sec?: number) {
  if (sec == null) return '—'
  if (sec < 1) return `${(sec * 1000).toFixed(0)} ms`
  if (sec < 60) return `${sec.toFixed(1)} 秒`
  const m = Math.floor(sec / 60)
  const s = Math.round(sec % 60)
  return `${m} 分 ${s} 秒`
}

function rateStatus(rate: number) {
  const r = Number(rate) || 0
  if (r >= 95) return 'success'
  if (r >= 80) return undefined
  return 'exception'
}

function sortByDuration(a: SummaryItem, b: SummaryItem) {
  return (a.avgDurationSec || 0) - (b.avgDurationSec || 0)
}

function sortByRate(a: SummaryItem, b: SummaryItem) {
  return (a.successRate || 0) - (b.successRate || 0)
}

async function loadSummary() {
  loading.summary = true
  try {
    const token = useUserStore().token
    const res = await fetch(`/api/reports/job-def-summary?days=${daysRange.value}`, { headers: { Authorization: `Bearer ${token}` } })
    const json = await res.json()
    if (json.code === 0) summary.value = json.data || []
    await nextTick()
    renderBarChart()
  } finally {
    loading.summary = false
  }
}

function renderBarChart() {
  if (!barChartRef.value) return
  if (!barChart) barChart = echarts.init(barChartRef.value)
  const sorted = [...summary.value].sort((a, b) => (a.successRate || 0) - (b.successRate || 0))
  barChart.setOption({
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
      formatter: (p: any) => `${p[0].name}<br/>成功率: ${p[0].value}%`,
    },
    grid: { left: 10, right: 40, top: 20, bottom: 30, containLabel: true },
    xAxis: { type: 'value', max: 100, name: '成功率(%)' },
    yAxis: { type: 'category', data: sorted.map(i => i.jobName), axisLabel: { fontSize: 11 } },
    series: [{
      type: 'bar',
      data: sorted.map(i => Number(i.successRate) || 0),
      itemStyle: {
        borderRadius: [0, 4, 4, 0],
        color: (params: any) => {
          const v = params.value
          if (v >= 95) return '#67c23a'
          if (v >= 80) return '#409eff'
          if (v >= 50) return '#e6a23c'
          return '#f56c6c'
        },
      },
      label: { show: true, position: 'right', fontSize: 11, formatter: '{c}%' },
    }],
  })
  barChart.resize()
}

async function loadAll() {
  await loadSummary()
}

function handleResize() {
  barChart?.resize()
}

onMounted(() => {
  loadAll()
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  barChart?.dispose()
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

.chart-card { margin-bottom: 16px; }
.chart { width: 100%; height: 320px; }
.chart-tall { height: 420px; }
.card-header { display: flex; justify-content: space-between; align-items: center; font-weight: 600; }

@media (max-width: 768px) {
  .header-left h2 { font-size: 16px; }
  .chart { height: 260px; }
  .chart-tall { height: 340px; }
}
</style>
