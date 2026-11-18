<template>
  <div class="monitor-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><Monitor /></el-icon>
        <span>监控纳管</span>
        <span class="page-sub">选中主机可改模板、主机组和宏</span>
      </div>
      <div class="header-actions">
        <el-button v-permission="'monitor:operate'" @click="openRegister">登记配置项</el-button>
        <el-button :loading="govLoading" @click="loadGovernance">刷新治理</el-button>
        <el-button :loading="syncing" @click="doSync">同步对照</el-button>
        <el-button :icon="Refresh" @click="loadHosts">刷新</el-button>
      </div>
    </div>

    <el-collapse v-if="governance" class="gov-fold">
      <el-collapse-item name="health">
        <template #title>
          <span>平台健康</span>
          <span class="gov-summary">
            队列 {{ governance.queue.available ? governance.queue.count : '不可用' }}
            · Proxy 离线 {{ governance.offlineProxyCount }}
            <template v-if="governance.standardTemplates.length"> · 模板偏离 {{ governance.driftCount }}</template>
          </span>
        </template>
        <el-row :gutter="16" class="gov-row">
          <el-col :span="governance.standardTemplates.length ? 8 : 12">
            <el-card shadow="never">
              <template #header>采集队列</template>
              <div v-if="governance.queue.available" class="gov-stat">
                <span class="gov-num" :class="{ danger: governance.queue.backedUp }">{{ governance.queue.count }}</span>
                <span class="gov-hint">阈值 {{ governance.queue.warn }}{{ governance.queue.backedUp ? ' · 积压' : '' }}</span>
              </div>
              <el-empty v-else :description="governance.queue.message || '队列接口不可用'" :image-size="48" />
            </el-card>
          </el-col>
          <el-col :span="governance.standardTemplates.length ? 8 : 12">
            <el-card shadow="never">
              <template #header>Proxy 离线</template>
              <div class="gov-stat">
                <span class="gov-num" :class="{ danger: governance.offlineProxyCount > 0 }">{{ governance.offlineProxyCount }}</span>
                <span class="gov-hint">共 {{ governance.proxies.length }} 个 Proxy</span>
              </div>
            </el-card>
          </el-col>
          <el-col v-if="governance.standardTemplates.length" :span="8">
            <el-card shadow="never">
              <template #header>模板偏离</template>
              <div class="gov-stat">
                <span class="gov-num" :class="{ danger: governance.driftCount > 0 }">{{ governance.driftCount }}</span>
                <span class="gov-hint">相对标准模板</span>
              </div>
            </el-card>
          </el-col>
        </el-row>
        <el-row :gutter="16" class="gov-row">
          <el-col :span="governance.standardTemplates.length ? 12 : 24">
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
          <el-col v-if="governance.standardTemplates.length" :span="12">
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
      </el-collapse-item>
    </el-collapse>

    <el-card shadow="never">
      <div v-if="instanceCode" class="roster-summary">
        <button type="button" class="summary-total" :class="{ active: !groupId }" @click="pickGroup('')">
          <span class="summary-num">{{ total }}</span>
          <span class="summary-label">主机总数</span>
        </button>
        <div class="summary-groups">
          <button
            v-for="item in groupStats"
            :key="item.groupId"
            type="button"
            class="summary-group"
            :class="{ active: groupId === item.groupId }"
            :title="item.name"
            @click="pickGroup(item.groupId)"
          >
            <span class="summary-num">{{ item.hostCount ?? 0 }}</span>
            <span class="summary-label">{{ item.name }}</span>
          </button>
        </div>
        <div class="summary-matched">当前 {{ matched }} 台</div>
      </div>
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
        <el-select v-model="hostFilter" placeholder="筛选" clearable style="width: 140px" @change="reloadHosts">
          <el-option label="Agent 离线" value="offline" />
          <el-option label="未关联 CMDB" value="unlinked" />
          <el-option label="已关联 CMDB" value="linked" />
        </el-select>
        <el-button type="primary" @click="reloadHosts">查询</el-button>
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
        :data="hosts"
        highlight-current-row
        @selection-change="onSelection"
        @current-change="onSelect"
      >
        <el-table-column type="selection" width="48" />
        <el-table-column prop="displayName" label="名称" min-width="160" />
        <el-table-column prop="hostname" label="主机名" min-width="140" />
        <el-table-column prop="ip" label="IP" width="150" />
        <el-table-column label="主机组" min-width="160">
          <template #default="{ row }">{{ (row.groups || []).map((g: { name: string }) => g.name).join('、') || '-' }}</template>
        </el-table-column>
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
      <div v-if="instanceCode" class="pager">
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
    </el-card>

    <el-card v-if="selected" v-loading="detailLoading" shadow="never" class="detail">
      <template #header>
        <div class="detail-head">
          <div>
            <div class="detail-title">{{ selected.displayName || selected.hostname }}</div>
            <div class="detail-sub">{{ selected.hostname }} · {{ selected.ip || '无 IP' }}</div>
          </div>
        </div>
      </template>

      <div class="status-row">
        <div class="status-item">
          <span class="status-label">Agent</span>
          <el-tag v-if="selected.agentAvailable === true" type="success" size="small">通</el-tag>
          <el-tag v-else-if="selected.agentAvailable === false" type="danger" size="small">断</el-tag>
          <el-tag v-else type="info" size="small">未知</el-tag>
        </div>
        <div class="status-item">
          <span class="status-label">版本</span>
          <span>{{ selected.agentVersion || '-' }}</span>
        </div>
        <div class="status-item">
          <span class="status-label">配置项</span>
          <el-tag v-if="selected.cmdbLinked" type="success" size="small">已关联</el-tag>
          <el-tag v-else type="info" size="small">未关联</el-tag>
        </div>
        <div class="status-item">
          <span class="status-label">当前问题</span>
          <span :class="{ danger: problems.length > 0 }">{{ problems.length }}</span>
        </div>
      </div>

      <el-tabs v-model="detailTab" class="detail-tabs" @tab-change="onDetailTab">
        <el-tab-pane label="概况" name="overview">
          <div class="block">
            <div class="block-title">当前问题</div>
            <el-table v-loading="problemLoading" :data="problems" size="small" empty-text="当前没有未关闭问题">
              <el-table-column prop="name" label="问题" min-width="220" />
              <el-table-column label="级别" width="90">
                <template #default="{ row }">{{ severityLabel(row.severity) }}</template>
              </el-table-column>
              <el-table-column label="操作" width="80">
                <template #default="{ row }">
                  <el-button v-permission="'monitor:operate'" link type="primary" @click="openAck(row)">确认</el-button>
                </template>
              </el-table-column>
            </el-table>
          </div>

          <div class="block">
            <div class="block-title">监控方案</div>
            <el-alert
              v-if="schemeBlockedReason"
              type="info"
              :closable="false"
              show-icon
              :title="schemeBlockedReason"
              class="scheme-alert"
            />
            <template v-else>
              <div class="scheme-section">
                <div class="scheme-label">已套模板</div>
                <el-tag
                  v-for="item in linkage.templates"
                  :key="item.templateId"
                  class="tag"
                  :closable="canEditScheme"
                  :disable-transitions="true"
                  @close="changeTemplates('unlink', [item.templateId])"
                >
                  {{ item.name }}
                </el-tag>
                <el-select
                  v-if="canEditScheme"
                  v-model="templateToAdd"
                  filterable
                  placeholder="套用模板"
                  style="width: 260px"
                >
                  <el-option
                    v-for="item in templateChoices"
                    :key="item.templateId"
                    :label="item.name"
                    :value="item.templateId"
                  />
                </el-select>
                <el-button
                  v-if="canEditScheme"
                  type="primary"
                  :disabled="!templateToAdd"
                  @click="changeTemplates('link', [templateToAdd])"
                >
                  套用
                </el-button>
              </div>
              <div class="scheme-section">
                <div class="scheme-label">所属主机组</div>
                <el-tag
                  v-for="item in linkage.groups"
                  :key="item.groupId"
                  class="tag"
                  :closable="canEditScheme"
                  :disable-transitions="true"
                  @close="changeGroups('unlink', [item.groupId])"
                >
                  {{ item.name }}
                </el-tag>
                <el-select
                  v-if="canEditScheme"
                  v-model="groupToAdd"
                  filterable
                  placeholder="加入主机组"
                  style="width: 220px"
                >
                  <el-option
                    v-for="item in groupChoices"
                    :key="item.groupId"
                    :label="item.name"
                    :value="item.groupId"
                  />
                </el-select>
                <el-button v-if="canEditScheme" :disabled="!groupToAdd" @click="changeGroups('link', [groupToAdd])">
                  加入
                </el-button>
              </div>
              <div class="scheme-section">
                <div class="scheme-label">主机宏</div>
                <el-table :data="macros" size="small" empty-text="暂无主机宏">
                  <el-table-column prop="macro" label="宏" min-width="180" />
                  <el-table-column label="值" min-width="160">
                    <template #default="{ row }">
                      <el-input
                        v-model="row.value"
                        :disabled="!canEditScheme"
                        :placeholder="row.secret ? '密钥留空则不修改' : ''"
                      />
                    </template>
                  </el-table-column>
                  <el-table-column prop="description" label="说明" min-width="140" />
                </el-table>
                <div v-if="canEditScheme" class="macro-add">
                  <el-input v-model="newMacro.macro" placeholder="{$MACRO}" style="width: 180px" />
                  <el-input v-model="newMacro.value" placeholder="值" style="width: 160px" />
                  <el-button @click="addMacro">添加</el-button>
                  <el-button type="primary" :loading="savingMacros" @click="saveMacros">保存宏</el-button>
                </div>
              </div>
            </template>
          </div>
        </el-tab-pane>

        <el-tab-pane name="items">
          <template #label>监控项 / 最新数据（{{ hostItemRows.length }}）</template>
          <div class="tab-toolbar">
            <el-input
              v-model="itemSearch"
              clearable
              placeholder="按名称或键搜索"
              style="width: 260px"
              @keyup.enter="loadHostItems"
              @clear="loadHostItems"
            />
            <el-button type="primary" :loading="itemsLoading" @click="loadHostItems">查询</el-button>
            <span class="tab-count">共 {{ hostItemRows.length }} 项</span>
          </div>
          <el-table v-loading="itemsLoading" :data="hostItemRows" size="small" max-height="420" empty-text="没有监控项">
            <el-table-column prop="name" label="名称" min-width="180" />
            <el-table-column prop="key" label="键" min-width="200" show-overflow-tooltip />
            <el-table-column label="最新数据" min-width="140">
              <template #default="{ row }">{{ formatLastValue(row) }}</template>
            </el-table-column>
            <el-table-column label="更新时间" width="160">
              <template #default="{ row }">{{ formatLastClock(row.lastClock) }}</template>
            </el-table-column>
            <el-table-column label="来自模板" min-width="180">
              <template #default="{ row }">
                <el-button
                  v-if="row.templateId"
                  link
                  type="primary"
                  @click="openTemplate(row.templateId)"
                >
                  {{ row.templateName || row.templateId }}
                </el-button>
                <span v-else>本机</span>
              </template>
            </el-table-column>
            <el-table-column label="状态" width="90">
              <template #default="{ row }">
                <el-tag v-if="row.status === '1'" type="info" size="small">禁用</el-tag>
                <el-tag v-else-if="row.state === '1'" type="danger" size="small">不支持</el-tag>
                <el-tag v-else type="success" size="small">启用</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <el-tab-pane name="triggers">
          <template #label>触发器（{{ hostTriggerRows.length }}）</template>
          <div class="tab-toolbar">
            <span class="tab-count">共 {{ hostTriggerRows.length }} 条</span>
          </div>
          <el-table
            v-loading="triggersLoading"
            :data="hostTriggerRows"
            size="small"
            max-height="420"
            empty-text="没有触发器"
          >
            <el-table-column prop="name" label="名称" min-width="240" />
            <el-table-column label="级别" width="90">
              <template #default="{ row }">{{ severityLabel(row.severity) }}</template>
            </el-table-column>
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag v-if="row.status === '1'" type="info" size="small">禁用</el-tag>
                <el-tag v-else-if="row.value === '1'" type="danger" size="small">问题</el-tag>
                <el-tag v-else type="success" size="small">正常</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="来自模板" min-width="180">
              <template #default="{ row }">
                <el-button
                  v-if="row.templateId"
                  link
                  type="primary"
                  @click="openTemplate(row.templateId)"
                >
                  {{ row.templateName || row.templateId }}
                </el-button>
                <span v-else>本机</span>
              </template>
            </el-table-column>
            <el-table-column label="最近变化" width="160">
              <template #default="{ row }">{{ formatLastClock(row.lastChange) }}</template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>

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

    <el-dialog v-model="ackVisible" title="确认问题" width="420px">
      <el-form label-width="80px">
        <el-form-item label="说明">
          <el-input v-model="ackMessage" type="textarea" :rows="3" />
        </el-form-item>
        <el-form-item label="级别">
          <el-select v-model="ackSeverity" clearable placeholder="不修改" style="width: 100%">
            <el-option v-for="(label, index) in severityLabels" :key="label" :label="label" :value="index" />
          </el-select>
        </el-form-item>
        <el-form-item label="关闭">
          <el-switch v-model="ackClose" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="ackVisible = false">取消</el-button>
        <el-button type="primary" :loading="ackSubmitting" @click="submitAck">提交</el-button>
      </template>
    </el-dialog>

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
            <el-option v-for="item in schemeGroups" :key="item.groupId" :label="item.name" :value="item.groupId" />
          </el-select>
        </el-form-item>
        <el-form-item label="模板">
          <el-select v-model="registerTemplates" multiple filterable style="width: 100%">
            <el-option v-for="item in schemeTemplates" :key="item.templateId" :label="item.name" :value="item.templateId" />
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
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { Monitor, Refresh } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useUserStore } from '../../stores/user'
import { listCiInstances } from '../../api/cmdb'
import {
  ackMonitorEvent,
  createTask,
  getGovernance,
  getTask,
  hostItems,
  hostLinkage,
  hostMacros,
  hostProblems,
  hostTriggers,
  linkHostGroups,
  linkHostTemplates,
  listHostGroups,
  listHosts,
  listInstances,
  listTemplates,
  registerMonitorHost,
  saveHostMacros,
  syncHosts,
  type GovernanceData,
  type HostItem,
  type HostMacro,
  type HostTrigger,
  type MonitorAction,
  type MonitorGroupRef,
  type MonitorHost,
  type MonitorInstance,
  type MonitorProblem,
  type MonitorTask,
  type MonitorTemplate,
  type MonitorTemplateRef,
} from '../../api/monitor'

