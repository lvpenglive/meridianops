<template>
  <div class="aiops-page">
    <!-- 概览卡片 -->
    <el-row :gutter="12" class="overview-row">
      <el-col :xs="12" :sm="8" :md="4">
        <el-card shadow="hover" class="stat-card stat-card--danger" @click="filterStatus('firing,acknowledged')">
          <div class="stat-card__body">
            <el-icon :size="22" color="#F56C6C"><Warning /></el-icon>
            <div class="stat-card__text">
              <div class="stat-value">{{ overview?.activeAlerts ?? '-' }}</div>
              <div class="stat-label">活跃告警</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-card__body">
            <el-icon :size="22" color="#409EFF"><Bell /></el-icon>
            <div class="stat-card__text">
              <div class="stat-value">{{ overview?.todayNew ?? '-' }}</div>
              <div class="stat-label">今日新增</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-card__body">
            <el-icon :size="22" color="#8B5CF6"><Reading /></el-icon>
            <div class="stat-card__text">
              <div class="stat-value">{{ overview?.knowledgeCount ?? '-' }}</div>
              <div class="stat-label">知识条目</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-card__body">
            <el-icon :size="22" color="#909399"><Tickets /></el-icon>
            <div class="stat-card__text">
              <div class="stat-value">{{ overview?.ticketCount ?? '-' }}</div>
              <div class="stat-label">工单总数</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <el-card shadow="hover" class="stat-card" :class="{ 'stat-card--warning': (anomaliesResult?.stats?.anomalyCount ?? 0) > 0 }">
          <div class="stat-card__body">
            <el-icon :size="22" :color="(anomaliesResult?.stats?.anomalyCount ?? 0) > 0 ? '#E6A23C' : '#67C23A'"><DataAnalysis /></el-icon>
            <div class="stat-card__text">
              <div class="stat-value">{{ anomaliesResult?.stats?.anomalyCount ?? '-' }}</div>
              <div class="stat-label">异常检测</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <el-tooltip :content="overview?.llmEnabled ? 'LLM 诊断已启用' : '未配置 LLM API，请在系统设置中开启'" placement="bottom">
          <el-card shadow="hover" class="stat-card" :class="{ 'stat-card--success': overview?.llmEnabled }">
            <div class="stat-card__body">
              <el-icon :size="22" :color="overview?.llmEnabled ? '#67C23A' : '#C0C4CC'"><ChatDotRound /></el-icon>
              <div class="stat-card__text">
                <div class="stat-value">{{ overview?.llmEnabled ? 'ON' : 'OFF' }}</div>
                <div class="stat-label">LLM 诊断</div>
              </div>
            </div>
          </el-card>
        </el-tooltip>
      </el-col>
    </el-row>

    <!-- 告警级别分布条 -->
    <div v-if="severityList.length" class="severity-bar">
      <span class="severity-bar__label">告警级别分布</span>
      <div class="severity-bar__track">
        <div
          v-for="s in severityList"
          :key="s.key"
          class="severity-bar__seg"
          :class="'seg-' + s.key"
          :style="{ width: s.percent + '%' }"
          :title="`${s.label}: ${s.count}`"
        />
      </div>
      <div class="severity-bar__legend">
        <span v-for="s in severityList" :key="s.key" class="legend-item">
          <i class="dot" :class="'seg-' + s.key" />
          {{ s.label }} {{ s.count }}
        </span>
      </div>
    </div>

    <el-row :gutter="16" style="margin-top:8px">
      <!-- 左侧主面板 -->
      <el-col :xs="24" :sm="24" :md="16">
        <el-card shadow="never">
          <template #header>
            <div class="page-header">
              <span>AIOps 根因分析</span>
              <div class="alert-selector">
                <el-radio-group v-model="alertFilter" size="small" @change="onFilterChange" style="margin-right:8px">
                  <el-radio-button value="all">全部</el-radio-button>
                  <el-radio-button value="firing,acknowledged">活跃</el-radio-button>
                  <el-radio-button value="resolved">已解决</el-radio-button>
                </el-radio-group>
                <el-select
                  v-model="selectedAlertId"
                  placeholder="选择告警进行诊断"
                  filterable
                  clearable
                  style="width:360px"
                  :loading="alertLoading"
                  @change="onAlertSelect"
                >
                  <el-option
                    v-for="a in filteredAlerts"
                    :key="a.id"
                    :label="`[${a.severity}] ${a.title}`"
                    :value="a.id"
                  >
                    <div class="alert-option">
                      <span class="alert-option__sev" :class="'sev-' + a.severity">{{ a.severity }}</span>
                      <span class="alert-option__status" :class="'status-' + a.status">{{ statusLabel(a.status) }}</span>
                      <span class="alert-option__title">{{ a.title }}</span>
                      <span class="alert-option__time">{{ fmtTime(a.firedAt) }}</span>
                    </div>
                  </el-option>
                </el-select>
              </div>
            </div>
          </template>

          <div v-loading="rcaLoading">
            <template v-if="rcaResult">
              <!-- 告警信息头 -->
              <div class="rca-alert-header">
                <span class="alert-option__sev" :class="'sev-' + rcaResult.rootAlert.severity">{{ rcaResult.rootAlert.severity }}</span>
                <span class="rca-alert-header__title">{{ rcaResult.rootAlert.title }}</span>
                <el-tag size="small" type="info">{{ rcaResult.rootAlert.source }}</el-tag>
                <el-tag v-if="rcaResult.rootAlert.ciName" size="small" type="warning">{{ rcaResult.rootAlert.ciName }}</el-tag>
                <span class="rca-alert-header__time">{{ fmtTime(rcaResult.rootAlert.firedAt) }}</span>
              </div>

              <!-- 根因摘要 -->
              <div class="rca-summary">
                <el-alert
                  v-if="rcaResult.rootCause"
                  type="error"
                  :closable="false"
                  show-icon
                  :title="`疑似根因：${rcaResult.rootCause.ciName}`"
                  :description="`置信度 ${Math.round(rcaResult.rootCause.confidence)}%`"
                />
                <el-alert
                  v-else
                  type="info"
                  :closable="false"
                  show-icon
                  title="暂无拓扑关联"
                  description="该告警未关联 CMDB 资产，无法进行拓扑根因分析。可参考下方知识推荐。"
                />
              </div>

              <!-- 拓扑图 + 诊断树 -->
              <el-row :gutter="16" v-if="rcaResult.topology?.nodes?.length">
                <el-col :span="12">
                  <div class="diagnosis-section">
                    <h3>诊断路径</h3>
                    <div class="diagnosis-tree">
                      <el-tree :data="rcaResult.diagnosisTree" node-key="id" default-expand-all>
                        <template #default="{ node, data }">
                          <span class="tree-node" :class="data.level">
                            <el-icon v-if="data.level === 'root'" color="#F56C6C"><Warning /></el-icon>
                            <el-icon v-else-if="data.level === 'cause'" color="#E6A23C"><InfoFilled /></el-icon>
                            <el-icon v-else color="#67C23A"><CircleCheck /></el-icon>
                            <span class="node-label">{{ node.label }}</span>
                            <el-tag v-if="data.confidence" size="small" class="confidence">{{ Math.round(data.confidence) }}%</el-tag>
                          </span>
                        </template>
                      </el-tree>
                    </div>
                  </div>
                </el-col>
                <el-col :span="12">
                  <div class="diagnosis-section">
                    <h3>关联告警 ({{ rcaResult.relatedAlerts?.length || 0 }})</h3>
                    <el-table
                      :data="rcaResult.relatedAlerts"
                      size="small"
                      max-height="300"
                      stripe
                      highlight-current-row
                      @row-click="(row: any) => switchAlert(row.id)"
                    >
                      <el-table-column prop="severity" label="级别" width="60">
                        <template #default="{ row }">
                          <span class="alert-option__sev" :class="'sev-' + row.severity">{{ row.severity }}</span>
                        </template>
                      </el-table-column>
                      <el-table-column prop="title" label="告警" show-overflow-tooltip />
                      <el-table-column prop="ciName" label="资产" width="120" show-overflow-tooltip />
                      <el-table-column prop="firedAt" label="时间" width="140">
                        <template #default="{ row }">{{ fmtTime(row.firedAt) }}</template>
                      </el-table-column>
                    </el-table>
                    <div class="table-hint">点击行可切换分析目标</div>
                  </div>
                </el-col>
              </el-row>

              <!-- 无拓扑时只显示诊断树 -->
              <div v-else class="diagnosis-section">
                <h3>诊断路径</h3>
                <div class="diagnosis-tree">
                  <el-tree :data="rcaResult.diagnosisTree" node-key="id" default-expand-all>
                    <template #default="{ node, data }">
                      <span class="tree-node" :class="data.level">
                        <el-icon v-if="data.level === 'root'" color="#F56C6C"><Warning /></el-icon>
                        <el-icon v-else-if="data.level === 'cause'" color="#E6A23C"><InfoFilled /></el-icon>
                        <el-icon v-else color="#67C23A"><CircleCheck /></el-icon>
                        <span class="node-label">{{ node.label }}</span>
                        <el-tag v-if="data.confidence" size="small" class="confidence">{{ Math.round(data.confidence) }}%</el-tag>
                      </span>
                    </template>
                  </el-tree>
                </div>
              </div>

              <el-divider />

              <!-- 处置建议 -->
              <div class="suggestion-section">
                <h3>处置建议</h3>
                <el-timeline v-if="rcaResult.suggestions?.length">
                  <el-timeline-item
                    v-for="(s, idx) in rcaResult.suggestions"
                    :key="idx"
                    :timestamp="s.time"
                    :type="s.type"
                  >
                    <div class="suggestion-title">{{ s.title }}</div>
                    <div class="suggestion-desc">{{ s.desc }}</div>
                  </el-timeline-item>
                </el-timeline>
                <el-empty v-else description="暂无处置建议" :image-size="50" />
              </div>
            </template>

            <el-empty v-else description="选择一条告警开始根因分析" :image-size="80" />
          </div>
        </el-card>
      </el-col>

      <!-- 右侧面板 -->
      <el-col :xs="24" :sm="24" :md="8">
        <!-- 异常检测 -->
        <el-card shadow="never" style="margin-bottom:12px">
          <template #header>
            <div class="page-header">
              <span>异常检测</span>
              <el-button text :icon="Refresh" @click="loadAnomalies" :loading="anomalyLoading" />
            </div>
          </template>
          <div v-loading="anomalyLoading">
            <el-alert
              v-for="(a, idx) in anomaliesResult?.anomalies || []"
              :key="idx"
              :type="a.severity === 'critical' ? 'error' : 'warning'"
              :closable="false"
              show-icon
              :title="a.title"
              :description="a.message"
              style="margin-bottom:8px"
            />
            <div v-if="!anomaliesResult?.anomalies?.length && !anomalyLoading" class="empty-mini">
              <el-icon color="#67C23A" :size="24"><CircleCheck /></el-icon>
              <span>暂无异常</span>
            </div>
          </div>
        </el-card>

        <!-- 告警趋势图 -->
        <el-card shadow="never" style="margin-bottom:12px">
          <template #header><span>告警趋势 (24h)</span></template>
          <div v-if="trendHasData" ref="chartRef" style="width:100%;height:180px" />
          <div v-else class="chart-empty">
            <el-icon :size="28" color="#C0C4CC"><TrendCharts /></el-icon>
            <span>暂无趋势数据</span>
          </div>
        </el-card>

        <!-- 知识推荐 -->
        <el-card shadow="never" style="margin-bottom:12px">
          <template #header><span>知识案例推荐</span></template>
          <div v-loading="recommendLoading">
            <div v-for="kb in recommendResult?.knowledge || []" :key="kb.id" class="kb-item" @click="go(`/knowledge/${kb.id}`)">
              <div class="kb-item__title">{{ kb.title }}</div>
              <div class="kb-item__meta">
                <el-tag size="small">{{ kb.category }}</el-tag>
                <el-tag size="small" :type="kb.score >= 80 ? 'success' : 'warning'">匹配 {{ kb.score }}%</el-tag>
              </div>
            </div>
            <div v-for="t in recommendResult?.tickets || []" :key="t.id" class="kb-item" @click="go(`/tickets/${t.id}`)">
              <div class="kb-item__title">{{ t.title }}</div>
              <div class="kb-item__meta">
                <el-tag size="small" type="info">历史工单</el-tag>
                <el-tag size="small" :type="t.score >= 80 ? 'success' : 'warning'">匹配 {{ t.score }}%</el-tag>
              </div>
            </div>
            <div v-if="!recommendResult?.knowledge?.length && !recommendResult?.tickets?.length && !recommendLoading" class="empty-mini">
              <span>暂无推荐</span>
            </div>
          </div>
        </el-card>

        <!-- LLM 诊断 -->
        <el-card v-if="overview?.llmEnabled" shadow="never">
          <template #header>
            <div class="page-header">
              <span>AI 诊断助手</span>
              <div>
                <el-button v-if="llmResult" text size="small" @click="llmResult = null">清除</el-button>
                <el-button type="primary" size="small" @click="runLlmDiagnose" :loading="llmLoading">
                  诊断
                </el-button>
              </div>
            </div>
          </template>
          <div v-loading="llmLoading">
            <div v-if="llmResult" class="llm-answer" v-html="renderMarkdown(llmResult.answer)" />
            <el-input
              v-if="!llmResult"
              v-model="llmQuestion"
              type="textarea"
              :rows="3"
              placeholder="输入问题，或选择告警后直接点诊断"
            />
            <div v-if="llmResult" class="llm-followup">
              <el-input
                v-model="llmQuestion"
                type="textarea"
                :rows="2"
                placeholder="追问..."
                @keydown.enter.prevent="runLlmDiagnose"
              />
            </div>
          </div>
        </el-card>
        <el-card v-else shadow="never" class="llm-hint-card">
          <div class="llm-hint">
            <el-icon :size="20" color="#909399"><ChatDotRound /></el-icon>
            <span>LLM 诊断未启用</span>
            <el-button text type="primary" size="small" @click="go('/system')">去配置</el-button>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue'
