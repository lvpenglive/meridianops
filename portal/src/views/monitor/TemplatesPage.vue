<template>
  <div class="templates-page">
    <div class="page-header">
      <div class="page-title">
        <span>模板库</span>
        <span class="page-sub">按模板组查看。生产实例的修改会先生成标准变更工单</span>
      </div>
    </div>

    <el-alert
      v-if="current?.protected"
      type="warning"
      show-icon
      :closable="false"
      title="该实例已保护。保存、导入和删除会提交变更工单，工单关闭后在下方执行。"
      class="banner"
    />

    <el-row :gutter="16">
      <el-col :span="7">
        <el-card shadow="never">
          <el-form label-width="72px">
            <el-form-item label="实例">
              <el-select v-model="instanceCode" style="width: 100%" @change="loadAll">
                <el-option v-for="item in instances" :key="item.code" :label="item.name" :value="item.code" />
              </el-select>
            </el-form-item>
          </el-form>
          <div class="group-head">
            <span>模板组</span>
            <el-button v-permission="'monitor:template'" link type="primary" @click="openGroup('template')">新建</el-button>
          </div>
          <el-menu :default-active="groupId" @select="groupId = $event">
            <el-menu-item index="">全部</el-menu-item>
            <el-menu-item v-for="item in templateGroups" :key="item.groupId" :index="item.groupId">
              <span>{{ item.name }}</span>
              <el-button v-permission="'monitor:template'" link @click.stop="renameGroup('template', item)">改名</el-button>
              <el-button v-permission="'monitor:template'" link type="danger" @click.stop="removeGroup('template', item)">删除</el-button>
            </el-menu-item>
          </el-menu>
          <div class="group-head">
            <span>主机组</span>
            <el-button v-permission="'monitor:template'" link type="primary" @click="openGroup('host')">新建</el-button>
          </div>
          <div v-for="item in hostGroups" :key="item.groupId" class="host-group">
            <span>{{ item.name }}</span>
            <span>
              <el-button v-permission="'monitor:template'" link @click="renameGroup('host', item)">改名</el-button>
              <el-button v-permission="'monitor:template'" link type="danger" @click="removeGroup('host', item)">删除</el-button>
            </span>
          </div>
        </el-card>
      </el-col>
      <el-col :span="17">
        <el-card shadow="never">
          <div class="toolbar">
            <el-input v-model="keyword" placeholder="搜索模板名称" clearable style="width: 240px" />
            <el-button v-permission="'monitor:template'" type="primary" @click="openTemplate">新建模板</el-button>
            <el-button v-permission="'monitor:template'" @click="exportSelected">导出</el-button>
            <el-button v-permission="'monitor:template'" @click="importVisible = true">导入</el-button>
          </div>
          <el-table :data="filtered" size="small" @selection-change="selected = $event">
            <el-table-column type="selection" width="42" />
            <el-table-column prop="name" label="名称" min-width="180" />
            <el-table-column prop="host" label="技术名" min-width="160" />
            <el-table-column label="模板组" min-width="160">
              <template #default="{ row }">{{ groupNames(row) }}</template>
            </el-table-column>
            <el-table-column label="" width="90">
              <template #default="{ row }">
                <el-button link type="primary" @click="openDesigner(row.templateId)">设计</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
        <el-card v-if="changes.length" shadow="never" class="changes">
          <template #header>待执行的模板变更</template>
          <el-table :data="changes" size="small">
            <el-table-column prop="title" label="标题" min-width="180" />
            <el-table-column prop="status" label="状态" width="100" />
            <el-table-column prop="ticketId" label="工单" min-width="180" />
            <el-table-column width="90">
              <template #default="{ row }">
                <el-button
                  v-if="row.status === 'pending'"
                  v-permission="'monitor:template'"
                  link
                  type="primary"
                  @click="apply(row.id)"
                >执行</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
    </el-row>

    <el-dialog v-model="groupVisible" :title="groupKind === 'template' ? '模板组' : '主机组'" width="420px">
      <el-input v-model="groupName" placeholder="组名称" />
      <template #footer>
        <el-button @click="groupVisible = false">取消</el-button>
        <el-button type="primary" @click="saveGroup">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="templateVisible" title="新建模板" width="520px">
      <el-form label-width="88px">
        <el-form-item label="技术名"><el-input v-model="draft.host" /></el-form-item>
        <el-form-item label="名称"><el-input v-model="draft.name" /></el-form-item>
        <el-form-item label="模板组">
          <el-select v-model="draft.groupIds" multiple style="width: 100%">
            <el-option v-for="item in templateGroups" :key="item.groupId" :label="item.name" :value="item.groupId" />
          </el-select>
        </el-form-item>
        <el-form-item label="上级模板">
          <el-select v-model="draft.parentTemplateIds" multiple filterable style="width: 100%">
            <el-option v-for="item in templates" :key="item.templateId" :label="item.name" :value="item.templateId" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="templateVisible = false">取消</el-button>
        <el-button type="primary" @click="saveTemplate">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="importVisible" title="导入模板 JSON" width="640px">
      <el-input v-model="importSource" type="textarea" :rows="12" placeholder="粘贴 configuration.export 的 JSON" />
      <template #footer>
        <el-button @click="importVisible = false">取消</el-button>
        <el-button type="primary" @click="doImport">导入</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  applyTemplateChange,
  createDesignerTemplate,
  createMonitorGroup,
  deleteMonitorGroup,
  exportDesignerTemplates,
  importDesignerTemplates,
  listHostGroups,
  listInstances,
  listTemplateChanges,
  listTemplateGroups,
  listTemplates,
  renameMonitorGroup,
  submitTemplateChange,
  type MonitorGroupRef,
  type MonitorInstance,
  type MonitorTemplate,
  type TemplateChange,
} from '../../api/monitor'