const router = useRouter()
const user = useUserStore()
const canOperate = computed(() => user.hasPermission('monitor:operate'))
const isAdmin = computed(() => user.role === 'admin')
const canEditScheme = computed(() => {
  if (!selected.value || !canOperate.value) return false
  return isAdmin.value || !!selected.value.cmdbLinked
})
const schemeBlockedReason = computed(() => {
  if (!selected.value) return ''
  if (isAdmin.value || selected.value.cmdbLinked) return ''
  return '这台主机还没关联配置项。用右上角「登记配置项」挂上后再改监控方案。'
})

const instances = ref<MonitorInstance[]>([])
const instanceCode = ref('')
const search = ref('')
const hostFilter = ref('')
const groupId = ref('')
const hosts = ref<MonitorHost[]>([])
const total = ref(0)
const matched = ref(0)
const page = ref(1)
const pageSize = ref(20)
const groupStats = ref<MonitorGroupRef[]>([])
const loading = ref(false)
const syncing = ref(false)
const govLoading = ref(false)
const governance = ref<GovernanceData | null>(null)
const selected = ref<MonitorHost | null>(null)
const selectedIds = ref<string[]>([])
const problems = ref<MonitorProblem[]>([])
const problemLoading = ref(false)
const detailTab = ref('overview')
const hostItemRows = ref<HostItem[]>([])
const hostTriggerRows = ref<HostTrigger[]>([])
const itemsLoading = ref(false)
const triggersLoading = ref(false)
const itemSearch = ref('')
const itemsLoadedFor = ref('')
const triggersLoadedFor = ref('')
const task = ref<MonitorTask | null>(null)
const taskSubmitting = ref(false)
const upgradeVisible = ref(false)
const targetVersion = ref('')
const ackVisible = ref(false)
const ackMessage = ref('')
const ackSeverity = ref<number | undefined>()
const ackClose = ref(false)
const ackSubmitting = ref(false)
const ackEvent = ref<MonitorProblem | null>(null)
const schemeLoading = ref(false)
const schemeGroups = ref<MonitorGroupRef[]>([])
const schemeTemplates = ref<MonitorTemplate[]>([])
const linkage = ref<{ groups: MonitorGroupRef[]; templates: MonitorTemplateRef[] }>({ groups: [], templates: [] })
const macros = ref<HostMacro[]>([])
const templateToAdd = ref('')
const groupToAdd = ref('')
const savingMacros = ref(false)
const newMacro = reactive({ macro: '', value: '' })
const registerVisible = ref(false)
const registerCiId = ref('')
const registerGroups = ref<string[]>([])
const registerTemplates = ref<string[]>([])
const ciOptions = ref<{ id: string; name: string }[]>([])
const ciLoading = ref(false)
const registering = ref(false)
const detailLoading = computed(() => problemLoading.value || schemeLoading.value)
const templateChoices = computed(() =>
  schemeTemplates.value.filter((item) => !linkage.value.templates.some((linked) => linked.templateId === item.templateId)),
)
const groupChoices = computed(() =>
  schemeGroups.value.filter((item) => !linkage.value.groups.some((linked) => linked.groupId === item.groupId)),
)
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
      await Promise.all([loadHosts(), loadGovernance(), loadSchemeCatalog()])
    }
  } catch {
    instances.value = []
  }
})

