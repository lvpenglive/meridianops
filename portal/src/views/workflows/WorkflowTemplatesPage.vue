<template>
  <div class="wf-page">
    <!-- 顶部工具栏 -->
    <div class="wf-topbar">
      <div class="wf-title">
        <el-icon size="18"><Share /></el-icon>
        <span class="wf-title-txt">流程模板管理（LogicFlow 拖拽编排）</span>
      </div>
      <div class="wf-actions">
        <el-input v-model="keyword" placeholder="搜索模板名/代码" clearable style="width:260px">
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-button v-if="hasPermission('workflow:admin')" type="primary" :icon="Plus" @click="onNewTemplate">新建模板</el-button>
        <el-button v-if="current" :disabled="!dirty" type="success" :icon="Check" @click="onSaveCurrent" :loading="saving">保存变更</el-button>
        <el-button v-if="current" :icon="Download" @click="onExportJSON">导出 JSON</el-button>
        <el-button :icon="Refresh" @click="loadTemplates">刷新</el-button>
      </div>
    </div>

    <!-- 主区三栏 -->
    <div class="wf-main">
      <!-- 左：模板列表 -->
      <div class="wf-sidebar-left">
        <div class="wf-sidebar-title">
          <span>模板列表</span>
          <el-tag size="small" type="info">{{ filtered.length }}/{{ templates.length }}</el-tag>
        </div>
        <div class="wf-template-list" v-loading="loading">
          <div v-for="t in filtered" :key="t.id"
               class="wf-template-card"
               :class="{ active: current?.id === t.id }"
               @click="openTemplate(t)">
            <div class="wf-card-head">
              <span class="wf-card-name" :title="displayOf(t)">{{ displayOf(t) }}</span>
              <el-switch v-if="hasPermission('workflow:admin')"
                :model-value="t.enabled"
                size="small"
                :disabled="t.scope === 'builtin'"
                @change="(v: boolean | string | number) => toggleEnabled(t, Boolean(v))"
                @click.stop
              />
            </div>
            <div class="wf-card-meta">
              <el-tag size="small" :type="t.scope === 'builtin' ? 'primary' : 'success'" effect="plain">
                {{ t.scope === 'builtin' ? '内置' : '自定义' }}
              </el-tag>
              <el-tag size="small" effect="plain">{{ t.ticketType }}</el-tag>
              <span>v{{ t.version }}</span>
              <span class="wf-mono">{{ t.name }}</span>
            </div>
            <div class="wf-card-desc" v-if="t.description">{{ t.description }}</div>
            <div class="wf-card-foot">
              <span class="wf-uid">{{ t.id.slice(0, 8) }}</span>
              <div class="wf-card-actions" v-if="hasPermission('workflow:admin')">
                <el-button link size="small" :icon="CopyDocument" @click.stop="cloneTemplate(t)">复制</el-button>
                <el-popconfirm title="确定删除此模板？" v-if="t.scope !== 'builtin'"
                  @confirm="deleteTemplateAction(t)" confirm-button-text="删除" cancel-button-text="取消">
                  <template #reference>
                    <el-button link size="small" type="danger" :icon="Delete" @click.stop>删除</el-button>
                  </template>
                </el-popconfirm>
              </div>
            </div>
          </div>
          <el-empty v-if="!loading && !filtered.length" description="无匹配模板" />
        </div>
      </div>

      <!-- 中：画布 + 顶部节点工具条 -->
      <div class="wf-canvas-wrap">
        <div class="wf-canvas-toolbar" v-if="current">
          <el-tooltip placement="bottom" content="开始节点（拖入画布）">
            <div class="wf-palette-node start" draggable="true" @dragstart="onDragStart($event, 'start')">
              <el-icon><VideoPlay /></el-icon><span>开始</span>
            </div>
          </el-tooltip>
          <el-tooltip placement="bottom" content="结束节点">
            <div class="wf-palette-node end" draggable="true" @dragstart="onDragStart($event, 'end')">
              <el-icon><CircleClose /></el-icon><span>结束</span>
            </div>
          </el-tooltip>
          <el-divider direction="vertical" />
          <el-tooltip placement="bottom" content="单人审批节点（指定审批人/角色）">
            <div class="wf-palette-node" draggable="true" @dragstart="onDragStart($event, 'single_approval')">
              <el-icon><User /></el-icon><span>单人审批</span>
            </div>
          </el-tooltip>
          <el-tooltip placement="bottom" content="会签审批节点（多人全过 all_approval）">
            <div class="wf-palette-node" draggable="true" @dragstart="onDragStart($event, 'all_approval')">
              <el-icon><Avatar /></el-icon><span>会签审批</span>
            </div>
          </el-tooltip>
          <el-tooltip placement="bottom" content="或签审批节点（多人任一通过 any_approval）">
            <div class="wf-palette-node" draggable="true" @dragstart="onDragStart($event, 'any_approval')">
              <el-icon><Connection /></el-icon><span>或签审批</span>
            </div>
          </el-tooltip>
          <el-tooltip placement="bottom" content="加签节点（多人加签依次过 countersign）">
            <div class="wf-palette-node" draggable="true" @dragstart="onDragStart($event, 'countersign')">
              <el-icon><Avatar /></el-icon><span>加签审批</span>
            </div>
          </el-tooltip>
          <el-tooltip placement="bottom" content="条件网关（按 field/op/value 走分支 condition_gateway）">
            <div class="wf-palette-node diamond" draggable="true" @dragstart="onDragStart($event, 'condition_gateway')">
              <el-icon><Promotion /></el-icon><span>条件网关</span>
            </div>
          </el-tooltip>
          <el-tooltip placement="bottom" content="自动节点（自动通过，比如通知）">
            <div class="wf-palette-node auto" draggable="true" @dragstart="onDragStart($event, 'auto_pass')">
              <el-icon><Lightning /></el-icon><span>自动通过</span>
            </div>
          </el-tooltip>
          <el-tooltip placement="bottom" content="并行分支（拆分多路并行处理 parallel_split）">
            <div class="wf-palette-node diamond" draggable="true" @dragstart="onDragStart($event, 'parallel_split')">
              <el-icon><Connection /></el-icon><span>并行分支</span>
            </div>
          </el-tooltip>
          <el-tooltip placement="bottom" content="并行汇聚（等待多路合并 parallel_join）">
            <div class="wf-palette-node diamond" draggable="true" @dragstart="onDragStart($event, 'parallel_join')">
              <el-icon><Connection /></el-icon><span>并行汇聚</span>
            </div>
          </el-tooltip>
          <el-divider direction="vertical" />
          <el-button size="small" :icon="RefreshLeft" @click="onUndo">撤销</el-button>
          <el-button size="small" :icon="RefreshRight" @click="onRedo">重做</el-button>
          <el-divider direction="vertical" />
          <el-button size="small" :icon="Delete" type="danger" plain :disabled="!selectedNode && !selectedEdge" @click="onDeleteSelected">删除选中</el-button>
          <el-divider direction="vertical" />
          <el-button size="small" :icon="ZoomIn" @click="onZoomIn">放大</el-button>
          <el-button size="small" :icon="ZoomOut" @click="onZoomOut">缩小</el-button>
          <el-button size="small" :icon="Aim" @click="onFit">适应画布</el-button>
          <el-button size="small" :icon="FullScreen" @click="onToggleJSON">{{ showJSON ? '隐藏 JSON' : '查看 JSON' }}</el-button>
        </div>

        <div class="wf-canvas-holder" @dragover.prevent @drop="onDrop">
          <div v-if="!current" class="wf-canvas-empty">
            <el-empty description="选择左侧模板进行编辑，或点击新建模板">
              <el-button type="primary" :icon="Plus" :disabled="!hasPermission('workflow:admin')" @click="onNewTemplate">新建模板</el-button>
            </el-empty>
          </div>
          <div ref="lfRef" v-else class="lf-canvas"></div>
        </div>

        <div v-if="showJSON && current" class="wf-json-preview">
          <div class="wf-json-head">
            <b>LogicFlow definition.json 预览（保存时提交到后端）</b>
            <el-button size="small" link @click="copyJSON">复制</el-button>
          </div>
          <pre class="wf-json-body">{{ JSON.stringify(definition, null, 2) }}</pre>
        </div>
      </div>

      <!-- 右：属性面板 -->
      <div class="wf-sidebar-right">
        <div class="wf-sidebar-title">属性面板</div>
        <div class="wf-property">
          <div v-if="!current" class="wf-mono">请选择模板</div>
          <template v-else>
            <el-form label-width="92px" label-position="right" size="default">
              <el-divider>模板基础信息</el-divider>
              <el-form-item label="模板名称">
                <el-input v-model="current.displayName" :disabled="!hasPermission('workflow:admin') || current.scope === 'builtin'" @change="touch" />
              </el-form-item>
              <el-form-item label="模板代码">
                <el-input v-model="current.name" :disabled="!hasPermission('workflow:admin') || current.scope === 'builtin'" @change="touch" />
              </el-form-item>
              <el-form-item label="适用类型">
                <el-select v-model="current.ticketType" :disabled="!hasPermission('workflow:admin') || current.scope === 'builtin'" style="width:100%" @change="touch">
                  <el-option v-for="t in ticketTypeOptions" :key="t.value" :label="t.label" :value="t.value" />
                </el-select>
              </el-form-item>
              <el-form-item label="描述">
                <el-input v-model="current.description" type="textarea" :rows="2"
                  :disabled="!hasPermission('workflow:admin') || current.scope === 'builtin'" @change="touch" />
              </el-form-item>
              <el-form-item label="默认 SLA">
                <span style="color:#909399; margin-right:8px">按优先级继承，可在工单创建时覆盖</span>
              </el-form-item>

              <template v-if="selectedNode">
                <el-divider>节点属性：{{ nodeKindLabel(selectedNode.type) }}</el-divider>
                <el-form-item label="节点 ID">
                  <span class="wf-mono">{{ selectedNode.id }}</span>
                </el-form-item>
                <el-form-item label="节点名称">
                  <el-input v-model="nodeName" @change="applyNodeName" />
                </el-form-item>

                <template v-if="isApproval(selectedNode.type)">
                  <el-form-item label="审批人选择器">
                    <el-select v-model="nodeApprover" filterable allow-create placeholder="选择或输入自定义值（如 user:x、role:xxx）" style="width:100%" @change="applyApprover">
                      <el-option
                        v-for="item in approverOptions"
                        :key="item.value"
                        :label="item.label"
                        :value="item.value"
                      />
                    </el-select>
                  </el-form-item>
                </template>

                <template v-if="selectedNode.type === 'auto_pass'">
                  <el-form-item label="动作描述">
                    <el-input v-model="nodeDesc" placeholder="例：发送邮件通知处理人" @change="applyDesc" />
                  </el-form-item>
                </template>

                <template v-if="selectedNode.type === 'condition_gateway'">
                  <el-form-item label="网关说明">
                    <el-input v-model="nodeDesc" placeholder="例：按优先级分支 / 按紧急度路由" @change="applyDesc" />
                  </el-form-item>
                </template>

                <template v-if="selectedNode.type === 'parallel_split' || selectedNode.type === 'parallel_join'">
                  <el-form-item label="说明">
                    <el-input v-model="nodeDesc" placeholder="并行分支/汇聚说明" @change="applyDesc" />
                  </el-form-item>
                </template>

                <template v-if="selectedNode.type === 'start' || selectedNode.type === 'end'">
                  <el-form-item label="说明">
                    <el-input v-model="nodeDesc" placeholder="开始/结束节点说明（可选）" @change="applyDesc" />
                  </el-form-item>
                </template>

                <el-form-item>
                  <el-button type="danger" :icon="Delete" size="default" style="width:100%" @click="onDeleteNode">
                    删除节点
                  </el-button>
                </el-form-item>
              </template>

              <template v-if="selectedEdge">
                <el-divider>连线（条件分支）</el-divider>
                <el-form-item label="线 ID"><span class="wf-mono">{{ selectedEdge.id }}</span></el-form-item>
                <el-form-item label="标签">
                  <el-input v-model="edgeLabel" @change="applyEdgeLabel" placeholder="例：同意 / 拒绝 / 紧急" />
                </el-form-item>
                <el-form-item label="条件字段">
                  <el-input v-model="edgeCondField" @change="applyEdgeCondition"
                    placeholder="留空表示默认分支；例：priority / category / ticketType" />
                </el-form-item>
                <el-form-item label="比较符">
                  <el-select v-model="edgeCondOp" @change="applyEdgeCondition" style="width:100%" clearable placeholder="选择比较符">
                    <el-option v-for="op in EDGE_OP_OPTIONS" :key="op" :label="op" :value="op" />
                  </el-select>
                </el-form-item>
                <el-form-item label="比较值">
                  <el-input v-model="edgeCondValue" @change="applyEdgeCondition"
                    placeholder="数字直接填(1)、字符串带引号('high')、数组用 [1,2,3]；配合 in 操作符" />
                </el-form-item>
                <div style="line-height:1.6; color:#909399; font-size:12px; padding-left:92px; margin-top:-8px">
                  <div>· 字段+比较符+值三填齐才生效；任一为空视为默认分支</div>
                  <div>· 常用：<code>priority &gt;= 3</code> / <code>priority &lt;= 2</code> / <code>category in ['db','net']</code></div>
                  <div>· 字段取自工单上下文：priority(1-4) / category / ticketType / assigneeId 等</div>
                </div>
                <el-form-item>
                  <el-button type="danger" :icon="Delete" size="default" style="width:100%" @click="onDeleteEdge">
                    删除连线
                  </el-button>
                </el-form-item>
              </template>
            </el-form>
          </template>
        </div>
      </div>
    </div>
  </div>

  <!-- 新建模板对话框 -->
  <el-dialog v-model="newDialogVisible" title="新建流程模板" width="540px" :close-on-click-modal="false">
    <el-form :model="newForm" label-width="92px" ref="newFormRef" :rules="newFormRules">
      <el-form-item label="模板名称" prop="displayName">
        <el-input v-model="newForm.displayName" placeholder="例：数据库紧急变更审批流程" />
      </el-form-item>
      <el-form-item label="模板代码" prop="name">
        <el-input v-model="newForm.name" placeholder="英文/下划线/数字，例：change_emergency_custom" />
      </el-form-item>
      <el-form-item label="工单类型" prop="ticketType">
        <el-select v-model="newForm.ticketType" style="width:100%">
          <el-option v-for="t in ticketTypeOptions" :key="t.value" :label="t.label" :value="t.value" />
        </el-select>
      </el-form-item>
      <el-form-item label="分类">
        <el-select v-model="newForm.category" allow-create filterable style="width:100%" clearable placeholder="例：数据库 / 网络（选填）">
          <el-option v-for="c in ['数据库','网络','安全','主机','应用','存储','中间件','办公网','监控']" :key="c" :label="c" :value="c" />
        </el-select>
      </el-form-item>
      <el-form-item label="描述">
        <el-input v-model="newForm.description" type="textarea" :rows="2" maxlength="300" show-word-limit placeholder="模板说明（选填）" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="newDialogVisible = false">取消</el-button>
      <el-button type="primary" :loading="newDialogSaving" @click="submitNewTemplate">创建</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