import {
  Warning, InfoFilled, CircleCheck, Refresh, ChatDotRound,
  Bell, Reading, Tickets, DataAnalysis, TrendCharts,
} from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import { useRouter } from 'vue-router'
import { getAiopsOverview, getAnomalies, getRca, getRecommend, llmDiagnose } from '../../api/aiops'
import { listAlertEvents, type AlertEvent } from '../../api/alert'

const router = useRouter()
const overview = ref<Awaited<ReturnType<typeof getAiopsOverview>> | null>(null)
const allAlerts = ref<AlertEvent[]>([])
const selectedAlertId = ref('')
const rcaResult = ref<Awaited<ReturnType<typeof getRca>> | null>(null)
const anomaliesResult = ref<Awaited<ReturnType<typeof getAnomalies>> | null>(null)
const recommendResult = ref<Awaited<ReturnType<typeof getRecommend>> | null>(null)
const llmResult = ref<{ answer: string; model: string } | null>(null)
const llmQuestion = ref('')

const alertFilter = ref('all')
const alertLoading = ref(false)
const rcaLoading = ref(false)
const anomalyLoading = ref(false)
const recommendLoading = ref(false)
const llmLoading = ref(false)

const chartRef = ref<HTMLElement>()
let chart: echarts.ECharts | null = null
let refreshTimer: ReturnType<typeof setInterval> | null = null

