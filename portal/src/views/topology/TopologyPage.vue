<template>
  <div class="topology-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><Share /></el-icon>
        <span>拓扑视图</span>
        <span class="page-sub">基于 CI 关系的资产依赖图：业务系统 → 主机 → 中间件 → 数据库</span>
      </div>
      <div class="header-actions">
        <el-select
          v-model="modelFilter"
          placeholder="全部模型"
          clearable
          style="width: 160px"
          @change="fetchData"
        >
          <el-option v-for="m in models" :key="m.id" :label="m.name" :value="m.id" />
        </el-select>
        <el-select
          v-model="statusFilter"
          placeholder="全部状态"
          clearable
          style="width: 140px"
          @change="fetchData"
        >
          <el-option label="运行中" value="running" />
          <el-option label="已停止" value="stopped" />
          <el-option label="维护中" value="maintenance" />
          <el-option label="未知" value="unknown" />
        </el-select>
        <el-button :icon="Refresh" @click="fetchData">刷新</el-button>
      </div>
    </div>

    <el-card shadow="never">
      <div class="stats-bar">
        <div class="stat-item">
          <span class="stat-label">节点数</span>
          <span class="stat-val">{{ topoData?.nodeCount ?? 0 }}</span>
        </div>
        <div class="stat-item">
          <span class="stat-label">关系数</span>
          <span class="stat-val">{{ topoData?.linkCount ?? 0 }}</span>
        </div>
        <el-divider direction="vertical" />
        <div class="legend">
          <span v-for="cat in categories" :key="cat.name" class="legend-item">
            <span class="legend-dot" :style="{ background: cat.color }"></span>
            {{ cat.name }}
          </span>
        </div>
        <div class="spacer"></div>
        <el-tooltip content="节点拖拽、滚轮缩放；点击节点跳转资产详情" placement="left">
          <el-icon class="help-icon"><QuestionFilled /></el-icon>
        </el-tooltip>
      </div>

      <div v-loading="loading" class="chart-wrap">
        <div ref="chartRef" class="chart" :style="{ height: chartHeight + 'px' }"></div>
        <el-empty
          v-if="!loading && (topoData?.nodeCount ?? 0) === 0"
          description="暂无资产或关系数据，请先在资产清单中创建资产并建立关系"
          class="empty-overlay"
        />
      </div>
    </el-card>

    <!-- 关系类型说明 -->
    <el-card shadow="never" class="relation-types-card">
      <template #header>
        <div class="card-header-inner">
          <span>关系类型说明</span>
          <span class="rt-count">共 {{ relationTypes.length }} 种</span>
        </div>
      </template>
      <div class="relation-types">
        <div v-for="rt in relationTypes" :key="rt.code" class="rt-item">
          <el-tag size="small">{{ rt.name }}</el-tag>
          <span class="rt-code">{{ rt.code }}</span>
          <el-tag v-if="rt.directional" size="small" type="primary" effect="plain">有向</el-tag>
          <el-tag v-else size="small" type="info" effect="plain">无向</el-tag>
          <el-tag v-if="!rt.enabled" size="small" type="danger" effect="plain">已禁用</el-tag>
          <span v-if="rt.description" class="rt-desc">{{ rt.description }}</span>
        </div>
        <el-empty v-if="relationTypes.length === 0" description="暂无关系类型" :image-size="60" />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import * as echarts from 'echarts'
import { Share, Refresh, QuestionFilled } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { getTopology, listCiModels, listCiRelationTypes } from '../../api/cmdb'
import type { TopologyData, CiModel, CiRelationType } from '../../api/types'

const router = useRouter()

const loading = ref(false)
const topoData = ref<TopologyData | null>(null)
const models = ref<CiModel[]>([])
const modelFilter = ref('')
const statusFilter = ref('')
const chartRef = ref<HTMLElement>()
let chart: echarts.ECharts | null = null
const chartHeight = ref(560)

// 模型分类（颜色 + 名称），按 modelCode 映射
const modelColorMap: Record<string, string> = {
  business_system: '#f5576c',
  host: '#4facfe',
  middleware: '#43e97b',
  database: '#fa709a',
  network_device: '#a8edea',
}
const defaultColor = '#764ba2'

const categories = ref<{ name: string; color: string }[]>([])

// 关系类型字典（动态加载）
const relationTypes = ref<CiRelationType[]>([])
// code → 中文名映射（供边 label、tooltip 查表）
const relTypeNameMap = ref<Record<string, string>>({})

async function fetchRelationTypes() {
  try {
    relationTypes.value = await listCiRelationTypes()
    const map: Record<string, string> = {}
    relationTypes.value.forEach(t => { map[t.code] = t.name })
    relTypeNameMap.value = map
  } catch {
    // 静默失败：UI 兜底显示原始 relationType code
  }
}

/** 通过 code 查询关系类型中文名，找不到则返回 code 本身 */
function relTypeName(code: string): string {
  return relTypeNameMap.value[code] || code
}

async function fetchModels() {
  try {
    models.value = await listCiModels()
  } catch { /* ignore */ }
}

async function fetchData() {
  loading.value = true
  try {
    topoData.value = await getTopology({
      modelId: modelFilter.value || undefined,
      status: statusFilter.value || undefined,
    })
    renderChart()
  } catch (e: any) {
    ElMessage.error(e?.message || '加载拓扑失败')
  } finally {
    loading.value = false
  }
}