// @logicflow/core 1.x 的 dist/logic-flow.js 是 webpack UMD 输出，
// 没有真正的 ESM default 导出。Vite 在 ESM 下把整个 exports 对象当作 default，
// 导致 `new LogicFlow()` 报 "not a constructor"。这里用 namespace 导入 + 多重 fallback 兜底。
import * as LF from '@logicflow/core'
// @logicflow/core 1.x 是 webpack UMD 输出,Vite 在 ESM 下处理时:
//   - LF.default 是整个 exports 对象(object,不是构造函数)
//   - LF.LogicFlow 才是真正的构造函数(命名导出)
// 因此必须优先取 LF.LogicFlow,不能用 ?? 链(default 是 truthy object 会短路)
const LogicFlow: any = (LF as any).LogicFlow
  ?? (typeof (LF as any).default === 'function' ? (LF as any).default : undefined)
  ?? LF
// 节点 view/model 类直接从命名空间取(1.2.x 没有 defaultCtor API)
// 矩形(开始/结束/审批/auto_pass)用 RectNode + RectNodeModel；
// 菱形(条件网关/并行)用 DiamondNode + DiamondNodeModel（rx/ry 控制大小，比 PolygonNode 手动设 points 更简洁）。
const { RectNode, RectNodeModel, DiamondNode, DiamondNodeModel } = LF as any
import '@logicflow/core/dist/style/index.css'
import {
  Share, Plus, Check, Download, Refresh, Search, VideoPlay, CircleClose, User, Avatar,
  Connection, Promotion, Lightning, RefreshLeft, RefreshRight, ZoomIn, ZoomOut, Aim,
  FullScreen, CopyDocument, Delete,
} from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox, type FormInstance } from 'element-plus'
import { useUserStore } from '../../stores/user'
import {
  listTemplates, createTemplate, updateTemplate, deleteTemplate, getTemplate,
  enableTemplate, disableTemplate,
  type WorkflowTemplateSummary, type WorkflowTemplateDetail,
} from '../../api/template'
import { listDictItems, type DictItem } from '../../api/dict'