const filteredAlerts = computed(() => {
  if (alertFilter.value === 'all') return allAlerts.value
  return allAlerts.value.filter(a => alertFilter.value.split(',').includes(a.status))
})

const severityList = computed(() => {
  const raw = overview.value?.bySeverity || {}
  const labels: Record<string, string> = { '1': 'P1', '2': 'P2', '3': 'P3', '4': 'P4', '5': 'P5' }
  const entries = Object.entries(raw).map(([k, v]) => ({
    key: k,
    label: labels[k] || k,
    count: v as number,
    percent: 0,
  })).filter(s => s.count > 0).sort((a, b) => Number(b.key) - Number(a.key))
  const total = entries.reduce((s, e) => s + e.count, 0)
  entries.forEach(e => e.percent = total > 0 ? (e.count / total * 100) : 0)
  return entries
})

const trendHasData = computed(() => {
  const data = anomaliesResult.value?.trend || overview.value?.trend24h || []
  return data.length > 0
})

function go(path: string) {
  router.push(path)
}

function statusLabel(status: string): string {
  const map: Record<string, string> = {
    firing: '活跃',
    acknowledged: '已确认',
    resolved: '已解决',
    suppressed: '已抑制',
  }
  return map[status] || status
}

function fmtTime(t: string): string {
  if (!t) return '-'
  return t.replace('T', ' ').substring(0, 16)
}

