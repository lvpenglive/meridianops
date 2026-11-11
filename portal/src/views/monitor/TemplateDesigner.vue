<template>
  <div class="designer-page" v-loading="loading">
    <div class="page-header">
      <div class="page-title">
        <el-button link @click="router.push('/monitor/templates')">模板库</el-button>
        <span>{{ template?.name || '模板' }}</span>
        <span class="page-sub">{{ template?.host }}</span>
      </div>
    </div>
    <el-alert
      v-if="protectedInstance"
      type="warning"
      show-icon
      :closable="false"
      title="该实例已保护。这里的保存会提交标准变更工单，不会立刻改 Zabbix。"
      class="banner"
    />
    <el-tabs v-model="tab">
      <el-tab-pane label="基本" name="base">
        <el-form v-if="template" label-width="96px" :disabled="!canWrite">
          <el-form-item label="技术名"><el-input v-model="template.host" style="width: 360px" /></el-form-item>
          <el-form-item label="名称"><el-input v-model="template.name" style="width: 360px" /></el-form-item>
          <el-form-item label="模板组">
            <el-select v-model="groupIds" multiple style="width: 420px">
              <el-option v-for="item in templateGroups" :key="item.groupId" :label="item.name" :value="item.groupId" />
            </el-select>
          </el-form-item>
          <el-form-item label="上级模板">
            <el-select v-model="parentIds" multiple filterable style="width: 420px">
              <el-option v-for="item in allTemplates" :key="item.templateId" :label="item.name" :value="item.templateId" />
            </el-select>
          </el-form-item>
          <el-form-item>
            <el-button v-permission="'monitor:template'" type="primary" @click="saveBase">保存</el-button>
            <el-button v-permission="'monitor:template'" type="danger" @click="removeTemplate">删除模板</el-button>
          </el-form-item>
        </el-form>
      </el-tab-pane>
      <el-tab-pane label="监控项" name="items">
        <el-button v-permission="'monitor:template'" type="primary" @click="openItem()">新建监控项</el-button>
        <el-table :data="items" size="small" class="table">
          <el-table-column prop="name" label="名称" min-width="160" />
          <el-table-column prop="key" label="键" min-width="180" />
          <el-table-column prop="typeName" label="类型" width="140" />
          <el-table-column label="可编辑" width="90">
            <template #default="{ row }">{{ row.editable ? '是' : '只读' }}</template>
          </el-table-column>
          <el-table-column width="120">
            <template #default="{ row }">
              <el-button link @click="openItem(row)">{{ row.editable && canWrite ? '编辑' : '查看' }}</el-button>
              <el-button v-if="row.editable" v-permission="'monitor:template'" link type="danger" @click="removeItem(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
      <el-tab-pane label="触发器" name="triggers">
        <el-button v-permission="'monitor:template'" type="primary" @click="openTrigger()">新建触发器</el-button>
        <el-table :data="triggers" size="small" class="table">
          <el-table-column prop="description" label="名称" min-width="180" />
          <el-table-column prop="expression" label="表达式" min-width="240" />
          <el-table-column label="级别" width="100">
            <template #default="{ row }">{{ severityName(row.priority) }}</template>
          </el-table-column>
          <el-table-column width="120">
            <template #default="{ row }">
              <el-button link @click="openTrigger(row)">{{ row.editable && canWrite ? '编辑' : '查看' }}</el-button>
              <el-button v-if="row.editable" v-permission="'monitor:template'" link type="danger" @click="removeTrigger(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
      <el-tab-pane label="发现规则" name="lld">
        <el-button v-permission="'monitor:template'" type="primary" @click="openRule()">新建发现规则</el-button>
        <el-table :data="rules" size="small" class="table">
          <el-table-column prop="name" label="名称" min-width="160" />
          <el-table-column prop="key" label="键" min-width="180" />
          <el-table-column prop="typeName" label="类型" width="140" />
          <el-table-column label="可编辑" width="90">
            <template #default="{ row }">{{ row.editable ? '是' : '只读' }}</template>
          </el-table-column>
          <el-table-column width="160">
            <template #default="{ row }">
              <el-button link @click="openRule(row)">{{ row.editable && canWrite ? '编辑' : '查看' }}</el-button>
              <el-button v-if="row.editable" v-permission="'monitor:template'" link type="danger" @click="removeRule(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>
      <el-tab-pane label="宏" name="macros">
        <el-table :data="macros" size="small">
          <el-table-column label="宏" min-width="180">
            <template #default="{ row }"><el-input v-model="row.macro" :disabled="!canWrite" /></template>
          </el-table-column>
          <el-table-column label="值" min-width="180">
            <template #default="{ row }">
              <el-input v-model="row.value" :disabled="!canWrite" :placeholder="row.secret ? '留空则不覆盖' : ''" />
            </template>
          </el-table-column>
          <el-table-column label="说明" min-width="180">
            <template #default="{ row }"><el-input v-model="row.description" :disabled="!canWrite" /></template>
          </el-table-column>
        </el-table>
        <div class="actions">
          <el-button v-permission="'monitor:template'" @click="macros.push({ hostMacroId: '', macro: '{$}', value: '', description: '', secret: false })">添加宏</el-button>
          <el-button v-permission="'monitor:template'" type="primary" @click="saveMacros">保存宏</el-button>
        </div>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="itemVisible" :title="prototypeMode ? '监控项原型' : '监控项'" width="640px">
      <el-form label-width="108px" :disabled="itemReadonly">
        <el-form-item label="类型">
          <el-select v-model="item.type" style="width: 100%" :disabled="Boolean(item.itemId)">
            <el-option v-for="kind in editableTypes" :key="kind.type" :label="kind.name" :value="kind.type" />
          </el-select>
          <span v-if="item.itemId && !item.editable" class="hint">{{ item.typeName }}，只读</span>
        </el-form-item>
        <el-form-item v-if="showField('name')" label="名称"><el-input v-model="item.name" /></el-form-item>
        <el-form-item v-if="showField('key')" label="键"><el-input v-model="item.key" /></el-form-item>
        <el-form-item v-if="showField('delay')" label="更新间隔"><el-input v-model="item.delay" placeholder="如 1m" /></el-form-item>
        <el-form-item v-if="showField('valueType')" label="值类型">
          <el-select v-model="item.valueType" style="width: 100%">
            <el-option v-for="kind in catalog?.valueTypes || []" :key="kind.value" :label="kind.name" :value="kind.value" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="showField('units')" label="单位"><el-input v-model="item.units" /></el-form-item>
        <el-form-item v-if="showField('snmpOid')" label="SNMP OID"><el-input v-model="item.snmpOid" /></el-form-item>
        <el-form-item v-if="showField('masterItemId')" label="主监控项">
          <el-select v-model="item.masterItemId" filterable style="width: 100%">
            <el-option v-for="row in items" :key="row.itemId" :label="row.name" :value="row.itemId" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="showField('params')" label="计算公式"><el-input v-model="item.params" type="textarea" :rows="3" /></el-form-item>
        <el-form-item v-if="showField('url')" label="URL"><el-input v-model="item.url" /></el-form-item>
        <el-form-item v-if="showField('requestMethod')" label="HTTP 方法">
          <el-select v-model="item.requestMethod">
            <el-option label="GET" :value="0" />
            <el-option label="POST" :value="1" />
            <el-option label="PUT" :value="2" />
            <el-option label="HEAD" :value="3" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="showField('statusCodes')" label="状态码"><el-input v-model="item.statusCodes" /></el-form-item>
        <el-form-item v-if="showField('preprocessing')" label="预处理">
          <div v-for="(step, index) in item.preprocessing" :key="index" class="step">
            <el-select v-model="step.type" style="width: 180px">
              <el-option v-for="kind in catalog?.preprocessing || []" :key="kind.type" :label="kind.name" :value="kind.type" />
            </el-select>
            <el-input v-model="step.params" :placeholder="stepHint(step.type)" />
            <el-button link type="danger" @click="item.preprocessing.splice(index, 1)">移除</el-button>
          </div>
          <el-button link type="primary" @click="item.preprocessing.push({ type: 11, params: '', errorHandler: 0, errorHandlerParams: '' })">添加一步</el-button>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="itemVisible = false">关闭</el-button>
        <el-button v-if="!itemReadonly" type="primary" @click="saveItem">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="ruleVisible" :title="rule.ruleId ? '发现规则' : '新建发现规则'" width="760px">
      <el-form label-width="108px" :disabled="ruleReadonly">
        <el-form-item label="名称"><el-input v-model="rule.name" /></el-form-item>
        <el-form-item label="键"><el-input v-model="rule.key" placeholder="如 vfs.fs.dependent.discovery" /></el-form-item>
        <el-form-item label="类型">
          <el-select v-model="rule.type" :disabled="Boolean(rule.ruleId)">
            <el-option v-for="kind in editableTypes" :key="kind.type" :label="kind.name" :value="kind.type" />
          </el-select>
        </el-form-item>
        <el-form-item label="更新间隔"><el-input v-model="rule.delay" placeholder="1h" style="width: 160px" /></el-form-item>
        <el-form-item label="保留">
          <el-select v-model="rule.lifetimeType" style="width: 160px">
            <el-option label="到期删除" :value="0" />
            <el-option label="不删除" :value="1" />
            <el-option label="立即删除" :value="2" />
          </el-select>
          <el-input v-if="rule.lifetimeType === 0" v-model="rule.lifetime" placeholder="7d" style="width: 120px; margin-left: 8px" />
        </el-form-item>
        <el-form-item label="过滤">
          <el-select v-model="rule.filterEvalType" style="width: 140px">
            <el-option label="与/或" :value="0" />
            <el-option label="并且" :value="1" />
            <el-option label="或者" :value="2" />
          </el-select>
          <div v-for="(condition, index) in rule.filter" :key="index" class="step">
            <el-input v-model="condition.macro" placeholder="{#FSNAME}" style="width: 140px" />
            <el-select v-model="condition.operator" style="width: 120px">
              <el-option label="匹配" :value="8" />
              <el-option label="不匹配" :value="9" />
            </el-select>
            <el-input v-model="condition.value" placeholder="@File systems for discovery" />
            <el-button link type="danger" @click="rule.filter.splice(index, 1)">移除</el-button>
          </div>
          <el-button link type="primary" @click="rule.filter.push({ macro: '{#FSNAME}', operator: 8, value: '' })">添加条件</el-button>
        </el-form-item>
        <el-form-item v-if="rule.ruleId" label="覆盖">
          <div v-for="(override, index) in rule.overrides" :key="index" class="override">
            <el-input v-model="override.name" placeholder="名称" style="width: 140px" />
            <el-input-number v-model="override.step" :min="1" :max="99" />
            <el-checkbox v-model="override.stop">停止后续</el-checkbox>
            <div v-for="(condition, cIndex) in override.filter" :key="cIndex" class="step">
              <el-input v-model="condition.macro" placeholder="{#FSNAME}" style="width: 140px" />
              <el-select v-model="condition.operator" style="width: 110px">
                <el-option label="匹配" :value="8" />
                <el-option label="不匹配" :value="9" />
              </el-select>
              <el-input v-model="condition.value" placeholder="过滤值" />
              <el-button link type="danger" @click="override.filter.splice(cIndex, 1)">移除条件</el-button>
            </div>
            <el-button link @click="override.filter.push({ macro: '{#FSNAME}', operator: 8, value: '' })">添加过滤</el-button>
            <div v-for="(op, opIndex) in override.operations" :key="opIndex" class="step">
              <el-select v-model="op.operationObject" style="width: 140px">
                <el-option label="监控项原型" :value="0" />
                <el-option label="触发器原型" :value="1" />
              </el-select>
              <el-select v-model="op.operator" style="width: 110px">
                <el-option label="匹配" :value="8" />
                <el-option label="等于" :value="0" />
                <el-option label="包含" :value="2" />
              </el-select>
              <el-input v-model="op.value" placeholder="名称匹配" />
              <el-select v-model="op.status" clearable placeholder="状态" style="width: 110px">
                <el-option label="启用" :value="0" />
                <el-option label="禁用" :value="1" />
              </el-select>
              <el-select v-model="op.discover" clearable placeholder="发现" style="width: 110px">
                <el-option label="发现" :value="0" />
                <el-option label="不发现" :value="1" />
              </el-select>
              <el-button link type="danger" @click="override.operations.splice(opIndex, 1)">移除</el-button>
            </div>
            <el-button link @click="override.operations.push({ operationObject: 0, operator: 8, value: '', status: 1, discover: null })">添加动作</el-button>
            <el-button link type="danger" @click="rule.overrides.splice(index, 1)">删除这条覆盖</el-button>
          </div>
          <el-button link type="primary" @click="addOverride">添加覆盖</el-button>
        </el-form-item>
      </el-form>
      <div v-if="rule.ruleId" class="actions">
        <el-button @click="loadPrototypes">刷新原型</el-button>
        <el-button v-permission="'monitor:template'" @click="openPrototypeItem()">监控项原型</el-button>
        <el-button v-permission="'monitor:template'" @click="openPrototypeTrigger()">触发器原型</el-button>
      </div>
      <el-table v-if="rule.ruleId" :data="prototypeItems" size="small">
        <el-table-column prop="name" label="监控项原型" />
        <el-table-column prop="key" label="键" />
        <el-table-column width="120">
          <template #default="{ row }">
            <el-button link @click="openPrototypeItem(row)">{{ row.editable && canWrite ? '编辑' : '查看' }}</el-button>
            <el-button v-if="row.editable" v-permission="'monitor:template'" link type="danger" @click="removePrototypeItem(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-table v-if="rule.ruleId" :data="prototypeTriggers" size="small">
        <el-table-column prop="description" label="触发器原型" />
        <el-table-column prop="expression" label="表达式" />
        <el-table-column width="120">
          <template #default="{ row }">
            <el-button link @click="openPrototypeTrigger(row)">{{ row.editable && canWrite ? '编辑' : '查看' }}</el-button>
            <el-button v-if="row.editable" v-permission="'monitor:template'" link type="danger" @click="removePrototypeTrigger(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <template #footer>
        <el-button @click="ruleVisible = false">关闭</el-button>
        <el-button v-if="!ruleReadonly" type="primary" @click="saveRule">保存</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="triggerVisible" :title="prototypeMode ? '触发器原型' : '触发器'" width="680px">
      <el-form label-width="108px" :disabled="triggerReadonly">
        <el-form-item label="名称"><el-input v-model="trigger.description" /></el-form-item>
        <el-form-item label="级别">
          <el-select v-model="trigger.priority">
            <el-option v-for="item in catalog?.severities || []" :key="item.value" :label="item.name" :value="item.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="表达式">
          <el-input v-model="trigger.expression" type="textarea" :rows="4" />
          <div class="step">
            <el-select v-model="fnName" placeholder="函数" style="width: 140px">
              <el-option v-for="item in catalog?.functions || []" :key="item.name" :label="`${item.name} ${item.hint}`" :value="item.name" />
            </el-select>
            <el-input v-model="fnKey" placeholder="键，如 system.cpu.util" />
            <el-input v-model="fnTime" placeholder="5m" style="width: 90px" />
            <el-button @click="insertFunction">插入</el-button>
          </div>
        </el-form-item>
        <el-form-item label="恢复表达式">
          <el-input v-model="trigger.recoveryExpression" type="textarea" :rows="2" placeholder="留空表示表达式为假时恢复" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="triggerVisible = false">关闭</el-button>
        <el-button v-if="!triggerReadonly" type="primary" @click="saveTrigger">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useUserStore } from '../../stores/user'