// 本页面内部使用 扁平节点/连线 定义（与后端 workflow_engine 保存/读取的格式保持一致）
interface FlatNode {
  id: string; type: string; name: string; x?: number; y?: number;
  description?: string; approverSelector?: string;
  timeoutHours?: number; timeoutAction?: string; rejectBackTo?: string;
}
interface FlatEdge {
  id: string;
  /** 源/目标节点ID，兼容后端 sourceNodeId/targetNodeId 与本地 source/target 两种命名 */
  source?: string; target?: string;
  sourceNodeId?: string; targetNodeId?: string;
  label?: string;
  /** 条件对象 {field, op, value}；留空表示默认分支 */
  condition?: { field?: string; op?: string; value?: any } | null;
  sourceAnchor?: string | number; targetAnchor?: string | number;
}
interface FlatDefinition { nodes: FlatNode[]; edges: FlatEdge[]; }
type WorkflowDefinition = FlatDefinition;


const { hasPermission } = useUserStore()

// 工单类型选项：从字典加载
const ticketTypeOptions = ref<DictItem[]>([])
async function loadTicketTypeOptions() {
  try {
    ticketTypeOptions.value = await listDictItems('workflow_ticket_type')
  } catch {}
}

const loading = ref(false)
const saving  = ref(false)
const keyword = ref('')
const dirty   = ref(false)
const templates = ref<WorkflowTemplateSummary[]>([])
const current   = ref<WorkflowTemplateDetail | null>(null)
const showJSON  = ref(false)

// 新建模板 Dialog state
const newDialogVisible = ref(false)
const newDialogSaving = ref(false)
const newFormRef = ref<FormInstance | null>(null)
const newForm = reactive({
  displayName: '', name: '', ticketType: 'incident',
  category: '' as string | undefined, description: ''
})
const newFormRules = {
  displayName: [{ required: true, message: '请填写模板名称', trigger: 'blur' }],
  name: [
    { required: true, message: '请填写模板代码', trigger: 'blur' },
    { pattern: /^[A-Za-z][A-Za-z0-9_]{2,63}$/, message: '3-64 位英文/数字/下划线，且英文开头', trigger: 'blur' }
  ],
  ticketType: [{ required: true, message: '请选择工单类型', trigger: 'change' }],
} as any

const filtered = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return templates.value
  return templates.value.filter(t =>
    displayOf(t).toLowerCase().includes(kw) ||
    String(t.name || '').toLowerCase().includes(kw) ||
    String(t.ticketType || '').toLowerCase().includes(kw) ||
    String(t.category || '').toLowerCase().includes(kw)
  )
})
function displayOf(t: { name: string; displayName?: string | null }): string {
  return (t.displayName && String(t.displayName).trim()) || t.name
}

// LogicFlow
type LF = InstanceType<typeof LogicFlow>
const lfRef = ref<HTMLDivElement | null>(null)
let lf: LF | null = null

const selectedNode = ref<any>(null)
const selectedEdge = ref<any>(null)
const nodeName = ref('')
const nodeApprover = ref('')
const nodeDesc = ref('')
const edgeLabel = ref('')
// 条件表达式结构化：后端 pick_next_node 期望 {field, op, value} 对象
const edgeCondField = ref('')
const edgeCondOp = ref('')
const edgeCondValue = ref('')
const EDGE_OP_OPTIONS = ['==', '!=', '>', '>=', '<', '<=', 'in', 'contains']

// 审批人选择器：从字典加载
const approverOptions = ref<DictItem[]>([])
async function loadApproverOptions() {
  try {
    approverOptions.value = await listDictItems('workflow_approver_selector')
  } catch {}
}

function isApproval(t: string) {
  return t === 'single_approval' || t === 'all_approval' || t === 'any_approval' || t === 'countersign'
}
function nodeKindLabel(t: string) {
  const map: Record<string, string> = {
    start: '开始', end: '结束', auto_pass: '自动通过',
    single_approval: '单人审批', all_approval: '会签审批',
    any_approval: '或签审批', countersign: '加签审批',
    condition_gateway: '条件网关',
    parallel_split: '并行分支', parallel_join: '并行汇聚',
  }
  return map[t] || t
}

// 节点图标映射（emoji，轻量方案，避免引入 SVG icon 依赖）
const NODE_ICONS: Record<string, string> = {
  start: '🏁', end: '✅', auto_pass: '⏭️',
  single_approval: '👤', all_approval: '👥',
  any_approval: '👤', countersign: '➕',
  condition_gateway: '🔀',
  parallel_split: '⇆', parallel_join: '⇅',
}

// 根据节点类型和属性生成副标题（审批人选择器/状态等）
function nodeSubtitle(n: FlatNode): string {
  const t = n.type
  if (t === 'start' || t === 'end') return ''
  if (t === 'auto_pass') return n.description ? n.description.slice(0, 18) : '自动流转'
  if (t === 'condition_gateway') return n.description ? n.description.slice(0, 18) : '条件分支'
  if (t === 'parallel_split' || t === 'parallel_join') return n.description ? n.description.slice(0, 18) : ''
  // 审批类节点显示审批人选择器
  if (t === 'single_approval' || t === 'all_approval' || t === 'any_approval' || t === 'countersign') {
    const sel = n.approverSelector || ''
    if (!sel) return '未设置审批人'
    // 简化显示：取最后一段
    const short = sel.length > 16 ? sel.slice(0, 16) + '…' : sel
    return short
  }
  return ''
}

async function loadTemplates() {
  try {
    loading.value = true
    templates.value = await listTemplates({})
  } catch (e: any) { ElMessage.error(e?.message || String(e)) }
  finally { loading.value = false }
}

async function openTemplate(t: WorkflowTemplateSummary) {
  try {
    const detail = await getTemplate(t.id)
    current.value = detail
    dirty.value = false
    showJSON.value = false
    selectedNode.value = null; selectedEdge.value = null
    await nextTick()
    renderFlow(normalizeDef(detail.definition))
  } catch (e: any) { ElMessage.error(e?.message || String(e)) }
}
function normalizeDef(def: any): WorkflowDefinition {
  if (!def) return emptyDefinition() as any
  // 兼容 template.ts LfDefinition 的 properties 包装格式 与 后端/后端 nodes.edges 扁平结构
  const hasLfProps = (n: any) => !!n && (typeof n === 'object') && 'properties' in n
  const nodes = Array.isArray(def.nodes)
    ? def.nodes.map((n: any) => {
      const p = hasLfProps(n) ? n.properties : (n || {})
      const id = n.id
      const type = n.type || 'rect'
      return {
        id, type,
        name: p?.name || n.name || n.text || id,
        x: typeof n.x === 'number' ? n.x : 200,
        y: typeof n.y === 'number' ? n.y : 200,
        description: p?.description || n.description || undefined,
        approverSelector: p?.approverSelector
          ? (Array.isArray(p.approverSelector) ? p.approverSelector[0] : p.approverSelector)
          : (n.approverSelector || undefined),
      } as any
    }) : []
  const edges = Array.isArray(def.edges)
    ? def.edges.map((e: any) => {
      const p = (e as any).properties || {}
      const src = e.source ?? e.sourceNodeId
      const tgt = e.target ?? e.targetNodeId
      // condition 保留为 {field, op, value} 对象，不 stringify
      let cond: { field?: string; op?: string; value?: any } | null | undefined = undefined
      const cFromProps = p.condition
      const cFromEdge = e.condition
      const cRaw = (cFromProps !== undefined && cFromProps !== null) ? cFromProps : cFromEdge
      if (cRaw && typeof cRaw === 'object') {
        cond = cRaw as { field?: string; op?: string; value?: any }
      } else if (typeof cRaw === 'string' && cRaw.trim() !== '') {
        // 兼容历史字符串：尝试解析回对象
        try {
          const parsed = JSON.parse(cRaw)
          cond = (parsed && typeof parsed === 'object') ? parsed : { field: cRaw }
        } catch { cond = { field: cRaw } }
      }
      return {
        id: e.id, source: src, target: tgt,
        label: p.label ?? e.label ?? undefined,
        condition: cond,
        sourceAnchor: p.sourceAnchor ?? e.sourceAnchor ?? undefined,
        targetAnchor: p.targetAnchor ?? e.targetAnchor ?? undefined,
      } as any
    }) : []
  if (!nodes.length) return emptyDefinition()
  return { nodes, edges } as unknown as WorkflowDefinition
}