onBeforeUnmount(() => {
  if (pollTimer) window.clearInterval(pollTimer)
})

function rosterFilter(): { agent?: 'online' | 'offline' | 'unknown'; cmdb?: 'linked' | 'unlinked' } {
  if (hostFilter.value === 'offline') return { agent: 'offline' }
  if (hostFilter.value === 'linked') return { cmdb: 'linked' }
  if (hostFilter.value === 'unlinked') return { cmdb: 'unlinked' }
  return {}
}

function reloadHosts() {
  if (page.value !== 1) page.value = 1
  else return loadHosts()
}

function onPageSize() {
  if (page.value !== 1) page.value = 1
  else return loadHosts()
}

function pickGroup(id: string) {
  groupId.value = id
  return reloadHosts()
}

async function loadHosts(keepSelection?: boolean) {
  if (!instanceCode.value) return
  const keepId = keepSelection === true ? selected.value?.hostId : ''
  loading.value = true
  try {
    const roster = await listHosts(instanceCode.value, {
      search: search.value.trim(),
      page: page.value,
      pageSize: pageSize.value,
      groupId: groupId.value,
      ...rosterFilter(),
    })
    hosts.value = roster.items
    total.value = roster.total
    matched.value = roster.matched
    groupStats.value = roster.groups
    if (keepId) {
      const row = roster.items.find((item) => item.hostId === keepId)
      if (row) selected.value = row
    } else {
      selected.value = null
      selectedIds.value = []
      problems.value = []
      linkage.value = { groups: [], templates: [] }
      macros.value = []
    }
  } catch {
    hosts.value = []
    total.value = 0
    matched.value = 0
    groupStats.value = []
  } finally {
    loading.value = false
  }
}

