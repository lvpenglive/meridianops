<template>
  <div class="alerts-page">
    <!-- Tab 切换：告警事件 / 静默规则 -->
    <el-tabs v-model="activeTab" class="page-tabs">
      <el-tab-pane label="告警事件" name="events">
        <!-- 统计卡片 -->
        <el-row :gutter="12" class="stats-row" v-loading="statsLoading">
          <el-col :xs="12" :sm="6">
            <div class="stat-card stat-active">
              <div class="stat-icon"><el-icon><Bell /></el-icon></div>
              <div class="stat-body">
                <div class="stat-label">活跃告警</div>
                <div class="stat-value">{{ stats?.activeTotal ?? 0 }}</div>
                <div class="stat-sub">未解决告警总数</div>
              </div>
            </div>
          </el-col>
          <el-col :xs="12" :sm="6">
            <div class="stat-card stat-p0">
              <div class="stat-icon"><el-icon><Warning /></el-icon></div>
              <div class="stat-body">
                <div class="stat-label">P0 紧急</div>
                <div class="stat-value">{{ stats?.bySeverity?.P0 ?? 0 }}</div>
                <div class="stat-sub">需立即处置</div>
              </div>
            </div>
          </el-col>
          <el-col :xs="12" :sm="6">
            <div class="stat-card stat-p1">
              <div class="stat-icon"><el-icon><CircleClose /></el-icon></div>
              <div class="stat-body">
                <div class="stat-label">P1 重要</div>
                <div class="stat-value">{{ stats?.bySeverity?.P1 ?? 0 }}</div>
                <div class="stat-sub">影响部分业务</div>
              </div>
            </div>
          </el-col>
          <el-col :xs="12" :sm="6">
            <div class="stat-card stat-today">
              <div class="stat-icon"><el-icon><Plus /></el-icon></div>
              <div class="stat-body">
                <div class="stat-label">今日新增</div>
                <div class="stat-value">{{ stats?.todayNew ?? 0 }}</div>
                <div class="stat-sub">今日触发的告警</div>
              </div>
            </div>
          </el-col>
        </el-row>

        <!-- 筛选区 -->
        <el-card shadow="never" class="filter-card">
          <el-form :inline="true" :model="filter" @submit.prevent>
            <el-form-item label="级别">
              <el-select v-model="filter.severity" placeholder="全部" clearable style="width: 140px" @change="onFilter">
                <el-option label="灾难 Disaster" value="disaster" />
                <el-option label="严重 Critical" value="critical" />
                <el-option label="重要 High" value="high" />
                <el-option label="一般 Average" value="average" />
                <el-option label="警告 Warning" value="warning" />
                <el-option label="提示 Information" value="information" />
                <el-option label="提示 Info" value="info" />
              </el-select>
            </el-form-item>
            <el-form-item label="状态">
              <el-select v-model="filter.status" placeholder="全部" clearable style="width: 120px" @change="onFilter">
                <el-option label="触发中" value="firing" />
                <el-option label="已认领" value="acknowledged" />
                <el-option label="已解决" value="resolved" />
                <el-option label="待评估" value="pending" />
                <el-option label="已静默" value="suppressed" />
              </el-select>
            </el-form-item>
            <el-form-item label="来源">
              <el-select v-model="filter.source" placeholder="全部" clearable style="width: 130px" @change="onFilter">
                <el-option label="Zabbix" value="zabbix" />
                <el-option label="Prometheus" value="prometheus" />
                <el-option label="SNMP Trap" value="snmptrap" />
                <el-option label="Kafka 接入" value="kafka" />
                <el-option label="Eventide 推送" value="eventide" />
                <el-option label="人工上报" value="manual" />
                <el-option label="作业执行" value="job" />
                <el-option label="系统内置" value="system" />
              </el-select>
            </el-form-item>
            <el-form-item label="关键字">
              <el-input v-model="filter.keyword" placeholder="标题/详情/资产名" clearable style="width: 220px"
                @keyup.enter="onFilter" @clear="onFilter" />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :icon="Search" @click="onFilter">查询</el-button>
              <el-button :icon="Refresh" @click="resetFilter">重置</el-button>
              <el-button v-if="hasPermission('alert:create')" type="success" :icon="Plus"
                @click="openCreateDialog">新建告警</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <!-- 列表 -->
        <el-card shadow="never" class="list-card">
          <el-table v-loading="listLoading" :data="events" stripe @row-click="openDetail" row-class-name="clickable-row">
            <el-table-column label="级别" width="100">
              <template #default="{ row }">
                <el-tag :type="severityTagType(row.severity)" effect="dark" size="small">
                  {{ severityLabel(row.severity) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="状态" width="110">
              <template #default="{ row }">
                <el-tag :type="statusTagType(row.status)" effect="plain" size="small">
                  {{ statusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="title" label="告警标题" min-width="220" show-overflow-tooltip />
            <el-table-column label="来源" width="110">
              <template #default="{ row }">
                <el-tag size="small" type="info" effect="plain">{{ sourceLabel(row.source) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="关联资产" min-width="180" show-overflow-tooltip>
              <template #default="{ row }">
                <span v-if="row.ciName">{{ row.ciName }}</span>
                <span v-else class="text-muted">—</span>
              </template>
            </el-table-column>
            <el-table-column label="触发次数" width="90" align="center">
              <template #default="{ row }">
                <el-badge v-if="row.fireCount > 1" :value="row.fireCount" type="warning" />
                <span v-else>1</span>
              </template>
            </el-table-column>
            <el-table-column label="触发时间" width="170">
              <template #default="{ row }">{{ formatTime(row.firedAt) }}</template>
            </el-table-column>
            <el-table-column label="认领人" width="100">
              <template #default="{ row }">
                <span v-if="row.acknowledgedBy">{{ row.acknowledgedBy }}</span>
                <span v-else class="text-muted">—</span>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="240" fixed="right">
              <template #default="{ row }">
                <el-button v-if="hasPermission('alert:update') && row.status === 'firing'" link type="primary"
                  size="small" @click.stop="ackEvent(row)">认领</el-button>
                <el-button v-if="hasPermission('alert:update') && row.status !== 'resolved'" link type="success"
                  size="small" @click.stop="openResolveDialog(row)">解决</el-button>
                <el-button link type="primary" size="small" @click.stop="openDetail(row)">详情</el-button>
                <el-button v-if="hasPermission('alert:delete')" link type="danger" size="small"
                  @click.stop="deleteEvent(row)">删除</el-button>
              </template>
            </el-table-column>
            <template #empty>
              <el-empty description="暂无告警事件" />
            </template>
          </el-table>

          <div class="pager">
            <el-pagination v-model:current-page="filter.page" v-model:page-size="filter.pageSize"
              :total="total" :page-sizes="[10, 20, 50, 100]" layout="total, sizes, prev, pager, next, jumper"
              @size-change="loadEvents" @current-change="loadEvents" />
          </div>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="静默规则" name="silences">
        <el-card shadow="never" class="list-card">
          <template #header>
            <div class="card-header-row">
              <span>静默规则列表</span>
              <el-button v-if="hasPermission('alert:update')" type="primary" :icon="Plus" size="small"
                @click="openSilenceDialog(null)">新建静默规则</el-button>
            </div>
          </template>
          <el-table v-loading="silenceLoading" :data="silences" stripe>
            <el-table-column prop="name" label="规则名称" min-width="160" show-overflow-tooltip />
            <el-table-column label="匹配条件" min-width="220" show-overflow-tooltip>
              <template #default="{ row }">
                <code class="match-code">{{ formatMatchLabels(row.matchLabels) }}</code>
              </template>
            </el-table-column>
            <el-table-column label="生效时间" min-width="280">
              <template #default="{ row }">
                {{ formatTime(row.startsAt) }} ~ {{ formatTime(row.endsAt) }}
              </template>
            </el-table-column>
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="row.active ? 'success' : 'info'" effect="plain" size="small">
                  {{ row.active ? '生效中' : '未生效' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="createdBy" label="创建人" width="100" />
            <el-table-column label="操作" width="180" fixed="right">
              <template #default="{ row }">
                <el-button v-if="hasPermission('alert:update')" link type="primary" size="small"
                  @click="openSilenceDialog(row)">编辑</el-button>
                <el-button v-if="hasPermission('alert:update')" link type="danger" size="small"
                  @click="deleteSilence(row)">删除</el-button>
              </template>
            </el-table-column>
            <template #empty>
              <el-empty description="暂无静默规则" />
            </template>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <!-- 详情抽屉 -->
    <el-drawer v-model="detailVisible" :title="`告警详情 #${detail?.id?.slice(-8) ?? ''}`" size="600px" direction="rtl">
      <div v-if="detail" v-loading="detailLoading" class="detail-body">
        <el-descriptions :column="1" border>
          <el-descriptions-item label="告警级别">
            <el-tag :type="severityTagType(detail.severity)" effect="dark" size="small">
              {{ severityLabel(detail.severity) }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="statusTagType(detail.status)" effect="plain" size="small">
              {{ statusLabel(detail.status) }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="来源">{{ sourceLabel(detail.source) }}</el-descriptions-item>
          <el-descriptions-item label="标题">{{ detail.title }}</el-descriptions-item>
          <el-descriptions-item v-if="detail.message" label="详情">
            <div class="msg-block">{{ detail.message }}</div>
          </el-descriptions-item>
          <el-descriptions-item v-if="detail.ciName" label="关联资产">
            <router-link v-if="detail.ciId" :to="`/assets/${detail.ciId}`" class="asset-link">
              {{ detail.ciName }}
            </router-link>
            <span v-else>{{ detail.ciName }}</span>
          </el-descriptions-item>
          <el-descriptions-item v-if="detail.labels && Object.keys(detail.labels).length" label="标签">
            <div class="labels-block">
              <el-tag v-for="(v, k) in detail.labels" :key="k" size="small" type="info" effect="plain" class="label-tag">
                {{ k }}: {{ v }}
              </el-tag>
            </div>
          </el-descriptions-item>
          <el-descriptions-item label="触发次数">{{ detail.fireCount }} 次</el-descriptions-item>
          <el-descriptions-item label="首次触发">{{ formatTime(detail.firstFiredAt) }}</el-descriptions-item>
          <el-descriptions-item label="最近触发">{{ formatTime(detail.firedAt) }}</el-descriptions-item>
          <el-descriptions-item v-if="detail.acknowledgedBy" label="认领人">
            {{ detail.acknowledgedBy }}（{{ formatTime(detail.acknowledgedAt!) }}）
          </el-descriptions-item>
          <el-descriptions-item v-if="detail.resolvedBy" label="解决人">
            {{ detail.resolvedBy }}（{{ formatTime(detail.resolvedAt!) }}）
          </el-descriptions-item>
          <el-descriptions-item v-if="detail.resolutionNote" label="解决备注">
            <div class="msg-block">{{ detail.resolutionNote }}</div>
          </el-descriptions-item>
        </el-descriptions>

        <div v-if="hasPermission('alert:update') && detail.status !== 'resolved'" class="detail-actions">
          <el-button v-if="detail.status === 'firing'" type="primary" :icon="Check" @click="ackEvent(detail, true)">认领告警</el-button>
          <el-button type="success" :icon="CircleCheck" @click="openResolveDialog(detail)">解决告警</el-button>
          <el-button :icon="EditPen" @click="openNoteDialog(detail)">更新备注</el-button>
        </div>
      </div>
    </el-drawer>

    <!-- 新建告警对话框 -->
    <el-dialog v-model="createVisible" title="新建告警" width="600px">
      <el-form ref="createFormRef" :model="createForm" :rules="createRules" label-width="100px">
        <el-form-item label="来源" prop="source">
          <el-select v-model="createForm.source" style="width: 100%">
            <el-option label="人工上报" value="manual" />
            <el-option label="Zabbix" value="zabbix" />
            <el-option label="Prometheus" value="prometheus" />
            <el-option label="作业执行" value="job" />
            <el-option label="系统内置" value="system" />
          </el-select>
        </el-form-item>
        <el-form-item label="级别" prop="severity">
          <el-select v-model="createForm.severity" style="width: 100%">
            <el-option label="P0 紧急" value="P0" />
            <el-option label="P1 重要" value="P1" />
            <el-option label="P2 次要" value="P2" />
            <el-option label="P3 警告" value="P3" />
            <el-option label="提示" value="info" />
          </el-select>
        </el-form-item>
        <el-form-item label="标题" prop="title">
          <el-input v-model="createForm.title" placeholder="如：核心数据库 CPU 100%" />
        </el-form-item>
        <el-form-item label="详情" prop="message">
          <el-input v-model="createForm.message" type="textarea" :rows="4" placeholder="告警详情描述" />
        </el-form-item>
        <el-form-item label="关联资产">
          <el-input v-model="createForm.ci_name_snapshot" placeholder="资产名称（可选，仅用于快照显示）" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="createLoading" @click="submitCreate">创建</el-button>
      </template>
    </el-dialog>

    <!-- 解决告警对话框 -->
    <el-dialog v-model="resolveVisible" title="解决告警" width="500px">
      <el-form label-width="100px">
        <el-form-item label="告警标题">
          <span>{{ resolveTarget?.title }}</span>
        </el-form-item>
        <el-form-item label="解决备注">
          <el-input v-model="resolveNote" type="textarea" :rows="4" placeholder="处置过程、原因、措施说明" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="resolveVisible = false">取消</el-button>
        <el-button type="success" :loading="resolveLoading" @click="submitResolve">确认解决</el-button>
      </template>
    </el-dialog>

    <!-- 更新备注对话框 -->
    <el-dialog v-model="noteVisible" title="更新备注" width="500px">
      <el-form label-width="100px">
        <el-form-item label="告警标题">
          <span>{{ noteTarget?.title }}</span>
        </el-form-item>
        <el-form-item label="备注内容">
          <el-input v-model="noteContent" type="textarea" :rows="4" placeholder="处置过程说明" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="noteVisible = false">取消</el-button>
        <el-button type="primary" :loading="noteLoading" @click="submitNote">保存</el-button>
      </template>
    </el-dialog>

    <!-- 静默规则对话框 -->
    <el-dialog v-model="silenceDialogVisible" :title="silenceForm.id ? '编辑静默规则' : '新建静默规则'" width="640px">
      <el-form ref="silenceFormRef" :model="silenceForm" :rules="silenceRules" label-width="100px">
        <el-form-item label="规则名称" prop="name">
          <el-input v-model="silenceForm.name" placeholder="如：变更窗口静默" />
        </el-form-item>
        <el-form-item label="静默理由">
          <el-input v-model="silenceForm.reason" type="textarea" :rows="2" placeholder="说明为什么需要静默" />
        </el-form-item>
        <el-form-item label="匹配条件">
          <div class="match-editor">
            <div v-for="(item, idx) in matchItems" :key="idx" class="match-row">
              <el-input v-model="item.key" placeholder="字段名（如 source）" style="width: 130px" />
              <el-input v-model="item.value" placeholder="值（逗号分隔多个值）" style="flex: 1" />
              <el-button :icon="Delete" link type="danger" @click="matchItems.splice(idx, 1)" />
            </div>
            <el-button :icon="Plus" link type="primary" @click="matchItems.push({ key: '', value: '' })">添加条件</el-button>
            <div class="match-tip">常用字段：source / severity / ciId；severity 可填 P0,P1 多值</div>
          </div>
        </el-form-item>
        <el-form-item label="生效时间" prop="range">
          <el-date-picker v-model="silenceForm.range" type="datetimerange" range-separator="至"
            start-placeholder="开始时间" end-placeholder="结束时间" format="YYYY-MM-DD HH:mm" value-format="YYYY-MM-DDTHH:mm:ssZ"
            style="width: 100%" />
        </el-form-item>
        <el-form-item v-if="silenceForm.id" label="状态">
          <el-switch v-model="silenceForm.active" active-text="启用" inactive-text="停用" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="silenceDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="silenceSaveLoading" @click="submitSilence">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance } from 'element-plus'
import {
  Bell, Warning, CircleClose, Plus, Search, Refresh, Check, CircleCheck,
  EditPen, Delete,
} from '@element-plus/icons-vue'
import { useUserStore } from '../../stores/user'
import {
  listAlertEvents, getAlertEvent, createAlertEvent, acknowledgeAlert, resolveAlert,
  updateAlertNote, deleteAlertEvent, getAlertStats,
  listAlertSilences, createAlertSilence, updateAlertSilence, deleteAlertSilence,
  type AlertEvent, type AlertStats, type AlertSilence,
} from '../../api/alert'

const userStore = useUserStore()
const hasPermission = (perm: string) => userStore.hasPermission(perm)

// ============ 事件列表 ============
const activeTab = ref('events')
const listLoading = ref(false)
const events = ref<AlertEvent[]>([])
const total = ref(0)
const filter = reactive({
  page: 1,
  pageSize: 20,
  severity: '',
  status: '',
  source: '',
  keyword: '',
})

async function loadEvents() {
  listLoading.value = true
  try {
    const params: Record<string, unknown> = { page: filter.page, page_size: filter.pageSize }
    if (filter.severity) params.severity = filter.severity
    if (filter.status) params.status = filter.status
    if (filter.source) params.source = filter.source
    if (filter.keyword) params.keyword = filter.keyword
    const res = await listAlertEvents(params)
    events.value = res.items
    total.value = res.total
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    listLoading.value = false
  }
}

function onFilter() {
  filter.page = 1
  loadEvents()
}

function resetFilter() {
  filter.severity = ''
  filter.status = ''
  filter.source = ''
  filter.keyword = ''
  filter.page = 1
  loadEvents()
}

// ============ 统计 ============
const statsLoading = ref(false)
const stats = ref<AlertStats | null>(null)

async function loadStats() {
  statsLoading.value = true
  try {
    stats.value = await getAlertStats()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    statsLoading.value = false
  }
}

// ============ 详情 ============
const detailVisible = ref(false)
const detailLoading = ref(false)
const detail = ref<AlertEvent | null>(null)

async function openDetail(row: AlertEvent) {
  detailVisible.value = true
  detailLoading.value = true
  detail.value = row
  try {
    detail.value = await getAlertEvent(row.id)
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    detailLoading.value = false
  }
}

// ============ 新建告警 ============
const createVisible = ref(false)
const createLoading = ref(false)
const createFormRef = ref<FormInstance>()
const createForm = reactive({
  source: 'manual',
  severity: 'P2',
  title: '',
  message: '',
  ci_name_snapshot: '',
})
const createRules = {
  title: [{ required: true, message: '请输入告警标题', trigger: 'blur' }],
  severity: [{ required: true, message: '请选择级别', trigger: 'change' }],
  source: [{ required: true, message: '请选择来源', trigger: 'change' }],
}

function openCreateDialog() {
  createForm.source = 'manual'
  createForm.severity = 'P2'
  createForm.title = ''
  createForm.message = ''
  createForm.ci_name_snapshot = ''
  createVisible.value = true
}

async function submitCreate() {
  await createFormRef.value?.validate()
  createLoading.value = true
  try {
    const payload: Record<string, unknown> = {
      source: createForm.source,
      severity: createForm.severity,
      title: createForm.title,
    }
    if (createForm.message) payload.message = createForm.message
    if (createForm.ci_name_snapshot) payload.ci_name_snapshot = createForm.ci_name_snapshot
    const res = await createAlertEvent(payload)
    ElMessage.success(res.merged ? '告警已合并（同指纹告警触发次数 +1）' : '告警创建成功')
    createVisible.value = false
    loadEvents()
    loadStats()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    createLoading.value = false
  }
}

// ============ 认领 ============
async function ackEvent(row: AlertEvent, fromDetail = false) {
  try {
    await ElMessageBox.confirm(`确认认领告警「${row.title}」？`, '认领告警', { type: 'warning' })
  } catch {
    return
  }
  try {
    await acknowledgeAlert(row.id)
    ElMessage.success('认领成功')
    if (fromDetail && detail.value) {
      detail.value = { ...detail.value, status: 'acknowledged', acknowledgedBy: userStore.user?.username ?? '', acknowledgedAt: new Date().toISOString() }
    }
    loadEvents()
    loadStats()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  }
}

// ============ 解决 ============
const resolveVisible = ref(false)
const resolveLoading = ref(false)
const resolveTarget = ref<AlertEvent | null>(null)
const resolveNote = ref('')

function openResolveDialog(row: AlertEvent) {
  resolveTarget.value = row
  resolveNote.value = row.resolutionNote ?? ''
  resolveVisible.value = true
}

async function submitResolve() {
  if (!resolveTarget.value) return
  resolveLoading.value = true
  try {
    await resolveAlert(resolveTarget.value.id, resolveNote.value)
    ElMessage.success('告警已标记为解决')
    resolveVisible.value = false
    if (detail.value && detail.value.id === resolveTarget.value.id) {
      detail.value = {
        ...detail.value,
        status: 'resolved',
        resolvedBy: userStore.user?.username ?? '',
        resolvedAt: new Date().toISOString(),
        resolutionNote: resolveNote.value || null,
      }
    }
    loadEvents()
    loadStats()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    resolveLoading.value = false
  }
}

// ============ 备注 ============
const noteVisible = ref(false)
const noteLoading = ref(false)
const noteTarget = ref<AlertEvent | null>(null)
const noteContent = ref('')

function openNoteDialog(row: AlertEvent) {
  noteTarget.value = row
  noteContent.value = row.resolutionNote ?? ''
  noteVisible.value = true
}

async function submitNote() {
  if (!noteTarget.value) return
  noteLoading.value = true
  try {
    await updateAlertNote(noteTarget.value.id, noteContent.value)
    ElMessage.success('备注已更新')
    if (detail.value && detail.value.id === noteTarget.value.id) {
      detail.value = { ...detail.value, resolutionNote: noteContent.value }
    }
    noteVisible.value = false
    loadEvents()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    noteLoading.value = false
  }
}

// ============ 删除 ============
async function deleteEvent(row: AlertEvent) {
  try {
    await ElMessageBox.confirm(`确认删除告警「${row.title}」？此操作不可恢复。`, '删除告警', { type: 'warning' })
  } catch {
    return
  }
  try {
    await deleteAlertEvent(row.id)
    ElMessage.success('已删除')
    loadEvents()
    loadStats()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  }
}

// ============ 静默规则 ============
const silenceLoading = ref(false)
const silences = ref<AlertSilence[]>([])

async function loadSilences() {
  silenceLoading.value = true
  try {
    silences.value = await listAlertSilences()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    silenceLoading.value = false
  }
}

const silenceDialogVisible = ref(false)
const silenceSaveLoading = ref(false)
const silenceFormRef = ref<FormInstance>()
const silenceForm = reactive<{
  id: string
  name: string
  reason: string
  range: [string, string] | [Date, Date] | null
  active: boolean
}>({
  id: '',
  name: '',
  reason: '',
  range: null,
  active: true,
})
const silenceRules = {
  name: [{ required: true, message: '请输入规则名称', trigger: 'blur' }],
  range: [{ required: true, message: '请选择生效时间', trigger: 'change' }],
}
const matchItems = reactive<{ key: string; value: string }[]>([])

function openSilenceDialog(row: AlertSilence | null) {
  if (row) {
    silenceForm.id = row.id
    silenceForm.name = row.name
    silenceForm.reason = row.reason ?? ''
    silenceForm.active = row.active
    silenceForm.range = [new Date(row.startsAt), new Date(row.endsAt)] as [Date, Date]
    matchItems.length = 0
    if (row.matchLabels) {
      for (const [k, v] of Object.entries(row.matchLabels)) {
        if (Array.isArray(v)) {
          matchItems.push({ key: k, value: (v as unknown[]).join(',') })
        } else {
          matchItems.push({ key: k, value: String(v) })
        }
      }
    }
  } else {
    silenceForm.id = ''
    silenceForm.name = ''
    silenceForm.reason = ''
    silenceForm.active = true
    const now = new Date()
    const later = new Date(now.getTime() + 2 * 60 * 60 * 1000)
    silenceForm.range = [now, later] as [Date, Date]
    matchItems.length = 0
  }
  silenceDialogVisible.value = true
}

async function submitSilence() {
  await silenceFormRef.value?.validate()
  if (!silenceForm.range || silenceForm.range.length !== 2) {
    ElMessage.warning('请选择生效时间')
    return
  }
  const labels: Record<string, string[]> = {}
  for (const item of matchItems) {
    if (item.key.trim() && item.value.trim()) {
      labels[item.key.trim()] = item.value.split(',').map(s => s.trim()).filter(Boolean)
    }
  }
  silenceSaveLoading.value = true
  try {
    const payload: Record<string, unknown> = {
      name: silenceForm.name,
      reason: silenceForm.reason || undefined,
      match_labels: Object.keys(labels).length ? labels : undefined,
      starts_at: formatDateForApi(silenceForm.range[0]),
      ends_at: formatDateForApi(silenceForm.range[1]),
    }
    if (silenceForm.id) payload.active = silenceForm.active
    if (silenceForm.id) {
      await updateAlertSilence(silenceForm.id, payload as Parameters<typeof updateAlertSilence>[1])
      ElMessage.success('静默规则已更新')
    } else {
      await createAlertSilence(payload as Parameters<typeof createAlertSilence>[0])
      ElMessage.success('静默规则已创建')
    }
    silenceDialogVisible.value = false
    loadSilences()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    silenceSaveLoading.value = false
  }
}

async function deleteSilence(row: AlertSilence) {
  try {
    await ElMessageBox.confirm(`确认删除静默规则「${row.name}」？`, '删除', { type: 'warning' })
  } catch {
    return
  }
  try {
    await deleteAlertSilence(row.id)
    ElMessage.success('已删除')
    loadSilences()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  }
}

// ============ 工具函数 ============
function errMsg(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e) return (e as { message: string }).message
  return String(e)
}

function severityTagType(s: string): 'danger' | 'warning' | 'info' | 'success' | 'primary' {
  if (s === 'disaster' || s === 'critical') return 'danger'
  if (s === 'high') return 'warning'
  if (s === 'average') return 'primary'
  if (s === 'warning') return 'info'
  return 'success'
}

function severityLabel(s: string): string {
  const map: Record<string, string> = {
    disaster: '灾难 Disaster',
    critical: '严重 Critical',
    high: '重要 High',
    average: '一般 Average',
    warning: '警告 Warning',
    information: '提示 Information',
    info: '提示 Info',
    P0: 'P0 紧急', P1: 'P1 重要', P2: 'P2 次要', P3: 'P3 警告',
  }
  return map[s] ?? s
}

function statusTagType(s: string): 'danger' | 'warning' | 'success' | 'info' {
  if (s === 'firing') return 'danger'
  if (s === 'acknowledged') return 'warning'
  if (s === 'resolved') return 'success'
  return 'info'
}

function statusLabel(s: string): string {
  const map: Record<string, string> = {
    firing: '触发中', acknowledged: '已认领', resolved: '已解决',
    pending: '待评估', suppressed: '已静默',
  }
  return map[s] ?? s
}

function sourceLabel(s: string): string {
  const map: Record<string, string> = {
    zabbix: 'Zabbix', prometheus: 'Prometheus',
    snmptrap: 'SNMP Trap', kafka: 'Kafka 接入', eventide: 'Eventide 推送',
    manual: '人工上报', job: '作业执行', system: '系统内置',
  }
  return map[s] ?? s
}

function formatTime(s: string | null | undefined): string {
  if (!s) return '—'
  try {
    const d = new Date(s)
    if (isNaN(d.getTime())) return s
    return d.toLocaleString('zh-CN', { hour12: false })
  } catch {
    return s
  }
}

function formatDateForApi(d: Date | string): string {
  const date = typeof d === 'string' ? new Date(d) : d
  // 转换为本地时区 RFC3339
  const tz = date.getTimezoneOffset() * 60000
  const local = new Date(date.getTime() - tz)
  return local.toISOString().replace(/\.\d{3}Z$/, 'Z')
}

function formatMatchLabels(labels: Record<string, unknown> | null): string {
  if (!labels) return '匹配所有告警'
  const parts: string[] = []
  for (const [k, v] of Object.entries(labels)) {
    if (Array.isArray(v)) parts.push(`${k}=${v.join('|')}`)
    else parts.push(`${k}=${String(v)}`)
  }
  return parts.length ? parts.join(' & ') : '匹配所有告警'
}

// ============ 初始化 ============
onMounted(() => {
  loadStats()
  loadEvents()
  loadSilences()
})
</script>

<style scoped>
.alerts-page { padding: 16px; }
.page-tabs { background: #fff; padding: 12px 16px; border-radius: 4px; }
.stats-row { margin-bottom: 12px; }
.stat-card {
  display: flex; align-items: center; gap: 12px;
  padding: 16px; border-radius: 8px; background: #fff;
  border-left: 4px solid #dcdfe6; box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
}
.stat-icon {
  width: 48px; height: 48px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  background: #f4f4f5; color: #909399; font-size: 22px;
}
.stat-active { border-left-color: #f56c6c; }
.stat-active .stat-icon { background: #fef0f0; color: #f56c6c; }
.stat-p0 { border-left-color: #f56c6c; }
.stat-p0 .stat-icon { background: #fef0f0; color: #f56c6c; }
.stat-p1 { border-left-color: #e6a23c; }
.stat-p1 .stat-icon { background: #fdf6ec; color: #e6a23c; }
.stat-today { border-left-color: #409eff; }
.stat-today .stat-icon { background: #ecf5ff; color: #409eff; }
.stat-label { font-size: 13px; color: #909399; }
.stat-value { font-size: 26px; font-weight: 600; color: #303133; line-height: 1.2; margin: 2px 0; }
.stat-sub { font-size: 12px; color: #c0c4cc; }

.filter-card { margin-bottom: 12px; }
.filter-card :deep(.el-form-item) { margin-bottom: 0; }
.list-card { margin-top: 0; }
.card-header-row { display: flex; justify-content: space-between; align-items: center; }
.pager { margin-top: 12px; display: flex; justify-content: flex-end; }

.detail-body { padding: 0 8px; }
.msg-block { white-space: pre-wrap; word-break: break-word; color: #606266; }
.labels-block { display: flex; flex-wrap: wrap; gap: 6px; }
.label-tag { font-size: 12px; }
.asset-link { color: #409eff; text-decoration: none; }
.asset-link:hover { text-decoration: underline; }
.detail-actions {
  margin-top: 20px; padding-top: 16px; border-top: 1px solid #ebeef5;
  display: flex; gap: 8px;
}

.match-editor { width: 100%; }
.match-row {
  display: flex; gap: 8px; margin-bottom: 8px; align-items: center;
}
.match-tip { font-size: 12px; color: #909399; margin-top: 4px; }
.match-code {
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px; background: #f5f7fa; padding: 2px 6px; border-radius: 3px;
}

.text-muted { color: #c0c4cc; }
:deep(.clickable-row) { cursor: pointer; }
:deep(.clickable-row:hover) { background: #f5f7fa !important; }
</style>