import {
  deleteDesignerItem,
  deleteDesignerTemplate,
  deleteDesignerTrigger,
  deleteDiscoveryRule,
  deletePrototypeItem,
  deletePrototypeTrigger,
  getDesignerTemplate,
  listDesignerItems,
  listDesignerMacros,
  listDesignerTriggers,
  listDiscoveryRules,
  listInstances,
  listPrototypeItems,
  listPrototypeTriggers,
  listTemplateGroups,
  listTemplates,
  monitorCatalog,
  saveDesignerItem,
  saveDesignerMacros,
  saveDesignerTrigger,
  saveDiscoveryRule,
  savePrototypeItem,
  savePrototypeTrigger,
  submitTemplateChange,
  updateDesignerTemplate,
  type DesignerItem,
  type DesignerMacro,
  type DesignerTemplate,
  type DesignerTrigger,
  type DiscoveryRule,
  type MonitorCatalog,
  type MonitorGroupRef,
  type MonitorTemplate,
} from '../../api/monitor'

const route = useRoute()
const router = useRouter()
const user = useUserStore()
const canWrite = computed(() => user.hasPermission('monitor:template'))
const code = computed(() => String(route.query.code || ''))
const templateId = computed(() => String(route.params.id || ''))
const loading = ref(false)
const tab = ref('base')
const catalog = ref<MonitorCatalog | null>(null)
const template = ref<DesignerTemplate | null>(null)
const templateGroups = ref<MonitorGroupRef[]>([])
const allTemplates = ref<MonitorTemplate[]>([])
const groupIds = ref<string[]>([])
const parentIds = ref<string[]>([])
const items = ref<DesignerItem[]>([])
const triggers = ref<DesignerTrigger[]>([])
const macros = ref<DesignerMacro[]>([])
const rules = ref<DiscoveryRule[]>([])
const prototypeItems = ref<DesignerItem[]>([])
const prototypeTriggers = ref<DesignerTrigger[]>([])
const protectedInstance = ref(false)
const itemVisible = ref(false)
const triggerVisible = ref(false)
const ruleVisible = ref(false)
const prototypeMode = ref(false)
const rule = ref<DiscoveryRule>(blankRule())
const item = ref<DesignerItem>(blankItem())
const trigger = ref<DesignerTrigger>(blankTrigger())
const fnName = ref('last')
const fnKey = ref('')
const fnTime = ref('5m')