async function loadSchemeCatalog() {
  if (!instanceCode.value) return
  const [groups, templates] = await Promise.all([
    listHostGroups(instanceCode.value),
    listTemplates(instanceCode.value),
  ])
  schemeGroups.value = groups
  schemeTemplates.value = templates
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
  groupId.value = ''
  await Promise.all([loadHosts(), loadGovernance(), loadSchemeCatalog()])
}

function openAck(row: MonitorProblem) {
  ackEvent.value = row
  ackMessage.value = ''
  ackSeverity.value = undefined
  ackClose.value = false
  ackVisible.value = true
}

async function submitAck() {
  if (!ackEvent.value || !instanceCode.value || !selected.value) return
  ackSubmitting.value = true
  try {
    await ackMonitorEvent(instanceCode.value, ackEvent.value.eventId, {
      message: ackMessage.value,
      severity: ackSeverity.value,
      close: ackClose.value,
    })
    ElMessage.success('已提交确认')
    ackVisible.value = false
    problems.value = await hostProblems(instanceCode.value, selected.value.hostId)
  } finally {
    ackSubmitting.value = false
  }
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
  detailTab.value = 'overview'
  templateToAdd.value = ''
  groupToAdd.value = ''
  linkage.value = { groups: [], templates: [] }
  macros.value = []
  problems.value = []
  hostItemRows.value = []
  hostTriggerRows.value = []
  itemSearch.value = ''
  itemsLoadedFor.value = ''
  triggersLoadedFor.value = ''
  problemLoading.value = true
  schemeLoading.value = true
  try {
    const loadScheme = isAdmin.value || !!row.cmdbLinked
    const [current, link, items] = await Promise.all([
      hostProblems(instanceCode.value, row.hostId),
      loadScheme ? hostLinkage(instanceCode.value, row.hostId) : Promise.resolve({ groups: [], templates: [] }),
      loadScheme ? hostMacros(instanceCode.value, row.hostId) : Promise.resolve([] as HostMacro[]),
    ])
    if (selected.value?.hostId !== row.hostId) return
    problems.value = current
    linkage.value = link
    macros.value = items
  } catch {
    problems.value = []
    linkage.value = { groups: [], templates: [] }
    macros.value = []
  } finally {
    problemLoading.value = false
    schemeLoading.value = false
  }
}

