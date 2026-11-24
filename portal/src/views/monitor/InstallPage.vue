<template>
  <div class="install-page">
    <div class="page-header">
      <div class="page-title">
        <span>安装 Agent</span>
        <span class="page-sub">选包 SCP 下发 → 解压到可配目录 → Proxy Group 映射 conf → 启动</span>
      </div>
      <div class="header-links">
        <el-button link type="primary" @click="$router.push('/monitor/agents')">代理管理</el-button>
        <el-button link type="primary" @click="$router.push('/monitor/agent-packages')">部署配置</el-button>
      </div>
    </div>

    <el-card shadow="never">
      <el-form label-width="130px">
        <el-form-item label="Zabbix 实例">
          <el-select v-model="instanceCode" style="width: 280px" @change="loadOptions">
            <el-option v-for="item in instances" :key="item.code" :label="item.name" :value="item.code" />
          </el-select>
        </el-form-item>
        <el-form-item label="安装包" required>
          <el-select
            v-model="packageId"
            filterable
            style="width: 560px"
            placeholder="选择已上传的 Agent 包"
            :disabled="!packages.length"
          >
            <el-option
              v-for="p in packages"
              :key="p.id"
              :label="`${p.name} · ${p.version}${p.osArch ? ' · ' + p.osArch : ''}`"
              :value="p.id"
            />
          </el-select>
          <div v-if="!packages.length" class="field-hint">
            尚无可用安装包，请先到
            <el-button link type="primary" @click="$router.push('/monitor/agent-packages')">部署配置</el-button>
            上传压缩包。
          </div>
        </el-form-item>
        <el-form-item label="安装目录">
          <el-input v-model="installPath" style="width: 560px" :placeholder="installPathPlaceholder" />
          <div v-if="installPathPlaceholder" class="field-hint">
            留空则安装到默认目录 {{ installPathPlaceholder }}，可在 Gateway 配置 [zabbix_ctl].default_install_path 修改。
          </div>
        </el-form-item>
        <el-form-item label="配置项">
          <el-select
            v-model="ciIds"
            multiple
            filterable
            remote
            reserve-keyword
            placeholder="搜索资产名称"
            :remote-method="searchCi"
            :loading="ciLoading"
            style="width: 560px"
          >
            <el-option v-for="item in ciOptions" :key="item.id" :label="item.name" :value="item.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="主机组">
          <el-select v-model="groupIds" multiple filterable style="width: 560px">
            <el-option v-for="item in hostGroups" :key="item.groupId" :label="item.name" :value="item.groupId" />
          </el-select>
        </el-form-item>
        <el-form-item label="模板">
          <el-select v-model="templateIds" multiple filterable style="width: 560px">
            <el-option v-for="item in templates" :key="item.templateId" :label="item.name" :value="item.templateId" />
          </el-select>
        </el-form-item>
        <el-form-item label="监控通道">
          <el-radio-group v-model="channel" @change="onChannelChange">
            <el-radio-button value="proxyGroup">经 Proxy Group</el-radio-button>
            <el-radio-button value="proxy">经单 Proxy</el-radio-button>
            <el-radio-button value="direct">直连 Server</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item v-if="channel === 'proxyGroup'" label="Proxy Group">
          <el-select
            v-model="proxyGroupId"
            filterable
            clearable
            placeholder="选择 Proxy Group（自动带出 conf 后缀）"
            style="width: 560px"
            :loading="proxyLoading"
            @change="onProxyGroupChange"
          >
            <el-option
              v-for="item in proxyGroups"
              :key="item.proxyGroupId"
              :label="proxyGroupLabel(item)"
              :value="item.proxyGroupId"
            />
          </el-select>
        </el-form-item>
        <el-form-item v-if="channel === 'proxy'" label="Zabbix Proxy">
          <el-select
            v-model="proxyId"
            filterable
            clearable
            style="width: 560px"
            :loading="proxyLoading"
            @change="onProxyChange"
          >
            <el-option
              v-for="item in proxies"
              :key="item.proxyId"
              :label="proxyLabel(item)"
              :value="item.proxyId"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="conf 后缀">
          <el-input
            v-model="confSuffix"
            style="width: 320px"
            placeholder="如 227_9，对应 zabbix_agentd.conf_227_9"
          />
          <div class="field-hint">选 Proxy Group 后若已配置映射会自动填入；有模板时可不再手填 Server。</div>
        </el-form-item>
        <el-form-item label="Server（覆盖）">
          <el-input
            v-model="server"
            style="width: 560px"
            placeholder="可选；有 conf 模板时一般留空。被动检查多地址用逗号"
          />
        </el-form-item>
        <el-form-item label="端口（覆盖）">
          <el-input
            v-model="agentPort"
            style="width: 200px"
            placeholder="默认 10050"
          />
          <div class="field-hint">写入 Agent ListenPort，并作为 Zabbix 主机接口端口；与模板不一致时请在此覆盖。</div>
        </el-form-item>
        <el-form-item label="ServerActive">
          <el-input
            v-model="serverActive"
            style="width: 560px"
            placeholder="可选；Proxy Group 主动检查用分号分隔"
          />
        </el-form-item>
        <el-form-item label="登记状态">
          <el-checkbox v-model="disableAfterInstall">
            验证通过后禁用监控
          </el-checkbox>
          <div class="field-hint">
            安装后仍会先启用主机并等 agent.ping 通，通过后再禁用，避免立刻告警。就绪后可在名册里启用。
          </div>
        </el-form-item>
        <el-form-item>
          <el-button
            v-permission="'monitor:agent'"
            type="primary"
            :loading="submitting"
            :disabled="!packages.length || !packageId"
            @click="submit"
          >
            开始安装
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card v-if="task" shadow="never" class="result">
      <template #header>任务 {{ task.id.slice(0, 8) }} · {{ task.status }}</template>
      <el-table :data="task.hosts" size="small">
        <el-table-column prop="ciId" label="配置项" min-width="220" />
        <el-table-column prop="hostId" label="Host ID" width="140" />
        <el-table-column prop="status" label="状态" width="110" />
        <el-table-column prop="error" label="说明" min-width="280" />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { listCiInstances } from '../../api/cmdb'