function emptyDefinition(): WorkflowDefinition {
  return {
    nodes: [
      { id: 'n_start', type: 'start', name: '开始', x: 180, y: 120 },
      { id: 'n_end',   type: 'end',   name: '结束', x: 180, y: 420 },
    ],
    edges: [
      { id: 'e_1', source: 'n_start', target: 'n_end' }
    ]
  }
}

function touch() { dirty.value = true }

function renderFlow(def: WorkflowDefinition | undefined) {
  if (!lfRef.value) return
  if (lf) { try { (lf as any).destroy?.(); } catch {} lf = null }

  const lfAny: any = new LogicFlow({
    container: lfRef.value,
    grid: { size: 10, type: 'dot' },
    keyboard: { enabled: true },
    background: { color: 'transparent' },
  })
  lf = lfAny as unknown as LF
  // 暴露到 window 方便调试
  ;(window as any).__lf = lfAny
  // 注册所有可能的事件名用于诊断
  const possibleEvents = ['node:click','node:mousedown','node:mouseup','element:click','element:mousedown','blank:click','selection:selected','node:dblclick','edge:click','edge:mousedown','node:add','node:delete']
  possibleEvents.forEach(ev => {
    try { lfAny.on(ev, (...args: any[]) => { console.log('[LF-EVENT]', ev, args[0] ? JSON.stringify(Object.keys(args[0])) : '') }) } catch {}
  })

  // ===== 全局主题 =====
  lfAny.setTheme?.({
    baseEdge: { stroke: '#c0c4cc', strokeWidth: 1.5 },
    edgeText: { color: '#606266', fontSize: 12, background: { fill: '#ffffff', stroke: 'none', wrapPadding: '2px 6px', rx: 4, ry: 4 } },
    arrow: { offset: 10, verticalLength: 6, fill: '#c0c4cc', stroke: 'none' },
    nodeText: { color: '#1f2d3d', fontSize: 13, overflowMode: 'autoWrap', lineHeight: 1.4 },
  })

  // ===== 样式表 =====
  const STYLE: any = {
    start:   { fill: '#e8f5e3', stroke: '#67c23a', text: '#1a5e0e', w: 120, h: 44, r: 22, bold: true  },
    end:     { fill: '#fde2e2', stroke: '#f56c6c', text: '#7a1818', w: 120, h: 44, r: 22, bold: true  },
    auto:    { fill: '#eef0f3', stroke: '#909399', text: '#3d4045', w: 160, h: 44, r: 22, bold: false },
    single:  { fill: '#ecf5ff', stroke: '#409EFF', text: '#1f3d70', w: 200, h: 60, r: 12, bold: false },
    multi:   { fill: '#e8f5e3', stroke: '#67c23a', text: '#1a5e0e', w: 200, h: 60, r: 12, bold: false },
    cond:    { fill: '#fdf3e6', stroke: '#e6a23c', text: '#73540b', rx: 50, ry: 36, bold: false },
    para:    { fill: '#e3f5f5', stroke: '#13c2c2', text: '#1a6f6f', rx: 44, ry: 32, bold: false },
  }

  // ===== 节点注册 =====
  // 为每种节点类型注册独立的 view+model，通过 getNodeStyle/getTextStyle 返回样式
  // initNodeData 用于设置尺寸（比 constructor 更可靠，此时 data 已就绪）

  try {
    // 胶囊节点 - 使用方式二注册(更可靠)
    lfAny.register('start', ({ RectNode: RN, RectNodeModel: RM }: any) => {
      class StartView extends RN {}
      class StartModel extends RM {
        initNodeData(d: any) { super.initNodeData(d); this.width = STYLE.start.w; this.height = STYLE.start.h; this.radius = STYLE.start.r }
        getNodeStyle() { return { fill: STYLE.start.fill, stroke: STYLE.start.stroke, strokeWidth: 2 } }
        getTextStyle() { return { fontSize: 14, color: STYLE.start.text, fontWeight: 600, lineHeight: 1.4 } }
      }
      return { view: StartView, model: StartModel }
    })
    lfAny.register('end', ({ RectNode: RN, RectNodeModel: RM }: any) => {
      class EndView extends RN {}
      class EndModel extends RM {
        initNodeData(d: any) { super.initNodeData(d); this.width = STYLE.end.w; this.height = STYLE.end.h; this.radius = STYLE.end.r }
        getNodeStyle() { return { fill: STYLE.end.fill, stroke: STYLE.end.stroke, strokeWidth: 2 } }
        getTextStyle() { return { fontSize: 14, color: STYLE.end.text, fontWeight: 600, lineHeight: 1.4 } }
      }
      return { view: EndView, model: EndModel }
    })
    lfAny.register('auto_pass', ({ RectNode: RN, RectNodeModel: RM }: any) => {
      class View extends RN {}
      class Model extends RM {
        initNodeData(d: any) { super.initNodeData(d); this.width = STYLE.auto.w; this.height = STYLE.auto.h; this.radius = STYLE.auto.r }
        getNodeStyle() { return { fill: STYLE.auto.fill, stroke: STYLE.auto.stroke, strokeWidth: 2 } }
        getTextStyle() { return { fontSize: 13, color: STYLE.auto.text, lineHeight: 1.4 } }
      }
      return { view: View, model: Model }
    })
    lfAny.register('single_approval', ({ RectNode: RN, RectNodeModel: RM }: any) => {
      class View extends RN {}
      class Model extends RM {
        initNodeData(d: any) { super.initNodeData(d); this.width = STYLE.single.w; this.height = STYLE.single.h; this.radius = STYLE.single.r }
        getNodeStyle() { return { fill: STYLE.single.fill, stroke: STYLE.single.stroke, strokeWidth: 2 } }
        getTextStyle() { return { fontSize: 13, color: STYLE.single.text, lineHeight: 1.4 } }
      }
      return { view: View, model: Model }
    })
    lfAny.register('all_approval', ({ RectNode: RN, RectNodeModel: RM }: any) => {
      class View extends RN {}
      class Model extends RM {
        initNodeData(d: any) { super.initNodeData(d); this.width = STYLE.multi.w; this.height = STYLE.multi.h; this.radius = STYLE.multi.r }
        getNodeStyle() { return { fill: STYLE.multi.fill, stroke: STYLE.multi.stroke, strokeWidth: 2 } }
        getTextStyle() { return { fontSize: 13, color: STYLE.multi.text, lineHeight: 1.4 } }
      }
      return { view: View, model: Model }
    })
    lfAny.register('any_approval', ({ RectNode: RN, RectNodeModel: RM }: any) => {
      class View extends RN {}
      class Model extends RM {
        initNodeData(d: any) { super.initNodeData(d); this.width = STYLE.multi.w; this.height = STYLE.multi.h; this.radius = STYLE.multi.r }
        getNodeStyle() { return { fill: STYLE.multi.fill, stroke: STYLE.multi.stroke, strokeWidth: 2 } }
        getTextStyle() { return { fontSize: 13, color: STYLE.multi.text, lineHeight: 1.4 } }
      }
      return { view: View, model: Model }
    })
    lfAny.register('countersign', ({ RectNode: RN, RectNodeModel: RM }: any) => {
      class View extends RN {}
      class Model extends RM {
        initNodeData(d: any) { super.initNodeData(d); this.width = STYLE.multi.w; this.height = STYLE.multi.h; this.radius = STYLE.multi.r }
        getNodeStyle() { return { fill: STYLE.multi.fill, stroke: STYLE.multi.stroke, strokeWidth: 2 } }
        getTextStyle() { return { fontSize: 13, color: STYLE.multi.text, lineHeight: 1.4 } }
      }
      return { view: View, model: Model }
    })
    lfAny.register('condition_gateway', ({ DiamondNode: DN, DiamondNodeModel: DM }: any) => {
      class View extends DN {}
      class Model extends DM {
        initNodeData(d: any) { super.initNodeData(d); this.rx = STYLE.cond.rx; this.ry = STYLE.cond.ry }
        getNodeStyle() { return { fill: STYLE.cond.fill, stroke: STYLE.cond.stroke, strokeWidth: 2 } }
        getTextStyle() { return { fontSize: 12, color: STYLE.cond.text, lineHeight: 1.3 } }
      }
      return { view: View, model: Model }
    })
    lfAny.register('parallel_split', ({ DiamondNode: DN, DiamondNodeModel: DM }: any) => {
      class View extends DN {}
      class Model extends DM {
        initNodeData(d: any) { super.initNodeData(d); this.rx = STYLE.para.rx; this.ry = STYLE.para.ry }
        getNodeStyle() { return { fill: STYLE.para.fill, stroke: STYLE.para.stroke, strokeWidth: 2 } }
        getTextStyle() { return { fontSize: 12, color: STYLE.para.text, lineHeight: 1.3 } }
      }
      return { view: View, model: Model }
    })
    lfAny.register('parallel_join', ({ DiamondNode: DN, DiamondNodeModel: DM }: any) => {
      class View extends DN {}
      class Model extends DM {
        initNodeData(d: any) { super.initNodeData(d); this.rx = STYLE.para.rx; this.ry = STYLE.para.ry }
        getNodeStyle() { return { fill: STYLE.para.fill, stroke: STYLE.para.stroke, strokeWidth: 2 } }
        getTextStyle() { return { fontSize: 12, color: STYLE.para.text, lineHeight: 1.3 } }
      }
      return { view: View, model: Model }
    })
    console.log('[LF] custom nodes registered')
  } catch (e) {
    console.error('[LogicFlow] 节点注册失败:', e)
  }

  // ===== 事件绑定 =====
  // LogicFlow 1.x 的事件系统可能因版本差异不触发,同时绑定 DOM 原生事件做兜底

  function extractNodeData(e: any): any | null {
    if (!e) return null
    if (e.data && typeof e.data === 'object' && !e.data.id) return extractNodeData(e.data)
    if (e.data && e.data.id) return e.data
    if (e.model && e.model.id) return { id: e.model.id, type: e.model.type, text: e.model.text, properties: e.model.properties }
    if (e.id) return e
    return null
  }

  function bindCanvasEvents() {
    const container = lfRef.value
    if (!container) {
      console.warn('[WF] bindCanvasEvents: lfRef is null')
      return
    }
    console.log('[WF] bindCanvasEvents called, container:', container.className, 'child count:', container.childElementCount)

    // LogicFlow 可能在冒泡阶段阻止了 click 事件,
    // 因此我们在 capture 阶段用 pointerdown 来捕获
    container.addEventListener('pointerdown', (ev: PointerEvent) => {
      const target = ev.target as Element
      if (!target) return
      console.log('[WF] pointerdown, target:', target.tagName, 'class:', (target.className||'').toString().substring(0,40))
      // 向上查找 data-id (LogicFlow 1.x 用 data-id 属性标识节点/边)
      let el: Element | null = target
      let foundId: string | null = null
      let isNode = false
      let isEdge = false
      let isBlank = true

      while (el && el !== container) {
        const id = el.getAttribute?.('data-id')
        if (id) {
          foundId = id
          isBlank = false
          const cls = el.getAttribute('class') || ''
          const clsStr = typeof cls === 'string' ? cls : (cls?.baseVal?.value || '')
          // LogicFlow 1.x 节点 group 通常有: class="lf-node" 或 data-type 属性
          // 边 group 通常有: class="lf-edge"
          if (clsStr.includes('lf-edge') || clsStr.includes('edge')) {
            isEdge = true
          } else if (clsStr.includes('lf-node') || clsStr.includes('node')) {
            isNode = true
          } else {
            // 通过子元素判断
            const children = el.children
            for (const c of children) {
              const tag = c.tagName?.toLowerCase()
              if (tag === 'path') { isEdge = true; break }
              if (tag === 'rect' || tag === 'polygon' || tag === 'circle' || tag === 'ellipse') { isNode = true; break }
            }
            if (!isNode && !isEdge && el.firstElementChild) {
              const tag = el.firstElementChild.tagName?.toLowerCase()
              if (tag === 'path') isEdge = true
              else if (tag === 'rect' || tag === 'polygon' || tag === 'circle') isNode = true
            }
          }
          break
        }
        el = el.parentElement
      }

      console.log('[WF] found id:', foundId, 'isNode:', isNode, 'isEdge:', isEdge, 'isBlank:', isBlank)

      if (isBlank || (!isNode && !isEdge)) {
        if (isBlank) {
          selectedNode.value = null
          selectedEdge.value = null
        }
        return
      }

      // 不要阻止默认行为,让 LogicFlow 也能处理
      // ev.stopPropagation() // 注释掉,不阻止 LF

      if (foundId && isNode) {
        const model = getNodeModelById(foundId)
        if (model) {
          selectedEdge.value = null
          const props = model.properties || {}
          selectedNode.value = { id: model.id, type: model.type, text: model.text, properties: props }
          const rawText = (model.text && typeof model.text === 'object') ? (model.text.value || '') : (model.text || '')
          const firstLine = rawText.split('\n')[0] || ''
          const pureTitle = firstLine.replace(/^[^\s]+\s+/, '')
          nodeName.value = props.name || pureTitle || nodeKindLabel(model.type)
          nodeApprover.value = props.approverSelector || ''
          nodeDesc.value = props.description || ''
          console.log('[WF] selected node:', model.id, model.type, 'name:', nodeName.value)
        } else {
          console.warn('[WF] node model not found for id:', foundId)
        }
      } else if (foundId && isEdge) {
        const model = getEdgeModelById(foundId)
        if (model) {
          selectedNode.value = null
          const props = model.properties || {}
          selectedEdge.value = { id: model.id, source: model.sourceNodeId, target: model.targetNodeId, properties: props }
          edgeLabel.value = props.label || ''
          const c = props.condition
          edgeCondField.value = (c && typeof c === 'object' && c.field) ? String(c.field) : ''
          edgeCondOp.value   = (c && typeof c === 'object' && c.op)   ? String(c.op)   : ''
          edgeCondValue.value = (c && typeof c === 'object' && c.value !== undefined && c.value !== null)
            ? (typeof c.value === 'string' ? c.value : JSON.stringify(c.value)) : ''
          console.log('[WF] selected edge:', model.id)
        } else {
          console.warn('[WF] edge model not found for id:', foundId)
        }
      }
    }, true) // capture: true - 在捕获阶段处理
  }

  // LogicFlow 事件(备用)
  lfAny.on('node:click', (e: any) => {
    const d = extractNodeData(e)
    if (!d) return
    selectedEdge.value = null
    const props = d.properties || {}
    selectedNode.value = { id: d.id, type: d.type, text: d.text, properties: props }
    const rawText = (d.text && typeof d.text === 'object') ? (d.text.value || '') : (d.text || '')
    const firstLine = rawText.split('\n')[0] || ''
    const pureTitle = firstLine.replace(/^[^\s]+\s+/, '')
    nodeName.value = props.name || pureTitle || nodeKindLabel(d.type)
    nodeApprover.value = props.approverSelector || ''
    nodeDesc.value = props.description || ''
  })

  lfAny.on('edge:click', (e: any) => {
    const d = extractNodeData(e)
    if (!d) return
    selectedNode.value = null
    const props = d.properties || {}
    selectedEdge.value = { id: d.id, source: d.sourceNodeId, target: d.targetNodeId, properties: props }
    edgeLabel.value = props.label || ''
    const c = props.condition
    edgeCondField.value = (c && typeof c === 'object' && c.field) ? String(c.field) : ''
    edgeCondOp.value   = (c && typeof c === 'object' && c.op)   ? String(c.op)   : ''
    edgeCondValue.value = (c && typeof c === 'object' && c.value !== undefined && c.value !== null)
      ? (typeof c.value === 'string' ? c.value : JSON.stringify(c.value)) : ''
  })

  lfAny.on('blank:click', () => { selectedNode.value = null; selectedEdge.value = null })
  // 选中项被删除时清理状态
  lfAny.on('node:delete', () => {
    selectedNode.value = null
    nodeName.value = ''
    nodeApprover.value = ''; nodeDesc.value = ''
    touch()
  })
  lfAny.on('edge:delete', () => {
    selectedEdge.value = null
    edgeLabel.value = ''; edgeCondField.value = ''
    edgeCondOp.value = ''; edgeCondValue.value = ''
    touch()
  })
  ;['node:drop','edge:add','node:add','node:mousemove','edge:adjust','history:change'].forEach(ev => {
    lfAny.on(ev, () => touch())
  })

  // ===== 渲染 =====
  const graphData = mapDefToLF(def || emptyDefinition())
  try {
    lfAny.render(graphData)
    console.log('[LF] rendered, nodes:', graphData.nodes?.length, 'edges:', graphData.edges?.length)
    // 检查 DOM
    const svgCount = lfRef.value?.querySelectorAll('svg').length || 0
    const gCount = lfRef.value?.querySelectorAll('g').length || 0
    const nodeCount = lfRef.value?.querySelectorAll('[data-id]').length || 0
    console.log('[LF] DOM check: svg=', svgCount, 'g=', gCount, 'data-id=', nodeCount)
    // 检查 LogicFlow 内部节点
    try {
      const nodes = lfAny.getNodes?.() || []
      const edges = lfAny.getEdges?.() || []
      console.log('[LF] getNodes:', nodes.length, 'getEdges:', edges.length)
      if (nodes.length > 0) {
        console.log('[LF] first node:', nodes[0].id, nodes[0].type, 'has onclick:', typeof nodes[0].onClick)
      }
    } catch(e) { console.warn('[LF] getNodes error:', e) }
    // 渲染后为每个节点应用类型相关的样式
    applyNodeStyles()
    // 绑定 DOM 事件兜底（LogicFlow 事件可能不触发）
    bindCanvasEvents()
  } catch (e) {
    console.error('[LogicFlow] 渲染失败:', e)
  }
}