function buildCategories() {
  // 收集当前数据中出现的 modelCode
  const codes = new Set<string>()
  topoData.value?.nodes.forEach(n => codes.add(n.modelCode))
  // 用模型的 name 作为分类显示名
  const list: { name: string; color: string }[] = []
  codes.forEach(code => {
    const model = models.value.find(m => m.code === code)
    list.push({
      name: model?.name || code,
      color: modelColorMap[code] || defaultColor,
    })
  })
  categories.value = list
  return list
}

function renderChart() {
  if (!chartRef.value) return
  if (!chart) {
    chart = echarts.init(chartRef.value)
    chart.on('click', (params: any) => {
      if (params.dataType === 'node' && params.data?.id) {
        router.push(`/assets/${params.data.id}`)
      }
    })
  }

  const cats = buildCategories()
  const catNameToIndex = new Map(cats.map((c, i) => [c.name, i]))

  const nodes = (topoData.value?.nodes || []).map(n => {
    const model = models.value.find(m => m.code === n.modelCode)
    const catName = model?.name || n.modelCode
    return {
      id: n.id,
      name: n.name,
      category: catNameToIndex.get(catName) ?? 0,
      symbolSize: 30,
      itemStyle: { color: modelColorMap[n.modelCode] || defaultColor },
      label: { show: true, position: 'bottom', fontSize: 11 },
      data: n,
    }
  })

  const links = (topoData.value?.links || []).map(l => {
    const typeName = relTypeName(l.relationType)
    return {
      source: l.sourceId,
      target: l.targetId,
      value: typeName,
      rawRelationType: l.relationType,
      label: {
        show: true,
        formatter: typeName,
        fontSize: 10,
        color: '#909399',
      },
      lineStyle: { curveness: 0.2 },
    }
  })

  // 根据连接数调整节点大小
  const degreeMap: Record<string, number> = {}
  links.forEach(l => {
    degreeMap[l.source] = (degreeMap[l.source] || 0) + 1
    degreeMap[l.target] = (degreeMap[l.target] || 0) + 1
  })
  nodes.forEach(n => {
    const d = degreeMap[n.id] || 0
    n.symbolSize = 28 + Math.min(d * 4, 24)
  })

  chart.setOption({
    tooltip: {
      formatter: (p: any) => {
        if (p.dataType === 'node') {
          const d = p.data.data
          return `<b>${d.name}</b><br/>类型：${d.modelName}<br/>状态：${d.status}<br/>${d.source ? '来源：' + d.source : ''}<br/><br/><i>点击查看详情</i>`
        }
        if (p.dataType === 'edge') {
          const srcName = topoData.value?.nodes.find(n => n.id === p.data.source)?.name || p.data.source
          const tgtName = topoData.value?.nodes.find(n => n.id === p.data.target)?.name || p.data.target
          return `${srcName} <b>${p.data.value}</b> ${tgtName}`
        }
        return p.name
      },
    },
    legend: {
      data: cats.map(c => c.name),
      top: 10,
      textStyle: { fontSize: 12 },
    },
    series: [{
      type: 'graph',
      layout: 'force',
      roam: true,
      draggable: true,
      label: { show: true },
      edgeLabel: { show: true },
      force: {
        repulsion: 400,
        edgeLength: [120, 220],
        gravity: 0.08,
      },
      emphasis: {
        focus: 'adjacency',
        lineStyle: { width: 3 },
        label: { fontSize: 13 },
      },
      categories: cats.map(c => ({ name: c.name })),
      data: nodes,
      links,
      lineStyle: { color: '#c0c4cc', width: 1.5, opacity: 0.7 },
    }],
  }, true)
}

function handleResize() {
  chart?.resize()
}

onMounted(async () => {
  await fetchModels()
  await fetchRelationTypes()
  await fetchData()
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize)
  chart?.dispose()
  chart = null
})
</script>

<style scoped>
.topology-page { padding: 0; }

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.page-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}
.page-sub {
  font-size: 12px;
  font-weight: normal;
  color: #909399;
  margin-left: 8px;
}
.header-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.stats-bar {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 12px;
  padding: 8px 12px;
  background: #f5f7fa;
  border-radius: 6px;
}
.stat-item {
  display: flex;
  flex-direction: column;
}
.stat-label { font-size: 12px; color: #909399; }
.stat-val { font-size: 20px; font-weight: 700; color: #303133; }
.legend {
  display: flex;
  gap: 14px;
  flex-wrap: wrap;
}
.legend-item {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: #606266;
}
.legend-dot {
  width: 10px; height: 10px;
  border-radius: 50%;
  display: inline-block;
}
.spacer { flex: 1; }
.help-icon { color: #909399; cursor: help; }

.chart-wrap {
  position: relative;
}
.chart { width: 100%; }
.empty-overlay {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
}

.relation-types-card { margin-top: 16px; }
.card-header-inner {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.rt-count { font-size: 12px; color: #909399; }
.relation-types {
  display: flex;
  flex-wrap: wrap;
  gap: 20px;
}
.rt-item {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 260px;
  padding: 6px 10px;
  border-radius: 6px;
  background: #fafafa;
  border: 1px solid #ebeef5;
}
.rt-code {
  font-size: 12px;
  font-family: 'Courier New', Courier, monospace;
  color: #409eff;
}
.rt-desc { font-size: 13px; color: #606266; }
</style>