const router = useRouter()
const instances = ref<MonitorInstance[]>([])
const instanceCode = ref('')
const templateGroups = ref<MonitorGroupRef[]>([])
const hostGroups = ref<MonitorGroupRef[]>([])
const templates = ref<MonitorTemplate[]>([])
const changes = ref<TemplateChange[]>([])
const groupId = ref('')
const keyword = ref('')
const selected = ref<MonitorTemplate[]>([])
const groupVisible = ref(false)
const groupKind = ref<'template' | 'host'>('template')
const groupName = ref('')
const editingGroup = ref<MonitorGroupRef | null>(null)
const templateVisible = ref(false)
const importVisible = ref(false)
const importSource = ref('')
const draft = ref({ host: '', name: '', groupIds: [] as string[], parentTemplateIds: [] as string[] })

const current = computed(() => instances.value.find((item) => item.code === instanceCode.value))

function groupNames(row: MonitorTemplate) {
  return (row.groups || []).map((group) => group.name).join('、')
}
const filtered = computed(() => {
  const q = keyword.value.trim().toLowerCase()
  return templates.value.filter((item) => {
    const inGroup = !groupId.value || (item.groups || []).some((group) => group.groupId === groupId.value)
    const matched = !q || item.name.toLowerCase().includes(q) || item.host.toLowerCase().includes(q)
    return inGroup && matched
  })
})

onMounted(async () => {
  instances.value = await listInstances()
  instanceCode.value = instances.value[0]?.code || ''
  if (instanceCode.value) await loadAll()
})

async function loadAll() {
  if (!instanceCode.value) return
  const [groups, hosts, rows, pending] = await Promise.all([
    listTemplateGroups(instanceCode.value),
    listHostGroups(instanceCode.value),
    listTemplates(instanceCode.value),
    listTemplateChanges(instanceCode.value),
  ])
  templateGroups.value = groups
  hostGroups.value = hosts
  templates.value = rows
  changes.value = pending
}

function openDesigner(id: string) {
  router.push({ name: 'MonitorTemplateDesigner', params: { id }, query: { code: instanceCode.value } })
}

function openGroup(kind: 'template' | 'host') {
  groupKind.value = kind
  editingGroup.value = null
  groupName.value = ''
  groupVisible.value = true
}

function renameGroup(kind: 'template' | 'host', item: MonitorGroupRef) {
  groupKind.value = kind
  editingGroup.value = item
  groupName.value = item.name
  groupVisible.value = true
}

