<template>
  <div class="scheme-page">
    <div class="page-header">
      <div class="page-title">
        <span>改方案</span>
        <span class="page-sub">主机组、模板组和宏直接写回 Zabbix</span>
      </div>
      <el-select v-model="instanceCode" placeholder="Zabbix 实例" style="width: 220px" @change="onInstanceChange">
        <el-option v-for="item in instances" :key="item.code" :label="item.name" :value="item.code" />
      </el-select>
    </div>

    <el-row :gutter="16">
      <el-col :span="7">
        <el-card shadow="never">
          <el-radio-group v-model="side" size="small" @change="onSideChange">
            <el-radio-button value="host">主机组</el-radio-button>
            <el-radio-button value="template">模板组</el-radio-button>
          </el-radio-group>
          <el-table
            v-loading="groupLoading"
            :data="side === 'host' ? hostGroups : templateGroups"
            height="520"
            highlight-current-row
            size="small"
            class="group-table"
            @current-change="onGroup"
          >
            <el-table-column prop="name" label="名称" min-width="140" />
          </el-table>
        </el-card>
      </el-col>
      <el-col :span="17">
        <el-card shadow="never">
          <template v-if="side === 'host'">
            <div class="toolbar">
              <span>{{ currentGroup?.name || '请选择主机组' }}</span>
              <div>
                <el-button v-permission="'monitor:operate'" :disabled="!selectedHost" @click="openRegister">
                  登记配置项
                </el-button>
              </div>
            </div>
            <el-table
              v-loading="listLoading"
              :data="groupHosts"
              height="240"
              highlight-current-row
              size="small"
              @current-change="onHost"
            >
              <el-table-column prop="displayName" label="名称" min-width="140" />
              <el-table-column prop="hostname" label="主机名" min-width="140" />
              <el-table-column prop="ip" label="IP" width="140" />
              <el-table-column label="主机组" min-width="180">
                <template #default="{ row }">{{ groupNames(row.groups) }}</template>
              </el-table-column>
            </el-table>

            <div v-if="selectedHost" class="host-editor">
              <h4>{{ selectedHost.displayName || selectedHost.hostname }}</h4>
              <div class="block">
                <div class="block-title">已套模板</div>
                <el-tag
                  v-for="item in linkage.templates"
                  :key="item.templateId"
                  class="tag"
                  :closable="canOperate"
                  :disable-transitions="true"
                  @close="changeTemplates('unlink', [item.templateId])"
                >
                  {{ item.name }}
                </el-tag>
                <el-select v-model="templateToAdd" filterable placeholder="套用模板" style="width: 260px">
                  <el-option v-for="item in allTemplates" :key="item.templateId" :label="item.name" :value="item.templateId" />
                </el-select>
                <el-button
                  v-permission="'monitor:operate'"
                  type="primary"
                  :disabled="!templateToAdd"
                  @click="changeTemplates('link', [templateToAdd])"
                >
                  套用
                </el-button>
              </div>
              <div class="block">
                <div class="block-title">所属主机组</div>
                <el-tag
                  v-for="item in linkage.groups"
                  :key="item.groupId"
                  class="tag"
                  :closable="canOperate"
                  @close="changeGroups('unlink', [item.groupId])"
                >
                  {{ item.name }}
                </el-tag>
                <el-select v-model="groupToAdd" filterable placeholder="加入主机组" style="width: 220px">
                  <el-option v-for="item in hostGroups" :key="item.groupId" :label="item.name" :value="item.groupId" />
                </el-select>
                <el-button
                  v-permission="'monitor:operate'"
                  :disabled="!groupToAdd"
                  @click="changeGroups('link', [groupToAdd])"
                >
                  加入
                </el-button>
              </div>
              <div class="block">
                <div class="block-title">主机宏</div>
                <el-table :data="macros" size="small">
                  <el-table-column prop="macro" label="宏" min-width="180" />
                  <el-table-column label="值" min-width="160">
                    <template #default="{ row }">
                      <el-input v-model="row.value" :placeholder="row.secret ? '密钥留空则不修改' : ''" />
                    </template>
                  </el-table-column>
                  <el-table-column prop="description" label="说明" min-width="140" />
                </el-table>
                <div class="macro-add">
                  <el-input v-model="newMacro.macro" placeholder="{$MACRO}" style="width: 180px" />
                  <el-input v-model="newMacro.value" placeholder="值" style="width: 160px" />
                  <el-button @click="addMacro">添加</el-button>
                  <el-button v-permission="'monitor:operate'" type="primary" :loading="saving" @click="saveMacros">
                    保存宏
                  </el-button>
                </div>
              </div>
            </div>
          </template>
          <template v-else>
            <div class="toolbar">
              <span>{{ currentGroup?.name || '请选择模板组' }}</span>
            </div>
            <el-table v-loading="listLoading" :data="templates" height="520" size="small">
              <el-table-column prop="name" label="模板" min-width="180" />
              <el-table-column prop="host" label="技术名" min-width="160" />
              <el-table-column label="上级模板" min-width="180">
                <template #default="{ row }">{{ groupNames(row.parentTemplates) }}</template>
              </el-table-column>
            </el-table>
          </template>
        </el-card>
      </el-col>
    </el-row>

    <el-dialog v-model="registerVisible" title="把配置项登记到 Zabbix" width="520px">
      <el-form label-width="90px">
        <el-form-item label="配置项">
          <el-select
            v-model="registerCiId"
            filterable
            remote
            reserve-keyword
            placeholder="搜索资产名称"
            :remote-method="searchCi"
            :loading="ciLoading"
            style="width: 100%"
          >
            <el-option v-for="item in ciOptions" :key="item.id" :label="item.name" :value="item.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="主机组">
          <el-select v-model="registerGroups" multiple filterable style="width: 100%">
            <el-option v-for="item in hostGroups" :key="item.groupId" :label="item.name" :value="item.groupId" />
          </el-select>
        </el-form-item>
        <el-form-item label="模板">
          <el-select v-model="registerTemplates" multiple filterable style="width: 100%">
            <el-option v-for="item in allTemplates" :key="item.templateId" :label="item.name" :value="item.templateId" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="registerVisible = false">取消</el-button>
        <el-button type="primary" :loading="registering" @click="submitRegister">登记</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { useUserStore } from '../../stores/user'