import {
  getAgentInstallDefaults,
  getTask,
  installAgents,
  listAgentPackages,
  listHostGroups,
  listInstances,
  listProxies,
  listProxyConfMaps,
  listProxyGroups,
  listTemplates,
  type AgentPackage,
  type MonitorGroupRef,
  type MonitorInstance,
  type MonitorProxy,
  type MonitorProxyGroup,
  type MonitorTask,
  type MonitorTemplate,
  type ProxyConfMap,
} from '../../api/monitor'

const instances = ref<MonitorInstance[]>([])
const instanceCode = ref('')
const packages = ref<AgentPackage[]>([])
const packageId = ref('')
const installPath = ref('')
/** 占位与初始值同源，都来自 Gateway 配置的 defaultInstallPath，不在前端写死。 */
const installPathPlaceholder = ref('')
const confSuffix = ref('')
const confMaps = ref<ProxyConfMap[]>([])
const hostGroups = ref<MonitorGroupRef[]>([])
const templates = ref<MonitorTemplate[]>([])
const proxies = ref<MonitorProxy[]>([])
const proxyGroups = ref<MonitorProxyGroup[]>([])
const proxyLoading = ref(false)
const channel = ref<'proxyGroup' | 'proxy' | 'direct'>('proxyGroup')
const proxyId = ref('')
const proxyGroupId = ref('')
const ciIds = ref<string[]>([])
const ciOptions = ref<{ id: string; name: string }[]>([])
const ciLoading = ref(false)
const groupIds = ref<string[]>([])
const templateIds = ref<string[]>([])
const server = ref('')
const agentPort = ref('')
const serverActive = ref('')
/** 默认勾选：装完验证后禁用，减少误告警 */
const disableAfterInstall = ref(true)
const submitting = ref(false)
const task = ref<MonitorTask | null>(null)
let pollTimer: number | undefined

onMounted(async () => {
  const [inst, pkgs, maps, defaults] = await Promise.all([
    listInstances(),
    listAgentPackages().catch(() => [] as AgentPackage[]),
    listProxyConfMaps().catch(() => [] as ProxyConfMap[]),
    // 失败不阻塞页面：留空提交时后端仍会回退内置默认。
    getAgentInstallDefaults().catch(() => null),
  ])
  instances.value = inst
  packages.value = pkgs.filter((p) => p.enabled)
  confMaps.value = maps
  packageId.value = packages.value[0]?.id || ''
  const defaultPath =
    defaults?.defaultInstallPath || packages.value[0]?.defaultInstallPath || ''
  if (defaultPath) {
    installPathPlaceholder.value = defaultPath
    // 不覆盖用户已输入的路径
    if (!installPath.value.trim()) installPath.value = defaultPath
  }
  instanceCode.value = instances.value[0]?.code || ''
  if (instanceCode.value) await loadOptions()
})

onBeforeUnmount(() => {
  if (pollTimer) window.clearInterval(pollTimer)
})

async function loadOptions() {
  if (!instanceCode.value) return
  proxyId.value = ''
  proxyGroupId.value = ''
  confSuffix.value = ''
  const [groups, tpls] = await Promise.all([
    listHostGroups(instanceCode.value),
    listTemplates(instanceCode.value),
  ])
  hostGroups.value = groups
  templates.value = tpls
  await loadProxyData()
  if (!proxyGroups.value.length && channel.value === 'proxyGroup') {
    channel.value = proxies.value.length ? 'proxy' : 'direct'
  }
}

async function loadProxyData() {
  if (!instanceCode.value) return
  proxyLoading.value = true
  try {
    const [p, g] = await Promise.all([
      listProxies(instanceCode.value).catch(() => [] as MonitorProxy[]),
      listProxyGroups(instanceCode.value).catch(() => [] as MonitorProxyGroup[]),
    ])
    proxies.value = p
    proxyGroups.value = g
  } finally {
    proxyLoading.value = false
  }
}

function proxyLabel(item: MonitorProxy) {
  const addr = item.address?.trim()
  const state = item.online ? '在线' : '离线'
  return addr ? `${item.name} (${addr}) · ${state}` : `${item.name} · ${state}`
}