// 渲染后为每个节点应用类型相关的样式（通过修改 model 属性实现）
function applyNodeStyles() {
  if (!lf) return
  try {
    const nodes: any[] = (lf as any).getNodes?.() || []
    nodes.forEach((model: any) => {
      const type = model.type
      const cfg = (STYLE as any)[type]
      if (!cfg) return
      // 存储类型信息到 properties，方便后续编辑时使用
      if (!model.properties) model.properties = {}
      model.properties._nodeType = type
      // 强制刷新视图：LogicFlow 1.x 通过 set 属性或直接调用视图更新
      try {
        // 尝试触发视图更新
        model.setStyles?.({
          fill: cfg.fill,
          stroke: cfg.stroke,
          strokeWidth: 2,
        })
        // 直接设置 model 属性（部分版本支持）
        model.fill = cfg.fill
        model.stroke = cfg.stroke
        model.strokeWidth = 2
      } catch {}
    })
  } catch (e) {
    console.warn('[LogicFlow] applyNodeStyles 异常:', e)
  }
}

const definition = computed(() => {
  if (!lf) return emptyDefinition()
  try {
    const g = (lf as any).getGraphData?.() || { nodes: [], edges: [] }
    return mapLFToDef(g)
  } catch { return emptyDefinition() }
})