import { listCiInstances } from '../../api/cmdb'
import {
  hostLinkage,
  hostMacros,
  hostsInGroup,
  linkHostGroups,
  linkHostTemplates,
  listHostGroups,
  listInstances,
  listTemplateGroups,
  listTemplates,
  registerMonitorHost,
  saveHostMacros,
  type HostMacro,
  type MonitorGroupRef,
  type MonitorHost,
  type MonitorInstance,
  type MonitorTemplate,
  type MonitorTemplateRef,
} from '../../api/monitor'

const user = useUserStore()
const canOperate = computed(() => user.hasPermission('monitor:operate'))
const instances = ref<MonitorInstance[]>([])
const instanceCode = ref('')
const side = ref<'host' | 'template'>('host')
const hostGroups = ref<MonitorGroupRef[]>([])
const templateGroups = ref<MonitorGroupRef[]>([])
const groupLoading = ref(false)
const listLoading = ref(false)
const currentGroup = ref<MonitorGroupRef | null>(null)
const groupHosts = ref<MonitorHost[]>([])
const templates = ref<MonitorTemplate[]>([])
const allTemplates = ref<MonitorTemplate[]>([])
const selectedHost = ref<MonitorHost | null>(null)
const linkage = ref<{ groups: MonitorGroupRef[]; templates: MonitorTemplateRef[] }>({ groups: [], templates: [] })
const macros = ref<HostMacro[]>([])
const templateToAdd = ref('')
const groupToAdd = ref('')
const saving = ref(false)
const newMacro = reactive({ macro: '', value: '' })
const registerVisible = ref(false)
const registerCiId = ref('')
const registerGroups = ref<string[]>([])
const registerTemplates = ref<string[]>([])
const ciOptions = ref<{ id: string; name: string }[]>([])
const ciLoading = ref(false)
const registering = ref(false)

onMounted(async () => {
  instances.value = await listInstances()
  instanceCode.value = instances.value[0]?.code || ''
  if (instanceCode.value) await loadGroups()
})

async function onInstanceChange() {
  currentGroup.value = null
  selectedHost.value = null
  groupHosts.value = []
  templates.value = []
  await loadGroups()
}

