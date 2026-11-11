<template>
  <div class="install-page">
    <div class="page-header">
      <div class="page-title">
        <span>安装 Agent</span>
        <span class="page-sub">SSH 安装 zabbix-agent2，登记主机后等待 agent.ping</span>
      </div>
    </div>

    <el-card shadow="never">
      <el-form label-width="120px">
        <el-form-item label="Zabbix 实例">
          <el-select v-model="instanceCode" style="width: 280px" @change="loadOptions">
            <el-option v-for="item in instances" :key="item.code" :label="item.name" :value="item.code" />
          </el-select>
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
            style="width: 480px"
          >
            <el-option v-for="item in ciOptions" :key="item.id" :label="item.name" :value="item.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="主机组">
          <el-select v-model="groupIds" multiple filterable style="width: 480px">
            <el-option v-for="item in hostGroups" :key="item.groupId" :label="item.name" :value="item.groupId" />
          </el-select>
        </el-form-item>
        <el-form-item label="模板">
          <el-select v-model="templateIds" multiple filterable style="width: 480px">
            <el-option v-for="item in templates" :key="item.templateId" :label="item.name" :value="item.templateId" />
          </el-select>
        </el-form-item>
        <el-form-item label="Server">
          <el-input v-model="server" placeholder="Zabbix Server 地址，如 10.0.0.8" style="width: 320px" />
        </el-form-item>
        <el-form-item label="ServerActive">
          <el-input v-model="serverActive" placeholder="默认同 Server" style="width: 320px" />
        </el-form-item>
        <el-form-item>
          <el-button v-permission="'monitor:agent'" type="primary" :loading="submitting" @click="submit">
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
  getTask,
  installAgents,
  listHostGroups,
  listInstances,
  listTemplates,
  type MonitorGroupRef,
  type MonitorInstance,
  type MonitorTask,
  type MonitorTemplate,
} from '../../api/monitor'

const instances = ref<MonitorInstance[]>([])
const instanceCode = ref('')
const hostGroups = ref<MonitorGroupRef[]>([])
const templates = ref<MonitorTemplate[]>([])
const ciIds = ref<string[]>([])
const ciOptions = ref<{ id: string; name: string }[]>([])
const ciLoading = ref(false)
const groupIds = ref<string[]>([])
const templateIds = ref<string[]>([])
const server = ref('')
const serverActive = ref('')
const submitting = ref(false)
const task = ref<MonitorTask | null>(null)
let pollTimer: number | undefined

onMounted(async () => {
  instances.value = await listInstances()
  instanceCode.value = instances.value[0]?.code || ''
  if (instanceCode.value) await loadOptions()
})

onBeforeUnmount(() => {
  if (pollTimer) window.clearInterval(pollTimer)
})

async function loadOptions() {
  if (!instanceCode.value) return
  const [groups, tpls] = await Promise.all([
    listHostGroups(instanceCode.value),
    listTemplates(instanceCode.value),
  ])
  hostGroups.value = groups
  templates.value = tpls
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
  if (!instanceCode.value || !ciIds.value.length || !groupIds.value.length || !templateIds.value.length || !server.value.trim()) {
    ElMessage.warning('请填实例、配置项、主机组、模板和 Server 地址')
    return
  }
  submitting.value = true
  try {
    const res = await installAgents({
      instanceCode: instanceCode.value,
      ciIds: ciIds.value,
      groupIds: groupIds.value,
      templateIds: templateIds.value,
      server: server.value.trim(),
      serverActive: serverActive.value.trim() || undefined,
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
.result {
  margin-top: 16px;
}
</style>