function renderMarkdown(text: string): string {
  let html = text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
  html = html
    .replace(/^### (.+)$/gm, '<h3>$1</h3>')
    .replace(/^## (.+)$/gm, '<h2>$1</h2>')
    .replace(/^# (.+)$/gm, '<h1>$1</h1>')
    .replace(/^\- (.+)$/gm, '<li>$1</li>')
    .replace(/(<li>.*<\/li>\n?)+/g, '<ul>$&</ul>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/`(.+?)`/g, '<code>$1</code>')
    .replace(/\[(.+?)\]\((.+?)\)/g, '<a href="$2" target="_blank">$1</a>')
    .replace(/\n/g, '<br>')
  return html
}

function filterStatus(status: string) {
  alertFilter.value = status
}

async function loadOverview() {
  try {
    overview.value = await getAiopsOverview()
  } catch { /* ignore */ }
}

async function loadAlerts() {
  alertLoading.value = true
  try {
    const res = await listAlertEvents({ pageSize: 100 })
    allAlerts.value = res.items || []
    if (allAlerts.value.length && !selectedAlertId.value) {
      const first = allAlerts.value.find(a => a.status === 'firing' || a.status === 'acknowledged') || allAlerts.value[0]
      selectedAlertId.value = first.id
      await onAlertSelect(first.id)
    }
  } catch { /* ignore */ }
  alertLoading.value = false
}

function onFilterChange() {
  if (selectedAlertId.value && !filteredAlerts.value.find(a => a.id === selectedAlertId.value)) {
    selectedAlertId.value = ''
    rcaResult.value = null
    recommendResult.value = null
  }
}

async function loadAnomalies() {
  anomalyLoading.value = true
  try {
    anomaliesResult.value = await getAnomalies()
    await nextTick()
    renderChart()
  } catch { /* ignore */ }
  anomalyLoading.value = false
}

async function onAlertSelect(alertId: string) {
  if (!alertId) {
    rcaResult.value = null
    recommendResult.value = null
    return
  }
  rcaLoading.value = true
  recommendLoading.value = true
  try {
    const [rca, rec] = await Promise.all([
      getRca(alertId),
      getRecommend({ alertId }),
    ])
    rcaResult.value = rca
    recommendResult.value = rec
  } catch { /* ignore */ }
  rcaLoading.value = false
  recommendLoading.value = false
}

async function switchAlert(alertId: string) {
  if (!alertId || alertId === selectedAlertId.value) return
  selectedAlertId.value = alertId
  await onAlertSelect(alertId)
}

async function runLlmDiagnose() {
  llmLoading.value = true
  try {
    const res = await llmDiagnose({
      alertId: selectedAlertId.value || undefined,
      question: llmQuestion.value || undefined,
    })
    llmResult.value = res
    llmQuestion.value = ''
  } catch { /* ignore */ }
  llmLoading.value = false
}

function renderChart() {
  if (!chartRef.value) return
  if (!chart) chart = echarts.init(chartRef.value)
  const data = anomaliesResult.value?.trend || overview.value?.trend24h || []
  chart.setOption({
    tooltip: { trigger: 'axis' },
    grid: { left: 35, right: 10, top: 15, bottom: 25 },
    xAxis: {
      type: 'category',
      data: data.map((d: any) => d.hour),
      axisLabel: { fontSize: 10, rotate: 30 },
    },
    yAxis: { type: 'value', minInterval: 1 },
    series: [
      {
        type: 'bar',
        data: data.map((d: any) => d.count),
        itemStyle: { color: '#409EFF', borderRadius: [4, 4, 0, 0] },
        barWidth: '60%',
      },
      {
        type: 'line',
        data: data.map((d: any) => d.count),
        smooth: true,
        symbol: 'circle',
        symbolSize: 6,
        lineStyle: { color: '#E6A23C', width: 2 },
        itemStyle: { color: '#E6A23C' },
      },
    ],
  })
}

function onResize() {
  chart?.resize()
}

watch(() => overview.value?.trend24h, () => {
  nextTick(() => renderChart())
}, { deep: true })

watch(trendHasData, (v) => {
  if (v) nextTick(() => renderChart())
})

onMounted(async () => {
  await Promise.all([loadOverview(), loadAlerts(), loadAnomalies()])
  nextTick(() => renderChart())
  refreshTimer = setInterval(() => {
    loadOverview()
    loadAnomalies()
  }, 30000)
  window.addEventListener('resize', onResize)
})

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer)
  chart?.dispose()
  window.removeEventListener('resize', onResize)
})
</script>

<style scoped>
.overview-row {
  margin-bottom: 4px;
}

.stat-card {
  cursor: pointer;
  transition: transform 0.15s;
}

.stat-card:hover {
  transform: translateY(-2px);
}

.stat-card__body {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 0;
}

.stat-card__text {
  text-align: left;
}

.stat-card .stat-value {
  font-size: 26px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}

.stat-card .stat-label {
  font-size: 12px;
  color: #909399;
  margin-top: 2px;
}

.stat-card--danger .stat-value {
  color: #F56C6C;
}

.stat-card--success .stat-value {
  color: #67C23A;
}

.stat-card--warning .stat-value {
  color: #E6A23C;
}

/* 告警级别分布条 */
.severity-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 6px 12px;
  background: #fafafa;
  border-radius: 4px;
  margin-bottom: 4px;
  flex-wrap: wrap;
}

