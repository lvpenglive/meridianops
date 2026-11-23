<template>
  <div class="pkg-page">
    <div class="page-header">
      <div class="page-title">
        <span>部署配置</span>
        <span class="page-sub">安装包、Proxy Group 映射与 SSH 跳板（Gateway 无法直连业务机时）</span>
      </div>
    </div>

    <el-tabs v-model="activeTab">
      <el-tab-pane label="安装包" name="packages">
        <div class="tab-toolbar">
          <el-button v-permission="'monitor:agent'" type="primary" @click="showUpload = true">上传安装包</el-button>
        </div>
        <el-table :data="packages" v-loading="loading" size="small">
          <el-table-column prop="name" label="名称" min-width="160" />
          <el-table-column prop="version" label="版本" width="110" />
          <el-table-column prop="osArch" label="系统/架构" width="120" />
          <el-table-column prop="agentKind" label="类型" width="90" />
          <el-table-column prop="filename" label="文件名" min-width="180" />
          <el-table-column label="大小" width="100">
            <template #default="{ row }">{{ formatSize(row.fileSize) }}</template>
          </el-table-column>
          <el-table-column label="启用" width="80">
            <template #default="{ row }">
              <el-switch
                v-permission="'monitor:agent'"
                :model-value="row.enabled"
                @change="(v: boolean) => toggleEnabled(row, v)"
              />
            </template>
          </el-table-column>
          <el-table-column prop="createdAt" label="上传时间" width="170" />
          <el-table-column label="操作" width="90" fixed="right">
            <template #default="{ row }">
              <el-button v-permission="'monitor:agent'" link type="danger" @click="removePkg(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <el-tab-pane label="conf 映射" name="maps">
        <div class="tab-toolbar">
          <el-button v-permission="'monitor:agent'" type="primary" @click="openMapDialog()">新增映射</el-button>
        </div>
        <el-table :data="maps" v-loading="mapLoading" size="small">
          <el-table-column prop="instanceCode" label="实例" width="140" />
          <el-table-column prop="proxyGroupId" label="Proxy Group ID" width="140" />
          <el-table-column prop="confSuffix" label="conf 后缀" width="140" />
          <el-table-column label="跳板 Proxy" width="140">
            <template #default="{ row }">{{ row.jumpProxyId || '—' }}</template>
          </el-table-column>
          <el-table-column prop="remark" label="备注" min-width="160" />
          <el-table-column label="操作" width="90">
            <template #default="{ row }">
              <el-button v-permission="'monitor:agent'" link type="danger" @click="removeMap(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <el-tab-pane label="SSH 跳板" name="jumps">
        <div class="tab-toolbar">
          <el-button v-permission="'monitor:agent'" type="primary" @click="openJumpDialog()">新增跳板</el-button>
        </div>
        <el-table :data="jumps" v-loading="jumpLoading" size="small">
          <el-table-column prop="instanceCode" label="实例" width="120" />
          <el-table-column label="Proxy" min-width="160">
            <template #default="{ row }">
              {{ row.proxyName || row.proxyId }}
              <span v-if="row.proxyName" class="muted">（{{ row.proxyId }}）</span>
            </template>
          </el-table-column>
          <el-table-column label="跳板地址" min-width="160">
            <template #default="{ row }">{{ row.jumpHost }}:{{ row.jumpPort }}</template>
          </el-table-column>
          <el-table-column prop="credentialName" label="SSH 凭据" width="140">
            <template #default="{ row }">{{ row.credentialName || row.credentialId }}</template>
          </el-table-column>
          <el-table-column label="启用" width="80">
            <template #default="{ row }">
              <el-tag :type="row.enabled ? 'success' : 'info'" size="small" effect="plain">
                {{ row.enabled ? '是' : '否' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="remark" label="备注" min-width="140" />
          <el-table-column label="操作" width="140" fixed="right">
            <template #default="{ row }">
              <el-button v-permission="'monitor:agent'" link type="primary" @click="openJumpDialog(row)">编辑</el-button>
              <el-button v-permission="'monitor:agent'" link type="danger" @click="removeJump(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="showUpload" title="上传 Agent 包" width="480px" @closed="resetUpload">
      <el-form label-width="90px">
        <el-form-item label="文件" required>
          <input type="file" accept=".tar,.tar.gz,.tgz,.zip" @change="onFile" />
        </el-form-item>
        <el-form-item label="名称">
          <el-input v-model="uploadForm.name" placeholder="默认用文件名" />
        </el-form-item>
        <el-form-item label="版本" required>
          <el-input v-model="uploadForm.version" placeholder="如 7.0.26" />
        </el-form-item>
        <el-form-item label="系统/架构">
          <el-input v-model="uploadForm.osArch" placeholder="如 redhat8 / kylin" />
        </el-form-item>
        <el-form-item label="类型">
          <el-select v-model="uploadForm.agentKind" style="width: 100%">
            <el-option label="agentd" value="agentd" />
            <el-option label="agent2" value="agent2" />
          </el-select>
        </el-form-item>
        <el-form-item label="说明">
          <el-input v-model="uploadForm.description" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showUpload = false">取消</el-button>
        <el-button type="primary" :loading="uploading" @click="doUpload">上传</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showMap" title="Proxy Group 映射" width="520px">
      <el-form label-width="120px">
        <el-form-item label="Zabbix 实例" required>
          <el-select v-model="mapForm.instanceCode" style="width: 100%" @change="loadGroupsForMap">
            <el-option v-for="i in instances" :key="i.code" :label="i.name" :value="i.code" />
          </el-select>
        </el-form-item>
        <el-form-item label="Proxy Group" required>
          <el-select v-model="mapForm.proxyGroupId" filterable style="width: 100%" @change="onMapGroupChange">
            <el-option
              v-for="g in mapGroups"
              :key="g.proxyGroupId"
              :label="`${g.name} (${g.proxyGroupId})`"
              :value="g.proxyGroupId"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="conf 后缀" required>
          <el-input v-model="mapForm.confSuffix" placeholder="如 227_9 → zabbix_agentd.conf_227_9" />
        </el-form-item>
        <el-form-item label="跳板 Proxy">
          <el-select
            v-model="mapForm.jumpProxyId"
            filterable
            clearable
            placeholder="可选：组内哪台 Proxy 作 SSH 跳板"
            style="width: 100%"
          >
            <el-option
              v-for="p in mapGroupProxies"
              :key="p.proxyId"
              :label="`${p.name} (${p.proxyId})`"
              :value="p.proxyId"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="mapForm.remark" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showMap = false">取消</el-button>
        <el-button type="primary" :loading="mapSaving" @click="saveMap">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="showJump" :title="jumpEditId ? '编辑 SSH 跳板' : '新增 SSH 跳板'" width="520px">
      <el-form label-width="120px">
        <el-form-item label="Zabbix 实例" required>
          <el-select v-model="jumpForm.instanceCode" style="width: 100%" @change="loadProxiesForJump">
            <el-option v-for="i in instances" :key="i.code" :label="i.name" :value="i.code" />
          </el-select>
        </el-form-item>
        <el-form-item label="Proxy" required>
          <el-select v-model="jumpForm.proxyId" filterable style="width: 100%" @change="onJumpProxyChange">
            <el-option
              v-for="p in jumpProxies"
              :key="p.proxyId"
              :label="`${p.name} (${p.proxyId})`"
              :value="p.proxyId"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="跳板地址" required>
          <el-input v-model="jumpForm.jumpHost" placeholder="Proxy 管理 IP / 主机名" />
        </el-form-item>
        <el-form-item label="SSH 端口">
          <el-input-number v-model="jumpForm.jumpPort" :min="1" :max="65535" controls-position="right" />
        </el-form-item>
        <el-form-item label="SSH 凭据" required>
          <el-select v-model="jumpForm.credentialId" filterable style="width: 100%">
            <el-option
              v-for="c in credentials"
              :key="c.id"
              :label="`${c.name}（${c.username}）`"
              :value="c.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="jumpForm.enabled" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="jumpForm.remark" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showJump = false">取消</el-button>
        <el-button type="primary" :loading="jumpSaving" @click="saveJump">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { listAllCredentials, type SshCredentialSimple } from '../../api/credential'
import {
  deleteAgentPackage,
  deleteProxyConfMap,
  deleteSshJump,
  listAgentPackages,
  listInstances,
  listProxies,
  listProxyConfMaps,
  listProxyGroups,
  listSshJumps,
  updateAgentPackage,
  uploadAgentPackage,
  upsertProxyConfMap,
  upsertSshJump,
  type AgentPackage,
  type MonitorInstance,
  type MonitorProxy,
  type MonitorProxyGroup,
  type MonitorProxyGroupMember,
  type MonitorSshJump,
  type ProxyConfMap,
} from '../../api/monitor'

const activeTab = ref('packages')
const packages = ref<AgentPackage[]>([])
const maps = ref<ProxyConfMap[]>([])
const jumps = ref<MonitorSshJump[]>([])
const loading = ref(false)
const mapLoading = ref(false)
const jumpLoading = ref(false)
const showUpload = ref(false)
const showMap = ref(false)
const showJump = ref(false)
const uploading = ref(false)
const mapSaving = ref(false)
const jumpSaving = ref(false)
const jumpEditId = ref<number | null>(null)
const file = ref<File | null>(null)
const instances = ref<MonitorInstance[]>([])
const mapGroups = ref<MonitorProxyGroup[]>([])
const jumpProxies = ref<MonitorProxy[]>([])
const credentials = ref<SshCredentialSimple[]>([])

const uploadForm = reactive({
  name: '',
  version: '',
  osArch: '',
  agentKind: 'agentd',
  description: '',
})

const mapForm = reactive({
  instanceCode: '',
  proxyGroupId: '',
  confSuffix: '',
  jumpProxyId: '' as string,
  remark: '',
})

const jumpForm = reactive({
  instanceCode: '',
  proxyId: '',
  proxyName: '',
  jumpHost: '',
  jumpPort: 22,
  credentialId: null as number | null,
  enabled: true,
  remark: '',
})

const mapGroupProxies = computed<MonitorProxyGroupMember[]>(() => {
  const g = mapGroups.value.find((x) => x.proxyGroupId === mapForm.proxyGroupId)
  return g?.proxies || []
})

onMounted(async () => {
  await Promise.all([loadPackages(), loadMaps(), loadJumps()])
  instances.value = await listInstances().catch(() => [])
  credentials.value = await listAllCredentials().catch(() => [])
})

async function loadPackages() {
  loading.value = true
  try {
    packages.value = await listAgentPackages()
  } finally {
    loading.value = false
  }
}

async function loadMaps() {
  mapLoading.value = true
  try {
    maps.value = await listProxyConfMaps()
  } finally {
    mapLoading.value = false
  }
}

async function loadJumps() {
  jumpLoading.value = true
  try {
    jumps.value = await listSshJumps()
  } finally {
    jumpLoading.value = false
  }
}

function formatSize(n: number) {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / 1024 / 1024).toFixed(1)} MB`
}

function onFile(e: Event) {
  const input = e.target as HTMLInputElement
  file.value = input.files?.[0] || null
  if (file.value && !uploadForm.name) uploadForm.name = file.value.name
}

function resetUpload() {
  file.value = null
  uploadForm.name = ''
  uploadForm.version = ''
  uploadForm.osArch = ''
  uploadForm.agentKind = 'agentd'
  uploadForm.description = ''
}

async function doUpload() {
  if (!file.value || !uploadForm.version.trim()) {
    ElMessage.warning('请选择文件并填写版本')
    return
  }
  const fd = new FormData()
  fd.append('file', file.value)
  fd.append('name', uploadForm.name.trim())
  fd.append('version', uploadForm.version.trim())
  fd.append('osArch', uploadForm.osArch.trim())
  fd.append('agentKind', uploadForm.agentKind)
  fd.append('description', uploadForm.description.trim())
  uploading.value = true
  try {
    await uploadAgentPackage(fd)
    ElMessage.success('上传成功')
    showUpload.value = false
    await loadPackages()
  } finally {
    uploading.value = false
  }
}

async function toggleEnabled(row: AgentPackage, enabled: boolean) {
  await updateAgentPackage(row.id, { enabled })
  row.enabled = enabled
}

async function removePkg(row: AgentPackage) {
  await ElMessageBox.confirm(`删除安装包 ${row.name}？`, '确认')
  await deleteAgentPackage(row.id)
  ElMessage.success('已删除')
  await loadPackages()
}

async function openMapDialog() {
  showMap.value = true
  mapForm.instanceCode = instances.value[0]?.code || ''
  mapForm.proxyGroupId = ''
  mapForm.confSuffix = ''
  mapForm.jumpProxyId = ''
  mapForm.remark = ''
  if (mapForm.instanceCode) await loadGroupsForMap()
}

async function loadGroupsForMap() {
  mapForm.proxyGroupId = ''
  mapForm.jumpProxyId = ''
  if (!mapForm.instanceCode) {
    mapGroups.value = []
    return
  }
  mapGroups.value = await listProxyGroups(mapForm.instanceCode).catch(() => [])
}

function onMapGroupChange() {
  mapForm.jumpProxyId = ''
}

async function saveMap() {
  if (!mapForm.instanceCode || !mapForm.proxyGroupId || !mapForm.confSuffix.trim()) {
    ElMessage.warning('请填完整映射')
    return
  }
  mapSaving.value = true
  try {
    maps.value = await upsertProxyConfMap({
      instanceCode: mapForm.instanceCode,
      proxyGroupId: mapForm.proxyGroupId,
      confSuffix: mapForm.confSuffix.trim(),
      jumpProxyId: mapForm.jumpProxyId || undefined,
      remark: mapForm.remark.trim() || undefined,
    })
    ElMessage.success('已保存')
    showMap.value = false
  } finally {
    mapSaving.value = false
  }
}

async function removeMap(row: ProxyConfMap) {
  await ElMessageBox.confirm(`删除映射 ${row.confSuffix}？`, '确认')
  await deleteProxyConfMap(row.id)
  ElMessage.success('已删除')
  await loadMaps()
}

async function openJumpDialog(row?: MonitorSshJump) {
  showJump.value = true
  jumpEditId.value = row?.id ?? null
  jumpForm.instanceCode = row?.instanceCode || instances.value[0]?.code || ''
  jumpForm.proxyId = row?.proxyId || ''
  jumpForm.proxyName = row?.proxyName || ''
  jumpForm.jumpHost = row?.jumpHost || ''
  jumpForm.jumpPort = row?.jumpPort || 22
  jumpForm.credentialId = row?.credentialId ?? null
  jumpForm.enabled = row?.enabled ?? true
  jumpForm.remark = row?.remark || ''
  if (jumpForm.instanceCode) await loadProxiesForJump(false)
  if (!jumpForm.jumpHost && jumpForm.proxyId) onJumpProxyChange()
}

async function loadProxiesForJump(resetProxy = true) {
  if (resetProxy) {
    jumpForm.proxyId = ''
    jumpForm.proxyName = ''
    jumpForm.jumpHost = ''
  }
  if (!jumpForm.instanceCode) {
    jumpProxies.value = []
    return
  }
  jumpProxies.value = await listProxies(jumpForm.instanceCode).catch(() => [])
}

function onJumpProxyChange() {
  const p = jumpProxies.value.find((x) => x.proxyId === jumpForm.proxyId)
  if (!p) return
  jumpForm.proxyName = p.name
  if (!jumpForm.jumpHost.trim() && p.address) {
    jumpForm.jumpHost = p.address
  }
}

async function saveJump() {
  if (!jumpForm.instanceCode || !jumpForm.proxyId || !jumpForm.jumpHost.trim() || !jumpForm.credentialId) {
    ElMessage.warning('请填完整跳板配置')
    return
  }
  jumpSaving.value = true
  try {
    jumps.value = await upsertSshJump({
      instanceCode: jumpForm.instanceCode,
      proxyId: jumpForm.proxyId,
      proxyName: jumpForm.proxyName || undefined,
      jumpHost: jumpForm.jumpHost.trim(),
      jumpPort: jumpForm.jumpPort || 22,
      credentialId: jumpForm.credentialId,
      enabled: jumpForm.enabled,
      remark: jumpForm.remark.trim() || undefined,
    })
    ElMessage.success('已保存')
    showJump.value = false
  } finally {
    jumpSaving.value = false
  }
}

async function removeJump(row: MonitorSshJump) {
  await ElMessageBox.confirm(`删除跳板 ${row.proxyName || row.proxyId}？`, '确认')
  await deleteSshJump(row.id)
  ElMessage.success('已删除')
  await loadJumps()
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
  align-items: baseline;
  gap: 8px;
  font-size: 18px;
}
.page-sub {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.tab-toolbar {
  display: flex;
  justify-content: flex-end;
  margin-bottom: 12px;
}
.muted {
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
</style>