const editableTypes = computed(() => (catalog.value?.itemTypes || []).filter((item) => item.editable))
const itemReadonly = computed(() => !canWrite.value || (Boolean(item.value.itemId) && !item.value.editable))
const triggerReadonly = computed(() => !canWrite.value || (Boolean(trigger.value.triggerId) && !trigger.value.editable))
const ruleReadonly = computed(() => !canWrite.value || (Boolean(rule.value.ruleId) && !rule.value.editable))

onMounted(async () => {
  if (!code.value || !templateId.value) return
  loading.value = true
  try {
    const [book, instances] = await Promise.all([monitorCatalog(), listInstances()])
    catalog.value = book
    protectedInstance.value = instances.find((item) => item.code === code.value)?.protected === true
    await reload()
  } finally {
    loading.value = false
  }
})

async function reload() {
  const [row, groups, all, itemRows, triggerRows, macroRows, ruleRows] = await Promise.all([
    getDesignerTemplate(code.value, templateId.value),
    listTemplateGroups(code.value),
    listTemplates(code.value),
    listDesignerItems(code.value, templateId.value),
    listDesignerTriggers(code.value, templateId.value),
    listDesignerMacros(code.value, templateId.value),
    listDiscoveryRules(code.value, templateId.value),
  ])
  template.value = row
  templateGroups.value = groups
  allTemplates.value = all.filter((item) => item.templateId !== templateId.value)
  groupIds.value = row.groups.map((item) => item.groupId)
  parentIds.value = row.parentTemplates.map((item) => item.templateId)
  items.value = itemRows
  triggers.value = triggerRows
  macros.value = macroRows
  rules.value = ruleRows
}