function proxyGroupLabel(item: MonitorProxyGroup) {
  const mapped = confMaps.value.find(
    (m) => m.instanceCode === instanceCode.value && m.proxyGroupId === item.proxyGroupId,
  )
  const suffix = mapped ? ` → conf_${mapped.confSuffix}` : ''
  const state = item.online ? '在线' : '非在线'
  return `${item.name} · ${item.proxies?.length || 0} Proxy · ${state}${suffix}`
}

function onChannelChange() {
  if (channel.value === 'direct') {
    proxyId.value = ''
    proxyGroupId.value = ''
  } else if (channel.value === 'proxy') {
    proxyGroupId.value = ''
  } else {
    proxyId.value = ''
    if (proxyGroupId.value) onProxyGroupChange(proxyGroupId.value)
  }
}

function onProxyChange(id: string) {
  const item = proxies.value.find((p) => p.proxyId === id)
  const addr = item?.address?.trim() || ''
  if (addr && !confSuffix.value) {
    server.value = addr
    serverActive.value = addr
  }
}

function onProxyGroupChange(id: string) {
  const mapped = confMaps.value.find(
    (m) => m.instanceCode === instanceCode.value && m.proxyGroupId === id,
  )
  if (mapped?.confSuffix) {
    confSuffix.value = mapped.confSuffix
    ElMessage.success(`已带出 conf 后缀：${mapped.confSuffix}`)
  } else if (id) {
    ElMessage.info('该 Proxy Group 尚未配置 conf 映射，请手工填写后缀或到部署配置添加映射')
  }
  const item = proxyGroups.value.find((g) => g.proxyGroupId === id)
  if (item && !confSuffix.value) {
    const csv = item.serverCsv?.trim() || ''
    const semi = item.serverActiveSemicolon?.trim() || ''
    if (csv) server.value = csv
    if (semi) serverActive.value = semi
  }
}

async function searchCi(keyword: string) {
  ciLoading.value = true
  try {
    const page = await listCiInstances({ keyword, page: 1, pageSize: 20 })
    const selected = ciOptions.value.filter((item) => ciIds.value.includes(item.id))
    const merged = [...selected]
    for (const item of page.items) {
      if (!merged.some((row) => row.id === item.id)) {
        merged.push({ id: item.id, name: item.name })
      }
    }
    ciOptions.value = merged
  } finally {
    ciLoading.value = false
  }
}

async function submit() {
  if (!instanceCode.value || !packageId.value || !ciIds.value.length || !groupIds.value.length || !templateIds.value.length) {
    ElMessage.warning('请填实例、安装包、配置项、主机组和模板')
    return
  }
  const path = installPath.value.trim()
  if (path && !path.startsWith('/')) {
    ElMessage.warning('安装目录需为绝对路径')
    return
  }
  if (channel.value === 'proxy' && !proxyId.value) {
    ElMessage.warning('请选择 Zabbix Proxy')
    return
  }
  if (channel.value === 'proxyGroup' && !proxyGroupId.value) {
    ElMessage.warning('请选择 Proxy Group')
    return
  }
  if (!confSuffix.value.trim() && !server.value.trim()) {
    ElMessage.warning('请填写 conf 后缀或 Server 覆盖地址')
    return
  }
  const portRaw = agentPort.value.trim()
  if (portRaw) {
    const n = Number(portRaw)
    if (!Number.isInteger(n) || n < 1 || n > 65535) {
      ElMessage.warning('端口须为 1–65535 的整数')
      return
    }
  }
  submitting.value = true
  try {
    const res = await installAgents({
      instanceCode: instanceCode.value,
      ciIds: ciIds.value,
      groupIds: groupIds.value,
      templateIds: templateIds.value,
      packageId: packageId.value,
      installPath: path || undefined,
      confSuffix: confSuffix.value.trim() || undefined,
      server: server.value.trim() || undefined,
      agentPort: portRaw || undefined,
      serverActive: serverActive.value.trim() || undefined,
      proxyId: channel.value === 'proxy' ? proxyId.value : undefined,
      proxyGroupId: channel.value === 'proxyGroup' ? proxyGroupId.value : undefined,
      disableAfterInstall: disableAfterInstall.value,
    })
    ElMessage.success('安装任务已创建')
    await refresh(res.id)
    if (pollTimer) window.clearInterval(pollTimer)
    pollTimer = window.setInterval(() => refresh(res.id), 3000)
  } finally {
    submitting.value = false
  }
}

async function refresh(id: string) {
  task.value = await getTask(id)
  if (task.value && ['succeeded', 'failed', 'partial'].includes(task.value.status) && pollTimer) {
    window.clearInterval(pollTimer)
    pollTimer = undefined
  }
}
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.header-links {
  display: flex;
  gap: 4px;
}
.page-title {
  display: flex;
  align-items: baseline;
  gap: 8px;
  font-size: 18px;
}
.page-sub {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.field-hint {
  margin-top: 6px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
  line-height: 1.5;
  max-width: 560px;
}
.result {
  margin-top: 16px;
}
</style>
