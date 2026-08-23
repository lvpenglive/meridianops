<template>
  <div class="ticket-stats-page">
    <!-- 页面头部 -->
    <div class="page-header">
      <h2 class="page-title">工单统计看板</h2>
      <el-button :icon="Refresh" :loading="loading" @click="loadStats">刷新</el-button>
    </div>

    <!-- 统计卡片 -->
    <el-row :gutter="12" class="stats-cards" v-loading="loading">
      <el-col :xs="12" :sm="8" :md="4">
        <div class="stat-card stat-total">
          <div class="stat-icon"><el-icon><Tickets /></el-icon></div>
          <div class="stat-body">
            <div class="stat-label">总工单数</div>
            <div class="stat-value">{{ stats.total ?? 0 }}</div>
            <div class="stat-sub">历史累计</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <div class="stat-card stat-pending">
          <div class="stat-icon"><el-icon><Clock /></el-icon></div>
          <div class="stat-body">
            <div class="stat-label">待处理</div>
            <div class="stat-value">{{ stats.pending ?? 0 }}</div>
            <div class="stat-sub">进行中 / 待审批</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <div class="stat-card stat-closed">
          <div class="stat-icon"><el-icon><CircleCheckFilled /></el-icon></div>
          <div class="stat-body">
            <div class="stat-label">已关闭</div>
            <div class="stat-value">{{ stats.closed ?? 0 }}</div>
            <div class="stat-sub">已完成工单</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <div class="stat-card stat-new">
          <div class="stat-icon"><el-icon><TrendCharts /></el-icon></div>
          <div class="stat-body">
            <div class="stat-label">本月新增</div>
            <div class="stat-value">{{ stats.newThisMonth ?? 0 }}</div>
            <div class="stat-sub">{{ currentMonth }} 月</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="24" :md="8">
        <div class="stat-card stat-avg">
          <div class="stat-icon"><el-icon><Timer /></el-icon></div>
          <div class="stat-body">
            <div class="stat-label">平均处理时长</div>
            <div class="stat-value">{{ avgResolveText }}</div>
            <div class="stat-sub">从创建到关闭</div>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 图表区域 -->
    <el-row :gutter="12" class="charts-row">
      <!-- 按类型分布 - 饼图 -->
      <el-col :xs="24" :md="8">
        <el-card shadow="never" class="chart-card">
          <div class="chart-title">工单类型分布</div>
          <div ref="typeChartRef" class="chart-container"></div>
        </el-card>
      </el-col>
      <!-- 按状态分布 - 柱状图 -->
      <el-col :xs="24" :md="8">
        <el-card shadow="never" class="chart-card">
          <div class="chart-title">工单状态分布</div>
          <div ref="statusChartRef" class="chart-container"></div>
        </el-card>
      </el-col>
      <!-- 近30天趋势 - 折线图 -->
      <el-col :xs="24" :md="8">
        <el-card shadow="never" class="chart-card">
          <div class="chart-title">近 30 天趋势</div>
          <div ref="trendChartRef" class="chart-container"></div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Tickets, Clock, CircleCheckFilled, TrendCharts, Timer, Refresh,
} from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import { getTicketStats, type TicketStats } from '../../api/ticket'

const loading = ref(false)
const stats = reactive<Partial<TicketStats>>({
  total: 0,
  pending: 0,
  closed: 0,
  newThisMonth: 0,
  avgResolveHours: 0,
  byType: [],
  byStatus: [],
  trend30d: [],
})

const currentMonth = computed(() => new Date().getMonth() + 1)

const avgResolveText = computed(() => {
  const hours = stats.avgResolveHours ?? 0
  if (hours < 1) return `${Math.round(hours * 60)} 分钟`
  if (hours < 24) return `${hours.toFixed(1)} 小时`
  const days = hours / 24
  return `${days.toFixed(1)} 天`
})

// ---- ECharts 引用 ----
const typeChartRef = ref<HTMLElement>()
const statusChartRef = ref<HTMLElement>()
const trendChartRef = ref<HTMLElement>()

let typeChart: echarts.ECharts | null = null
let statusChart: echarts.ECharts | null = null
let trendChart: echarts.ECharts | null = null

const TYPE_COLORS: Record<string, string> = {
  incident: '#F56C6C',
  problem: '#E6A23C',
  change: '#409EFF',
  change_emergency: '#F56C6C',
  task: '#67C23A',
}

function getTypeColor(name: string): string {
  return TYPE_COLORS[name] || '#909399'
}

function initTypeChart() {
  if (!typeChartRef.value) return
  typeChart = echarts.init(typeChartRef.value)
  const data = stats.byType || []
  typeChart.setOption({
    tooltip: {
      trigger: 'item',
      formatter: '{b}: {c} ({d}%)',
    },
    legend: {
      orient: 'vertical',
      right: 10,
      top: 'center',
      textStyle: { fontSize: 12, color: '#606266' },
    },
    series: [
      {
        type: 'pie',
        radius: ['40%', '70%'],
        center: ['35%', '50%'],
        avoidLabelOverlap: false,
        itemStyle: {
          borderRadius: 4,
          borderColor: '#fff',
          borderWidth: 2,
        },
        label: { show: false },
        emphasis: {
          label: { show: true, fontSize: 14, fontWeight: 'bold' },
        },
        data: data.map(d => ({
          name: d.name,
          value: d.value,
          itemStyle: { color: getTypeColor(d.name) },
        })),
      },
    ],
  })
}

