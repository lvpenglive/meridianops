<template>
  <div class="report-page">
    <div class="report-header">
      <div class="header-left">
        <el-button text @click="router.push('/report')">
          <el-icon><ArrowLeft /></el-icon> 报表中心
        </el-button>
        <h2>知识库分类统计</h2>
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
          <div class="summary-icon"><el-icon><Document /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">文章总数</div>
            <div class="summary-value">{{ totalArticles }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8">
        <div class="summary-card success">
          <div class="summary-icon"><el-icon><View /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">总查看数</div>
            <div class="summary-value">{{ totalViews }}</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8">
        <div class="summary-card warning">
          <div class="summary-icon"><el-icon><Star /></el-icon></div>
          <div class="summary-body">
            <div class="summary-label">总有用量</div>
            <div class="summary-value">{{ totalHelpful }}</div>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- 分类饼图 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header"><span>知识库分类分布</span></div>
      </template>
      <div ref="pieChartRef" class="chart" v-loading="loading" />
    </el-card>

    <!-- 分类明细表格 -->
    <el-card shadow="never" class="chart-card">
      <template #header>
        <div class="card-header"><span>分类明细（{{ categories.length }}）</span></div>
      </template>
      <el-table :data="categories" v-loading="loading" size="default" stripe>
        <el-table-column type="index" label="#" width="50" />
        <el-table-column prop="category" label="分类" min-width="160" />
        <el-table-column prop="count" label="文章数" width="100" sortable>
          <template #default="{ row }">
            <el-tag type="primary" size="small">{{ row.count }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="totalViews" label="总查看数" width="120" sortable />
        <el-table-column prop="totalHelpful" label="总有用" width="120" sortable>
          <template #default="{ row }">
            <el-tag type="warning" size="small">{{ row.totalHelpful }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="占比" width="100">
          <template #default="{ row }">{{ percent(row.count) }}%</template>
        </el-table-column>
      </el-table>
      <el-empty v-if="!loading && categories.length === 0" description="暂无数据" />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick, computed } from 'vue'
import { useRouter } from 'vue-router'
import { ArrowLeft, Refresh, Document, View, Star } from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import { useUserStore } from '../../stores/user'

const router = useRouter()

interface CategoryItem { category: string; count: number; totalViews: number; totalHelpful: number }
const categories = ref<CategoryItem[]>([])
const loading = ref(false)

const pieChartRef = ref<HTMLElement>()
let pieChart: echarts.ECharts | null = null

const totalArticles = computed(() => categories.value.reduce((s, i) => s + i.count, 0))
const totalViews = computed(() => categories.value.reduce((s, i) => s + i.totalViews, 0))
const totalHelpful = computed(() => categories.value.reduce((s, i) => s + i.totalHelpful, 0))

function percent(count: number) {
  if (totalArticles.value === 0) return '0.0'
  return ((count / totalArticles.value) * 100).toFixed(1)
}

async function loadAll() {
  loading.value = true
  try {
    const token = useUserStore().token
    const res = await fetch('/api/reports/knowledge-category', { headers: { Authorization: `Bearer ${token}` } })
    const json = await res.json()
    if (json.code === 0) categories.value = json.data || []
    await nextTick()
    renderPieChart()
  } finally {
    loading.value = false
  }
}

function renderPieChart() {
  if (!pieChartRef.value) return
  if (!pieChart) pieChart = echarts.init(pieChartRef.value)
  pieChart.setOption({
    tooltip: { trigger: 'item', formatter: '{b}: {c} 篇 ({d}%)' },
    legend: { type: 'scroll', orient: 'vertical', right: 10, top: 20, bottom: 20 },
    series: [{
      name: '知识分类', type: 'pie', radius: ['40%', '70%'], center: ['40%', '50%'],
      itemStyle: { borderRadius: 6, borderColor: '#fff', borderWidth: 2 },
      label: { show: false },
      emphasis: { label: { show: true, fontSize: 14, fontWeight: 'bold' } },
      data: categories.value.map(i => ({ name: i.category, value: i.count })),
    }],
  })
  pieChart.resize()
}

function handleResize() {
  pieChart?.resize()
}

onMounted(() => {
  loadAll()
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  pieChart?.dispose()
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
