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
            <span class="tab-tip">该资产的所有依赖关系（作为源或目标）</span>
            <el-button
              v-if="hasPermission('asset:update')"
              size="small"
              type="primary"
              :icon="Plus"
              @click="relationDialogVisible = true"
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

    <!-- 编辑对话框（复用基础信息编辑） -->
    <el-dialog v-model="editDialogVisible" title="编辑资产" width="640px">
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-form-item label="资产名称" prop="name">
          <el-input v-model="formData.name" />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-select v-model="formData.status" style="width: 100%">
            <el-option label="运行中" value="running" />
            <el-option label="已停止" value="stopped" />
            <el-option label="维护中" value="maintenance" />
            <el-option label="未知" value="unknown" />
          </el-select>
        </el-form-item>
        <el-form-item label="标签">
          <el-input v-model="formData.tags" placeholder="多个标签用逗号分隔" />
        </el-form-item>
        <div v-if="modelAttrs.length" class="form-section">
          <div class="section-title">扩展属性</div>
          <el-form-item
            v-for="attr in modelAttrs"
            :key="attr.code"
            :label="attr.name"
            :prop="'attributes.' + attr.code"
            :rules="attr.isRequired ? [{ required: true, message: `${attr.name}不能为空`, trigger: 'blur' }] : []"
          >
            <el-select v-if="attr.valueType === 'enum'" v-model="formData.attributes[attr.code]" clearable style="width: 100%">
              <el-option v-for="opt in attr.options || []" :key="opt" :label="opt" :value="opt" />
            </el-select>
            <el-input-number v-else-if="attr.valueType === 'number'" v-model="formData.attributes[attr.code]" :controls="false" style="width: 100%" />
            <el-switch v-else-if="attr.valueType === 'boolean'" v-model="formData.attributes[attr.code]" />
            <el-input v-else v-model="formData.attributes[attr.code]" />
          </el-form-item>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="editDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onSubmitEdit">保存</el-button>
      </template>
    </el-dialog>

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
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { ArrowLeft, Edit, Plus, Refresh, Monitor, Cpu, Coin, Connection, Grid } from '@element-plus/icons-vue'
import {
  getCiInstance,
  getCiModel,
  updateCiInstance,
  listCiRelations,
  createCiRelation,
  deleteCiRelation,
  listCiInstances,
  listSyncLogs,
  listCiRelationTypes,
} from '../../api/cmdb'
import { listAuditLogs } from '../../api/audit'
import type { CiInstance, CiModelAttr, CiRelation, CiRelationType, SyncLog, AuditLog } from '../../api/types'
import { useUserStore } from '../../stores/user'

const route = useRoute()
const router = useRouter()
const userStore = useUserStore()
const hasPermission = (code: string) => userStore.hasPermission(code)

const instanceId = computed(() => route.params.id as string)
const loading = ref(true)
const instance = ref<CiInstance | null>(null)
const modelAttrs = ref<CiModelAttr[]>([])
const modelName = ref('')

const activeTab = ref('info')

// 模型图标
const iconMap: Record<string, any> = { Monitor, Cpu, Coin, Connection, Grid }
const iconComp = computed(() => {
  const code = modelAttrs.value.length ? '' : ''
  return iconMap['Monitor']
})
const iconStyle = computed(() => {
  // 通过模型 code 匹配颜色（instance 没有 modelCode，需从属性推断；这里用 modelAttrs 的 modelId 无法直接拿 code，简化用默认色）
  return { background: 'linear-gradient(135deg, #4facfe, #00f2fe)' }
})

// ---- 基本信息 ----
const extraAttrs = computed(() => {
  if (!instance.value?.attributes) return []
  const defined = new Set(modelAttrs.value.map(a => a.code))
  return Object.keys(instance.value.attributes).filter(k => !defined.has(k))
})

async function fetchInstance() {
  loading.value = true
  try {
    instance.value = await getCiInstance(instanceId.value)
    if (instance.value) {
      // 加载模型属性定义
      try {
        const detail = await getCiModel(instance.value.modelId)
        modelAttrs.value = detail.attributes
        modelName.value = detail.model.name
      } catch { /* ignore */ }
    }
  } catch (e: any) {
    ElMessage.error(e?.message || '加载资产失败')
  } finally {
    loading.value = false
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
const saving = ref(false)
const formRef = ref<FormInstance>()
const formData = reactive({
  name: '',
  status: 'running',
  tags: '',
  attributes: {} as Record<string, any>,
})
const formRules: FormRules = {
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  status: [{ required: true, message: '请选择状态', trigger: 'change' }],
}

function openEdit() {
  if (!instance.value) return
  formData.name = instance.value.name
  formData.status = instance.value.status
  formData.tags = instance.value.tags
  formData.attributes = { ...(instance.value.attributes || {}) }
  editDialogVisible.value = true
}

async function onSubmitEdit() {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    saving.value = true
    try {
      await updateCiInstance(instanceId.value, {
        name: formData.name.trim(),
        status: formData.status,
        tags: formData.tags,
        attributes: formData.attributes,
      })
      ElMessage.success('更新成功')
      editDialogVisible.value = false
      await fetchInstance()
    } catch (e: any) {
      ElMessage.error(e?.message || '更新失败')
    } finally {
      saving.value = false
    }
  })
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
  const map: Record<string, string> = { blueking: '蓝鲸', external_cmdb: '外部CMDB' }
  return map[s] ?? s
}
function actionLabel(a: string): string {
  const map: Record<string, string> = {
    create_ci: '创建', update_ci: '更新', delete_ci: '删除',
    sync_ci: '同步', pull_ci: '拉取',
    create_ci_relation: '建立关系', delete_ci_relation: '删除关系',
  }
  return map[a] || a
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

onMounted(() => {
  fetchInstance()
  fetchRelationTypes()
  fetchRelations()
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