function showField(name: string) {
  const spec = catalog.value?.itemTypes.find((itemType) => itemType.type === item.value.type)
  return spec?.fields.includes(name) ?? false
}

function stepHint(type: number) {
  return catalog.value?.preprocessing.find((item) => item.type === type)?.paramsHint || ''
}

function severityName(value: number) {
  return catalog.value?.severities.find((item) => item.value === value)?.name || String(value)
}

async function commit(title: string, method: 'POST' | 'PUT' | 'DELETE', resource: string, body?: unknown) {
  const res = await submitTemplateChange(code.value, { title, method, resource, body })
  ElMessage.success(`已提交工单 ${res.ticketNo}`)
}

async function saveBase() {
  if (!template.value) return
  const body = {
    host: template.value.host,
    name: template.value.name,
    groupIds: groupIds.value,
    parentTemplateIds: parentIds.value,
  }
  if (protectedInstance.value) {
    await commit(`修改模板 ${template.value.name}`, 'PUT', `/templates/${templateId.value}`, body)
    return
  }
  template.value = await updateDesignerTemplate(code.value, templateId.value, body)
  ElMessage.success('已保存')
}

async function removeTemplate() {
  await ElMessageBox.confirm('删除该模板？已链接主机会解除链接。', '删除模板')
  if (protectedInstance.value) {
    await commit(`删除模板 ${template.value?.name || ''}`, 'DELETE', `/templates/${templateId.value}`)
    return
  }
  await deleteDesignerTemplate(code.value, templateId.value)
  router.push('/monitor/templates')
}