function onDetailTab(name: string | number) {
  if (name === 'items') loadHostItems()
  if (name === 'triggers') loadHostTriggers()
}

async function loadHostItems() {
  if (!selected.value || !instanceCode.value) return
  itemsLoading.value = true
  try {
    hostItemRows.value = await hostItems(instanceCode.value, selected.value.hostId, itemSearch.value.trim())
    itemsLoadedFor.value = selected.value.hostId
  } catch {
    hostItemRows.value = []
  } finally {
    itemsLoading.value = false
  }
}

async function loadHostTriggers() {
  if (!selected.value || !instanceCode.value) return
  triggersLoading.value = true
  try {
    hostTriggerRows.value = await hostTriggers(instanceCode.value, selected.value.hostId)
    triggersLoadedFor.value = selected.value.hostId
  } catch {
    hostTriggerRows.value = []
  } finally {
    triggersLoading.value = false
  }
}

function openTemplate(templateId: string) {
  if (!templateId || !instanceCode.value) return
  router.push({
    name: 'MonitorTemplateDesigner',
    params: { id: templateId },
    query: { code: instanceCode.value },
  })
}

function formatLastValue(row: HostItem) {
  const value = row.lastValue?.trim()
  if (!value) return '-'
  return row.units ? `${value} ${row.units}` : value
}