async function loadGroups() {
  if (!instanceCode.value) return
  groupLoading.value = true
  try {
    const [hosts, tpls, all] = await Promise.all([
      listHostGroups(instanceCode.value),
      listTemplateGroups(instanceCode.value),
      listTemplates(instanceCode.value),
    ])
    hostGroups.value = hosts
    templateGroups.value = tpls
    allTemplates.value = all
  } finally {
    groupLoading.value = false
  }
}

function onSideChange() {
  currentGroup.value = null
  selectedHost.value = null
}

async function onGroup(row: MonitorGroupRef | undefined) {
  if (!row || !instanceCode.value) return
  currentGroup.value = row
  selectedHost.value = null
  listLoading.value = true
  try {
    if (side.value === 'host') {
      groupHosts.value = await hostsInGroup(instanceCode.value, row.groupId)
    } else {
      templates.value = await listTemplates(instanceCode.value, row.groupId)
    }
  } finally {
    listLoading.value = false
  }
}

async function onHost(row: MonitorHost | undefined) {
  if (!row || !instanceCode.value) return
  selectedHost.value = row
  const [link, items] = await Promise.all([
    hostLinkage(instanceCode.value, row.hostId),
    hostMacros(instanceCode.value, row.hostId),
  ])
  linkage.value = link
  macros.value = items
}

async function changeTemplates(action: 'link' | 'unlink', templateIds: string[]) {
  if (!selectedHost.value || !instanceCode.value || !templateIds.length) return
  linkage.value = await linkHostTemplates(instanceCode.value, selectedHost.value.hostId, action, templateIds)
  templateToAdd.value = ''
  ElMessage.success(action === 'link' ? '已套用模板' : '已卸下模板')
}

async function changeGroups(action: 'link' | 'unlink', groupIds: string[]) {
  if (!selectedHost.value || !instanceCode.value || !groupIds.length) return
  linkage.value = await linkHostGroups(instanceCode.value, selectedHost.value.hostId, action, groupIds)
  groupToAdd.value = ''
  ElMessage.success(action === 'link' ? '已加入主机组' : '已移出主机组')
}

function addMacro() {
  const name = newMacro.macro.trim()
  if (!name) return
  macros.value.push({
    hostMacroId: '',
    macro: name,
    value: newMacro.value,
    description: '',
    secret: false,
  })
  newMacro.macro = ''
  newMacro.value = ''
}

async function saveMacros() {
  if (!selectedHost.value || !instanceCode.value) return
  saving.value = true
  try {
    macros.value = await saveHostMacros(
      instanceCode.value,
      selectedHost.value.hostId,
      macros.value.map((item) => ({ macro: item.macro, value: item.value, description: item.description })),
    )
    ElMessage.success('宏已保存')
  } finally {
    saving.value = false
  }
}

function openRegister() {
  registerCiId.value = ''
  registerGroups.value = currentGroup.value ? [currentGroup.value.groupId] : []
  registerTemplates.value = []
  registerVisible.value = true
}

async function searchCi(keyword: string) {
  ciLoading.value = true
  try {
    const page = await listCiInstances({ keyword, page: 1, pageSize: 20 })
    ciOptions.value = page.items.map((item) => ({ id: item.id, name: item.name }))
  } finally {
    ciLoading.value = false
  }
}

async function submitRegister() {
  if (!instanceCode.value || !registerCiId.value || !registerGroups.value.length) {
    ElMessage.warning('请选择配置项和主机组')
    return
  }
  registering.value = true
  try {
    const res = await registerMonitorHost(instanceCode.value, {
      ciId: registerCiId.value,
      groupIds: registerGroups.value,
      templateIds: registerTemplates.value,
    })
    ElMessage.success(res.created ? '已创建 Zabbix 主机' : '已更新已有 Zabbix 主机')
    registerVisible.value = false
    if (currentGroup.value) await onGroup(currentGroup.value)
  } finally {
    registering.value = false
  }
}

function groupNames(items?: Array<{ name: string }>) {
  return (items || []).map((item) => item.name).join('、') || '-'
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
.group-table {
  margin-top: 12px;
}
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.host-editor {
  margin-top: 16px;
}
.block {
  margin-top: 12px;
}
.block-title {
  margin-bottom: 8px;
  font-weight: 600;
}
.tag {
  margin: 0 8px 8px 0;
}
.macro-add {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}
</style>
