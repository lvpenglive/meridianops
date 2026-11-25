<template>
  <div class="agent-manage-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><Cpu /></el-icon>
        <span>代理管理</span>
        <span class="page-sub">看清有没有 Agent、多少在跑、多少未运行；安装与启停从这里进</span>
      </div>
      <div class="header-actions">
        <el-button v-permission="'monitor:agent'" type="primary" @click="router.push('/monitor/install')">
          安装 Agent
        </el-button>
        <el-button v-permission="'monitor:agent'" @click="router.push('/monitor/agent-packages')">
          部署配置
        </el-button>
        <el-button
          v-permission="'monitor:agent'"
          :loading="syncing"
          @click="doSync"
        >
          同步对照
        </el-button>
        <el-button :icon="Refresh" :loading="loading" @click="loadHosts">刷新</el-button>
      </div>
    </div>

    <div class="summary-row">
      <button
        type="button"
        class="summary-card"
        :class="{ active: !agentStatus }"
        @click="pickStatus('')"
      >
        <span class="summary-num">{{ totalAll }}</span>
        <span class="summary-label">全部</span>
      </button>
      <button
        type="button"
        class="summary-card is-running"
        :class="{ active: agentStatus === 'running' }"
        @click="pickStatus('running')"
      >
        <span class="summary-num">{{ summary.running }}</span>
        <span class="summary-label">运行中</span>
      </button>
      <button
        type="button"
        class="summary-card is-stopped"
        :class="{ active: agentStatus === 'stopped' }"
        @click="pickStatus('stopped')"
      >
        <span class="summary-num">{{ summary.stopped }}</span>
        <span class="summary-label">未运行</span>
      </button>
      <button
        type="button"
        class="summary-card is-absent"
        :class="{ active: agentStatus === 'absent' }"
        @click="pickStatus('absent')"
      >
        <span class="summary-num">{{ summary.absent }}</span>
        <span class="summary-label">无 Agent</span>
      </button>
      <button
        type="button"
        class="summary-card is-unknown"
        :class="{ active: agentStatus === 'unknown' }"
        @click="pickStatus('unknown')"
      >
        <span class="summary-num">{{ summary.unknown }}</span>
        <span class="summary-label">未知</span>
      </button>
    </div>
    <p class="summary-hint">
      「未运行」= 有 Agent 接口但当前不通（含停服与链路异常，后续会拆开）。名册通断仍以 Zabbix 为准。
    </p>

    <div class="filter-bar">
      <el-select v-model="instanceCode" placeholder="Zabbix 实例" style="width: 220px" @change="onInstanceChange">
        <el-option v-for="item in instances" :key="item.code" :label="item.name" :value="item.code" />
      </el-select>
      <el-input
        v-model="search"
        placeholder="主机名 / IP"
        clearable
        style="width: 220px"
        @keyup.enter="reloadHosts"
        @clear="reloadHosts"
      />
      <el-select v-model="cmdbFilter" placeholder="CMDB" clearable style="width: 140px" @change="reloadHosts">
        <el-option label="已关联" value="linked" />
        <el-option label="未关联" value="unlinked" />
      </el-select>
      <el-button type="primary" @click="reloadHosts">查询</el-button>
      <el-dropdown
        v-if="canAgent"
        trigger="click"
        :disabled="!selectedHosts.length"
        @command="onAgentCommand"
      >
        <el-button :disabled="!selectedHosts.length">
          批量操作
          <el-icon class="el-icon--right"><ArrowDown /></el-icon>
        </el-button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="start">启动</el-dropdown-item>
            <el-dropdown-item command="stop">停止</el-dropdown-item>
            <el-dropdown-item command="restart">重启</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
      <span class="matched-hint">当前 {{ matched }} 台</span>
    </div>

    <el-table
      v-loading="loading"
      :data="hosts"
      border
      stripe
      row-key="hostId"
      @selection-change="onSelectionChange"
    >
      <el-table-column type="selection" width="44" />
      <el-table-column label="主机" min-width="180">
        <template #default="{ row }">
          <div class="host-cell">
            <span class="host-name">{{ row.displayName || row.hostname }}</span>
            <span class="host-sub">{{ row.hostname }}</span>
          </div>
        </template>
      </el-table-column>
      <el-table-column label="IP / 端口" min-width="180">
        <template #default="{ row }">
          {{ (row.ips || []).length > 1 ? row.ips.join(' / ') : (row.ip || '—') }}<template v-if="row.port">:{{ row.port }}</template>
        </template>
      </el-table-column>
      <el-table-column label="Agent 状态" width="110">
        <template #default="{ row }">
          <el-tag :type="statusTagType(row.agentStatus)" size="small">
            {{ statusLabel(row.agentStatus) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="版本" width="120">
        <template #default="{ row }">{{ row.agentVersion || '—' }}</template>
      </el-table-column>
      <el-table-column label="CMDB" width="100">
        <template #default="{ row }">
          <el-tag v-if="row.cmdbLinked" type="success" size="small" effect="plain">已关联</el-tag>
          <el-tag v-else type="info" size="small" effect="plain">未关联</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="SSH" width="90">
        <template #default="{ row }">
          <el-tag v-if="row.sshCredentialBound" type="success" size="small" effect="plain">已绑</el-tag>
          <span v-else class="muted">—</span>
        </template>
      </el-table-column>
      <el-table-column label="监控通道" width="120">
        <template #default="{ row }">{{ channelLabel(row) }}</template>
      </el-table-column>
      <el-table-column label="SSH 路径" width="110">
        <template #default="{ row }">
          <el-tag
            v-if="row.sshPath === 'jump'"
            type="warning"
            size="small"
            effect="plain"
          >跳板</el-tag>
          <el-tag v-else type="info" size="small" effect="plain">直连</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="220" fixed="right">
        <template #default="{ row }">
          <template v-if="canAgent">
            <el-button
              v-if="row.agentStatus === 'absent'"
              link
              type="primary"
              @click="router.push('/monitor/install')"
            >
              去安装
            </el-button>
            <template v-else>
              <el-button
                link
                type="success"
                :disabled="row.agentStatus === 'running'"
                @click="runOne('start', row)"
              >
                启动
              </el-button>
              <el-button
                link
                type="warning"
                :disabled="row.agentStatus === 'stopped' || row.agentStatus === 'absent'"
                @click="runOne('stop', row)"
              >
                停止
              </el-button>
              <el-button link type="primary" @click="runOne('restart', row)">重启</el-button>
            </template>
          </template>
          <el-button link @click="goRoster(row)">名册</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="pager">
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="matched"
        :page-sizes="[20, 50, 100]"
        layout="total, sizes, prev, pager, next"
        background
        @current-change="loadHosts"
        @size-change="onPageSize"
      />
    </div>

    <el-card v-if="task" class="task-card" shadow="never">
      <template #header>
        <div class="task-head">
          <span>最近任务 · {{ task.action }} · {{ task.status }}</span>
          <el-button link type="primary" @click="task = null">关闭</el-button>
        </div>
      </template>
      <el-table :data="task.hosts" size="small" max-height="220">
        <el-table-column prop="hostId" label="Host ID" width="120" />
        <el-table-column prop="status" label="状态" width="100" />
        <el-table-column prop="error" label="错误" min-width="200" show-overflow-tooltip />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowDown, Cpu, Refresh } from '@element-plus/icons-vue'
import { useUserStore } from '../../stores/user'
import {
  createTask,
  getTask,
  listHosts,
  listInstances,
  syncHosts,
  type AgentLifecycleStatus,
  type AgentStatusSummary,
  type MonitorAction,
  type MonitorHost,
  type MonitorInstance,
  type MonitorTask,
} from '../../api/monitor'

const router = useRouter()
const user = useUserStore()
const canAgent = computed(() => user.hasPermission('monitor:agent'))

const instances = ref<MonitorInstance[]>([])
const instanceCode = ref('')
const search = ref('')
const cmdbFilter = ref<'linked' | 'unlinked' | ''>('')
const agentStatus = ref<AgentLifecycleStatus | ''>('')
const hosts = ref<MonitorHost[]>([])
const selectedHosts = ref<MonitorHost[]>([])
const total = ref(0)
const matched = ref(0)
const page = ref(1)
const pageSize = ref(20)
const loading = ref(false)
const syncing = ref(false)
const summary = ref<AgentStatusSummary>({ running: 0, stopped: 0, absent: 0, unknown: 0 })
const task = ref<MonitorTask | null>(null)
const taskSubmitting = ref(false)
let pollTimer: number | undefined

const totalAll = computed(
  () => summary.value.running + summary.value.stopped + summary.value.absent + summary.value.unknown,
)

function statusLabel(st?: AgentLifecycleStatus | null) {
  switch (st) {
    case 'running':
      return '运行中'
    case 'stopped':
      return '未运行'
    case 'absent':
      return '无 Agent'
    default:
      return '未知'
  }
}

function statusTagType(st?: AgentLifecycleStatus | null) {
  switch (st) {
    case 'running':
      return 'success'
    case 'stopped':
      return 'danger'
    case 'absent':
      return 'info'
    default:
      return 'warning'
  }
}

function channelLabel(row: MonitorHost) {
  if (row.channel === 'proxyGroup') {
    return row.proxyGroupId ? `组 ${row.proxyGroupId}` : 'Proxy Group'
  }
  if (row.channel === 'proxy' || row.proxyId) {
    return row.proxyId ? `Proxy ${row.proxyId}` : 'Proxy'
  }
  return '直连 Server'
}

async function doSync() {
  if (!instanceCode.value || syncing.value) return
  try {
    await ElMessageBox.confirm(
      '将同步本实例全部启用主机对照（含 Proxy 通道），主机量大时可能较慢。是否继续？',
      '同步对照',
      { type: 'warning', confirmButtonText: '继续', cancelButtonText: '取消' },
    )
  } catch {
    return
  }
  syncing.value = true
  try {
    const res = await syncHosts(instanceCode.value)
    ElMessage.success(`已同步 ${res.synced} 台主机对照（含 Proxy 通道）`)
    await loadHosts()
  } catch {
    /* request 已提示 */
  } finally {
    syncing.value = false
  }
}

function pickStatus(st: AgentLifecycleStatus | '') {
  agentStatus.value = st
  reloadHosts()
}

function onSelectionChange(rows: MonitorHost[]) {
  selectedHosts.value = rows
}

function goRoster(row: MonitorHost) {
  router.push({ path: '/monitor', query: { search: row.hostname || row.ip || '' } })
}

async function onInstanceChange() {
  page.value = 1
  await loadHosts()
}

function reloadHosts() {
  if (page.value !== 1) page.value = 1
  else return loadHosts()
}

function onPageSize() {
  if (page.value !== 1) page.value = 1
  else return loadHosts()
}

async function loadHosts() {
  if (!instanceCode.value) return
  loading.value = true
  try {
    const res = await listHosts(instanceCode.value, {
      search: search.value.trim() || undefined,
      page: page.value,
      pageSize: pageSize.value,
      agentStatus: agentStatus.value || undefined,
      cmdb: cmdbFilter.value || undefined,
    })
    hosts.value = res.items || []
    total.value = res.total
    matched.value = res.matched
    summary.value = res.agentSummary || { running: 0, stopped: 0, absent: 0, unknown: 0 }
  } catch {
    hosts.value = []
  } finally {
    loading.value = false
  }
}

function resolveTargets(rows: MonitorHost[]): MonitorHost[] | null {
  if (!rows.length) {
    ElMessage.warning('请先勾选主机')
    return null
  }
  const noAgent = rows.filter((h) => h.agentStatus === 'absent' || !h.availability?.zbx)
  if (noAgent.length) {
    ElMessage.warning('所选主机中含无 Agent 接口的主机，请先安装或去掉勾选')
    return null
  }
  const unlinked = rows.filter((h) => !h.cmdbLinked || !h.ciId)
  if (unlinked.length) {
    ElMessage.warning('所选主机需先关联配置项（名册「关联资源」）')
    return null
  }
  return rows
}

async function onAgentCommand(cmd: string) {
  if (cmd !== 'start' && cmd !== 'stop' && cmd !== 'restart') return
  const targets = resolveTargets(selectedHosts.value)
  if (!targets) return
  await submitTask(cmd, targets.map((h) => h.hostId))
}

async function runOne(action: MonitorAction, row: MonitorHost) {
  const targets = resolveTargets([row])
  if (!targets) return
  const label = action === 'start' ? '启动' : action === 'stop' ? '停止' : '重启'
  try {
    await ElMessageBox.confirm(
      `确认对「${row.displayName || row.hostname}」执行${label}？`,
      '代理操作',
      { type: 'warning' },
    )
  } catch {
    return
  }
  await submitTask(action, [row.hostId])
}

async function submitTask(action: MonitorAction, hostIds: string[]) {
  if (taskSubmitting.value) return
  taskSubmitting.value = true
  try {
    const res = await createTask({
      instanceCode: instanceCode.value,
      action,
      hostIds,
    })
    ElMessage.success('任务已创建')
    await refreshTask(res.id)
    if (pollTimer) window.clearInterval(pollTimer)
    pollTimer = window.setInterval(() => refreshTask(res.id), 3000)
  } catch {
    /* request 已提示 */
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
      await loadHosts()
    }
  } catch {
    /* ignore */
  }
}