function mapDefToLF(def: WorkflowDefinition): any {
  return {
    nodes: (def.nodes || []).map((n: any) => {
      const flat: FlatNode = { id: n.id, type: n.type, name: n.name || '', x: n.x, y: n.y, description: n.description, approverSelector: n.approverSelector }
      const icon = NODE_ICONS[n.type] || '•'
      const title = n.name || nodeKindLabel(n.type)
      const subtitle = nodeSubtitle(flat)
      // 组合文本：图标+标题 为第一行，副标题为第二行
      const text = subtitle ? `${icon} ${title}\n${subtitle}` : `${icon} ${title}`
      return {
        id: n.id, type: n.type || 'rect', x: n.x ?? 200, y: n.y ?? 200,
        text,
        properties: {
          name: n.name,
          description: n.description || '',
          approverSelector: n.approverSelector || undefined,
          // 额外存储：用于 getTextStyle 区分标题/副标题样式
          icon,
          subtitle,
        }
      }
    }),
    edges: (def.edges || []).map((e: any) => {
      const cond = e.condition
      // 条件边显示条件摘要（如 "priority ≥ 3"），默认分支不显示
      let edgeText = e.label || ''
      if (cond && cond.field) {
        const opMap: Record<string, string> = { '==':'=','!=':'≠','>':'>','>=':'≥','<':'<','<=':'≤','in':'∈','contains':'contains' }
        const op = opMap[cond.op || '=='] || (cond.op || '')
        const val = Array.isArray(cond.value) ? cond.value.join(',') : String(cond.value ?? '')
        edgeText = `${cond.field} ${op} ${val}`
      }
      // 兼容两种字段命名：sourceNodeId/targetNodeId（新）或 source/target（旧）
      const src = e.sourceNodeId ?? e.source
      const tgt = e.targetNodeId ?? e.target
      return {
        id: e.id,
        type: e.type || 'polyline',
        sourceNodeId: src, targetNodeId: tgt,
        text: edgeText,
        properties: {
          label: e.label || '',
          condition: cond ?? null,
          sourceAnchor: e.sourceAnchor || undefined,
          targetAnchor: e.targetAnchor || undefined,
          isCondition: !!(cond && cond.field),
        }
      }
    }),
  }
}
function mapLFToDef(g: any): WorkflowDefinition {
  return {
    nodes: (g.nodes || []).map((n: any) => {
      const p = n.properties || {}
      // 从格式化文本中提取纯标题（去掉 emoji 图标前缀和副标题行）
      const rawText = (n.text && typeof n.text === 'object') ? (n.text.value || '') : (n.text || '')
      const firstLine = rawText.split('\n')[0] || ''
      const pureTitle = firstLine.replace(/^[^\s]+\s+/, '')
      const name = p.name || pureTitle || n.id
      // 节点 key：若前端未设置则用 id 作为兜底，保证后端校验通过
      const key = p.key || n.id
      return {
        id: n.id,
        type: n.type,
        x: n.x, y: n.y,
        properties: {
          name,
          key,
          description: p.description || undefined,
          approverSelector: p.approverSelector || undefined,
        },
      }
    }),
    edges: (g.edges || []).map((e: any) => {
      const p = e.properties || {}
      return {
        id: e.id,
        sourceNodeId: e.sourceNodeId,
        targetNodeId: e.targetNodeId,
        properties: {
          label: p.label || undefined,
          // 保持 condition 对象形态，不做 stringify
          condition: (p.condition && typeof p.condition === 'object') ? p.condition : undefined,
          sourceAnchor: p.sourceAnchor || undefined,
          targetAnchor: p.targetAnchor || undefined,
        },
      }
    })
  }
}

function onDragStart(ev: DragEvent, type: string) {
  ev.dataTransfer?.setData('application/x-node-type', type)
}
function onDrop(ev: DragEvent) {
  const type = ev.dataTransfer?.getData('application/x-node-type')
  if (!type || !lfRef.value || !lf) return
  const rect = lfRef.value.getBoundingClientRect()
  const x = ev.clientX - rect.left
  const y = ev.clientY - rect.top
  const icon = NODE_ICONS[type] || '•'
  const title = nodeKindLabel(type)
  const text = `${icon} ${title}`
  // LogicFlow 1.x addNode 返回节点模型
  try {
    const model = (lf as any).addNode?.({ type, x, y, text })
    if (model) {
      // 设置 properties
      model.properties = model.properties || {}
      model.properties.name = title
      model.properties.icon = icon
      model.properties._nodeType = type
      // 添加副标题
      const subtitle = (type === 'start' || type === 'end') ? ''
        : (type === 'auto_pass' ? '自动流转'
        : (isApproval(type) ? '未设置审批人' : ''))
      if (subtitle) {
        model.text = { value: `${icon} ${title}\n${subtitle}`, x, y }
      }
      touch()
    }
  } catch (e) {
    console.error('[LogicFlow] addNode 失败:', e)
  }
}
function onUndo() { (lf as any)?.undo?.() }
function onRedo() { (lf as any)?.redo?.() }
function onZoomIn() { (lf as any)?.zoom?.(true) }
function onZoomOut() { (lf as any)?.zoom?.(false) }
function onFit()   { (lf as any)?.fitView?.(16) }
function onToggleJSON() { showJSON.value = !showJSON.value }

// ===== LogicFlow 编辑辅助函数 =====
// LogicFlow 1.x 中：修改节点需要通过 model 属性直接操作
// lf.getNodes() 返回节点模型数组，每个模型有 id, type, properties, text, x, y 等属性

function getNodeModelById(id: string): any | null {
  if (!lf) return null
  try {
    const nodes: any[] = (lf as any).getNodes?.() || []
    return nodes.find(n => n.id === id) || null
  } catch { return null }
}