.severity-bar__label {
  font-size: 12px;
  color: #909399;
  white-space: nowrap;
}

.severity-bar__track {
  flex: 1;
  min-width: 120px;
  height: 10px;
  border-radius: 5px;
  overflow: hidden;
  display: flex;
  background: #f0f0f0;
}

.severity-bar__seg {
  transition: width 0.3s;
}

.severity-bar__legend {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.legend-item {
  font-size: 12px;
  color: #606266;
  display: flex;
  align-items: center;
  gap: 4px;
}

.legend-item .dot {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 50%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}

.alert-selector {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 4px;
}

.alert-option {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
}

.alert-option__sev {
  display: inline-block;
  width: 18px;
  height: 18px;
  line-height: 18px;
  text-align: center;
  border-radius: 50%;
  font-size: 11px;
  font-weight: 700;
  color: #fff;
  flex-shrink: 0;
}

.sev-1 { background: #909399; }
.sev-2 { background: #67C23A; }
.sev-3 { background: #E6A23C; }
.sev-4 { background: #F56C6C; }
.sev-5 { background: #c0392b; }

.alert-option__status {
  font-size: 11px;
  padding: 1px 6px;
  border-radius: 3px;
  flex-shrink: 0;
}

.status-firing { background: #fef0f0; color: #F56C6C; }
.status-acknowledged { background: #fdf6ec; color: #E6A23C; }
.status-resolved { background: #f0f9eb; color: #67C23A; }
.status-suppressed { background: #f4f4f5; color: #909399; }

.alert-option__title {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 13px;
}

.alert-option__time {
  font-size: 11px;
  color: #C0C4CC;
  white-space: nowrap;
  flex-shrink: 0;
}

/* RCA 告警信息头 */
.rca-alert-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.rca-alert-header__title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
}

.rca-alert-header__time {
  font-size: 12px;
  color: #909399;
  margin-left: auto;
}

.rca-summary {
  margin-bottom: 16px;
}

.diagnosis-section h3,
.suggestion-section h3 {
  margin: 0 0 12px 0;
  font-size: 15px;
}

.diagnosis-tree {
  margin-top: 8px;
}

.tree-node {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
}

.tree-node.root {
  color: #F56C6C;
  font-weight: 600;
}

.tree-node.cause {
  color: #E6A23C;
}

.confidence {
  margin-left: auto;
}

.table-hint {
  font-size: 11px;
  color: #C0C4CC;
  text-align: right;
  margin-top: 4px;
}

.suggestion-title {
  font-weight: 600;
  font-size: 14px;
}

.suggestion-desc {
  color: #606266;
  font-size: 13px;
  margin-top: 4px;
}

.kb-item {
  padding: 8px 0;
  border-bottom: 1px solid #f0f0f0;
  cursor: pointer;
}

.kb-item:hover .kb-item__title {
  color: #409EFF;
}

.kb-item__title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 4px;
}

.kb-item__meta {
  display: flex;
  gap: 8px;
}

.empty-mini {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 20px 0;
  color: #909399;
  font-size: 13px;
}

.chart-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  height: 180px;
  color: #C0C4CC;
  font-size: 13px;
}

.llm-hint-card {
  text-align: center;
}

.llm-hint {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 16px 0;
}

.llm-hint span {
  color: #909399;
  font-size: 13px;
}

.llm-answer {
  font-size: 14px;
  line-height: 1.6;
  max-height: 400px;
  overflow-y: auto;
  padding: 4px 0;
}

.llm-answer :deep(h1),
.llm-answer :deep(h2),
.llm-answer :deep(h3) {
  margin: 8px 0 4px;
}

.llm-answer :deep(code) {
  background: #f5f5f5;
  padding: 2px 4px;
  border-radius: 3px;
  font-size: 13px;
}

.llm-answer :deep(ul) {
  margin: 4px 0 8px 20px;
  padding: 0;
}

.llm-answer :deep(a) {
  color: #409EFF;
  text-decoration: none;
}

.llm-followup {
  margin-top: 8px;
}
</style>