async function saveGroup() {
  const name = groupName.value.trim()
  if (!name) return
  if (current.value?.protected) {
    const resource = editingGroup.value
      ? `/${groupKind.value === 'template' ? 'template-groups' : 'host-groups'}/${editingGroup.value.groupId}`
      : `/${groupKind.value === 'template' ? 'template-groups' : 'host-groups'}`
    const res = await submitTemplateChange(instanceCode.value, {
      title: `${editingGroup.value ? '改名' : '新建'}${groupKind.value === 'template' ? '模板组' : '主机组'} ${name}`,
      method: editingGroup.value ? 'PUT' : 'POST',
      resource,
      body: { name },
    })
    ElMessage.success(`已提交工单 ${res.ticketNo}`)
  } else if (editingGroup.value) {
    await renameMonitorGroup(instanceCode.value, groupKind.value, editingGroup.value.groupId, name)
    ElMessage.success('已改名')
  } else {
    await createMonitorGroup(instanceCode.value, groupKind.value, name)
    ElMessage.success('已创建')
  }
  groupVisible.value = false
  await loadAll()
}

async function removeGroup(kind: 'template' | 'host', item: MonitorGroupRef) {
  await ElMessageBox.confirm(`删除空组「${item.name}」？组内还有对象时会失败。`, '删除组')
  if (current.value?.protected) {
    const res = await submitTemplateChange(instanceCode.value, {
      title: `删除${kind === 'template' ? '模板组' : '主机组'} ${item.name}`,
      method: 'DELETE',
      resource: `/${kind === 'template' ? 'template-groups' : 'host-groups'}/${item.groupId}`,
    })
    ElMessage.success(`已提交工单 ${res.ticketNo}`)
  } else {
    await deleteMonitorGroup(instanceCode.value, kind, item.groupId)
    ElMessage.success('已删除')
  }
  await loadAll()
}

function openTemplate() {
  draft.value = { host: '', name: '', groupIds: groupId.value ? [groupId.value] : [], parentTemplateIds: [] }
  templateVisible.value = true
}

async function saveTemplate() {
  if (!draft.value.host.trim() || !draft.value.name.trim() || !draft.value.groupIds.length) {
    ElMessage.warning('请填写技术名、名称和模板组')
    return
  }
  if (current.value?.protected) {
    const res = await submitTemplateChange(instanceCode.value, {
      title: `新建模板 ${draft.value.name}`,
      method: 'POST',
      resource: '/templates',
      body: draft.value,
    })
    ElMessage.success(`已提交工单 ${res.ticketNo}`)
  } else {
    const created = await createDesignerTemplate(instanceCode.value, draft.value)
    ElMessage.success('模板已创建')
    openDesigner(created.templateId)
  }
  templateVisible.value = false
  await loadAll()
}

async function exportSelected() {
  if (!selected.value.length) {
    ElMessage.warning('请先勾选模板')
    return
  }
  const data = await exportDesignerTemplates(instanceCode.value, selected.value.map((item) => item.templateId))
  const blob = new Blob([data.source], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `templates-${instanceCode.value}.json`
  link.click()
  URL.revokeObjectURL(url)
}

async function doImport() {
  if (!importSource.value.trim()) return
  if (current.value?.protected) {
    const res = await submitTemplateChange(instanceCode.value, {
      title: '导入模板',
      method: 'POST',
      resource: '/templates/import',
      body: { source: importSource.value },
    })
    ElMessage.success(`已提交工单 ${res.ticketNo}`)
  } else {
    await importDesignerTemplates(instanceCode.value, importSource.value)
    ElMessage.success('已导入')
  }
  importVisible.value = false
  importSource.value = ''
  await loadAll()
}

async function apply(id: string) {
  await applyTemplateChange(id)
  ElMessage.success('变更已写入 Zabbix')
  await loadAll()
}
</script>

<style scoped>
.page-header { margin-bottom: 16px; }
.page-title { display: flex; align-items: baseline; gap: 8px; font-size: 18px; }
.page-sub { color: var(--el-text-color-secondary); font-size: 13px; }
.banner { margin-bottom: 16px; }
.group-head, .host-group, .toolbar { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
.group-head { margin: 8px 0; font-weight: 600; }
.host-group { padding: 6px 0; }
.toolbar { margin-bottom: 12px; }
.changes { margin-top: 16px; }
</style>