function getEdgeModelById(id: string): any | null {
  if (!lf) return null
  try {
    const edges: any[] = (lf as any).getEdges?.() || []
    return edges.find(e => e.id === id) || null
  } catch { return null }
}

// 重建节点显示文本（图标+标题+副标题）
// 注意：LogicFlow 1.x 的正确 API 是 model.updateText(value)，不是 setViewText
// 使用 lf.getModelById 重新获取 model，确保读到最新的 properties
function rebuildNodeText(id: string) {
  if (!lf) return
  let model: any = null
  try {
    // 优先用 getModelById，确保获取到最新 properties
    model = (lf as any).getModelById?.(id) || getNodeModelById(id)
  } catch {
    model = getNodeModelById(id)
  }
  if (!model) return
  const p = model.properties || {}
  const type = model.type
  const icon = p.icon || NODE_ICONS[type] || '•'
  const title = p.name || nodeKindLabel(type)
  // 生成副标题
  let sub = ''
  if (type === 'auto_pass') sub = p.description ? String(p.description).slice(0, 18) : '自动流转'
  else if (type === 'condition_gateway') sub = p.description ? String(p.description).slice(0, 18) : '条件分支'
  else if (type === 'parallel_split' || type === 'parallel_join') sub = p.description ? String(p.description).slice(0, 18) : ''
  else if (isApproval(type)) {
    const sel = p.approverSelector || ''
    sub = sel ? (sel.length > 16 ? sel.slice(0, 16) + '…' : sel) : '未设置审批人'
  }
  const newText = sub ? `${icon} ${title}\n${sub}` : `${icon} ${title}`
  // 用 LogicFlow 1.x 官方 API 更新文本
  try { model.updateText?.(newText) } catch (e) { console.warn('[WF] node updateText failed:', e) }
}

function applyNodeName() {
  if (!selectedNode.value?.id || !lf) return
  const id = selectedNode.value.id
  const model = getNodeModelById(id)
  // 先合并现有属性再设置，避免 setProperties 覆盖其他字段
  const merged = { ...(model?.properties || {}), name: nodeName.value }
  if (model) model.properties = merged
  try { (lf as any).setProperties?.(id, merged) } catch {}
  rebuildNodeText(id)
  touch()
}

function applyApprover() {
  if (!selectedNode.value?.id || !lf) return
  const id = selectedNode.value.id
  const model = getNodeModelById(id)
  const merged = { ...(model?.properties || {}), approverSelector: nodeApprover.value || undefined }
  if (model) model.properties = merged
  try { (lf as any).setProperties?.(id, merged) } catch {}
  rebuildNodeText(id)
  touch()
}

function applyDesc() {
  if (!selectedNode.value?.id || !lf) return
  const id = selectedNode.value.id
  const model = getNodeModelById(id)
  const merged = { ...(model?.properties || {}), description: nodeDesc.value || undefined }
  if (model) model.properties = merged
  try { (lf as any).setProperties?.(id, merged) } catch {}
  rebuildNodeText(id)
  touch()
}
function applyEdgeLabel() {
  if (!selectedEdge.value?.id || !lf) return
  const id = selectedEdge.value.id
  const model = getEdgeModelById(id)
  if (model) model.properties = { ...(model.properties || {}), label: edgeLabel.value }
  try { (lf as any).setProperties?.(id, { label: edgeLabel.value }) } catch {}
  // 用 LogicFlow 1.x 官方 API 更新边文本
  try { (lf as any).updateText?.(id, edgeLabel.value || '') } catch (e) { console.warn('[WF] edge updateText failed:', e) }
  touch()
}
function applyEdgeCondition() {
  if (!selectedEdge.value?.id || !lf) return
  const id = selectedEdge.value.id
  const f = edgeCondField.value.trim()
  const op = edgeCondOp.value.trim()
  const v = edgeCondValue.value.trim()
  let cond: { field?: string; op?: string; value?: any } | null = null
  if (f && op) {
    let parsed: any = v
    if (v !== '') {
      try { parsed = JSON.parse(v) } catch { parsed = v }
    }
    cond = { field: f, op, value: parsed }
  }
  // 双保险：直接写 model + lf.setProperties
  const model = getEdgeModelById(id)
  if (model) model.properties = { ...(model.properties || {}), condition: cond ?? null }
  try { (lf as any).setProperties?.(id, { condition: cond ?? null }) } catch {}
  // 更新边文本：条件边显示条件摘要
  let newText = ''
  if (cond && cond.field) {
    const opMap: Record<string, string> = { '==':'=','!=':'≠','>':'>','>=':'≥','<':'<','<=':'≤','in':'∈','contains':'contains' }
    const opSym = opMap[cond.op || '=='] || (cond.op || '')
    const val = Array.isArray(cond.value) ? cond.value.join(',') : String(cond.value ?? '')
    newText = `${cond.field} ${opSym} ${val}`
  }
  // 用 LogicFlow 1.x 官方 API 更新边文本
  try { (lf as any).updateText?.(id, newText) } catch (e) { console.warn('[WF] edge condition updateText failed:', e) }
  touch()
}

// ===== 删除操作 =====
function onDeleteSelected() {
  if (selectedNode.value) onDeleteNode()
  else if (selectedEdge.value) onDeleteEdge()
}

