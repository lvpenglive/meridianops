<template>
  <div class="report-page">
    <div class="report-header">
      <div class="header-left">
        <el-button text @click="router.push('/report')">
          <el-icon><ArrowLeft /></el-icon> 报表中心
        </el-button>
        <h2>资产分类统计</h2>
        <el-tag type="info" effect="dark" size="small">资产/CMDB</el-tag>
      </div>
      <div class="header-right">
        <el-button :icon="Refresh" circle @click="loadAll" />
      </div>
    </div>

    <!-- 摘要卡片 -->
    <el-row :gutter="16" class="summary-row">
      <el-col :xs="12" :sm="8">
        <div class="summary-card info">
          <div class="summary-icon"><el-icon><Box /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">模型总数</div>
            <div class="summary-value">{{ assetCategory.length }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8">
        <div class="summary-card success">
          <div class="summary-icon"><el-icon><Files /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">资产总数</div>
            <div class="summary-value">{{ totalAssets }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8">
        <div class="summary-card warning">
          <div class="summary-icon"><el-icon><Star /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">最大模型</div>
            <div class="summary-value text-value">{{ topModel?.name || '—' }}</div>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 模型分布饼图 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header"><span>资产模型分布</span></div>
      </template>
      <div ref="pieChartRef" class="chart" v-loading="loading.category" />
    </el-card>

    <el-row :gutter="16">
      <!-- 资产状态 -->
      <el-col :xs="24" :lg="12">
        <el-card shadow="never" class="chart-card">
          <template #header>
            <div class="card-header"><span>资产状态分布</span></div>
          </template>
          <div ref="statusChartRef" class="chart" v-loading="loading.status" />
        </el-card>
      </el-col>
      <!-- 模型明细 -->
      <el-col :xs="24" :lg="12">
        <el-card shadow="never" class="chart-card">
          <template #header>
            <div class="card-header"><span>模型明细（{{ assetCategory.length }}）</span></div>
          </template>
          <el-table :data="assetCategory" v-loading="loading.category" size="small" stripe height="320">
            <el-table-column type="index" label="#" width="50" />
            <el-table-column prop="name" label="模型名称" min-width="140" />
            <el-table-column prop="count" label="资产数量" width="100" sortable>
              <template #default="{ row }">
                <el-tag type="primary" size="small">{{ row.count }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="占比" width="100">
              <template #default="{ row }">{{ percent(row.count) }}%</template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onBeforeUnmount, nextTick, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowLeft, Refresh, Box, Files, Star } from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import { useUserStore } from '../../stores/user'

const router = useRouter()

interface AssetCategoryItem { code: string; name: string; count: number; icon?: string }
interface AssetStatusItem { status: string; count: number }

const assetCategory = ref<AssetCategoryItem[]>([])
const assetStatus = ref<AssetStatusItem[]>([])

const loading = reactive({ category: false, status: false })

const pieChartRef = ref<HTMLElement>()
const statusChartRef = ref<HTMLElement>()
let pieChart: echarts.ECharts | null = null
let statusChart: echarts.ECharts | null = null

const totalAssets = computed(() => assetCategory.value.reduce((s, i) => s + i.count, 0))
const topModel = computed(() => {
  if (assetCategory.value.length === 0) return undefined
  return [...assetCategory.value].sort((a, b) => b.count - a.count)[0]
})

function percent(count: number) {
  if (totalAssets.value === 0) return '0.0'
  return ((count / totalAssets.value) * 100).toFixed(1)
}

async function loadCategory() {
  loading.category = true
  try {
    const token = useUserStore().token
    const res = await fetch('/api/reports/asset-category', { headers: { Authorization: `Bearer ${token}` } })
    const json = await res.json()
    if (json.code === 0) assetCategory.value = json.data || []
    await nextTick()
    renderPieChart()
  } finally {
    loading.category = false
  }
}

async function loadStatus() {
  loading.status = true
  try {
    const token = useUserStore().token
    const res = await fetch('/api/reports/asset-status', { headers: { Authorization: `Bearer ${token}` } })
    const json = await res.json()
    if (json.code === 0) assetStatus.value = json.data || []
    await nextTick()
    renderStatusChart()
  } finally {
    loading.status = false
  }
}

function renderPieChart() {
  if (!pieChartRef.value) return
  if (!pieChart) pieChart = echarts.init(pieChartRef.value)
  pieChart.setOption({
    tooltip: { trigger: 'item', formatter: '{b}: {c} ({d}%)' },
    legend: { type: 'scroll', orient: 'vertical', right: 10, top: 20, bottom: 20 },
    series: [{
      name: '资产模型', type: 'pie', radius: ['40%', '70%'], center: ['40%', '50%'],
      avoidLabelOverlap: true,
      itemStyle: { borderRadius: 6, borderColor: '#fff', borderWidth: 2 },
      label: { show: false },
      emphasis: { label: { show: true, fontSize: 14, fontWeight: 'bold' } },
      data: assetCategory.value.map(i => ({ name: i.name, value: i.count })),
    }],
  })
  pieChart.resize()
}

function renderStatusChart() {
  if (!statusChartRef.value) return
  if (!statusChart) statusChart = echarts.init(statusChartRef.value)
  statusChart.setOption({
    tooltip: { trigger: 'item', formatter: '{b}: {c} ({d}%)' },
    legend: { bottom: 0, type: 'scroll' },
    series: [{
      name: '资产状态', type: 'pie', radius: '55%',
      label: { show: true, formatter: '{b}: {c}' },
      data: assetStatus.value.map(i => ({ name: i.status, value: i.count })),
    }],
  })
  statusChart.resize()
}

async function loadAll() {
  await Promise.all([loadCategory(), loadStatus()])
}

function handleResize() {
  pieChart?.resize()
  statusChart?.resize()
}

onMounted(() => {
  loadAll()
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  pieChart?.dispose()
  statusChart?.dispose()
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

.chart-card { margin-bottom: 16px; }
.chart { width: 100%; height: 320px; }
.card-header { display: flex; justify-content: space-between; align-items: center; font-weight: 600; }

@media (max-width: 768px) {
  .header-left h2 { font-size: 16px; }
  .chart { height: 260px; }
}
</style>
