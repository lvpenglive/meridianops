<template>
  <div class="report-page">
    <div class="report-header">
      <div class="header-left">
        <el-button text @click="router.push('/report')">
          <el-icon><ArrowLeft /></el-icon> 报表中心
        </el-button>
        <h2>审计操作趋势</h2>
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
      <el-col :xs="12" :sm="8">
        <div class="summary-card info">
          <div class="summary-icon"><el-icon><Operation /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">总操作次数</div>
            <div class="summary-value">{{ totalActions }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8">
        <div class="summary-card primary">
          <div class="summary-icon"><el-icon><Histogram /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">操作类型数</div>
            <div class="summary-value">{{ actionTypes.length }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8">
        <div class="summary-card warning">
          <div class="summary-icon"><el-icon><TrendCharts /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">最活跃日</div>
            <div class="summary-value text-value">{{ topDay ? topDay.date : '—' }}</div>
            <div v-if="topDay" class="summary-sub">操作 {{ topDay.count }} 次</div>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 堆叠柱状图 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header"><span>审计操作趋势（按天堆叠）</span></div>
      </template>
      <div ref="stackChartRef" class="chart chart-tall" v-loading="loading" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowLeft, Refresh, Operation, Histogram, TrendCharts } from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import { useUserStore } from '../../stores/user'

const router = useRouter()
const daysRange = ref(30)

interface AuditItem { date: string; action: string; count: number }
const items = ref<AuditItem[]>([])
const loading = ref(false)

const stackChartRef = ref<HTMLElement>()
let stackChart: echarts.ECharts | null = null

const totalActions = computed(() => items.value.reduce((s, i) => s + i.count, 0))
const actionTypes = computed(() => Array.from(new Set(items.value.map(i => i.action))))
const dates = computed(() => Array.from(new Set(items.value.map(i => i.date))).sort())
const topDay = computed(() => {
  const byDate = new Map<string, number>()
  items.value.forEach(i => byDate.set(i.date, (byDate.get(i.date) || 0) + i.count))
  let best: { date: string; count: number } | undefined
  byDate.forEach((count, date) => {
    if (!best || count > best.count) best = { date, count }
  })
  return best
})

async function loadAll() {
  loading.value = true
  try {
    const token = useUserStore().token
    const res = await fetch(`/api/reports/audit-trend?days=${daysRange.value}`, { headers: { Authorization: `Bearer ${token}` } })
    const json = await res.json()
    if (json.code === 0) items.value = json.data || []
    await nextTick()
    renderStackChart()
  } finally {
    loading.value = false
  }
}

function renderStackChart() {
  if (!stackChartRef.value) return
  if (!stackChart) stackChart = echarts.init(stackChartRef.value)
  const datesArr = dates.value
  const actions = actionTypes.value
  const palette = ['#409eff', '#67c23a', '#e6a23c', '#f56c6c', '#909399', '#9c27b0', '#00bcd4', '#ff9800', '#795548', '#3f51b5']
  const series = actions.map((action, idx) => ({
    name: action,
    type: 'bar',
    stack: 'total',
    emphasis: { focus: 'series' },
    itemStyle: { color: palette[idx % palette.length] },
    data: datesArr.map(d => {
      const item = items.value.find(i => i.date === d && i.action === action)
      return item ? item.count : 0
    }),
  }))
  stackChart.setOption({
    tooltip: { trigger: 'axis', axisPointer: { type: 'shadow' } },
    legend: { data: actions, top: 5, type: 'scroll' },
    grid: { left: 40, right: 20, top: 50, bottom: 30 },
    xAxis: { type: 'category', data: datesArr, axisLabel: { fontSize: 11 } },
    yAxis: { type: 'value', name: '次数' },
    series,
  })
  stackChart.resize()
}

function handleResize() {
  stackChart?.resize()
}

onMounted(() => {
  loadAll()
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  stackChart?.dispose()
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
.summary-value.text-value { font-size: 18px; }
.summary-sub { font-size: 12px; color: #909399; margin-top: 2px; }

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