onMounted(async () => {
  try {
    instances.value = await listInstances()
    if (instances.value.length) {
      instanceCode.value = instances.value[0].code
      await loadHosts()
    }
  } catch {
    instances.value = []
  }
})

onBeforeUnmount(() => {
  if (pollTimer) window.clearInterval(pollTimer)
})
</script>

<style scoped>
.agent-manage-page {
  padding: 4px 0 24px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  gap: 12px;
  flex-wrap: wrap;
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
  flex-wrap: wrap;
}
.summary-row {
  display: flex;
  gap: 10px;
  flex-wrap: wrap;
  margin-bottom: 8px;
}
.summary-card {
  min-width: 110px;
  padding: 12px 16px;
  border: 1px solid var(--el-border-color);
  border-radius: 6px;
  background: var(--el-bg-color);
  cursor: pointer;
  text-align: left;
}
.summary-card:hover {
  border-color: var(--el-color-primary-light-5);
}
.summary-card.active {
  border-color: var(--el-color-primary);
  box-shadow: 0 0 0 1px var(--el-color-primary-light-7);
}
.summary-num {
  display: block;
  font-size: 22px;
  font-weight: 600;
  line-height: 1.2;
}
.summary-label {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.summary-card.is-running .summary-num {
  color: var(--el-color-success);
}
.summary-card.is-stopped .summary-num {
  color: var(--el-color-danger);
}
.summary-card.is-absent .summary-num {
  color: var(--el-text-color-secondary);
}
.summary-card.is-unknown .summary-num {
  color: var(--el-color-warning);
}
.summary-hint {
  margin: 0 0 14px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.filter-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  margin-bottom: 12px;
}
.matched-hint {
  margin-left: auto;
  font-size: 13px;
  color: var(--el-text-color-secondary);
}
.host-cell {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.host-name {
  font-weight: 500;
}
.host-sub {
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
.muted {
  color: var(--el-text-color-placeholder);
}
.pager {
  margin-top: 12px;
  display: flex;
  justify-content: flex-end;
}
.task-card {
  margin-top: 16px;
}
.task-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>
