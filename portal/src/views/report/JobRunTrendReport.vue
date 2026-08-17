<template>
  <div class="report-page">
    <div class="report-header">
      <div class="header-left">
        <el-button text @click="router.push('/report')">
          <el-icon><ArrowLeft /></el-icon> 报表中心
        </el-button>
        <h2>作业执行趋势</h2>
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

    <!-- 摘要卡片 -->
    <el-row :gutter="16" class="summary-row">
      <el-col :xs="12" :sm="6">
        <div class="summary-card info">
          <div class="summary-icon"><el-icon><DataLine /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">总执行次数</div>
            <div class="summary-value">{{ totalRun }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="summary-card success">
          <div class="summary-icon"><el-icon><CircleCheck /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">总成功次数</div>
            <div class="summary-value">{{ totalSuccess }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="summary-card danger">
          <div class="summary-icon"><el-icon><CircleClose /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">总失败次数</div>
            <div class="summary-value">{{ totalFailed }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="6">
        <div class="summary-card warning">
          <div class="summary-icon"><el-icon><TrendCharts /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">成功率</div>
            <div class="summary-value">{{ successRate }}%</div>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 趋势折线图 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header">
          <span>作业执行趋势（总执行 / 成功 / 失败）</span>
        </div>
      </template>
      <div ref="trendChartRef" class="chart" v-loading="loading.trend" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowLeft, Refresh, CircleCheck, CircleClose, DataLine, TrendCharts } from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import { useUserStore } from '../../stores/user'

const router = useRouter()
const daysRange = ref(30)

interface TrendItem { date: string; total: number; success: number; failed: number }
const trend = ref<TrendItem[]>([])
const loading = reactive({ trend: false })

const trendChartRef = ref<HTMLElement>()
let trendChart: echarts.ECharts | null = null

const totalRun = ref(0)
const totalSuccess = ref(0)
const totalFailed = ref(0)
const successRate = ref(0)

async function loadTrend() {
  loading.trend = true
  try {
    const token = useUserStore().token
    const res = await fetch(`/api/reports/job-run-trend?days=${daysRange.value}`, { headers: { Authorization: `Bearer ${token}` } })
    const json = await res.json()
    if (json.code === 0) trend.value = json.data || []
    totalRun.value = trend.value.reduce((s, i) => s + i.total, 0)
    totalSuccess.value = trend.value.reduce((s, i) => s + i.success, 0)
    totalFailed.value = trend.value.reduce((s, i) => s + i.failed, 0)
    successRate.value = totalRun.value === 0 ? 0 : Number(((totalSuccess.value / totalRun.value) * 100).toFixed(1))
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
    legend: { data: ['总执行', '成功', '失败'], top: 5 },
    grid: { left: 40, right: 20, top: 40, bottom: 30 },
    xAxis: {
      type: 'category',
      data: trend.value.map(i => i.date),
      axisLabel: { fontSize: 11 },
    },
    yAxis: { type: 'value', name: '次数' },
    series: [
      { name: '总执行', type: 'line', smooth: true, areaStyle: { opacity: 0.1 }, itemStyle: { color: '#409eff' }, data: trend.value.map(i => i.total) },
      { name: '成功', type: 'line', smooth: true, areaStyle: { opacity: 0.15 }, itemStyle: { color: '#67c23a' }, data: trend.value.map(i => i.success) },
      { name: '失败', type: 'line', smooth: true, areaStyle: { opacity: 0.15 }, itemStyle: { color: '#f56c6c' }, data: trend.value.map(i => i.failed) },
    ],
  })
  trendChart.resize()
}

async function loadAll() {
  await loadTrend()
}

function handleResize() {
  trendChart?.resize()
}

onMounted(() => {
  loadAll()
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  trendChart?.dispose()
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
.summary-card.primary .summary-icon { background: linear-gradient(135deg, #409eff, #66b1ff); }
.summary-label { font-size: 12px; color: #909399; }
.summary-value { font-size: 26px; font-weight: 600; color: #303133; margin-top: 4px; }

.chart-card { margin-bottom: 16px; }
.chart { width: 100%; height: 320px; }
.card-header { display: flex; justify-content: space-between; align-items: center; font-weight: 600; }

@media (max-width: 768px) {
  .header-left h2 { font-size: 16px; }
  .chart { height: 260px; }
}
</style>