function blankItem(): DesignerItem {
  return {
    itemId: '', name: '', key: '', type: 0, typeName: 'Zabbix agent', valueType: 3, delay: '1m', units: '',
    status: 0, description: '', inherited: false, editable: true, masterItemId: '', snmpOid: '', url: '',
    requestMethod: 0, statusCodes: '200', params: '', preprocessing: [],
  }
}

function openItem(row?: DesignerItem) {
  prototypeMode.value = false
  item.value = row ? JSON.parse(JSON.stringify(row)) : blankItem()
  itemVisible.value = true
}

function itemResource(body: DesignerItem) {
  if (prototypeMode.value && rule.value.ruleId) {
    const base = `/templates/${templateId.value}/discovery-rules/${rule.value.ruleId}/items`
    return body.itemId ? `${base}/${body.itemId}` : base
  }
  return body.itemId ? `/templates/${templateId.value}/items/${body.itemId}` : `/templates/${templateId.value}/items`
}

async function saveItem() {
  const body = { ...item.value }
  const label = prototypeMode.value ? '监控项原型' : '监控项'
  if (protectedInstance.value) {
    await commit(`${body.itemId ? '修改' : '新建'}${label} ${body.name}`, body.itemId ? 'PUT' : 'POST', itemResource(body), body)
  } else if (prototypeMode.value && rule.value.ruleId) {
    await savePrototypeItem(code.value, templateId.value, rule.value.ruleId, body)
    ElMessage.success('监控项原型已保存')
    await loadPrototypes()
  } else {
    await saveDesignerItem(code.value, templateId.value, body)
    ElMessage.success('监控项已保存')
    await reload()
  }
  itemVisible.value = false
}