function initStatusChart() {
  if (!statusChartRef.value) return
  statusChart = echarts.init(statusChartRef.value)
  const data = stats.byStatus || []
  const colors = data.map(d => {
    const statusColorMap: Record<string, string> = {
      open: '#909399',
      assigned: '#409EFF',
      in_progress: '#E6A23C',
      pending_review: '#909399',
      resolved: '#67C23A',
      closed: '#67C23A',
      cancelled: '#F56C6C',
    }
    return statusColorMap[d.name] || '#909399'
  })
  statusChart.setOption({
    tooltip: {
      trigger: 'axis',
      axisPointer: { type: 'shadow' },
    },
    grid: {
      left: '3%',
      right: '4%',
      bottom: '3%',
      top: '10%',
      containLabel: true,
    },
    xAxis: {
      type: 'category',
      data: data.map(d => d.name),
      axisLabel: {
        fontSize: 11,
        color: '#606266',
        interval: 0,
        rotate: data.length > 5 ? 20 : 0,
      },
      axisLine: { lineStyle: { color: '#dcdfe6' } },
    },
    yAxis: {
      type: 'value',
      axisLabel: { fontSize: 11, color: '#909399' },
      splitLine: { lineStyle: { color: '#f0f2f5' } },
    },
    series: [
      {
        type: 'bar',
        data: data.map((d, i) => ({
          value: d.value,
          itemStyle: { color: colors[i], borderRadius: [4, 4, 0, 0] },
        })),
        barWidth: '40%',
      },
    ],
  })
}

function initTrendChart() {
  if (!trendChartRef.value) return
  trendChart = echarts.init(trendChartRef.value)
  const data = stats.trend30d || []
  trendChart.setOption({
    tooltip: {
      trigger: 'axis',
      formatter: '{b}<br/>新增工单数: {c}',
    },
    grid: {
      left: '3%',
      right: '4%',
      bottom: '3%',
      top: '10%',
      containLabel: true,
    },
    xAxis: {
      type: 'category',
      boundaryGap: false,
      data: data.map(d => d.date.slice(5)), // MM-DD
      axisLabel: {
        fontSize: 10,
        color: '#606266',
        interval: 4,
      },
      axisLine: { lineStyle: { color: '#dcdfe6' } },
    },
    yAxis: {
      type: 'value',
      axisLabel: { fontSize: 11, color: '#909399' },
      splitLine: { lineStyle: { color: '#f0f2f5' } },
    },
    series: [
      {
        type: 'line',
        data: data.map(d => d.count),
        smooth: true,
        symbol: 'circle',
        symbolSize: 6,
        lineStyle: { color: '#409EFF', width: 2 },
        itemStyle: { color: '#409EFF' },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: 'rgba(64, 158, 255, 0.3)' },
            { offset: 1, color: 'rgba(64, 158, 255, 0.02)' },
          ]),
        },
      },
    ],
  })
}

function resizeCharts() {
  typeChart?.resize()
  statusChart?.resize()
  trendChart?.resize()
}

async function loadStats() {
  try {
    loading.value = true
    const data = await getTicketStats()
    Object.assign(stats, data)
    await nextTick()
    initTypeChart()
    initStatusChart()
    initTrendChart()
  } catch (e) {
    ElMessage.error((e as any)?.message || '加载统计数据失败')
  } finally {
    loading.value = false
  }
}

onMounted(async () => {
  await loadStats()
  window.addEventListener('resize', resizeCharts)
})

onUnmounted(() => {
  window.removeEventListener('resize', resizeCharts)
  typeChart?.dispose()
  statusChart?.dispose()
  trendChart?.dispose()
})
</script>

<style scoped>
.ticket-stats-page {
  padding: 8px 0 40px;
}

.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 16px;
}

.page-title {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}

/* 统计卡片 */
.stats-cards {
  margin-bottom: 16px;
}
.stats-cards .el-col {
  margin-bottom: 10px;
}
.stat-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 18px 20px;
  background: #fff;
  border-radius: 10px;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
  transition: transform 0.15s, box-shadow 0.15s;
}
.stat-card:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(64, 158, 255, 0.12);
}
.stat-icon {
  width: 52px;
  height: 52px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 26px;
  color: #fff;
  flex-shrink: 0;
}
.stat-total .stat-icon  { background: linear-gradient(135deg, #409EFF, #5BADFF); }
.stat-pending .stat-icon { background: linear-gradient(135deg, #E6A23C, #F0C78E); }
.stat-closed .stat-icon  { background: linear-gradient(135deg, #67C23A, #95D475); }
.stat-new .stat-icon     { background: linear-gradient(135deg, #909399, #B1B3B8); }
.stat-avg .stat-icon     { background: linear-gradient(135deg, #667eea, #764ba2); }

.stat-body .stat-label {
  font-size: 13px;
  color: #909399;
}
.stat-body .stat-value {
  font-size: 28px;
  font-weight: 600;
  line-height: 1.3;
  color: #303133;
}
.stat-body .stat-sub {
  font-size: 11px;
  color: #c0c4cc;
}

/* 图表卡片 */
.charts-row {
  margin-top: 6px;
}
.charts-row .el-col {
  margin-bottom: 12px;
}
.chart-card {
  height: 100%;
}
.chart-card :deep(.el-card__body) {
  padding: 16px 20px;
}
.chart-title {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 12px;
}
.chart-container {
  width: 100%;
  height: 280px;
}
</style>
