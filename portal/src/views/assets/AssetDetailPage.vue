<template>
  <div class="asset-detail" v-loading="loading">
    <!-- 顶部信息栏 -->
    <div class="detail-header">
      <div class="header-left">
        <el-button :icon="ArrowLeft" text @click="goBack">返回列表</el-button>
        <el-divider direction="vertical" />
        <el-icon class="model-icon-bg" :style="iconStyle"><component :is="iconComp" /></el-icon>
        <div class="title-block">
          <div class="title-row">
            <span class="asset-name">{{ instance?.name || '加载中...' }}</span>
            <el-tag v-if="instance" :type="statusTagType(instance.status)" size="small">{{ statusLabel(instance.status) }}</el-tag>
            <el-tag v-if="instance?.source" type="warning" size="small" effect="plain">{{ sourceLabel(instance.source) }}</el-tag>
            <el-tag v-else-if="instance" type="info" size="small" effect="plain">手工录入</el-tag>
          </div>
          <div class="meta-row" v-if="instance">
            <span>类型：{{ modelName }}</span>
            <el-divider direction="vertical" />
            <span>创建：{{ formatTime(instance.createdAt) }}</span>
            <el-divider direction="vertical" />
            <span>更新：{{ formatTime(instance.updatedAt) }}</span>
            <template v-if="instance.lastSyncedAt">
              <el-divider direction="vertical" />
              <span>同步：{{ formatTime(instance.lastSyncedAt) }}</span>
            </template>
          </div>
        </div>
      </div>
      <div class="header-right">
        <el-button
          v-if="hasPermission('asset:update') && modelCode === 'business_system'"
          type="success"
          plain
          :icon="Plus"
          @click="openCreateService"
        >新建下属服务</el-button>
        <el-button
          v-if="hasPermission('asset:update') && modelCode === 'service'"
          type="success"
          plain
          :icon="Plus"
          @click="openLinkHost"
        >关联主机</el-button>
        <el-button
          v-if="hasPermission('asset:update')"
          type="primary"
          :icon="Edit"
          @click="openEdit"
        >编辑</el-button>
      </div>
    </div>

    <el-card shadow="never">
      <el-tabs v-model="activeTab" @tab-change="onTabChange">
        <!-- 基本信息 -->
        <el-tab-pane label="基本信息" name="info">
          <div v-if="instance" class="info-grid">
            <div class="info-section">
              <div class="section-title">基础属性</div>
              <el-descriptions :column="2" border>
                <el-descriptions-item label="资产名称">{{ instance.name }}</el-descriptions-item>
                <el-descriptions-item label="CI 模型">{{ modelName }}</el-descriptions-item>
                <el-descriptions-item label="状态">
                  <el-tag :type="statusTagType(instance.status)" size="small">{{ statusLabel(instance.status) }}</el-tag>
                </el-descriptions-item>
                <el-descriptions-item label="标签">
                  <span v-if="instance.tags">{{ instance.tags }}</span>
                  <span v-else class="text-muted">—</span>
                </el-descriptions-item>
                <el-descriptions-item label="负责人">
                  <span v-if="ownerText">{{ ownerText }}</span>
                  <span v-else class="text-muted">—</span>
                </el-descriptions-item>
                <el-descriptions-item label="SSH 凭据">
                  <span v-if="sshCredentialLabel">{{ sshCredentialLabel }}</span>
                  <span v-else class="text-muted">未绑定（作业将用剧本默认凭据）</span>
                </el-descriptions-item>
                <el-descriptions-item label="SSH 通道">
                  {{ instance.sshExecMode === 'direct' ? '强制直连' : '自动（按监控 Proxy 解析跳板）' }}
                </el-descriptions-item>
                <el-descriptions-item label="数据来源">
                  <el-tag v-if="instance.source" size="small" type="warning" effect="plain">{{ sourceLabel(instance.source) }}</el-tag>
                  <el-tag v-else size="small" type="info" effect="plain">手工录入</el-tag>
                </el-descriptions-item>
                <el-descriptions-item label="外部ID">
                  <span v-if="instance.externalId">{{ instance.externalId }}</span>
                  <span v-else class="text-muted">—</span>
                </el-descriptions-item>
                <el-descriptions-item label="创建时间">{{ formatTime(instance.createdAt) }}</el-descriptions-item>
                <el-descriptions-item label="更新时间">{{ formatTime(instance.updatedAt) }}</el-descriptions-item>
              </el-descriptions>
            </div>

            <div class="info-section" v-if="modelAttrs.length">
              <div class="section-title">扩展属性（{{ modelAttrs.length }}）</div>
              <el-descriptions :column="2" border>
                <el-descriptions-item
                  v-for="attr in modelAttrs"
                  :key="attr.code"
                  :label="attr.name"
                >
                  <template v-if="instance.attributes">
                    <span v-if="instance.attributes[attr.code] !== undefined && instance.attributes[attr.code] !== '' && instance.attributes[attr.code] !== null">
                      {{ formatAttr(attr, instance.attributes[attr.code]) }}
                    </span>
                    <span v-else class="text-muted">—</span>
                  </template>
                  <span v-else class="text-muted">—</span>
                </el-descriptions-item>
              </el-descriptions>
            </div>

            <div class="info-section" v-if="extraAttrs.length">
              <div class="section-title">其他属性（未在模型定义中）</div>
              <el-descriptions :column="2" border>
                <el-descriptions-item
                  v-for="key in extraAttrs"
                  :key="key"
                  :label="key"
                >
                  {{ instance.attributes[key] }}
                </el-descriptions-item>
              </el-descriptions>
            </div>
          </div>
        </el-tab-pane>

        <!-- 关系 -->
        <el-tab-pane :label="`关系（${relations.length}）`" name="relations">
          <div class="tab-toolbar">
            <span class="tab-tip">
              <template v-if="modelCode === 'business_system'">建议：业务系统「包含」应用服务；服务再挂主机 / 中间件</template>
              <template v-else-if="modelCode === 'service'">建议：服务「运行于」主机；「依赖」中间件或数据库</template>
              <template v-else>该资产的所有依赖关系（作为源或目标）</template>
            </span>
            <el-button
              v-if="hasPermission('asset:update')"
              size="small"
              type="primary"
              :icon="Plus"
              @click="openAddRelation()"
            >添加关系</el-button>
          </div>
          <el-table :data="relations" v-loading="loadingRelations" stripe size="default">
            <el-table-column label="关系类型" width="160">
              <template #default="{ row }">
                <el-tag size="small">
                  {{ row.relationTypeName || relTypeNameMap[row.relationType] || row.relationType }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="方向" width="100">
              <template #default="{ row }">
                <el-tag v-if="row.sourceId === instanceId" size="small" type="success">源 → 目标</el-tag>
                <el-tag v-else size="small" type="warning">目标 ← 源</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="对端资产" min-width="220">
              <template #default="{ row }">
                <el-link
                  type="primary"
                  :underline="false"
                  @click="goToAsset(row.sourceId === instanceId ? row.targetId : row.sourceId)"
                >
                  {{ relNameMap[row.sourceId === instanceId ? row.targetId : row.sourceId] || '加载中...' }}
                </el-link>
              </template>
            </el-table-column>
            <el-table-column prop="createdAt" label="建立时间" width="180">
              <template #default="{ row }">{{ formatTime(row.createdAt) }}</template>
            </el-table-column>
            <el-table-column label="操作" width="100" fixed="right">
              <template #default="{ row }">
                <el-button
                  v-if="hasPermission('asset:update')"
                  size="small" link type="danger"
                  @click="onDeleteRelation(row)"
                >删除</el-button>
              </template>
            </el-table-column>
          </el-table>
          <el-empty v-if="!loadingRelations && relations.length === 0" description="暂无关系，点击「添加关系」建立依赖" />
        </el-tab-pane>

        <!-- 同步历史 -->
        <el-tab-pane :label="`同步历史（${syncLogsTotal}）`" name="sync">
          <div class="tab-toolbar">
            <span class="tab-tip">该资产的同步日志记录</span>
            <el-button :icon="Refresh" size="small" @click="fetchSyncLogs">刷新</el-button>
          </div>
          <el-table :data="syncLogs" v-loading="loadingSync" stripe size="default">
            <el-table-column prop="batchId" label="批次ID" width="280" show-overflow-tooltip>
              <template #default="{ row }">
                <span class="mono">{{ row.batchId.slice(0, 8) }}...</span>
              </template>
            </el-table-column>
            <el-table-column prop="sourceCode" label="数据源" width="120" />
            <el-table-column prop="action" label="动作" width="90">
              <template #default="{ row }">
                <el-tag size="small">{{ row.action }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="结果" width="90">
              <template #default="{ row }">
                <el-tag size="small" :type="row.status === 'success' ? 'success' : 'danger'">{{ row.status }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="message" label="消息" min-width="180" show-overflow-tooltip />
            <el-table-column prop="externalId" label="外部ID" width="140" show-overflow-tooltip />
            <el-table-column prop="createdAt" label="时间" width="170">
              <template #default="{ row }">{{ formatTime(row.createdAt) }}</template>
            </el-table-column>
          </el-table>
          <div class="pagination-row">
            <el-pagination
              v-model:current-page="syncPage"
              v-model:page-size="syncPageSize"
              :total="syncLogsTotal"
              :page-sizes="[10, 20, 50]"
              layout="total, sizes, prev, pager, next"
              @size-change="fetchSyncLogs"
              @current-change="fetchSyncLogs"
            />
          </div>
          <el-empty v-if="!loadingSync && syncLogs.length === 0" description="该资产无同步记录（可能为手工录入）" />
        </el-tab-pane>

        <!-- 变更记录 -->
        <el-tab-pane :label="`变更记录（${auditTotal}）`" name="audit">
          <div class="tab-toolbar">
            <span class="tab-tip">该资产的操作审计记录</span>
            <el-button :icon="Refresh" size="small" @click="fetchAuditLogs">刷新</el-button>
          </div>
          <el-table :data="auditLogs" v-loading="loadingAudit" stripe size="default">
            <el-table-column prop="action" label="操作" width="160">
              <template #default="{ row }">
                <el-tag size="small" :type="actionTagType(row.action)">{{ actionLabel(row.action) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="actorUsername" label="操作人" width="120" />
            <el-table-column prop="status" label="结果" width="90">
              <template #default="{ row }">
                <el-tag size="small" :type="row.status === 'success' ? 'success' : 'danger'">{{ row.status }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="ip" label="IP" width="140" />
            <el-table-column prop="detail" label="详情" min-width="220" show-overflow-tooltip>
              <template #default="{ row }">
                <span v-if="row.detail" class="mono">{{ row.detail }}</span>
                <span v-else class="text-muted">—</span>
              </template>
            </el-table-column>
            <el-table-column prop="createdAt" label="时间" width="170">
              <template #default="{ row }">{{ formatTime(row.createdAt) }}</template>
            </el-table-column>
          </el-table>
          <div class="pagination-row">
            <el-pagination
              v-model:current-page="auditPage"
              v-model:page-size="auditPageSize"
              :total="auditTotal"
              :page-sizes="[10, 20, 50]"
              layout="total, sizes, prev, pager, next"
              @size-change="fetchAuditLogs"
              @current-change="fetchAuditLogs"
            />
          </div>
          <el-empty v-if="!loadingAudit && auditLogs.length === 0" description="该资产暂无变更记录" />
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <AssetFormDialog
      v-model="editDialogVisible"
      mode="edit"
      :models="modelsForForm"
      :instance="instance"
      @saved="fetchInstance"
    />

    <AssetFormDialog
      v-model="createServiceVisible"
      mode="create"
      :models="serviceModelsForForm"
      :default-model-id="serviceModelId"
      @saved="onServiceCreated"
    />

    <!-- 添加关系对话框 -->
    <el-dialog v-model="relationDialogVisible" title="添加关系" width="520px">
      <el-form :model="relForm" label-width="90px">
        <el-form-item label="方向">
          <el-radio-group v-model="relForm.direction">
            <el-radio value="out">当前资产 → 目标</el-radio>
            <el-radio value="in">源 → 当前资产</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="对端资产">
          <el-select
            v-model="relForm.peerId"
            filterable
            remote
            reserve-keyword
            placeholder="输入资产名称搜索"
            :remote-method="searchAssets"
            :loading="searching"
            style="width: 100%"
          >
            <el-option
              v-for="a in assetOptions"
              :key="a.id"
              :label="`${a.name}（${a.modelCode}）`"
              :value="a.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="关系类型">
          <el-select v-model="relForm.relationType" style="width: 100%">
            <el-option
              v-for="t in enabledRelationTypes"
              :key="t.code"
              :label="`${t.name}（${t.code}）`"
              :value="t.code"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="relationDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="savingRel" @click="onSubmitRelation">添加</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ArrowLeft, Edit, Plus, Refresh, Monitor, Cpu, Coin, Connection, Grid, Service } from '@element-plus/icons-vue'
import {
  getCiInstance,
  getCiModel,
  listCiModels,
  listCiRelations,
  createCiRelation,
  deleteCiRelation,
  listCiInstances,
  listSyncLogs,
  listCiRelationTypes,
} from '../../api/cmdb'
import { listAuditLogs } from '../../api/audit'
import { listAllCredentials, type SshCredentialSimple } from '../../api/credential'
import type { CiInstance, CiModel, CiModelAttr, CiRelation, CiRelationType, SyncLog, AuditLog } from '../../api/types'
import { useUserStore } from '../../stores/user'
import { useSystemDicts, labelOf } from '../../composables/useSystemDicts'
import AssetFormDialog from './AssetFormDialog.vue'

const dicts = useSystemDicts()
const route = useRoute()
const router = useRouter()
const userStore = useUserStore()
const hasPermission = (code: string) => userStore.hasPermission(code)

const instanceId = computed(() => route.params.id as string)
const loading = ref(true)
const instance = ref<CiInstance | null>(null)
const modelAttrs = ref<CiModelAttr[]>([])
const modelName = ref('')
const modelCode = ref('')
const modelIcon = ref('')

const activeTab = ref('info')
const createServiceVisible = ref(false)
const serviceModelId = ref('')
const allModels = ref<CiModel[]>([])

const serviceModelsForForm = computed(() =>
  allModels.value.filter((m) => m.code === 'service' && m.enabled),
)

// 模型图标
const iconMap: Record<string, any> = { Monitor, Cpu, Coin, Connection, Grid, Service }
const iconComp = computed(() => iconMap[modelIcon.value] || iconMap[modelCode.value === 'service' ? 'Service' : 'Monitor'] || Monitor)
const iconStyle = computed(() => {
  const map: Record<string, string> = {
    business_system: 'linear-gradient(135deg, #f093fb, #f5576c)',
    service: 'linear-gradient(135deg, #5b86e5, #36d1dc)',
    host: 'linear-gradient(135deg, #4facfe, #00f2fe)',
    middleware: 'linear-gradient(135deg, #43e97b, #38f9d7)',
    database: 'linear-gradient(135deg, #fa709a, #fee140)',
    network_device: 'linear-gradient(135deg, #a8edea, #fed6e3)',
  }
  return { background: map[modelCode.value] || 'linear-gradient(135deg, #667eea, #764ba2)' }
})

// ---- 基本信息 ----
const extraAttrs = computed(() => {
  if (!instance.value?.attributes) return []
  const defined = new Set(modelAttrs.value.map(a => a.code))
  return Object.keys(instance.value.attributes).filter(k => !defined.has(k))
})

const ownerText = computed(() => {
  const inst = instance.value
  if (!inst) return ''
  if (inst.ownerNames?.length) return inst.ownerNames.join('、')
  if (inst.owners?.length) {
    return inst.owners
      .map((o) => (o.displayName?.trim() ? o.displayName : o.username))
      .filter(Boolean)
      .join('、')
  }
  return ''
})

const credentialOptions = ref<SshCredentialSimple[]>([])
async function fetchCredentialOptions() {
  try {
    credentialOptions.value = await listAllCredentials()
  } catch {
    credentialOptions.value = []
  }
}
const sshCredentialLabel = computed(() => {
  const id = instance.value?.sshCredentialId
  if (id == null) return ''
  const c = credentialOptions.value.find((x) => x.id === id)
  if (!c) return `凭据 #${id}`
  return `${c.name}（${c.username}）`
})
const modelsForForm = computed<CiModel[]>(() => {
  if (!instance.value) return []
  return [
    {
      id: instance.value.modelId,
      name: modelName.value || instance.value.modelId,
      code: '',
      icon: '',
      description: '',
      enabled: true,
      sortOrder: 0,
      createdAt: '',
      updatedAt: '',
    } as CiModel,
  ]
})

async function fetchInstance() {
  loading.value = true
  try {
    instance.value = await getCiInstance(instanceId.value)
    if (instance.value) {
      try {
        const detail = await getCiModel(instance.value.modelId)
        modelAttrs.value = detail.attributes
        modelName.value = detail.model.name
        modelCode.value = detail.model.code || ''
        modelIcon.value = detail.model.icon || ''
      } catch { /* ignore */ }
    }
  } catch (e: any) {
    ElMessage.error(e?.message || '加载资产失败')
  } finally {
    loading.value = false
  }
}

async function ensureModels() {
  if (allModels.value.length) return
  try {
    allModels.value = await listCiModels()
  } catch {
    allModels.value = []
  }
  const svc = allModels.value.find((m) => m.code === 'service' && m.enabled)
  serviceModelId.value = svc?.id || ''
}

async function openCreateService() {
  await ensureModels()
  if (!serviceModelId.value) {
    ElMessage.warning('未找到「应用服务」模型，请先执行 CMDB 服务化迁移')
    return
  }
  createServiceVisible.value = true
}

async function onServiceCreated(created?: CiInstance) {
  await fetchInstance()
  if (!created?.id || !instance.value) return
  try {
    await createCiRelation({
      sourceId: instance.value.id,
      targetId: created.id,
      relationType: 'contains',
    })
    ElMessage.success('已自动建立「业务系统 → 包含 → 服务」关系')
    loadedTabs.value.add('relations')
    await fetchRelations()
    activeTab.value = 'relations'
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : '自动建关系失败，请在关系页手工添加 contains'
    ElMessage.warning(msg)
  }
}

// ---- 关系 ----
const loadingRelations = ref(false)
const relations = ref<CiRelation[]>([])
const relNameMap = ref<Record<string, string>>({})

// 关系类型字典（动态加载，启用项用于下拉选择；名称用于表格展示）
const relationTypes = ref<CiRelationType[]>([])
const enabledRelationTypes = computed(() => relationTypes.value.filter(t => t.enabled))
const relTypeNameMap = computed<Record<string, string>>(() => {
  const m: Record<string, string> = {}
  relationTypes.value.forEach(t => { m[t.code] = t.name })
  return m
})

async function fetchRelationTypes() {
  try {
    relationTypes.value = await listCiRelationTypes()
    // 当前默认值不在启用列表中时，回退到第一个启用项
    const enabled = relationTypes.value.filter(t => t.enabled)
    if (enabled.length && !enabled.some(t => t.code === relForm.relationType)) {
      relForm.relationType = enabled[0].code
    }
  } catch {
    // 静默失败：不影响详情页主流程，下拉降级为空
  }
}

async function fetchRelations() {
  loadingRelations.value = true
  try {
    relations.value = await listCiRelations(instanceId.value)
    // 接口已 JOIN 返回对端名称，直接构建映射，无需逐个请求（N+1）
    const map: Record<string, string> = {}
    relations.value.forEach(r => {
      if (r.sourceId !== instanceId.value && r.sourceName) {
        map[r.sourceId] = r.sourceName
      }
      if (r.targetId !== instanceId.value && r.targetName) {
        map[r.targetId] = r.targetName
      }
    })
    relNameMap.value = map
  } catch (e: any) {
    ElMessage.error(e?.message || '加载关系失败')
  } finally {
    loadingRelations.value = false
  }
}

// ---- 同步历史 ----
const loadingSync = ref(false)
const syncLogs = ref<SyncLog[]>([])
const syncLogsTotal = ref(0)
const syncPage = ref(1)
const syncPageSize = ref(10)

async function fetchSyncLogs() {
  loadingSync.value = true
  try {
    const res = await listSyncLogs({
      instanceId: instanceId.value,
      page: syncPage.value,
      pageSize: syncPageSize.value,
    })
    syncLogs.value = res.items
    syncLogsTotal.value = res.total
  } catch (e: any) {
    ElMessage.error(e?.message || '加载同步历史失败')
  } finally {
    loadingSync.value = false
  }
}

// ---- 变更记录 ----
const loadingAudit = ref(false)
const auditLogs = ref<AuditLog[]>([])
const auditTotal = ref(0)
const auditPage = ref(1)
const auditPageSize = ref(10)

async function fetchAuditLogs() {
  loadingAudit.value = true
  try {
    const res = await listAuditLogs({
      targetType: 'ci_instance',
      targetId: instanceId.value,
      page: auditPage.value,
      pageSize: auditPageSize.value,
    })
    auditLogs.value = res.items
    auditTotal.value = res.total
  } catch (e: any) {
    ElMessage.error(e?.message || '加载变更记录失败')
  } finally {
    loadingAudit.value = false
  }
}

// ---- Tab 切换懒加载 ----
const loadedTabs = ref<Set<string>>(new Set(['info']))
async function onTabChange(tab: string) {
  if (loadedTabs.value.has(tab)) return
  loadedTabs.value.add(tab)
  if (tab === 'relations') await fetchRelations()
  else if (tab === 'sync') await fetchSyncLogs()
  else if (tab === 'audit') await fetchAuditLogs()
}

// ---- 编辑 ----
const editDialogVisible = ref(false)

function openEdit() {
  if (!instance.value) return
  editDialogVisible.value = true
}

// ---- 添加关系 ----
const relationDialogVisible = ref(false)
const savingRel = ref(false)
const searching = ref(false)
const assetOptions = ref<Array<{ id: string; name: string; modelCode: string }>>([])
const relForm = reactive({
  direction: 'out' as 'out' | 'in',
  peerId: '',
  relationType: 'depends_on',
})

function openAddRelation(preset?: { relationType?: string; direction?: 'out' | 'in' }) {
  relForm.direction = preset?.direction || 'out'
  relForm.peerId = ''
  if (preset?.relationType) {
    relForm.relationType = preset.relationType
  } else if (modelCode.value === 'service') {
    relForm.relationType = 'runs_on'
  } else if (modelCode.value === 'business_system') {
    relForm.relationType = 'contains'
  } else {
    const enabled = relationTypes.value.filter((t) => t.enabled)
    relForm.relationType = enabled.length ? enabled[0].code : 'depends_on'
  }
  relationDialogVisible.value = true
}

function openLinkHost() {
  openAddRelation({ relationType: 'runs_on', direction: 'out' })
  activeTab.value = 'relations'
  loadedTabs.value.add('relations')
  void fetchRelations()
}

async function searchAssets(query: string) {
  if (!query) {
    assetOptions.value = []
    return
  }
  searching.value = true
  try {
    const res = await listCiInstances({ keyword: query, page: 1, pageSize: 20 })
    assetOptions.value = res.items
      .filter(i => i.id !== instanceId.value)
      .map(i => ({ id: i.id, name: i.name, modelCode: (i as any).modelCode || '' }))
  } catch { /* ignore */ } finally {
    searching.value = false
  }
}

async function onSubmitRelation() {
  if (!relForm.peerId) {
    ElMessage.warning('请选择对端资产')
    return
  }
  savingRel.value = true
  try {
    const sourceId = relForm.direction === 'out' ? instanceId.value : relForm.peerId
    const targetId = relForm.direction === 'out' ? relForm.peerId : instanceId.value
    await createCiRelation({ sourceId, targetId, relationType: relForm.relationType })
    ElMessage.success('关系已添加')
    relationDialogVisible.value = false
    relForm.peerId = ''
    relForm.direction = 'out'
    // 重置关系类型为第一个启用项（兜底 depends_on）
    const enabled = relationTypes.value.filter(t => t.enabled)
    relForm.relationType = enabled.length ? enabled[0].code : 'depends_on'
    await fetchRelations()
  } catch (e: any) {
    ElMessage.error(e?.message || '添加关系失败')
  } finally {
    savingRel.value = false
  }
}

async function onDeleteRelation(r: CiRelation) {
  try {
    await ElMessageBox.confirm('确定删除该关系吗？', '删除确认', { type: 'warning' })
    await deleteCiRelation(r.id)
    ElMessage.success('已删除')
    await fetchRelations()
  } catch (e: any) {
    if (e !== 'cancel' && e?.message) ElMessage.error(e.message)
  }
}

// ---- 辅助 ----
function goBack() {
  router.push('/assets')
}
function goToAsset(id: string) {
  router.push(`/assets/${id}`)
}
function statusLabel(s: string): string {
  const map: Record<string, string> = { running: '运行中', stopped: '已停止', maintenance: '维护中', unknown: '未知' }
  return map[s] ?? s
}
function statusTagType(s: string): string {
  const map: Record<string, string> = { running: 'success', stopped: 'danger', maintenance: 'warning', unknown: 'info' }
  return map[s] ?? 'info'
}
function sourceLabel(s: string): string {
  return labelOf(dicts.syncSourceTypes.value, s)
}
function actionLabel(a: string): string {
  return labelOf(dicts.cmdbActions.value, a)
}
function actionTagType(a: string): string {
  if (a.startsWith('create')) return 'success'
  if (a.startsWith('delete')) return 'danger'
  if (a.startsWith('sync') || a.startsWith('pull')) return 'warning'
  return ''
}
function formatAttr(attr: CiModelAttr, val: any): string {
  if (val === null || val === undefined || val === '') return '—'
  if (attr.valueType === 'boolean') return val ? '是' : '否'
  return String(val)
}
function formatTime(t: string): string {
  if (!t) return '—'
  try { return new Date(t).toLocaleString('zh-CN', { hour12: false }) } catch { return t }
}

onMounted(async () => {
  await dicts.load('cmdb_action', 'sync_source_type')
  void ensureModels()
  fetchInstance()
  fetchRelationTypes()
  fetchRelations()
  fetchCredentialOptions()
})

// 路由参数变化时（点击对端资产跳转）重新加载数据
watch(() => route.params.id, (newId) => {
  if (!newId) return
  // 重置状态
  activeTab.value = 'info'
  loadedTabs.value = new Set(['info'])
  relations.value = []
  relNameMap.value = {}
  syncLogs.value = []
  syncLogsTotal.value = 0
  auditLogs.value = []
  auditTotal.value = 0
  // 重新加载
  fetchInstance()
  fetchRelationTypes()
  fetchRelations()
})
</script>

<style scoped>
.asset-detail { padding: 0; }

.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: #fff;
  padding: 16px 20px;
  border-radius: 8px;
  margin-bottom: 16px;
  box-shadow: 0 1px 4px rgba(0,0,0,0.06);
}
.header-left {
  display: flex;
  align-items: center;
  gap: 4px;
}
.model-icon-bg {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 22px;
  margin-left: 8px;
  margin-right: 12px;
}
.title-block {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.asset-name {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
}
.meta-row {
  font-size: 12px;
  color: #909399;
  display: flex;
  align-items: center;
}
.header-right { display: flex; gap: 8px; }

.info-grid { display: flex; flex-direction: column; gap: 20px; }
.info-section .section-title,
.form-section .section-title {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 12px;
  padding-left: 8px;
  border-left: 3px solid #409eff;
}
.form-section { margin-top: 12px; }

.tab-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.tab-tip { font-size: 12px; color: #909399; }

.text-muted { color: #c0c4cc; }
.mono { font-family: 'Consolas', 'Monaco', monospace; font-size: 12px; }
.pagination-row {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