async function removeItem(row: DesignerItem) {
  await ElMessageBox.confirm(`删除监控项「${row.name}」？`, '删除监控项')
  if (protectedInstance.value) {
    await commit(`删除监控项 ${row.name}`, 'DELETE', `/templates/${templateId.value}/items/${row.itemId}`)
    return
  }
  await deleteDesignerItem(code.value, templateId.value, row.itemId)
  await reload()
}

function blankTrigger(): DesignerTrigger {
  return {
    triggerId: '', description: '', expression: '', priority: 3, status: 0,
    recoveryMode: 0, recoveryExpression: '', inherited: false, editable: true,
  }
}

function openTrigger(row?: DesignerTrigger) {
  prototypeMode.value = false
  trigger.value = row ? JSON.parse(JSON.stringify(row)) : blankTrigger()
  if (trigger.value.recoveryExpression) trigger.value.recoveryMode = 1
  triggerVisible.value = true
}

function insertFunction() {
  const spec = catalog.value?.functions.find((item) => item.name === fnName.value)
  if (!spec || !template.value || !fnKey.value.trim()) {
    ElMessage.warning('请选择函数并填写键')
    return
  }
  const text = spec.insert
    .replaceAll('{host}', template.value.host)
    .replaceAll('{key}', fnKey.value.trim())
    .replaceAll('{time}', fnTime.value.trim() || '5m')
    .replaceAll('{threshold}', '0')
  trigger.value.expression = trigger.value.expression
    ? `${trigger.value.expression} and ${text}`
    : text
}

