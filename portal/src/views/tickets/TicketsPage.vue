<template>
  <div class="tickets-page">
    <!-- ============ KPI 卡片 ============ -->
    <el-row :gutter="12" class="kpi-row" v-loading="kpiLoading">
      <el-col :xs="12" :sm="8" :md="4">
        <div class="kpi-card kpi-total" @click="filter.status=''; onFilter()">
          <div class="kpi-icon"><el-icon><Tickets /></el-icon></div>
          <div class="kpi-body">
            <div class="kpi-label">全部工单</div>
            <div class="kpi-value">{{ kpis.total ?? 0 }}</div>
            <div class="kpi-sub">累计提交</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <div class="kpi-card kpi-open" @click="filter.status='open'; onFilter()">
          <div class="kpi-icon"><el-icon><Edit /></el-icon></div>
          <div class="kpi-body">
            <div class="kpi-label">待处理</div>
            <div class="kpi-value">{{ kpis.open ?? 0 }}</div>
            <div class="kpi-sub">Open / Assigned / InProgress</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <div class="kpi-card kpi-review" @click="filter.status='pending_review'; onFilter()">
          <div class="kpi-icon"><el-icon><Stamp /></el-icon></div>
          <div class="kpi-body">
            <div class="kpi-label">待复核/评审</div>
            <div class="kpi-value">{{ kpis.pendingReview ?? 0 }}</div>
            <div class="kpi-sub">Manager review</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <div class="kpi-card kpi-closed" @click="filter.status='closed'; onFilter()">
          <div class="kpi-icon"><el-icon><SuccessFilled /></el-icon></div>
          <div class="kpi-body">
            <div class="kpi-label">已关闭</div>
            <div class="kpi-value">{{ kpis.closed ?? 0 }}</div>
            <div class="kpi-sub">历史关闭</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="12" :sm="8" :md="4">
        <div class="kpi-card kpi-sla">
          <div class="kpi-icon" :style="{color: (kpis.slaBreached ?? 0)>0?'#f56c6c':'#e6a23c'}">
            <el-icon><AlarmClock /></el-icon>
          </div>
          <div class="kpi-body">
            <div class="kpi-label">SLA 违约</div>
            <div class="kpi-value" :style="{color: (kpis.slaBreached ?? 0)>0?'#f56c6c':'#303133'}">
              {{ kpis.slaBreached ?? 0 }}
            </div>
            <div class="kpi-sub">超 SLA 未关闭</div>
          </div>
        </div>
      </el-col>
      <el-col :xs="24" :md="4">
        <div class="kpi-card kpi-priority">
          <div class="kpi-priority-row">
            <span class="lbl">P1/P2</span>
            <div class="bars">
              <span class="pbar p1" :style="{width: prWidth(kpis.byPriority, 1) + '%'}"></span>
              <span class="pbar p2" :style="{width: prWidth(kpis.byPriority, 2) + '%'}"></span>
              <span class="pbar p3" :style="{width: prWidth(kpis.byPriority, 3) + '%'}"></span>
              <span class="pbar p4" :style="{width: prWidth(kpis.byPriority, 4) + '%'}"></span>
            </div>
          </div>
          <div class="kpi-priority-row sub-row">
            <span>
              <el-tag size="small" effect="dark" type="danger" style="margin-right:6px">P1</el-tag>
              <b>{{ priorityVal(kpis.byPriority, 1) }}</b>
            </span>
            <span>
              <el-tag size="small" effect="dark" type="warning" style="margin-right:6px">P2</el-tag>
              <b>{{ priorityVal(kpis.byPriority, 2) }}</b>
            </span>
            <span>
              <el-tag size="small" effect="plain" type="primary" style="margin-right:6px">P3</el-tag>
              <b>{{ priorityVal(kpis.byPriority, 3) }}</b>
            </span>
            <span>
              <el-tag size="small" effect="plain" type="info" style="margin-right:6px">P4</el-tag>
              <b>{{ priorityVal(kpis.byPriority, 4) }}</b>
            </span>
          </div>
        </div>
      </el-col>
    </el-row>

    <!-- ============ 筛选 + 操作 ============ -->
    <el-card shadow="never" class="filter-card">
      <el-form :inline="true" :model="filter" @submit.prevent>
        <el-form-item label="关键字">
          <el-input v-model="filter.keyword" placeholder="工单号/标题/分类" clearable style="width:220px"
            @keyup.enter="onFilter" @clear="onFilter" />
        </el-form-item>
        <el-form-item label="类型">
          <el-select v-model="filter.ticketType" placeholder="全部" clearable style="width:150px" @change="onFilter">
            <el-option v-for="(t, k) in TICKET_TYPE_META" :key="k" :label="t.label" :value="k as any" />
          </el-select>
        </el-form-item>
        <el-form-item label="优先级">
          <el-select v-model="filter.priority" placeholder="全部" clearable style="width:110px" @change="onFilter">
            <el-option v-for="p in [1,2,3,4]" :key="p" :label="'P'+p" :value="p" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="filter.status" placeholder="全部" clearable style="width:130px" @change="onFilter">
            <el-option v-for="s in STATUS_OPTIONS" :key="s.value" :label="s.label" :value="s.value" />
          </el-select>
        </el-form-item>
        <el-form-item label="分类">
          <el-input v-model="filter.category" placeholder="数据库/网络…" clearable style="width:130px"
            @keyup.enter="onFilter" @clear="onFilter" />
        </el-form-item>
        <el-form-item label="处理人">
          <el-select v-model="filter.assigneeId" placeholder="全部" clearable filterable style="width:150px" @change="onFilter">
            <el-option v-for="u in users" :key="u.id" :label="u.displayName || u.username" :value="u.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="SLA">
          <el-select v-model="filter.slaState" placeholder="全部" clearable style="width:110px" @change="onFilter">
            <el-option label="正常" value="ok" />
            <el-option label="临期" value="warn" />
            <el-option label="违约" value="breached" />
          </el-select>
        </el-form-item>
        <el-form-item label="创建时间">
          <el-date-picker v-model="dateRange" type="datetimerange" range-separator="→"
            value-format="YYYY-MM-DDTHH:mm:ss" start-placeholder="开始" end-placeholder="结束"
            style="width:340px" @change="onDateRangeChange" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :icon="Search" @click="onFilter">查询</el-button>
          <el-button :icon="Refresh" @click="resetFilter">重置</el-button>
          <el-button v-if="hasPerm('ticket:create')" type="success" :icon="Plus"
            @click="openCreateDialog()">新建工单</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- ============ 工单列表 ============ -->
    <el-card shadow="never" class="list-card">
      <el-table v-loading="listLoading" :data="rows" stripe @row-click="openDetail">
        <el-table-column label="编号" width="180">
          <template #default="{ row }">
            <div class="no-col">
              <el-link type="primary" :underline="false" @click.stop="openDetail(row)">
                <b>{{ row.ticketNo }}</b>
              </el-link>
              <el-tag v-if="isSlaWarn(row)" size="small" type="warning" effect="plain" style="margin-left:6px">SLA临期</el-tag>
              <el-tag v-else-if="isSlaBreach(row)" size="small" type="danger" effect="dark" style="margin-left:6px">超SLA</el-tag>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="title" label="标题" min-width="260" show-overflow-tooltip />
        <el-table-column label="类型" width="110">
          <template #default="{ row }">
            <el-tag :type="TICKET_TYPE_META[row.ticketType as keyof typeof TICKET_TYPE_META]?.tag" size="small">
              {{ TICKET_TYPE_META[row.ticketType as keyof typeof TICKET_TYPE_META]?.label || row.ticketType }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="优先级" width="90">
          <template #default="{ row }">
            <el-tag :type="PRIORITY_META[row.priority].tag" effect="dark" size="small">
              P{{ row.priority }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <el-tag :type="STATUS_META[row.status as keyof typeof STATUS_META]?.tag || 'info'" size="small">
              {{ STATUS_META[row.status as keyof typeof STATUS_META]?.label || row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="分类" width="110" prop="category" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.category">{{ row.category }}</span>
            <span v-else style="color:#c0c4cc">-</span>
          </template>
        </el-table-column>
        <el-table-column label="处理人" width="110" show-overflow-tooltip>
          <template #default="{ row }">
            <el-tag v-if="row.assigneeName" size="small" type="info" effect="plain">{{ row.assigneeName }}</el-tag>
            <span v-else style="color:#c0c4cc">未分派</span>
          </template>
        </el-table-column>
        <el-table-column label="报告人" width="110" show-overflow-tooltip>
          <template #default="{ row }">
            {{ row.reporterName || '-' }}
          </template>
        </el-table-column>
        <el-table-column label="当前节点" width="140" show-overflow-tooltip>
          <template #default="{ row }">
            <span v-if="row.currentNodeKey && row.currentNodeKey !== '__end__'"
              style="color:#409EFF">{{ nodeName(row) }}</span>
            <el-tag v-else-if="row.currentNodeKey === '__end__'" size="small" type="success" effect="plain">流程已结束</el-tag>
            <span v-else style="color:#c0c4cc">-</span>
          </template>
        </el-table-column>
        <el-table-column label="创建时间" width="160" prop="createdAt" />
        <el-table-column label="操作" width="220" align="center" fixed="right">
          <template #default="{ row }">
            <el-button link size="small" type="primary" :icon="View" @click.stop="openDetail(row)">详情</el-button>
            <el-button v-if="canTakeAction(row)" link size="small" type="success" @click.stop="openDetail(row, true)">
              处理
            </el-button>
            <el-dropdown trigger="click" @command="(c:string) => onRowCommand(c,row)" v-if="hasPerm('ticket:update')">
              <el-button link size="small" type="primary">更多<el-icon class="el-icon--right"><ArrowDown /></el-icon></el-button>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item command="assign" :icon="User">分派…</el-dropdown-item>
                  <el-dropdown-item command="cancel" :icon="Warning" divided style="color:#f56c6c"
                    :disabled="['closed','cancelled'].includes(row.status as string)">取消工单</el-dropdown-item>
                  <el-dropdown-item command="delete" :icon="Delete" style="color:#f56c6c"
                    :disabled="hasPerm('ticket:delete')===false">删除</el-dropdown-item>
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </template>
        </el-table-column>
      </el-table>
      <div class="pager">
        <el-pagination v-model:current-page="filter.page" v-model:page-size="filter.pageSize"
          :total="total" :page-sizes="[10, 20, 50, 100]" layout="total, sizes, prev, pager, next, jumper"
          @size-change="onFilter" @current-change="onFilter" />
      </div>
    </el-card>

    <!-- ============ 详情抽屉 ============ -->
    <el-drawer v-model="detailVisible" :title="detail?.ticket ? `工单 ${detail.ticket.ticketNo}  ｜  ${detail.ticket.title}` : '工单详情'"
      size="88%" direction="rtl" :close-on-click-modal="false">
      <template v-if="detail?.ticket">
        <div class="detail-header">
          <div class="dh-row1">
            <el-tag :type="(TICKET_TYPE_META as any)[detail.ticket.ticketType]?.tag" size="large">
              {{ (TICKET_TYPE_META as any)[detail.ticket.ticketType]?.label || detail.ticket.ticketType }}
            </el-tag>
            <el-tag :type="PRIORITY_META[detail.ticket.priority].tag" effect="dark" size="large" style="margin-left:8px">
              P{{ detail.ticket.priority }}
            </el-tag>
            <el-tag :type="STATUS_META[detail.ticket.status as any]?.tag || 'info'" size="large" style="margin-left:8px">
              {{ STATUS_META[detail.ticket.status as any]?.label || detail.ticket.status }}
            </el-tag>
            <el-tag v-if="isSlaBreach(detail.ticket)" type="danger" effect="dark" size="large" style="margin-left:8px">SLA 违约</el-tag>
            <el-tag v-else-if="isSlaWarn(detail.ticket)" type="warning" size="large" style="margin-left:8px">SLA 临期</el-tag>
            <span class="dh-report">报告人：{{ detail.ticket.reporterName || '-' }}｜处理人：{{ detail.ticket.assigneeName || '<未分派>' }}</span>
            <el-button v-if="hasPerm('ticket:update') && !['closed','cancelled'].includes(detail.ticket.status as string)"
              type="primary" size="small" style="margin-left:auto" :icon="EditPen" @click="editDialogVisible = true">编辑工单</el-button>
          </div>
          <div class="dh-row2">
            <el-descriptions :column="3" size="small" border>
              <el-descriptions-item label="模板">{{ detail.ticket.templateName || '-' }}</el-descriptions-item>
              <el-descriptions-item label="分类">{{ detail.ticket.category || '-' }}</el-descriptions-item>
              <el-descriptions-item label="节点">
                <span v-if="detail.ticket.currentNodeKey && detail.ticket.currentNodeKey !== '__end__'" style="color:#409EFF">
                  {{ findNode(detail.workflowNodes, detail.ticket.currentNodeKey)?.nodeName || detail.ticket.currentNodeKey }}
                </span>
                <el-tag v-else size="small" type="success" effect="plain">已结束</el-tag>
              </el-descriptions-item>
              <el-descriptions-item label="MTTA / MTTR">{{ detail.sla.mttaHours }}h / {{ detail.sla.mttrHours }}h</el-descriptions-item>
              <el-descriptions-item label="SLA 截止">
                <span :style="{color: isSlaBreach(detail.ticket)?'#f56c6c': isSlaWarn(detail.ticket)?'#e6a23c':''}">
                  {{ detail.ticket.slaDueAt || '-' }}
                </span>
              </el-descriptions-item>
              <el-descriptions-item label="创建/关闭">{{ detail.ticket.createdAt }}<br/>{{ detail.ticket.closedAt || '未关闭' }}</el-descriptions-item>
            </el-descriptions>
            <div v-if="detail.ticket.description" class="description-box">
              <div class="desc-title">📝 问题描述</div>
              <pre class="desc-body">{{ detail.ticket.description }}</pre>
            </div>
            <div v-if="detail.ticket.resolution" class="resolution-box">
              <div class="desc-title">✅ 解决方案</div>
              <pre class="desc-body">{{ detail.ticket.resolution }}</pre>
            </div>
          </div>
        </div>

        <el-tabs v-model="detailTab" class="detail-tabs">
          <!-- Tab 1: 流程视图 -->
          <el-tab-pane label="流程视图" name="flow">
            <!-- 流程节点时间线 / 处理动作 -->
            <div class="flow-wrap">
              <el-steps direction="vertical" :active="activeStepIndex(detail.workflowNodes)" finish-status="success">
                <el-step
                  v-for="(n) in displayNodes(detail.workflowNodes)"
                  :key="n.id"
                  :status="stepStatus(n)"
                  :title="stepTitle(n)"
                  :description="stepDesc(n)"
                  :icon="stepIcon(n)"
                >
                  <div class="step-inline">
                    <div class="step-meta">
                      <el-tag size="small" type="info" effect="plain">{{ nodeKindLabel(n.nodeType) }}</el-tag>
                      <el-tag v-if="n.approvers?.length" size="small" type="primary" effect="plain" style="margin-left:6px">
                        审批人: {{ fmtApprovers(n.approvers) }}
                      </el-tag>
                      <span v-if="n.timeoutHours" style="margin-left:8px; color:#909399; font-size:12px">
                        ⏱ {{ n.timeoutHours }}h · {{ n.timeoutAction || '无动作' }}
                      </span>
                    </div>
                    <!-- 审批动作（仅 active 节点且非 start/end/auto/condition/parallel） -->
                    <div v-if="isActable(n) && activeNode?.nodeKey === n.nodeKey" class="action-box card-like">
                      <div class="ab-title">
                        <b>节点动作：{{ n.nodeName }}</b>
                        <el-tag size="small" type="warning" effect="plain" style="margin-left:6px">当前激活</el-tag>
                      </div>
                      <el-form :model="actionForm" label-width="80px" size="default" inline>
                        <el-form-item label="决策">
                          <el-radio-group v-model="actionForm.decision">
                            <el-radio value="approve">通过</el-radio>
                            <el-radio value="reject">驳回</el-radio>
                            <el-radio value="skip">跳过</el-radio>
                          </el-radio-group>
                        </el-form-item>
                        <el-form-item label="驳回至" v-if="actionForm.decision==='reject' && n.rejectBackTo">
                          <el-select v-model="actionForm.toNodeKey" style="width:180px">
                            <el-option :label="'默认 → ' + (findNode(detail.workflowNodes, n.rejectBackTo)?.nodeName || n.rejectBackTo)" :value="n.rejectBackTo" />
                            <el-option-group label="回退到前序节点（按配置顺序）">
                              <el-option v-for="pv in previousKeys(n)" :key="pv"
                                :label="findNode(detail.workflowNodes, pv)?.nodeName || pv" :value="pv" />
                            </el-option-group>
                          </el-select>
                        </el-form-item>
                        <el-form-item label="处理说明" style="width:100%">
                          <el-input v-model="actionForm.comment" type="textarea" :rows="2" placeholder="请填写处理备注…（审批通过建议填写）" />
                        </el-form-item>
                        <el-form-item label="解决方案" v-if="isResolveNode(n)" style="width:100%">
                          <el-input v-model="actionForm.resolution" type="textarea" :rows="2" placeholder="填入最终解决方案（会写入工单）" />
                        </el-form-item>
                        <el-form-item>
                          <el-button type="primary" :loading="actionLoading" :icon="Check" @click="submitAction(n)">提交决策</el-button>
                          <el-button @click="actionForm = defaultActionForm()">清空</el-button>
                        </el-form-item>
                      </el-form>
                    </div>
                  </div>
                </el-step>
              </el-steps>
            </div>
          </el-tab-pane>

          <!-- Tab 2: 评论 / 审计 -->
          <el-tab-pane label="评论 / 审计" name="comments">
            <div class="comment-header card-like">
              <el-input v-model="commentText" type="textarea" :rows="2" placeholder="写下评论或备注（支持 Markdown 文本，原样保存）" />
              <div class="c-actions">
                <el-button :disabled="!commentText.trim()" type="primary" :loading="commentLoading"
                  :icon="ChatDotRound" @click="submitComment">发布评论</el-button>
              </div>
            </div>
            <el-timeline class="comment-timeline">
              <el-timeline-item
                v-for="c in detail.comments"
                :key="c.id"
                :type="timelineType(c.action)"
                :timestamp="c.createdAt"
                :hollow="false"
              >
                <div class="comment-item">
                  <div class="ci-head">
                    <b>{{ c.userName || '系统' }}</b>
                    <el-tag size="small" style="margin-left:8px">{{ actionLabel(c.action) }}</el-tag>
                    <el-tag v-if="c.nodeKey" size="small" type="primary" effect="plain" style="margin-left:6px">
                      节点: {{ findNode(detail.workflowNodes, c.nodeKey)?.nodeName || c.nodeKey }}
                    </el-tag>
                  </div>
                  <div v-if="c.content" class="ci-body">{{ c.content }}</div>
                  <pre v-if="c.extra" class="ci-extra">{{ JSON.stringify(c.extra, null, 2) }}</pre>
                </div>
              </el-timeline-item>
              <div v-if="detail.comments.length===0" style="padding:16px;color:#909399">暂无评论 / 审计记录</div>
            </el-timeline>
          </el-tab-pane>

          <!-- Tab 3: 关联告警 -->
          <el-tab-pane label="关联告警" name="alerts">
            <div class="link-alert-actions card-like">
              <el-select v-model="linkAlertChoice" style="width:180px; margin-right:8px">
                <el-option label="关联已有工单" value="link" />
                <el-option label="立即创建工单并关联" value="create" />
              </el-select>
              <template v-if="linkAlertChoice==='link'">
                <el-select v-model="pickedTicketId" filterable placeholder="选择已有工单" style="width:300px; margin-right:8px">
                  <el-option v-for="t in recentTickets" :key="t.id"
                    :label="`${t.ticketNo}｜${t.title}`" :value="t.id" />
                </el-select>
                <el-button type="primary" :icon="Link" :disabled="!pickedTicketId" :loading="linkLoading"
                  @click="pickAndLinkAlertById(pickedTicketId)">关联</el-button>
              </template>
              <template v-else>
                <el-button type="success" :icon="Plus" @click="openCreateDialogFromAlert()">基于告警新建工单</el-button>
              </template>
              <el-button style="margin-left:12px" :icon="Refresh" @click="loadDetail(detail.ticket.id)">刷新</el-button>
            </div>
            <el-table :data="detail.alertLinks" stripe style="margin-top:8px">
              <el-table-column label="级别" width="90">
                <template #default="{ row }">
                  <el-tag :type="severityTagType(row.alertSeverity)" size="small">{{ row.alertSeverity || '-' }}</el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="alertTitle" label="告警标题" min-width="300" show-overflow-tooltip />
              <el-table-column prop="relation" label="关系" width="110" />
              <el-table-column prop="createdAt" label="关联时间" width="160" />
              <el-table-column label="操作" width="120" align="center">
                <template #default="{ row }">
                  <el-button v-if="hasPerm('ticket:update')" link size="small" type="danger"
                    @click="unlinkAlertFromCurrent(row.alertId)">解除关联</el-button>
                </template>
              </el-table-column>
            </el-table>
            <el-empty v-if="detail.alertLinks.length===0" description="暂无关联告警" />
          </el-tab-pane>
        </el-tabs>
      </template>
      <el-empty v-else-if="detailLoading" description="加载中…" />
    </el-drawer>
    <!-- ============ 新建 / 编辑工单 对话框 ============ -->
    <el-dialog v-model="formVisible" :title="formMode==='create' ? '新建工单' : '编辑工单'" width="720px" :close-on-click-modal="false">
      <el-form :model="form" :rules="formRules" ref="formRef" label-width="88px">
        <el-form-item label="类型" prop="ticketType">
          <el-select v-model="form.ticketType" style="width:300px" @change="onFormTypeChange">
            <el-option v-for="(meta, k) in TICKET_TYPE_META" :key="k" :value="k as any" :label="meta.label">
              <span style="display:flex; align-items:center; gap:8px">
                <el-tag :type="meta.tag" size="small">{{ meta.label }}</el-tag>
                <span style="color:#909399">{{ meta.desc }}</span>
              </span>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item label="模板" prop="templateId">
          <el-select v-model="form.templateId" filterable style="width:300px" placeholder="不填则按类型自动选默认">
            <el-option v-for="t in typeTemplates" :key="t.id"
              :label="`${t.displayName || t.name}${t.scope === 'builtin' ? ' (内置)' : ''}`" :value="t.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="标题" prop="title">
          <el-input v-model="form.title" maxlength="200" show-word-limit placeholder="一句话描述，例如：支付核心 DB 连接超时" />
        </el-form-item>
        <el-form-item label="优先级" prop="priority">
          <el-radio-group v-model="form.priority">
            <el-radio :value="1"><el-tag effect="dark" type="danger">P1 紧急</el-tag></el-radio>
            <el-radio :value="2"><el-tag effect="dark" type="warning">P2 高</el-tag></el-radio>
            <el-radio :value="3"><el-tag effect="plain" type="primary">P3 中</el-tag></el-radio>
            <el-radio :value="4"><el-tag effect="plain" type="info">P4 低</el-tag></el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="分类">
          <el-select v-model="form.category" allow-create default-first-option filterable
            placeholder="自定义或选择：数据库 / 网络 / 安全 / 主机 / 应用 / 存储 …" style="width:300px">
            <el-option v-for="c in CATEGORY_SUGGESTIONS" :key="c" :label="c" :value="c" />
          </el-select>
        </el-form-item>
        <el-form-item label="处理人">
          <el-select v-model="form.assigneeId" filterable placeholder="可选：不填则默认分派给对应审批人" style="width:300px">
            <el-option v-for="u in users" :key="u.id" :label="`${u.displayName || u.username}（${u.roleLabel || u.roleId || '-'}）`" :value="u.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="form.description" type="textarea" :rows="4" :maxlength="2000" show-word-limit
            placeholder="问题描述 / 背景 / 复现步骤 / 影响范围…" />
        </el-form-item>
        <el-form-item v-if="linkedAlertsInForm.length > 0" label="关联告警">
          <el-tag v-for="a in linkedAlertsInForm" :key="a.id" closable @close="removeLinkedAlert(a.id)"
            style="margin-right:8px; margin-bottom:4px" :type="severityTagType(a.severity)">
            {{ a.title || a.id }}
          </el-tag>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="formVisible = false">取消</el-button>
        <el-button type="primary" :loading="formLoading" :icon="Check" @click="submitForm">
          {{ formMode === 'create' ? '提交并启动流程' : '保存更改' }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch, nextTick } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import {
  Tickets, Edit, Stamp, SuccessFilled, AlarmClock, Search, Refresh, Plus,
  View, User, Warning, Delete, ArrowDown, EditPen, Check, ChatDotRound, Link
} from '@element-plus/icons-vue'
import {
  listTickets, getTicketKpis, getTicketDetail, createTicket, updateTicket,
  deleteTicket, assignTicket, executeNodeAction, addComment, linkAlert,
  unlinkAlert, cancelTicket,
  type TicketSummary, type TicketDetail, type TicketNode, type TicketListQuery,
  type TicketPriority, type TicketStatus, type CommentAction, type WorkflowActionReq,
} from '../../api/ticket'
import { listAllTemplates, type WorkflowTemplate } from '../../api/template'
import { listAlertEvents, type AlertEvent, type AlertEventQuery } from '../../api/alert'
import { useUserStore } from '../../stores/user'

// ---------------- 路由 & 权限 ----------------
const userStore = useUserStore()
function hasPerm(p: string) { return userStore.hasPermission(p) }

// ---------------- 常量 ----------------
const TICKET_TYPE_META = {
  incident:          { label: '事件工单',    desc: '告警拨测/事件驱动', tag: 'danger' as const },
  problem:           { label: '故障工单',    desc: '根因修复/RCA',   tag: 'warning' as const },
  change:            { label: '标准变更',    desc: '普通变更审批流', tag: 'primary' as const },
  change_emergency:  { label: '紧急变更',    desc: '快速紧急变更',   tag: 'danger' as const },
  task:              { label: '运维任务',    desc: '日常运维操作',   tag: 'info' as const },
} as const
const PRIORITY_META: Record<number, { label: string; tag: any; warn: number }> = {
  1: { label: 'P1 紧急', tag: 'danger',  warn: 1 },
  2: { label: 'P2 高',   tag: 'warning', warn: 2 },
  3: { label: 'P3 中',   tag: 'primary', warn: 8 },
  4: { label: 'P4 低',   tag: 'info',    warn: 24 },
}
const STATUS_META: Record<string, { label: string; tag: any }> = {
  open:           { label: '已创建',     tag: 'info' },
  assigned:       { label: '已分派',     tag: '' },
  in_progress:    { label: '处理中',     tag: 'warning' },
  pending_review: { label: '待复核',     tag: 'primary' },
  resolved:       { label: '已解决',     tag: 'success' },
  closed:         { label: '已关闭',     tag: 'success' },
  cancelled:      { label: '已取消',     tag: 'danger' },
}
const STATUS_OPTIONS = Object.entries(STATUS_META).map(([value, { label }]) => ({ value, label }))
const CATEGORY_SUGGESTIONS = ['数据库', '网络', '安全', '主机', '应用', '存储', '中间件', '办公网', '操作系统', '配置', '容量', '监控', '其他']

// ---------------- 列表 / KPI ----------------
interface UserLite { id: string; username: string; displayName?: string | null; roleId?: string | null; roleLabel?: string | null }
const users = ref<UserLite[]>([])
const kpis = reactive<Record<string, any>>({ byPriority: {}, byType: {} })
const kpiLoading = ref(false)
const listLoading = ref(false)
const rows = ref<TicketSummary[]>([])
const total = ref(0)
const filter = reactive<RequiredOnlySome<TicketListQuery, 'page' | 'pageSize'>>({
  page: 1, pageSize: 20,
})
const dateRange = ref<[string, string] | null>(null)

async function loadUsers() {
  // /api/users 返回的是数组（后端不支持筛选）
  try {
    // @ts-ignore request axios 解包后直接返回 array
    const arr: any[] = await fetch('/api/users', { headers: { Authorization: `Bearer ${localStorage.getItem('meridianops_token')}` } })
      .then(r => r.json()).then(r => r.data || r)
    users.value = (arr || []).map((u: any) => ({
      id: u.id, username: u.username, displayName: u.displayName || u.username,
      roleId: u.roleId, roleLabel: u.roleName || u.roleLabel
    }))
  } catch (e) { users.value = [] }
}

type RequiredOnlySome<T, K extends keyof T> = Pick<T, K> & Partial<T>

async function loadKpis() {
  try { kpiLoading.value = true; const r = await getTicketKpis(); Object.assign(kpis, r) }
  catch {} finally { kpiLoading.value = false }
}
async function loadList() {
  try {
    listLoading.value = true
    const params: TicketListQuery = { ...filter }
    if (dateRange.value?.[0]) params.createdAtFrom = dateRange.value[0]
    if (dateRange.value?.[1]) params.createdAtTo   = dateRange.value[1]
    const r = await listTickets(params)
    rows.value = r.list; total.value = r.total
  } finally { listLoading.value = false }
}
function onFilter() { filter.page = 1; void loadList() }
function resetFilter() {
  Object.keys(filter).forEach(k => { if (!['page','pageSize'].includes(k)) delete (filter as any)[k] })
  dateRange.value = null
  filter.page = 1; void loadList()
}
function onDateRangeChange(v: [string, string] | null) { dateRange.value = v || null; onFilter() }

function prWidth(by: Record<string, number>, p: 1|2|3|4) {
  const sum = (Object.values(by || {}) as number[]).reduce((a,b)=>a+(b||0),0) || 1
  return Math.min(100, Math.round(((by['P'+p] ?? by[p]) ?? 0) / sum * 100))
}
function priorityVal(by: Record<string, number>, p: 1|2|3|4) { return by?.['P'+p] ?? by?.[p] ?? 0 }

// ---------------- SLA helpers ----------------
function hoursUntil(iso?: string| null) {
  if (!iso) return Infinity
  const d = new Date(iso).getTime()
  if (Number.isNaN(d)) return Infinity
  return (d - Date.now()) / 3600000
}
function isSlaBreach(t?: { slaDueAt?: string | null; status?: string } | null) {
  if (!t || !t.slaDueAt) return false
  if (t.status === 'closed' || t.status === 'cancelled' || t.status === 'resolved') return false
  return hoursUntil(t.slaDueAt) < 0
}
function isSlaWarn(t: { slaDueAt?: string | null; status?: string; priority?: TicketPriority }) {
  if (!t.slaDueAt) return false
  if (t.status === 'closed' || t.status === 'cancelled') return false
  const left = hoursUntil(t.slaDueAt)
  const warn = PRIORITY_META[t.priority || 3].warn
  return left >= 0 && left < warn
}

// ---------------- 详情 + 流程 ----------------
const detailVisible = ref(false)
const detailLoading = ref(false)
const detail = ref<TicketDetail | null>(null)
const detailTab = ref<'flow' | 'comments' | 'alerts'>('flow')
const recentTickets = ref<TicketSummary[]>([])
const linkAlertChoice = ref<'link'|'create'>('link')
const pickedTicketId = ref('')
const linkLoading = ref(false)

function displayNodes(nodes: TicketNode[]) { return (nodes||[]).filter(n => n.nodeKey !== '__start__' && n.nodeKey !== '__end__') }

function findNode(nodes: TicketNode[], key?: string | null) { return (nodes||[]).find(n => n.nodeKey === key) }

const activeNode = computed<TicketNode | null>(() => {
  if (!detail.value) return null
  const k = detail.value.ticket.currentNodeKey
  return findNode(detail.value.workflowNodes, k) || null
})

function nodeName(row: TicketSummary) {
  if (!detail.value) return row.currentNodeKey
  return findNode(detail.value.workflowNodes, row.currentNodeKey)?.nodeName || row.currentNodeKey
}
function stepStatus(n: TicketNode): any {
  switch (n.status) {
    case 'done':     return 'success'
    case 'active':   return 'process'
    case 'rejected': return 'error'
    case 'skipped':  return 'wait'
    default:         return 'wait'
  }
}
function stepTitle(n: TicketNode) {
  return n.nodeName
}
function stepIcon(n: TicketNode) {
  const icons: any = {
    start: 'Finished', end: 'CircleCheck', condition_gateway: 'Connection',
    auto_pass: 'Promotion', single_approval: 'UserFilled', all_approval: 'Avatar',
    any_approval: 'User', countersign: 'CirclePlus', parallel_split: 'Share', parallel_join: 'FolderAdd'
  }
  return icons[n.nodeType] || 'Memo'
}
function stepDesc(n: TicketNode) {
  const who = fmtApprovers(n.approvers)
  const when = n.enteredAt ? `进入：${n.enteredAt.replace('T',' ').slice(0,19)}` : '未进入'
  const done = n.doneAt ? `｜完成：${n.doneAt.replace('T',' ').slice(0,19)}` : ''
  const dec = n.decision ? `｜决策：${n.decision}` : ''
  return `审批人：${who}  ｜  ${when}${done}${dec}`
}
function activeStepIndex(nodes: TicketNode[]) {
  const list = displayNodes(nodes)
  const actIdx = list.findIndex(n => n.status === 'active' || n.status === 'pending')
  return actIdx < 0 ? list.length : actIdx + 1
}
function nodeKindLabel(k: string) {
  const map: Record<string, string> = {
    start: '起始', end: '结束', auto_pass: '自动通过',
    single_approval: '单人审批', all_approval: '全员审批', any_approval: '或签审批',
    countersign: '会签', condition_gateway: '条件分支',
    parallel_split: '并行分叉', parallel_join: '并行汇聚'
  }
  return map[k] || k
}
function fmtApprovers(list?: Array<{id?: string; name?: string} | any> | null): string {
  if (!Array.isArray(list)) return '-'
  return list.map(a => String(a?.name || a?.id || '')).filter(Boolean).join(', ') || '-'
}
function isActable(n: TicketNode) {
  if (n.status !== 'active') return false
  return !['start','end','auto_pass','condition_gateway','parallel_split','parallel_join'].includes(n.nodeType)
}
function canTakeAction(row: TicketSummary) {
  if (!activeNode.value) return false
  if (row.status === 'closed' || row.status === 'cancelled') return false
  if (!hasPerm('ticket:update')) return false
  return true
}
function previousKeys(n: TicketNode) {
  if (!detail.value) return []
  return detail.value.workflowNodes
    .filter((x: TicketNode) => (x.outs||[]).some((o: any) => o.to === n.nodeKey))
    .map((x: TicketNode) => x.nodeKey)
    .filter((k: string) => k !== '__start__' && k !== '__end__' && k !== n.nodeKey)
}
function isResolveNode(n: TicketNode) {
  const key = n.nodeKey
  return key === 'resolve' || key === 'verify' || key === 'closure' || n.nodeName.includes('解决')
}

// 动作表单
const actionLoading = ref(false)
const actionForm = reactive<WorkflowActionReq>(defaultActionForm())
function defaultActionForm(): WorkflowActionReq { return { decision: 'approve', comment: '' } }

async function openDetail(row: TicketSummary, focusAction = false) {
  detailVisible.value = true
  detail.value = null
  detailTab.value = focusAction ? 'flow' : detailTab.value
  await loadDetail(row.id)
  if (focusAction) await nextTick()
}
async function loadDetail(id: string) {
  try {
    detailLoading.value = true
    detail.value = await getTicketDetail(id)
    actionForm.decision = 'approve'
    actionForm.comment = ''
    actionForm.resolution = undefined
    actionForm.toNodeKey = undefined
  } catch (e) { ElMessage.error(errMsg(e)) } finally { detailLoading.value = false }
}

async function submitAction(n: TicketNode) {
  if (!detail.value) return
  actionLoading.value = true
  try {
    const ticketId = detail.value.ticket.id
    const res = await executeNodeAction(ticketId, n.nodeKey, { ...actionForm })
    ElMessage.success(res.done ? '流程已全部走完，工单关闭' : `动作提交成功，当前节点: ${res.currentNodeKey || '-'}`)
    await loadDetail(ticketId)
    await Promise.all([loadList(), loadKpis()])
  } catch (e) { ElMessage.error(errMsg(e)) } finally { actionLoading.value = false }
}

// ---------------- 评论 ----------------
const commentLoading = ref(false)
const commentText = ref('')
async function submitComment() {
  if (!detail.value || !commentText.value.trim()) return
  try {
    commentLoading.value = true
    await addComment(detail.value.ticket.id, { content: commentText.value.trim() })
    commentText.value = ''
    await loadDetail(detail.value.ticket.id)
    ElMessage.success('评论已发布')
  } catch (e) { ElMessage.error(errMsg(e)) } finally { commentLoading.value = false }
}
function actionLabel(a: CommentAction | string) {
  const m: Record<string, string> = {
    create: '创建工单', comment: '评论', assign: '分派', approve: '审批通过', reject: '审批驳回',
    reassign: '改派', close: '关闭工单', cancel: '取消工单', link_alert: '关联告警', unlink_alert: '解除关联'
  }
  return m[a] || a
}
function timelineType(a: CommentAction | string) {
  switch (a) {
    case 'approve': case 'close': return 'success'
    case 'reject': case 'cancel': return 'danger'
    case 'assign': case 'reassign': return 'warning'
    case 'link_alert': case 'unlink_alert': return 'primary'
    default: return 'primary'
  }
}

// ---------------- 关联告警 ----------------
// 详情页中「关联已有告警」：弹窗提示用户，也支持在告警中心反向关联
async function pickAndLinkAlert(targetTicketId?: string) {
  const tid = targetTicketId || detail.value?.ticket.id
  if (!tid) return
  try {
    const { value: alertId } = await ElMessageBox.prompt(
      '请输入要关联的 alertId，或直接使用告警列表右键快捷操作「关联工单」',
      '关联告警', { inputPattern: /.+/, inputErrorMessage: 'ID 不能为空' }
    )
    await linkAlert(tid, { alertId })
    ElMessage.success('关联成功')
    if (detail.value?.ticket.id === tid) await loadDetail(tid)
  } catch {}
}
async function pickAndLinkAlertById(ticketId: string) { await pickAndLinkAlert(ticketId) }
async function unlinkAlertFromCurrent(alertId: string) {
  if (!detail.value) return
  try {
    await ElMessageBox.confirm(`确认解除与告警 ${alertId} 的关联？`, '解除关联', { type: 'warning' })
    await unlinkAlert(detail.value.ticket.id, alertId)
    ElMessage.success('已解除')
    await loadDetail(detail.value.ticket.id)
  } catch {}
}
function openCreateDialogFromAlert() {
  // 从详情页关联告警 tab 直接创建时，取当前已关联的告警做种子
  const alertIds = (detail.value?.alertLinks || []).map(l => l.alertId)
  openCreateDialog(undefined, alertIds)
}
function severityTagType(s?: string| null) {
  if (s === 'P0' || s === 'P1') return 'danger'
  if (s === 'P2') return 'warning'
  if (s === 'P3') return 'primary'
  if (s === 'info') return 'info'
  return 'info'
}

// ---------------- 行级操作 ----------------
async function onRowCommand(cmd: string, row: TicketSummary) {
  switch (cmd) {
    case 'assign': {
      // 稳妥：直接 prompt 一个 dialog，让输入处理人 ID；实际生产用下拉
      try {
        const opts = users.value.map(u => ` - ${u.displayName || u.username}（ID=${u.id}）`).join('\n')
        const { value: aid } = await ElMessageBox.prompt(
          `填入处理人 ID。可选用户:\n${opts.slice(0, 1500)}`,
          '分派处理人', { inputPattern: /^[A-Za-z0-9_-]{3,}$/, inputErrorMessage: 'ID 格式不正确' }
        )
        await assignTicket(row.id, { assigneeId: aid })
        ElMessage.success('分派成功')
        await Promise.all([loadList(), loadDetail(row.id)])
      } catch {}
      break
    }
    case 'cancel': {
      try {
        await ElMessageBox.confirm(`确认取消工单「${row.title}」？此操作不可撤销。`, '取消工单', { type: 'warning' })
        await cancelTicket(row.id)
        ElMessage.success('工单已取消')
        await Promise.all([loadList(), loadKpis()])
      } catch {}
      break
    }
    case 'delete': {
      try {
        await ElMessageBox.confirm(`确认删除工单「${row.title}」？此操作不可恢复（软删除）。`, '删除工单', { type: 'warning' })
        await deleteTicket(row.id)
        ElMessage.success('删除成功')
        await Promise.all([loadList(), loadKpis()])
      } catch {}
      break
    }
  }
}

// ---------------- 新建 / 编辑 ----------------
const formVisible = ref(false)
const formLoading = ref(false)
const formMode = ref<'create' | 'update'>('create')
const formRef = ref<FormInstance>()
const templates = ref<WorkflowTemplate[]>([])
const linkedAlertsInForm = ref<AlertEvent[]>([])

const form = reactive<{
  ticketType: any; templateId: string; title: string; priority: TicketPriority;
  category?: string; assigneeId?: string; description: string;
}>({
  ticketType: 'incident', templateId: '', title: '', priority: 3, description: '',
})

const formRules: FormRules = {
  ticketType: [{ required: true, message: '请选择工单类型', trigger: 'change' }],
  title: [{ required: true, message: '请填写标题', trigger: 'blur' }, { min: 4, message: '标题长度至少 4 字', trigger: 'blur' }],
  priority: [{ required: true, message: '请选择优先级', trigger: 'change' }],
  description: [{ required: false }],
}

const typeTemplates = computed<WorkflowTemplate[]>(() => {
  const tt = form.ticketType
  return templates.value.filter(t => t.ticketType === tt && t.enabled)
})

function onFormTypeChange() { form.templateId = typeTemplates.value.find(t => t.scope === 'builtin')?.id || typeTemplates.value[0]?.id || '' }

function removeLinkedAlert(id: string) {
  linkedAlertsInForm.value = linkedAlertsInForm.value.filter(a => a.id !== id)
}

function openCreateDialog(presetType?: string, alertIds?: string[]) {
  formMode.value = 'create'
  form.ticketType = presetType || 'incident'
  form.title = ''; form.description = ''; form.priority = 3
  form.category = ''; form.assigneeId = undefined; form.templateId = ''
  onFormTypeChange()
  linkedAlertsInForm.value = []
  if (alertIds?.length) void (async () => {
    try {
      // @ts-ignore
      const r: any = await listAlertEvents({ page: 1, pageSize: 200 } as AlertEventQuery)
      const list: AlertEvent[] = r.items || r.list || []
      linkedAlertsInForm.value = list.filter(e => alertIds.includes(e.id))
    } catch {}
  })()
  formVisible.value = true
}

const editDialogVisible = ref(false)
watch(editDialogVisible, (v) => {
  if (!v || !detail.value) return
  formMode.value = 'update'
  const t = detail.value.ticket
  form.ticketType = t.ticketType
  form.templateId = t.templateId || ''
  form.title = t.title
  form.description = t.description || ''
  form.priority = t.priority
  form.category = t.category || undefined
  form.assigneeId = t.assigneeId || undefined
  onFormTypeChange()
  formVisible.value = true
  editDialogVisible.value = false
})

async function submitForm() {
  await formRef.value?.validate()
  formLoading.value = true
  try {
    if (formMode.value === 'create') {
      const alertIds = linkedAlertsInForm.value.map(a => a.id)
      const { id, ticketNo } = await createTicket({
        ticketType: form.ticketType, title: form.title.trim(),
        description: form.description || undefined, priority: form.priority,
        category: form.category || undefined, assigneeId: form.assigneeId,
        templateId: form.templateId || undefined, alertIds,
      })
      ElMessage.success(`工单已创建：${ticketNo}`)
      formVisible.value = false
      await Promise.all([loadKpis(), loadList()])
      openDetail({ id, ticketNo, title: form.title, ticketType: form.ticketType, priority: form.priority, status: 'open', reporterId: (userStore.user as any)?.id || '', createdAt: '' } as TicketSummary, true)
    } else {
      if (!detail.value) return
      await updateTicket(detail.value.ticket.id, {
        title: form.title.trim(), description: form.description || undefined,
        priority: form.priority, category: form.category || undefined,
        status: detail.value.ticket.status as TicketStatus, assigneeId: form.assigneeId,
      })
      ElMessage.success('工单更新成功')
      formVisible.value = false
      await loadDetail(detail.value.ticket.id)
      await loadList()
    }
  } catch (e) { ElMessage.error(errMsg(e)) } finally { formLoading.value = false }
}

// ---------------- 错误解包 ----------------
function errMsg(e: unknown): string {
  const msg = (e as any)?.message || String(e || '请求失败')
  return msg.slice(0, 300)
}

// ---------------- 生命周期 ----------------
onMounted(async () => {
  await Promise.all([loadUsers(), loadKpis(), loadList()])
  try { templates.value = await listAllTemplates() as any } catch {}
  try {
    const recent = await listTickets({ page: 1, pageSize: 50 })
    recentTickets.value = recent.list
  } catch {}
})
</script>

<style scoped>
.tickets-page { padding: 8px 0 40px; }
.kpi-row { margin-bottom: 14px; }
.kpi-row .el-col { margin-bottom: 10px; }
.kpi-card {
  display:flex; align-items:center; gap:14px; padding:16px 18px;
  background: var(--el-bg-color); border-radius: 10px;
  box-shadow: 0 1px 4px rgba(0,0,0,.04);
  cursor: pointer; user-select: none; transition: transform .15s, box-shadow .15s;
}
.kpi-card:hover { transform: translateY(-1px); box-shadow: 0 6px 16px rgba(64,158,255,.15); }
.kpi-icon {
  width: 48px; height: 48px; border-radius: 12px; display:flex; align-items:center; justify-content:center;
  font-size: 24px; color: #fff; flex-shrink: 0;
}
.kpi-total .kpi-icon  { background: linear-gradient(135deg,#409EFF,#5BADFF); }
.kpi-open .kpi-icon   { background: linear-gradient(135deg,#E6A23C,#F0C78E); }
.kpi-review .kpi-icon { background: linear-gradient(135deg,#909399,#B1B3B8); }
.kpi-closed .kpi-icon { background: linear-gradient(135deg,#67C23A,#95D475); }
.kpi-sla .kpi-icon    { background: linear-gradient(135deg,#F56C6C,#F89898); color:#fff !important; }
.kpi-body .kpi-label { font-size: 12px; color: #909399; }
.kpi-body .kpi-value { font-size: 26px; font-weight: 600; line-height: 1.3; color: #303133; }
.kpi-body .kpi-sub   { font-size: 11px; color: #c0c4cc; }
.kpi-priority { flex-direction: column; align-items: stretch; gap: 8px; cursor: default; }
.kpi-priority-row { display:flex; align-items:center; justify-content: space-between; }
.kpi-priority-row .bars { display:flex; gap: 4px; width: calc(100% - 60px); }
.kpi-priority-row .bars .pbar { display:block; height: 8px; border-radius: 4px; min-width: 2px; }
.kpi-priority-row .bars .p1 { background: #f56c6c; }
.kpi-priority-row .bars .p2 { background: #e6a23c; }
.kpi-priority-row .bars .p3 { background: #409eff; }
.kpi-priority-row .bars .p4 { background: #909399; }
.kpi-priority-row.sub-row { flex-wrap: wrap; gap: 8px; }

.filter-card { margin-bottom: 12px; }
.filter-card .el-form-item { margin-bottom: 8px; margin-right: 6px; }

.list-card .no-col { display:flex; align-items:center; }

.pager { display:flex; justify-content:flex-end; padding: 12px 6px 0; }

.detail-header { margin-bottom: 14px; }
.detail-header .dh-row1 {
  display:flex; align-items:center; margin-bottom: 10px; gap: 6px; flex-wrap: wrap;
}
.detail-header .dh-report { margin-left: 18px; color: #606266; font-size: 13px; }
.description-box, .resolution-box {
  margin-top: 12px; padding: 10px 12px; border-radius: 8px;
}
.description-box { background: #f4f4f5; }
.resolution-box  { background: #f0f9eb; }
.desc-title { font-size: 13px; font-weight: 600; margin-bottom: 6px; }
.desc-body  { margin: 0; white-space: pre-wrap; word-break: break-word; font-size: 13px; }

.detail-tabs { margin-top: 18px; }
.flow-wrap { padding: 6px 4px 20px; min-height: 200px; }
.flow-wrap .el-steps--vertical { padding-left: 4px; }
.flow-wrap .el-step { min-height: 64px; }
.flow-wrap .el-step__icon { width: 34px; height: 34px; line-height: 34px; }
.flow-wrap .el-step__title { font-weight: 600; font-size: 14px; }
.flow-wrap .el-step__description { padding-bottom: 6px; }
.step-inline { padding: 4px 0 12px 4px; }
.step-meta  { margin-bottom: 8px; display:flex; flex-wrap: wrap; gap: 6px; align-items: center; }
.card-like  { background:#FAFBFC; border: 1px dashed #e4e7ed; padding: 12px; border-radius: 8px; }
.action-box .ab-title { margin-bottom: 10px; }

.comment-header { padding: 14px; border-radius: 8px; margin-bottom: 10px; }
.comment-header .c-actions { display:flex; justify-content:flex-end; margin-top: 8px; }
.comment-timeline { padding-left: 6px; }
.comment-item .ci-head { margin-bottom: 6px; }
.comment-item .ci-body { white-space: pre-wrap; color: #303133; }
.comment-item .ci-extra {
  background: #f5f7fa; padding: 6px 10px; border-radius: 6px;
  font-size: 12px; color: #606266; max-height: 160px; overflow: auto; margin-top: 6px;
}
.link-alert-actions { padding: 12px; border-radius: 8px; display:flex; align-items:center; flex-wrap: wrap; gap: 4px; }
</style>