<template>
  <div class="monitor-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><Monitor /></el-icon>
        <span>监控纳管</span>
        <span class="page-sub">指标来自 Zabbix；启停升级走维护期 + 作业中心</span>
      </div>
      <div class="header-actions">
        <el-button :loading="govLoading" @click="loadGovernance">刷新治理</el-button>
        <el-button :loading="syncing" @click="doSync">同步对照</el-button>
        <el-button :icon="Refresh" @click="loadHosts">刷新</el-button>
      </div>
    </div>

    <el-row v-if="governance" :gutter="16" class="gov-row">
      <el-col :span="8">
        <el-card shadow="never">
          <template #header>采集队列</template>
          <div v-if="governance.queue.available" class="gov-stat">
            <span class="gov-num" :class="{ danger: governance.queue.backedUp }">{{ governance.queue.count }}</span>
            <span class="gov-hint">阈值 {{ governance.queue.warn }}{{ governance.queue.backedUp ? ' · 积压' : '' }}</span>
          </div>
          <el-empty v-else :description="governance.queue.message || '队列接口不可用'" :image-size="48" />
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="never">
          <template #header>Proxy 离线</template>
          <div class="gov-stat">
            <span class="gov-num" :class="{ danger: governance.offlineProxyCount > 0 }">{{ governance.offlineProxyCount }}</span>
            <span class="gov-hint">共 {{ governance.proxies.length }} 个 Proxy</span>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="never">
          <template #header>模板偏离</template>
          <div class="gov-stat">
            <span class="gov-num" :class="{ danger: governance.driftCount > 0 }">{{ governance.driftCount }}</span>
            <span class="gov-hint">
              {{ governance.standardTemplates.length ? '相对标准模板' : '未配置标准模板，仅供查看' }}
            </span>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row v-if="governance" :gutter="16" class="gov-row">
      <el-col :span="12">
        <el-card shadow="never">
          <template #header>Proxy 状态</template>
          <el-table :data="governance.proxies" size="small" max-height="220">
            <el-table-column prop="name" label="名称" min-width="140" />
            <el-table-column label="状态" width="90">
              <template #default="{ row }">
                <el-tag :type="row.online ? 'success' : 'danger'" size="small">{{ row.online ? '在线' : '离线' }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="未上报(秒)" width="110">
              <template #default="{ row }">{{ row.ageSecs ?? '-' }}</template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="never">
          <template #header>模板偏离主机</template>
          <el-table :data="governance.templateDrift" size="small" max-height="220">
            <el-table-column prop="hostname" label="主机" min-width="120" />
            <el-table-column label="缺少" min-width="160">
              <template #default="{ row }">{{ (row.missing || []).join(', ') || '-' }}</template>
            </el-table-column>
            <el-table-column label="已挂模板" min-width="160">
              <template #default="{ row }">{{ (row.templates || []).join(', ') || '-' }}</template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="never">
      <div class="filter-bar">
        <el-select v-model="instanceCode" placeholder="Zabbix 实例" style="width: 220px" @change="onInstanceChange">
          <el-option v-for="item in instances" :key="item.code" :label="item.name" :value="item.code" />
        </el-select>
        <el-input
          v-model="search"
          placeholder="主机名"
          clearable
          style="width: 220px"
          @keyup.enter="loadHosts"
          @clear="loadHosts"
        />
        <el-select v-model="hostFilter" placeholder="筛选" clearable style="width: 140px">
          <el-option label="Agent 离线" value="offline" />
          <el-option label="未关联 CMDB" value="unlinked" />
          <el-option label="已关联 CMDB" value="linked" />
        </el-select>
        <el-button type="primary" @click="loadHosts">查询</el-button>
        <el-button v-permission="'monitor:operate'" :disabled="!selectedIds.length" @click="runAction('restart')">
          重启 Agent
        </el-button>
        <el-button v-permission="'monitor:operate'" :disabled="!selectedIds.length" @click="runAction('stop')">
          停止
        </el-button>
        <el-button v-permission="'monitor:operate'" :disabled="!selectedIds.length" @click="runAction('start')">
          启动
        </el-button>
        <el-button v-permission="'monitor:agent'" type="warning" :disabled="!selectedIds.length" @click="openUpgrade">
          升级
        </el-button>
      </div>

      <el-empty v-if="!instanceCode && !loading" description="未配置可用的 Zabbix 实例，请先启动 zabbix-ctl" />
      <el-table
        v-else
        v-loading="loading"
        :data="filteredHosts"
        highlight-current-row
        @selection-change="onSelection"
        @current-change="onSelect"
      >
        <el-table-column type="selection" width="48" />
        <el-table-column prop="displayName" label="名称" min-width="160" />
        <el-table-column prop="hostname" label="主机名" min-width="140" />
        <el-table-column prop="ip" label="IP" width="150" />
        <el-table-column label="Agent" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.agentAvailable === true" type="success" size="small">通</el-tag>
            <el-tag v-else-if="row.agentAvailable === false" type="danger" size="small">断</el-tag>
            <el-tag v-else type="info" size="small">未知</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="agentVersion" label="版本" width="120" />
        <el-table-column label="CMDB" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.cmdbLinked" type="success" size="small">已关联</el-tag>
            <el-tag v-else type="info" size="small">未关联</el-tag>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-row v-if="selected" :gutter="16" class="detail">
      <el-col :span="14">
        <el-card shadow="never">
          <template #header>{{ selected.displayName || selected.hostname }} · 近 24 小时</template>
          <div ref="chartRef" class="chart"></div>
        </el-card>
      </el-col>
      <el-col :span="10">
        <el-card shadow="never">
          <template #header>当前问题</template>
          <el-table v-loading="problemLoading" :data="problems" size="small">
            <el-table-column prop="name" label="问题" min-width="160" />
            <el-table-column label="级别" width="90">
              <template #default="{ row }">{{ severityLabel(row.severity) }}</template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
    </el-row>

    <el-card v-if="task" shadow="never" class="detail">
      <template #header>
        任务 {{ task.id.slice(0, 8) }} · {{ task.action }} · {{ task.status }}
      </template>
      <el-table :data="task.hosts" size="small">
        <el-table-column prop="hostId" label="Host ID" width="120" />
        <el-table-column prop="status" label="状态" width="110" />
        <el-table-column prop="versionBefore" label="升级前" width="120" />
        <el-table-column prop="versionAfter" label="升级后" width="120" />
        <el-table-column prop="error" label="说明" min-width="200" />
      </el-table>
    </el-card>

    <el-dialog v-model="upgradeVisible" title="升级 Agent" width="420px">
      <el-form label-width="90px">
        <el-form-item label="目标版本">
          <el-input v-model="targetVersion" placeholder="例如 6.0.31" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="upgradeVisible = false">取消</el-button>
        <el-button type="primary" :loading="taskSubmitting" @click="confirmUpgrade">提交</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { Monitor, Refresh } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import * as echarts from 'echarts'
import {
  createTask,
  getGovernance,
  getTask,
  hostMetrics,
  hostProblems,
  listHosts,
  listInstances,
  syncHosts,
  type GovernanceData,
  type MetricSeries,
  type MonitorAction,
  type MonitorHost,
  type MonitorInstance,
  type MonitorProblem,
  type MonitorTask,
} from '../../api/monitor'

const instances = ref<MonitorInstance[]>([])
const instanceCode = ref('')
const search = ref('')
const hostFilter = ref('')
const hosts = ref<MonitorHost[]>([])
const filteredHosts = computed(() => {
  if (hostFilter.value === 'offline') {
    return hosts.value.filter((h) => h.agentAvailable === false)
  }
  if (hostFilter.value === 'unlinked') {
    return hosts.value.filter((h) => !h.cmdbLinked)
  }
  if (hostFilter.value === 'linked') {
    return hosts.value.filter((h) => !!h.cmdbLinked)
  }
  return hosts.value
})
const loading = ref(false)
const syncing = ref(false)
const govLoading = ref(false)
const governance = ref<GovernanceData | null>(null)
const selected = ref<MonitorHost | null>(null)
const selectedIds = ref<string[]>([])
const problems = ref<MonitorProblem[]>([])
const problemLoading = ref(false)
const chartRef = ref<HTMLElement | null>(null)
let chart: echarts.ECharts | null = null
const task = ref<MonitorTask | null>(null)
const taskSubmitting = ref(false)
const upgradeVisible = ref(false)
const targetVersion = ref('')
let pollTimer: number | undefined

const severityLabels = ['未分类', '信息', '警告', '一般', '严重', '灾难']
function severityLabel(value: string) {
  return severityLabels[Number(value)] || value
}

onMounted(async () => {
  try {
    instances.value = await listInstances()
    instanceCode.value = instances.value[0]?.code || ''
    if (instanceCode.value) {
      await Promise.all([loadHosts(), loadGovernance()])
    }
  } catch {
    instances.value = []
  }
})

onBeforeUnmount(() => {
  chart?.dispose()
  window.removeEventListener('resize', resizeChart)
  if (pollTimer) window.clearInterval(pollTimer)
})

async function loadHosts() {
  if (!instanceCode.value) return
  loading.value = true
  try {
    hosts.value = await listHosts(instanceCode.value, search.value)
    selected.value = null
    selectedIds.value = []
    problems.value = []
    chart?.clear()
  } catch {
    hosts.value = []
  } finally {
    loading.value = false
  }
}

async function loadGovernance() {
  if (!instanceCode.value) return
  govLoading.value = true
  try {
    governance.value = await getGovernance(instanceCode.value)
  } catch {
    governance.value = null
  } finally {
    govLoading.value = false
  }
}

async function onInstanceChange() {
  await Promise.all([loadHosts(), loadGovernance()])
}

async function doSync() {
  if (!instanceCode.value) return
  syncing.value = true
  try {
    const res = await syncHosts(instanceCode.value)
    ElMessage.success(`已同步 ${res.synced} 台主机对照`)
  } finally {
    syncing.value = false
  }
}

function onSelection(rows: MonitorHost[]) {
  selectedIds.value = rows.map((r) => r.hostId)
}

async function onSelect(row: MonitorHost | undefined) {
  if (!row) return
  selected.value = row
  problemLoading.value = true
  try {
    const [series, current] = await Promise.all([
      hostMetrics(instanceCode.value, row.hostId, 24),
      hostProblems(instanceCode.value, row.hostId),
    ])
    problems.value = current
    await nextTick()
    renderChart(series)
  } catch {
    problems.value = []
    chart?.clear()
  } finally {
    problemLoading.value = false
  }
}

async function runAction(action: MonitorAction) {
  if (!selectedIds.value.length) return
  await ElMessageBox.confirm(
    `将对 ${selectedIds.value.length} 台主机执行 ${action}。操作前会创建 Zabbix 维护期。`,
    '确认操作',
    { type: 'warning' },
  )
  await submitTask(action)
}

function openUpgrade() {
  targetVersion.value = ''
  upgradeVisible.value = true
}

async function confirmUpgrade() {
  if (!targetVersion.value.trim()) {
    ElMessage.warning('请填写目标版本')
    return
  }
  upgradeVisible.value = false
  await submitTask('upgrade', targetVersion.value.trim())
}

async function submitTask(action: MonitorAction, version?: string) {
  taskSubmitting.value = true
  try {
    const res = await createTask({
      instanceCode: instanceCode.value,
      action,
      hostIds: selectedIds.value,
      targetVersion: version,
    })
    ElMessage.success('任务已创建')
    await refreshTask(res.id)
    if (pollTimer) window.clearInterval(pollTimer)
    pollTimer = window.setInterval(() => refreshTask(res.id), 3000)
  } finally {
    taskSubmitting.value = false
  }
}

async function refreshTask(id: string) {
  try {
    task.value = await getTask(id)
    if (task.value && ['succeeded', 'failed', 'partial'].includes(task.value.status)) {
      if (pollTimer) {
        window.clearInterval(pollTimer)
        pollTimer = undefined
      }
    }
  } catch {
    /* ignore */
  }
}

function resizeChart() {
  chart?.resize()
}

function renderChart(series: MetricSeries[]) {
  if (!chartRef.value) return
  if (!chart) {
    chart = echarts.init(chartRef.value)
    window.addEventListener('resize', resizeChart)
  }
  const names: Record<string, string> = { cpu: 'CPU %', memory: '内存 %', disk: '磁盘 %' }
  const axis = series.reduce(
    (best, item) => (item.points.length > best.points.length ? item : best),
    series[0] || { name: '', key: null, points: [] },
  )
  chart.setOption(
    {
      tooltip: { trigger: 'axis' },
      legend: { data: series.map((s) => names[s.name] || s.name) },
      grid: { left: 48, right: 16, top: 32, bottom: 28 },
      xAxis: {
        type: 'category',
        data: (axis?.points || []).map((p) => formatClock(p.clock)),
      },
      yAxis: { type: 'value', name: '%' },
      series: series.map((s) => ({
        name: names[s.name] || s.name,
        type: 'line',
        showSymbol: false,
        data: s.points.map((p) => Number(p.value)),
      })),
    },
    true,
  )
}

function formatClock(clock: string) {
  const date = new Date(Number(clock) * 1000)
  if (Number.isNaN(date.getTime())) return clock
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(date.getHours())}:${pad(date.getMinutes())}`
}
</script>

<style scoped>
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
}
.page-sub {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.header-actions {
  display: flex;
  gap: 8px;
}
.filter-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 12px;
}
.detail {
  margin-top: 16px;
}
.gov-row {
  margin-bottom: 16px;
}
.gov-stat {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.gov-num {
  font-size: 28px;
  font-weight: 600;
}
.gov-num.danger {
  color: var(--el-color-danger);
}
.gov-hint {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.chart {
  height: 280px;
}
</style>