function triggerResource(body: DesignerTrigger) {
  if (prototypeMode.value && rule.value.ruleId) {
    const base = `/templates/${templateId.value}/discovery-rules/${rule.value.ruleId}/triggers`
    return body.triggerId ? `${base}/${body.triggerId}` : base
  }
  return body.triggerId
    ? `/templates/${templateId.value}/triggers/${body.triggerId}`
    : `/templates/${templateId.value}/triggers`
}

async function saveTrigger() {
  const body = { ...trigger.value }
  body.recoveryMode = body.recoveryExpression.trim() ? 1 : 0
  const label = prototypeMode.value ? '触发器原型' : '触发器'
  if (protectedInstance.value) {
    await commit(`${body.triggerId ? '修改' : '新建'}${label} ${body.description}`, body.triggerId ? 'PUT' : 'POST', triggerResource(body), body)
  } else if (prototypeMode.value && rule.value.ruleId) {
    await savePrototypeTrigger(code.value, templateId.value, rule.value.ruleId, body)
    ElMessage.success('触发器原型已保存')
    await loadPrototypes()
  } else {
    await saveDesignerTrigger(code.value, templateId.value, body)
    ElMessage.success('触发器已保存')
    await reload()
  }
  triggerVisible.value = false
}

async function removeTrigger(row: DesignerTrigger) {
  await ElMessageBox.confirm(`删除触发器「${row.description}」？`, '删除触发器')
  if (protectedInstance.value) {
    await commit(`删除触发器 ${row.description}`, 'DELETE', `/templates/${templateId.value}/triggers/${row.triggerId}`)
    return
  }
  await deleteDesignerTrigger(code.value, templateId.value, row.triggerId)
  await reload()
}

function blankRule(): DiscoveryRule {
  return {
    ruleId: '', name: '', key: '', type: 0, typeName: '', delay: '1h', status: 0,
    lifetimeType: 0, lifetime: '7d', description: '', inherited: false, editable: true,
    filterEvalType: 0, filter: [], overrides: [],
  }
}

function openRule(row?: DiscoveryRule) {
  rule.value = row ? JSON.parse(JSON.stringify(row)) : blankRule()
  prototypeItems.value = []
  prototypeTriggers.value = []
  ruleVisible.value = true
  if (rule.value.ruleId) loadPrototypes()
}

function addOverride() {
  rule.value.overrides.push({
    name: '',
    step: rule.value.overrides.length + 1,
    stop: false,
    filterEvalType: 0,
    filter: [],
    operations: [],
  })
}

async function loadPrototypes() {
  if (!rule.value.ruleId) return
  const [itemRows, triggerRows] = await Promise.all([
    listPrototypeItems(code.value, templateId.value, rule.value.ruleId),
    listPrototypeTriggers(code.value, templateId.value, rule.value.ruleId),
  ])
  prototypeItems.value = itemRows
  prototypeTriggers.value = triggerRows
}