async function onDeleteNode() {
  if (!selectedNode.value?.id || !lf) return
  const nodeId = selectedNode.value.id
  const nodeType = selectedNode.value.type
  // 开始/结束节点不允许删除
  if (nodeType === 'start' || nodeType === 'end') {
    ElMessage.warning('开始和结束节点不可删除')
    return
  }
  try {
    await ElMessageBox.confirm(
      `确认删除节点「${nodeName.value || nodeId}」？`,
      '删除确认',
      { confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning' }
    )
  } catch { return }

  try {
    // LogicFlow 1.x: 先删除关联边，再删节点
    const edges: any[] = (lf as any).getEdges?.() || []
    const related = edges.filter(e => e.sourceNodeId === nodeId || e.targetNodeId === nodeId)
    for (const e of related) {
      try { (lf as any).deleteNode?.(e.id) || (lf as any).deleteEdge?.(e.id) } catch {}
    }
    // 删除节点
    ;(lf as any).deleteNode?.(nodeId)
    selectedNode.value = null
    nodeName.value = ''
    nodeApprover.value = ''
    nodeDesc.value = ''
    touch()
    ElMessage.success('节点已删除')
  } catch (e: any) {
    ElMessage.error('删除失败：' + (e?.message || String(e)))
  }
}

async function onDeleteEdge() {
  if (!selectedEdge.value?.id || !lf) return
  const edgeId = selectedEdge.value.id
  try {
    await ElMessageBox.confirm('确认删除此连线？', '删除确认', {
      confirmButtonText: '删除', cancelButtonText: '取消', type: 'warning'
    })
  } catch { return }

  try {
    ;(lf as any).deleteNode?.(edgeId) || (lf as any).deleteEdge?.(edgeId)
    selectedEdge.value = null
    edgeLabel.value = ''
    edgeCondField.value = ''
    edgeCondOp.value = ''
    edgeCondValue.value = ''
    touch()
    ElMessage.success('连线已删除')
  } catch (e: any) {
    ElMessage.error('删除失败：' + (e?.message || String(e)))
  }
}

function copyJSON() {
  const s = JSON.stringify(definition.value, null, 2)
  navigator.clipboard.writeText(s).then(() => ElMessage.success('已复制 JSON'), () => ElMessage.warning('复制失败'))
}
function onExportJSON() { copyJSON() }

function onNewTemplate() {
  newForm.displayName = ''
  newForm.name = ''
  newForm.ticketType = 'incident'
  newForm.category = ''
  newForm.description = ''
  newFormRef.value?.resetFields?.()
  newDialogVisible.value = true
}
async function submitNewTemplate() {
  try {
    await newFormRef.value?.validate?.()
  } catch { return }
  if (!newForm.displayName.trim() || !newForm.name.trim()) return
  const def = emptyDefinition()
  try {
    newDialogSaving.value = true
    const created = await createTemplate({
      name: newForm.name.trim(),
      displayName: newForm.displayName.trim(),
      ticketType: newForm.ticketType,
      category: newForm.category || undefined,
      description: newForm.description || undefined,
      definition: def as any,
      enabled: true,
    })
    ElMessage.success('模板已创建，进入编辑')
    newDialogVisible.value = false
    await loadTemplates()
    const d = await getTemplate(created.id)
    current.value = d
    dirty.value = false
    await nextTick()
    renderFlow(normalizeDef(d.definition))
  } catch (e: any) { ElMessage.error(e?.message || String(e)) }
  finally { newDialogSaving.value = false }
}

async function onSaveCurrent() {
  if (!current.value) return
  const canEdit = hasPermission('workflow:admin') && current.value.scope !== 'builtin'
  if (!canEdit) { ElMessage.warning('内置模板不可修改，可点击「复制」后编辑自定义版本'); return }
  if (!current.value.name.trim()) { ElMessage.warning('请填写模板代码'); return }
  const def = definition.value as any
  // 基本校验：必须 1 个开始 1 个结束
  const starts = (def.nodes || []).filter((n: any) => n.type === 'start').length
  const ends   = (def.nodes || []).filter((n: any) => n.type === 'end').length
  if (starts !== 1 || ends !== 1) {
    ElMessage.warning(`流程必须包含且仅包含 1 个开始节点和 1 个结束节点（当前：start=${starts} end=${ends}）`)
    return
  }
  // 每个审批节点需指定 approverSelector（允许 assignee 等动态选择）
  const bad = (def.nodes || []).filter((n: any) => {
    if (!isApproval(n.type)) return false
    // 兼容新旧格式：新格式在 properties 里，旧格式在顶层
    const sel = n.properties?.approverSelector ?? n.approverSelector
    return !sel
  })
  if (bad.length) {
    ElMessage.warning(`审批节点必须填写审批人选择器：${bad.map((b: any) => b.properties?.name || b.name || b.id).join('、')}`)
    return
  }
  try {
    saving.value = true
    const payload: any = {
      name: current.value.name,
      displayName: current.value.displayName || undefined,
      ticketType: current.value.ticketType,
      category: current.value.category || undefined,
      description: current.value.description || undefined,
      definition: def,
    }
    const updated = await updateTemplate(current.value.id, payload)
    current.value = { ...current.value, version: updated.version, definition: def }
    dirty.value = false
    ElMessage.success(`已保存（版本 v${updated.version}）`)
    await loadTemplates()
  } catch (e: any) { ElMessage.error(e?.message || String(e)) }
  finally { saving.value = false }
}

async function toggleEnabled(t: WorkflowTemplateSummary, v: boolean) {
  try {
    if (v) await enableTemplate(t.id)
    else   await disableTemplate(t.id)
    ElMessage.success(v ? '已启用' : '已禁用')
    t.enabled = v
    if (current.value?.id === t.id) current.value.enabled = v
  } catch (e: any) { ElMessage.error(e?.message || String(e)); t.enabled = !v }
}

async function cloneTemplate(t: WorkflowTemplateSummary) {
  try {
    const src = await getTemplate(t.id)
    const suffix = new Date().getTime().toString().slice(-4)
    const baseName = src.name.replace(/_copy\d*$/g, '').slice(0, 50)
    const baseDisp = String(src.displayName || src.name).replace(/\(副本.*\)$/g, '').trim()
    const created = await createTemplate({
      name: `${baseName}_copy${suffix}`,
      displayName: `${baseDisp}（副本）`,
      ticketType: src.ticketType,
      category: src.category || undefined,
      description: `复制自模板 ${src.name}${src.displayName ? ` / ${src.displayName}` : ''}。\n${src.description || ''}`,
      definition: src.definition as any,
      enabled: false
    })
    ElMessage.success('已复制为自定义模板')
    await loadTemplates()
    const d = await getTemplate(created.id)
    current.value = d; dirty.value = false
    await nextTick(); renderFlow(normalizeDef(d.definition))
  } catch (e: any) { ElMessage.error(e?.message || String(e)) }
}
async function deleteTemplateAction(t: WorkflowTemplateSummary) {
  try {
    await deleteTemplate(t.id)
    if (current.value?.id === t.id) current.value = null
    await loadTemplates()
    ElMessage.success('已删除')
  } catch (e: any) { ElMessage.error(e?.message || String(e)) }
}

onMounted(async () => { await loadTemplates(); await loadApproverOptions(); await loadTicketTypeOptions() })
onBeforeUnmount(() => { if (lf) { try { (lf as any).destroy?.() } catch {}; lf = null } })
</script>

<style scoped>
.wf-page { height: 100%; display: flex; flex-direction: column; gap: 10px; }
.wf-topbar { display:flex; align-items:center; justify-content: space-between; background:#fff; padding:10px 14px; border-radius:8px; }
.wf-title { display:flex; align-items:center; gap:8px; color:#303133; font-weight:600; }
.wf-title-txt { font-size:15px; }
.wf-actions { display:flex; align-items:center; gap:8px; }
.wf-main { flex:1; display:flex; gap:10px; min-height: 0; }

.wf-sidebar-left { width:300px; background:#fff; border-radius:8px; display:flex; flex-direction:column; overflow:hidden; }
.wf-sidebar-title { padding:10px 14px; border-bottom:1px solid #eee; display:flex; align-items:center; justify-content:space-between; font-weight:600; color:#303133; }
.wf-template-list { flex:1; overflow:auto; padding:10px; display:flex; flex-direction:column; gap:10px; }
.wf-template-card { border:1px solid #ebeef5; border-radius:8px; padding:10px; cursor:pointer; transition: all .15s; }
.wf-template-card:hover { border-color:#409EFF; box-shadow:0 2px 8px rgba(64,158,255,.15) }
.wf-template-card.active { border-color:#409EFF; background:#f5faff; box-shadow:0 2px 10px rgba(64,158,255,.2) }
.wf-card-head { display:flex; align-items:center; justify-content:space-between; }
.wf-card-name { font-weight:600; color:#303133; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; max-width: 180px; }
.wf-card-meta { display:flex; gap:6px; align-items:center; color:#909399; font-size:12px; margin:6px 0; }
.wf-card-desc { color:#606266; font-size:12px; line-height:1.45; display:-webkit-box; -webkit-line-clamp:2; -webkit-box-orient:vertical; overflow:hidden; }
.wf-card-foot { margin-top:8px; display:flex; justify-content:space-between; align-items:center; font-size:12px; }
.wf-uid { color:#c0c4cc; }
.wf-card-actions { display:flex; gap:6px; }

.wf-canvas-wrap { flex:1; display:flex; flex-direction:column; background:#fff; border-radius:8px; overflow:hidden; min-width: 0; }
.wf-canvas-toolbar { display:flex; align-items:center; padding:8px 12px; gap:10px; border-bottom:1px solid #f0f0f0; overflow-x:auto; }
.wf-palette-node {
  display:inline-flex; align-items:center; gap:6px; padding:6px 10px; border:1px dashed #c0c4cc; border-radius:6px;
  background:#fafbfc; color:#606266; font-size:13px; cursor:grab; user-select:none; white-space:nowrap;
}
.wf-palette-node:hover { border-color:#409EFF; color:#409EFF; background:#ecf5ff; }
.wf-palette-node.start  { border-style:solid; border-color:#409EFF; color:#409EFF; background:#ecf5ff }
.wf-palette-node.end    { border-style:solid; border-color:#f56c6c; color:#f56c6c; background:#fef0f0 }
.wf-palette-node.auto   { border-style:solid; border-color:#909399; color:#909399; background:#f4f4f5 }
.wf-palette-node.diamond { border-style:solid; border-color:#e6a23c; color:#b88230; background:#fdf6ec }

.wf-canvas-holder { flex:1; position:relative; min-height: 0; background:
    linear-gradient(#f7f8fa 1px, transparent 1px) 0 0/20px 20px,
    linear-gradient(90deg, #f7f8fa 1px, transparent 1px) 0 0/20px 20px; }
.lf-canvas { position:absolute; inset:0; }
.wf-canvas-empty { position:absolute; inset:0; display:flex; align-items:center; justify-content:center; }

.wf-json-preview { border-top:1px solid #eee; max-height:36vh; display:flex; flex-direction:column; }
.wf-json-head { display:flex; justify-content:space-between; align-items:center; padding:6px 12px; background:#fafafa; border-bottom:1px solid #eee; }
.wf-json-body { margin:0; padding:10px 14px; overflow:auto; font-size:12px; color:#303133; background:#fff; }

.wf-sidebar-right { width:340px; background:#fff; border-radius:8px; display:flex; flex-direction:column; overflow:hidden; }
.wf-property { flex:1; padding:6px 12px 16px; overflow:auto; }
.wf-mono { font-family: ui-monospace, Menlo, Consolas, monospace; color:#909399; font-size:12px; }
</style>