function formatLastClock(clock: string) {
  const n = Number(clock)
  if (!n) return '-'
  const date = new Date(n * 1000)
  if (Number.isNaN(date.getTime())) return clock
  const pad = (v: number) => String(v).padStart(2, '0')
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`
}

async function changeTemplates(action: 'link' | 'unlink', templateIds: string[]) {
  if (!selected.value || !instanceCode.value || !templateIds.length) return
  linkage.value = await linkHostTemplates(instanceCode.value, selected.value.hostId, action, templateIds)
  templateToAdd.value = ''
  ElMessage.success(action === 'link' ? '已套用模板' : '已卸下模板')
}

async function changeGroups(action: 'link' | 'unlink', groupIds: string[]) {
  if (!selected.value || !instanceCode.value || !groupIds.length) return
  linkage.value = await linkHostGroups(instanceCode.value, selected.value.hostId, action, groupIds)
  groupToAdd.value = ''
  if (selected.value) {
    selected.value.groups = linkage.value.groups
  }
  ElMessage.success(action === 'link' ? '已加入主机组' : '已移出主机组')
  await loadHosts(true)
}

function addMacro() {
  const name = newMacro.macro.trim()
  if (!name) return
  macros.value.push({ hostMacroId: '', macro: name, value: newMacro.value, description: '', secret: false })
  newMacro.macro = ''
  newMacro.value = ''
}

async function saveMacros() {
  if (!selected.value || !instanceCode.value) return
  savingMacros.value = true
  try {
    macros.value = await saveHostMacros(
      instanceCode.value,
      selected.value.hostId,
      macros.value.map((item) => ({ macro: item.macro, value: item.value, description: item.description })),
    )
    ElMessage.success('宏已保存')
  } finally {
    savingMacros.value = false
  }
}

function openRegister() {
  registerCiId.value = ''
  registerGroups.value = groupId.value ? [groupId.value] : []
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
    await loadHosts(true)
  } finally {
    registering.value = false
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
.roster-summary {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  margin: -4px 0 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}
.summary-total,
.summary-group {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 0 12px 8px;
  border: none;
  border-bottom: 2px solid transparent;
  background: transparent;
  cursor: pointer;
  font: inherit;
}
.summary-total {
  flex: none;
  align-items: flex-start;
  margin-right: 8px;
  padding-left: 0;
  padding-right: 20px;
  border-right: 1px solid var(--el-border-color-lighter);
  border-bottom-color: transparent;
}
.summary-total .summary-num {
  font-size: 28px;
  color: var(--el-color-primary);
}
.summary-groups {
  display: flex;
  flex: 1;
  gap: 4px;
  min-width: 0;
  overflow-x: auto;
}
.summary-group {
  flex: none;
  min-width: 72px;
  max-width: 120px;
}
.summary-num {
  font-size: 20px;
  font-weight: 600;
  line-height: 1.1;
  color: var(--el-text-color-primary);
}
.summary-label {
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--el-text-color-secondary);
  font-size: 12px;
}
.summary-total.active,
.summary-group.active {
  border-bottom-color: var(--el-color-primary);
}
.summary-group.active .summary-num,
.summary-total.active .summary-num {
  color: var(--el-color-primary);
}
.summary-matched {
  flex: none;
  align-self: center;
  margin-left: 12px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
  white-space: nowrap;
}
.filter-bar {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-bottom: 12px;
}
.pager {
  display: flex;
  justify-content: flex-end;
  margin-top: 12px;
}
.detail {
  margin-top: 16px;
}
.detail-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.detail-title {
  font-size: 16px;
  font-weight: 600;
}
.detail-sub {
  margin-top: 4px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
  font-weight: 400;
}
.status-row {
  display: flex;
  flex-wrap: wrap;
  gap: 24px;
  margin-bottom: 8px;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--el-border-color-lighter);
}
.status-item {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 120px;
}
.status-label {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.status-item .danger {
  color: var(--el-color-danger);
  font-weight: 600;
}
.detail-tabs {
  margin-top: 8px;
}
.tab-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
}
.tab-count {
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.scheme-alert {
  margin-bottom: 8px;
}
.block {
  margin-top: 20px;
}
.block-title {
  margin-bottom: 12px;
  font-size: 14px;
  font-weight: 600;
}
.scheme-section + .scheme-section {
  margin-top: 16px;
}
.scheme-label {
  margin-bottom: 8px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
}
.tag {
  margin: 0 8px 8px 0;
}
.macro-add {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}
.gov-fold {
  margin-bottom: 16px;
  border: none;
}
.gov-summary {
  margin-left: 12px;
  color: var(--el-text-color-secondary);
  font-size: 13px;
  font-weight: 400;
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
</style>