async function saveRule() {
  const body = { ...rule.value, overrides: rule.value.ruleId ? rule.value.overrides : undefined }
  if (protectedInstance.value) {
    const resource = body.ruleId
      ? `/templates/${templateId.value}/discovery-rules/${body.ruleId}`
      : `/templates/${templateId.value}/discovery-rules`
    await commit(`${body.ruleId ? '修改' : '新建'}发现规则 ${body.name}`, body.ruleId ? 'PUT' : 'POST', resource, body)
    return
  }
  const saved = await saveDiscoveryRule(code.value, templateId.value, body)
  ElMessage.success('发现规则已保存')
  if (!rule.value.ruleId) {
    rule.value.ruleId = saved.ruleId
    rule.value.editable = true
  }
  await reload()
  await loadPrototypes()
}

async function removeRule(row: DiscoveryRule) {
  await ElMessageBox.confirm(`删除发现规则「${row.name}」？其原型会一并删除。`, '删除发现规则')
  if (protectedInstance.value) {
    await commit(`删除发现规则 ${row.name}`, 'DELETE', `/templates/${templateId.value}/discovery-rules/${row.ruleId}`)
    return
  }
  await deleteDiscoveryRule(code.value, templateId.value, row.ruleId)
  await reload()
}

function openPrototypeItem(row?: DesignerItem) {
  prototypeMode.value = true
  item.value = row ? JSON.parse(JSON.stringify(row)) : blankItem()
  itemVisible.value = true
}

function openPrototypeTrigger(row?: DesignerTrigger) {
  prototypeMode.value = true
  trigger.value = row ? JSON.parse(JSON.stringify(row)) : blankTrigger()
  triggerVisible.value = true
}

async function removePrototypeItem(row: DesignerItem) {
  await ElMessageBox.confirm(`删除监控项原型「${row.name}」？`, '删除原型')
  if (protectedInstance.value) {
    await commit(`删除监控项原型 ${row.name}`, 'DELETE', `/templates/${templateId.value}/discovery-rules/${rule.value.ruleId}/items/${row.itemId}`)
    return
  }
  await deletePrototypeItem(code.value, templateId.value, rule.value.ruleId, row.itemId)
  await loadPrototypes()
}

async function removePrototypeTrigger(row: DesignerTrigger) {
  await ElMessageBox.confirm(`删除触发器原型「${row.description}」？`, '删除原型')
  if (protectedInstance.value) {
    await commit(`删除触发器原型 ${row.description}`, 'DELETE', `/templates/${templateId.value}/discovery-rules/${rule.value.ruleId}/triggers/${row.triggerId}`)
    return
  }
  await deletePrototypeTrigger(code.value, templateId.value, rule.value.ruleId, row.triggerId)
  await loadPrototypes()
}

async function saveMacros() {
  if (protectedInstance.value) {
    await commit(`修改模板宏 ${template.value?.name || ''}`, 'PUT', `/templates/${templateId.value}/macros`, { macros: macros.value })
    return
  }
  macros.value = await saveDesignerMacros(code.value, templateId.value, macros.value)
  ElMessage.success('宏已保存')
}
</script>

<style scoped>
.page-header { margin-bottom: 16px; }
.page-title { display: flex; align-items: baseline; gap: 8px; font-size: 18px; }
.page-sub { color: var(--el-text-color-secondary); font-size: 13px; }
.banner { margin-bottom: 16px; }
.table { margin-top: 12px; }
.actions, .step, .override { display: flex; gap: 8px; align-items: center; margin-top: 8px; flex-wrap: wrap; }
.override { align-items: flex-start; padding-bottom: 8px; border-bottom: 1px solid var(--el-border-color-lighter); }
.hint { margin-left: 8px; color: var(--el-text-color-secondary); }
</style>
