<template>
  <div class="monitor-page">
    <div class="page-header">
      <div class="page-title">
        <el-icon><Monitor /></el-icon>
        <span>主机名册</span>
        <span class="page-sub">通断与版本、启用禁用、编辑批量更新、启停升级</span>
      </div>
      <div class="header-actions">
        <el-button v-permission="'monitor:read'" @click="router.push('/monitor/agents')">
          代理管理
        </el-button>
        <el-button v-permission="'monitor:agent'" type="primary" @click="router.push('/monitor/install')">
          安装 Agent
        </el-button>
        <el-button v-permission="'monitor:operate'" type="success" @click="openCreateHost">添加主机</el-button>
        <el-button :loading="govLoading" @click="loadGovernance">健康检查</el-button>
        <el-button :loading="syncing" @click="doSync">关联资源</el-button>
        <el-button :icon="Refresh" @click="loadHosts">刷新名册</el-button>
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
        <el-select v-model="hostFilter" placeholder="筛选" clearable style="width: 160px" @change="reloadHosts">
          <el-option label="已启用" value="enabled" />
          <el-option label="已禁用" value="disabled" />
          <el-option label="Agent 离线" value="offline" />
          <el-option label="接口不可用" value="iface_offline" />
          <el-option label="未关联 CMDB" value="unlinked" />
          <el-option label="已关联 CMDB" value="linked" />
        </el-select>
        <el-button type="primary" @click="reloadHosts">查询</el-button>
        <el-dropdown
          v-if="canOperate"
          trigger="click"
          :disabled="!selectedIds.length"
          @command="onHostCommand"
        >
          <el-button :disabled="!selectedIds.length">
            主机
            <el-icon class="el-icon--right"><ArrowDown /></el-icon>
          </el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item command="enable" :disabled="!selectedIds.length">启用监控</el-dropdown-item>
              <el-dropdown-item command="disable" :disabled="!selectedIds.length">禁用监控</el-dropdown-item>
              <el-dropdown-item command="edit" :disabled="selectedIds.length !== 1">编辑主机</el-dropdown-item>
              <el-dropdown-item command="mass" :disabled="!selectedIds.length">批量更新</el-dropdown-item>
              <el-dropdown-item
                command="delete"
                divided
                :disabled="!selectedIds.length"
                style="color: var(--el-color-danger)"
              >
                删除主机
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
        <el-dropdown
          v-if="canOperate || canAgent"
          trigger="click"
          :disabled="!selectedIds.length"
          @command="onAgentCommand"
        >
          <el-button :disabled="!selectedIds.length">
            Agent
            <el-icon class="el-icon--right"><ArrowDown /></el-icon>
          </el-button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item
                v-if="canOperate"
                command="start"
                :disabled="!selectedIds.length"
              >
                启动 Agent 服务
              </el-dropdown-item>
              <el-dropdown-item
                v-if="canOperate"
                command="stop"
                :disabled="!selectedIds.length"
              >
                停止 Agent 服务
              </el-dropdown-item>
              <el-dropdown-item
                v-if="canOperate"
                command="restart"
                :disabled="!selectedIds.length"
              >
                重启 Agent 服务
              </el-dropdown-item>
              <el-dropdown-item
                v-if="canAgent"
                command="upgrade"
                :divided="canOperate"
                :disabled="!selectedIds.length"
              >
                升级 / 回滚
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
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
        <el-table-column prop="ip" label="IP" width="140" />
        <el-table-column label="端口" width="90">
          <template #default="{ row }">{{ row.port || '-' }}</template>
        </el-table-column>
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag v-if="row.status === '1'" type="info" size="small">禁用</el-tag>
            <el-tag v-else type="success" size="small">启用</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="主机组" min-width="160">
          <template #default="{ row }">{{ (row.groups || []).map((g: { name: string }) => g.name).join('、') || '-' }}</template>
        </el-table-column>
        <el-table-column label="可用性" width="168">
          <template #default="{ row }">
            <HostAvailabilityCell :host="row" />
          </template>
        </el-table-column>
        <el-table-column label="Agent" width="100">
          <template #default="{ row }">
            <template v-if="hasAgentInterface(row)">
              <el-tag v-if="row.agentAvailable === true" type="success" size="small">通</el-tag>
              <el-tag v-else-if="row.agentAvailable === false" type="danger" size="small">断</el-tag>
              <el-tag v-else type="info" size="small">未知</el-tag>
            </template>
            <span v-else class="zbx-avail-empty">—</span>
          </template>
        </el-table-column>
        <el-table-column prop="agentVersion" label="版本" width="120" />
        <el-table-column label="CMDB" width="100">
          <template #default="{ row }">
            <el-tag
              v-if="row.cmdbLinked && row.ciId"
              type="success"
              size="small"
              class="cmdb-link"
              @click.stop="goAsset(row.ciId)"
            >已关联</el-tag>
            <el-tag v-else-if="row.cmdbLinked" type="success" size="small">已关联</el-tag>
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
            <div class="detail-sub">
              {{ selected.hostname }} · {{ selected.ip || '无 IP' }}{{ selected.port ? `:${selected.port}` : '' }}
            </div>
          </div>
        </div>
      </template>

      <div class="status-row">
        <div class="status-item">
          <span class="status-label">可用性</span>
          <HostAvailabilityCell :host="selected" />
        </div>
        <div class="status-item">
          <span class="status-label">Agent</span>
          <template v-if="hasAgentInterface(selected)">
            <el-tag v-if="selected.agentAvailable === true" type="success" size="small">通</el-tag>
            <el-tag v-else-if="selected.agentAvailable === false" type="danger" size="small">断</el-tag>
            <el-tag v-else type="info" size="small">未知</el-tag>
          </template>
          <span v-else class="zbx-avail-empty">—</span>
        </div>
        <div class="status-item">
          <span class="status-label">版本</span>
          <span>{{ selected.agentVersion || '-' }}</span>
        </div>
        <div class="status-item">
          <span class="status-label">配置项</span>
          <el-tag
            v-if="selected.cmdbLinked && selected.ciId"
            type="success"
            size="small"
            class="cmdb-link"
            @click="goAsset(selected.ciId)"
          >已关联</el-tag>
          <el-tag v-else-if="selected.cmdbLinked" type="success" size="small">已关联</el-tag>
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

    <el-dialog v-model="upgradeVisible" title="升级 / 回滚 Agent" width="480px">
      <el-form label-width="90px">
        <el-form-item label="动作">
          <el-radio-group v-model="upgradeAction">
            <el-radio-button value="upgrade">升级</el-radio-button>
            <el-radio-button value="rollback">回滚</el-radio-button>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="目标包" required>
          <el-select
            v-model="upgradePackageId"
            filterable
            style="width: 100%"
            placeholder="选择包库中的 Agent 包"
            :disabled="!agentPackages.length"
          >
            <el-option
              v-for="p in agentPackages"
              :key="p.id"
              :label="`${p.name} · ${p.version}${p.osArch ? ' · ' + p.osArch : ''}`"
              :value="p.id"
            />
          </el-select>
          <div v-if="!agentPackages.length" style="margin-top: 6px; color: var(--el-text-color-secondary); font-size: 12px">
            无可用包，请先到「部署配置」上传。
          </div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="upgradeVisible = false">取消</el-button>
        <el-button
          type="primary"
          :loading="taskSubmitting"
          :disabled="!upgradePackageId"
          @click="confirmUpgrade"
        >
          提交
        </el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="createVisible"
      :title="editingHostId ? '编辑主机' : '添加主机'"
      width="920px"
      top="4vh"
      destroy-on-close
      class="create-host-dialog"
    >
      <el-tabs v-model="createTab">
        <el-tab-pane label="主机" name="host">
          <el-form label-width="100px">
            <el-form-item v-if="!editingHostId" label="配置项">
              <el-select
                v-model="createForm.ciId"
                filterable
                remote
                clearable
                reserve-keyword
                placeholder="可选：选中后自动带入主机名/IP，并关联资产"
                :remote-method="searchCi"
                :loading="ciLoading"
                style="width: 100%"
                @change="onCreateCiChange"
              >
                <el-option v-for="item in ciOptions" :key="item.id" :label="item.name" :value="item.id" />
              </el-select>
              <div class="field-hint">不选也可手工创建（如 ICMP）；选了会写入名册关联。</div>
            </el-form-item>
            <el-form-item label="主机名" required>
              <el-input v-model="createForm.hostname" placeholder="技术名，唯一，如 47.114.59.131_icmp" />
            </el-form-item>
            <el-form-item label="可见名称">
              <el-input v-model="createForm.displayName" placeholder="默认与主机名相同" />
            </el-form-item>
            <el-form-item label="状态">
              <el-radio-group v-model="createForm.status">
                <el-radio :value="0">启用</el-radio>
                <el-radio :value="1">禁用</el-radio>
              </el-radio-group>
            </el-form-item>
            <el-form-item label="主机组" required>
              <el-select v-model="createForm.groupIds" multiple filterable style="width: 100%">
                <el-option
                  v-for="item in schemeGroups"
                  :key="item.groupId"
                  :label="item.name"
                  :value="item.groupId"
                />
              </el-select>
            </el-form-item>
            <el-form-item label="模板">
              <el-select v-model="createForm.templateIds" multiple filterable style="width: 100%">
                <el-option
                  v-for="item in schemeTemplates"
                  :key="item.templateId"
                  :label="item.name"
                  :value="item.templateId"
                />
              </el-select>
            </el-form-item>
            <el-form-item label="监控通道">
              <el-radio-group v-model="createForm.channel">
                <el-radio-button value="direct">Server</el-radio-button>
                <el-radio-button value="proxy">Proxy</el-radio-button>
                <el-radio-button value="proxyGroup">Proxy Group</el-radio-button>
              </el-radio-group>
            </el-form-item>
            <el-form-item v-if="createForm.channel === 'proxy'" label="Proxy">
              <el-select v-model="createForm.proxyId" filterable clearable style="width: 100%" :loading="createProxyLoading">
                <el-option
                  v-for="p in createProxies"
                  :key="p.proxyId"
                  :label="`${p.name}${p.address ? ' · ' + p.address : ''}`"
                  :value="p.proxyId"
                />
              </el-select>
            </el-form-item>
            <el-form-item v-if="createForm.channel === 'proxyGroup'" label="Proxy Group">
              <el-select
                v-model="createForm.proxyGroupId"
                filterable
                clearable
                style="width: 100%"
                :loading="createProxyLoading"
              >
                <el-option
                  v-for="g in createProxyGroups"
                  :key="g.proxyGroupId"
                  :label="g.name"
                  :value="g.proxyGroupId"
                />
              </el-select>
            </el-form-item>
          </el-form>
        </el-tab-pane>

        <el-tab-pane label="接口" name="ifaces">
          <div class="create-toolbar">
            <el-button size="small" @click="addIface">添加接口</el-button>
          </div>
          <div v-for="(iface, idx) in createForm.interfaces" :key="idx" class="iface-card">
            <div class="iface-head">
              <span>接口 {{ idx + 1 }}</span>
              <el-button
                link
                type="danger"
                size="small"
                :disabled="createForm.interfaces.length <= 1"
                @click="removeIface(idx)"
              >
                删除
              </el-button>
            </div>
            <el-form label-width="100px" size="small">
              <el-form-item label="类型">
                <el-select v-model="iface.type" style="width: 160px" @change="onIfaceTypeChange(iface)">
                  <el-option :value="1" label="Agent" />
                  <el-option :value="2" label="SNMP" />
                  <el-option :value="3" label="IPMI" />
                  <el-option :value="4" label="JMX" />
                </el-select>
                <el-checkbox
                  v-model="iface.main"
                  style="margin-left: 16px"
                  @change="(v: boolean | string | number) => { if (v) setMainIface(idx) }"
                >
                  主接口
                </el-checkbox>
              </el-form-item>
              <el-form-item label="连接">
                <el-radio-group v-model="iface.useip">
                  <el-radio :value="1">IP</el-radio>
                  <el-radio :value="0">DNS</el-radio>
                </el-radio-group>
              </el-form-item>
              <el-form-item v-if="iface.useip === 1" label="IP">
                <el-input v-model="iface.ip" placeholder="如 47.114.59.131" />
              </el-form-item>
              <el-form-item v-else label="DNS">
                <el-input v-model="iface.dns" placeholder="主机 DNS 名" />
              </el-form-item>
              <el-form-item label="端口">
                <el-input v-model="iface.port" style="width: 140px" :placeholder="defaultPort(iface.type)" />
              </el-form-item>
              <template v-if="iface.type === 2">
                <el-form-item label="SNMP 版本">
                  <el-select v-model="iface.snmpVersion" style="width: 160px">
                    <el-option :value="1" label="SNMPv1" />
                    <el-option :value="2" label="SNMPv2c" />
                    <el-option :value="3" label="SNMPv3" />
                  </el-select>
                </el-form-item>
                <el-form-item v-if="iface.snmpVersion !== 3" label="Community">
                  <el-input v-model="iface.community" placeholder="{$SNMP_COMMUNITY}" />
                </el-form-item>
                <template v-else>
                  <el-form-item label="Context">
                    <el-input v-model="iface.contextName" />
                  </el-form-item>
                  <el-form-item label="Security name">
                    <el-input v-model="iface.securityName" />
                  </el-form-item>
                  <el-form-item label="Security level">
                    <el-select v-model="iface.securityLevel" style="width: 200px">
                      <el-option :value="0" label="noAuthNoPriv" />
                      <el-option :value="1" label="authNoPriv" />
                      <el-option :value="2" label="authPriv" />
                    </el-select>
                  </el-form-item>
                  <el-form-item label="Auth passphrase">
                    <el-input v-model="iface.authPassphrase" type="password" show-password />
                  </el-form-item>
                  <el-form-item label="Priv passphrase">
                    <el-input v-model="iface.privPassphrase" type="password" show-password />
                  </el-form-item>
                </template>
              </template>
            </el-form>
          </div>
        </el-tab-pane>

        <el-tab-pane label="宏" name="macros">
          <div class="create-toolbar">
            <el-button size="small" @click="createForm.macros.push({ macro: '', value: '', description: '' })">
              添加宏
            </el-button>
          </div>
          <el-table :data="createForm.macros" size="small">
            <el-table-column label="宏" min-width="160">
              <template #default="{ row }">
                <el-input v-model="row.macro" placeholder="{$MACRO}" />
              </template>
            </el-table-column>
            <el-table-column label="值" min-width="160">
              <template #default="{ row }">
                <el-input v-model="row.value" />
              </template>
            </el-table-column>
            <el-table-column label="说明" min-width="140">
              <template #default="{ row }">
                <el-input v-model="row.description" />
              </template>
            </el-table-column>
            <el-table-column width="70">
              <template #default="{ $index }">
                <el-button link type="danger" @click="createForm.macros.splice($index, 1)">删</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <el-tab-pane label="标签" name="tags">
          <div class="create-toolbar">
            <el-button size="small" @click="createForm.tags.push({ tag: '', value: '' })">添加标签</el-button>
          </div>
          <el-table :data="createForm.tags" size="small">
            <el-table-column label="标签" min-width="160">
              <template #default="{ row }">
                <el-input v-model="row.tag" />
              </template>
            </el-table-column>
            <el-table-column label="值" min-width="160">
              <template #default="{ row }">
                <el-input v-model="row.value" />
              </template>
            </el-table-column>
            <el-table-column width="70">
              <template #default="{ $index }">
                <el-button link type="danger" @click="createForm.tags.splice($index, 1)">删</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <el-tab-pane label="库存" name="inventory">
          <el-form label-width="110px" size="small">
            <el-form-item label="库存模式">
              <el-radio-group v-model="createForm.inventoryMode">
                <el-radio :value="-1">禁用</el-radio>
                <el-radio :value="0">手动</el-radio>
                <el-radio :value="1">自动</el-radio>
              </el-radio-group>
            </el-form-item>
            <template v-if="createForm.inventoryMode !== -1">
              <el-form-item v-for="f in inventoryFields" :key="f.key" :label="f.label">
                <el-input v-model="createForm.inventory[f.key]" />
              </el-form-item>
            </template>
          </el-form>
        </el-tab-pane>
      </el-tabs>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="creating" @click="submitCreateHost">
          {{ editingHostId ? '保存' : '创建' }}
        </el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="massVisible" title="批量更新" width="640px" destroy-on-close>
      <div class="field-hint" style="margin-bottom: 12px">已选 {{ selectedIds.length }} 台主机；只更新勾选的项。</div>
      <el-form label-width="110px">
        <el-form-item>
          <el-checkbox v-model="massForm.useStatus">状态</el-checkbox>
          <el-radio-group v-model="massForm.status" :disabled="!massForm.useStatus" style="margin-left: 12px">
            <el-radio :value="0">启用</el-radio>
            <el-radio :value="1">禁用</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item>
          <el-checkbox v-model="massForm.useGroups">主机组（替换）</el-checkbox>
        </el-form-item>
        <el-form-item v-if="massForm.useGroups" label="主机组">
          <el-select v-model="massForm.groupIds" multiple filterable style="width: 100%">
            <el-option
              v-for="item in schemeGroups"
              :key="item.groupId"
              :label="item.name"
              :value="item.groupId"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-checkbox v-model="massForm.useTemplates">模板</el-checkbox>
          <el-radio-group
            v-model="massForm.templateAction"
            :disabled="!massForm.useTemplates"
            style="margin-left: 12px"
          >
            <el-radio value="replace">替换</el-radio>
            <el-radio value="link">追加</el-radio>
            <el-radio value="unlink">取消链接</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item v-if="massForm.useTemplates" label="模板">
          <el-select v-model="massForm.templateIds" multiple filterable style="width: 100%">
            <el-option
              v-for="item in schemeTemplates"
              :key="item.templateId"
              :label="item.name"
              :value="item.templateId"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-checkbox v-model="massForm.useChannel">监控通道</el-checkbox>
        </el-form-item>
        <template v-if="massForm.useChannel">
          <el-form-item label="通道">
            <el-radio-group v-model="massForm.channel">
              <el-radio-button value="direct">Server</el-radio-button>
              <el-radio-button value="proxy">Proxy</el-radio-button>
              <el-radio-button value="proxyGroup">Proxy Group</el-radio-button>
            </el-radio-group>
          </el-form-item>
          <el-form-item v-if="massForm.channel === 'proxy'" label="Proxy">
            <el-select v-model="massForm.proxyId" filterable clearable style="width: 100%" :loading="createProxyLoading">
              <el-option
                v-for="p in createProxies"
                :key="p.proxyId"
                :label="`${p.name}${p.address ? ' · ' + p.address : ''}`"
                :value="p.proxyId"
              />
            </el-select>
          </el-form-item>
          <el-form-item v-if="massForm.channel === 'proxyGroup'" label="Proxy Group">
            <el-select v-model="massForm.proxyGroupId" filterable clearable style="width: 100%" :loading="createProxyLoading">
              <el-option
                v-for="g in createProxyGroups"
                :key="g.proxyGroupId"
                :label="g.name"
                :value="g.proxyGroupId"
              />
            </el-select>
          </el-form-item>
        </template>
      </el-form>
      <template #footer>
        <el-button @click="massVisible = false">取消</el-button>
        <el-button type="primary" :loading="massSaving" @click="submitMassUpdate">更新</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ArrowDown, Monitor, Refresh } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useUserStore } from '../../stores/user'
import { getCiInstance, listCiInstances } from '../../api/cmdb'
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
  listAgentPackages,
  linkHostGroups,
  linkHostTemplates,
  listHostGroups,
  listHosts,
  listInstances,
  listProxies,
  listProxyGroups,
  listTemplates,
  massUpdateMonitorHosts,
  deleteMonitorHosts,
  getMonitorHost,
  registerMonitorHost,
  updateMonitorHost,
  saveHostMacros,
  syncHosts,
  type AgentPackage,
  type CreateMonitorHostInput,
  type GovernanceData,
  type HostItem,
  type HostMacro,
  type HostTrigger,
  type MassUpdateHostsInput,
  type MonitorAction,
  type MonitorGroupRef,
  type MonitorHost,
  type MonitorInstance,
  type MonitorProblem,
  type MonitorProxy,
  type MonitorProxyGroup,
  type MonitorTask,
  type MonitorTemplate,
  type MonitorTemplateRef,
  type UpdateMonitorHostInput,
} from '../../api/monitor'
import HostAvailabilityCell from './HostAvailabilityCell.vue'

const router = useRouter()
const route = useRoute()
const user = useUserStore()
const canOperate = computed(() => user.hasPermission('monitor:operate'))
const canAgent = computed(() => user.hasPermission('monitor:agent'))
const isAdmin = computed(() => user.role === 'admin')

/** 无 ZBX 接口时 Agent 列显示 —，不显示「未知」 */
function hasAgentInterface(row: MonitorHost | null | undefined) {
  const zbx = row?.availability?.zbx
  if (zbx === null || zbx === undefined) return false
  if (typeof zbx === 'number') return true
  return typeof zbx.status === 'number'
}

function isCmdbLinked(row: MonitorHost) {
  return !!(row.cmdbLinked && row.ciId)
}

const AGENT_ACTION_LABEL: Record<string, string> = {
  start: '启动 Agent 服务',
  stop: '停止 Agent 服务',
  restart: '重启 Agent 服务',
  upgrade: '升级 Agent',
  rollback: '回滚 Agent',
}

/** 启停/升级前：必须有 ZBX 接口且已关联 CMDB */
function resolveAgentTargets(): MonitorHost[] | null {
  const rows = selectedHosts.value
  if (!rows.length) {
    ElMessage.warning('请先勾选主机')
    return null
  }
  const noAgent = rows.filter((h) => !hasAgentInterface(h))
  if (noAgent.length) {
    const sample = noAgent
      .slice(0, 3)
      .map((h) => h.displayName || h.hostname || h.hostId)
      .join('、')
    const more = noAgent.length > 3 ? ` 等 ${noAgent.length} 台` : ''
    ElMessage.warning(`以下主机无 Agent 接口，不能执行 Agent 操作：${sample}${more}`)
    return null
  }
  const unlinked = rows.filter((h) => !isCmdbLinked(h))
  if (unlinked.length) {
    const sample = unlinked
      .slice(0, 3)
      .map((h) => h.displayName || h.hostname || h.hostId)
      .join('、')
    const more = unlinked.length > 3 ? ` 等 ${unlinked.length} 台` : ''
    ElMessage.warning(`以下主机未关联配置项，请先点「关联资源」：${sample}${more}`)
    return null
  }
  const noCred = rows.filter((h) => !h.sshCredentialBound)
  if (noCred.length) {
    const sample = noCred
      .slice(0, 3)
      .map((h) => h.displayName || h.hostname || h.hostId)
      .join('、')
    const more = noCred.length > 3 ? ` 等 ${noCred.length} 台` : ''
    ElMessage.warning(
      `以下主机关联的配置项未绑定 SSH 凭据：${sample}${more}。请到资产详情绑定，或确保作业剧本已配置默认凭据后重试（提交时仍会校验）`,
    )
    // 不硬拦：作业可能有默认凭据；最终由 Gateway 校验
  }
  return rows
}

function onHostCommand(cmd: string) {
  if (cmd === 'enable') setHostsStatus(0)
  else if (cmd === 'disable') setHostsStatus(1)
  else if (cmd === 'edit') openEditHost()
  else if (cmd === 'mass') openMassUpdate()
  else if (cmd === 'delete') deleteSelectedHosts()
}

function onAgentCommand(cmd: string) {
  if (cmd === 'start' || cmd === 'stop' || cmd === 'restart') runAction(cmd)
  else if (cmd === 'upgrade') openUpgrade()
}
const canEditScheme = computed(() => {
  if (!selected.value || !canOperate.value) return false
  return isAdmin.value || !!selected.value.cmdbLinked
})
const schemeBlockedReason = computed(() => {
  if (!selected.value) return ''
  if (isAdmin.value || selected.value.cmdbLinked) return ''
  return '这台主机还没关联配置项。可用「添加主机」选配置项关联，或点「关联资源」按 IP 匹配。'
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
const selectedHosts = ref<MonitorHost[]>([])
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
const upgradeAction = ref<'upgrade' | 'rollback'>('upgrade')
const upgradePackageId = ref('')
const agentPackages = ref<AgentPackage[]>([])
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
const ciOptions = ref<{ id: string; name: string }[]>([])
const ciLoading = ref(false)

type CreateIface = {
  interfaceId?: string
  type: number
  main: boolean
  useip: number
  ip: string
  dns: string
  port: string
  snmpVersion: number
  community: string
  contextName: string
  securityName: string
  securityLevel: number
  authPassphrase: string
  privPassphrase: string
}

const createVisible = ref(false)
const createTab = ref('host')
const creating = ref(false)
const editingHostId = ref('')
const createProxyLoading = ref(false)
const createProxies = ref<MonitorProxy[]>([])
const createProxyGroups = ref<MonitorProxyGroup[]>([])
const massVisible = ref(false)
const massSaving = ref(false)
const massForm = reactive({
  useStatus: false,
  status: 0,
  useGroups: false,
  groupIds: [] as string[],
  useTemplates: false,
  templateAction: 'replace' as 'replace' | 'link' | 'unlink',
  templateIds: [] as string[],
  useChannel: false,
  channel: 'direct' as 'direct' | 'proxy' | 'proxyGroup',
  proxyId: '',
  proxyGroupId: '',
})
const inventoryFields = [
  { key: 'type', label: '类型' },
  { key: 'name', label: '名称' },
  { key: 'alias', label: '别名' },
  { key: 'os', label: '操作系统' },
  { key: 'os_full', label: 'OS 完整' },
  { key: 'serialno_a', label: '序列号 A' },
  { key: 'tag', label: '资产标签' },
  { key: 'macaddress_a', label: 'MAC A' },
  { key: 'hardware', label: '硬件' },
  { key: 'software', label: '软件' },
  { key: 'contact', label: '联系人' },
  { key: 'location', label: '位置' },
  { key: 'model', label: '型号' },
  { key: 'vendor', label: '厂商' },
  { key: 'notes', label: '备注' },
]

function blankIface(main = false): CreateIface {
  return {
    interfaceId: '',
    type: 1,
    main,
    useip: 1,
    ip: '',
    dns: '',
    port: '',
    snmpVersion: 2,
    community: '{$SNMP_COMMUNITY}',
    contextName: '',
    securityName: '',
    securityLevel: 0,
    authPassphrase: '',
    privPassphrase: '',
  }
}

const createForm = reactive({
  hostname: '',
  displayName: '',
  status: 0,
  groupIds: [] as string[],
  templateIds: [] as string[],
  channel: 'direct' as 'direct' | 'proxy' | 'proxyGroup',
  proxyId: '',
  proxyGroupId: '',
  ciId: '',
  interfaces: [blankIface(true)] as CreateIface[],
  macros: [] as Array<{ macro: string; value: string; description: string }>,
  tags: [] as Array<{ tag: string; value: string }>,
  inventoryMode: -1,
  inventory: {} as Record<string, string>,
})
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
    const qSearch = typeof route.query.search === 'string' ? route.query.search.trim() : ''
    if (qSearch) search.value = qSearch
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

function rosterFilter(): {
  agent?: 'online' | 'offline' | 'unknown'
  cmdb?: 'linked' | 'unlinked'
  status?: '0' | '1'
} {
  if (hostFilter.value === 'offline') return { agent: 'offline' }
  if (hostFilter.value === 'iface_offline') return { availability: 'unavailable' }
  if (hostFilter.value === 'linked') return { cmdb: 'linked' }
  if (hostFilter.value === 'unlinked') return { cmdb: 'unlinked' }
  if (hostFilter.value === 'enabled') return { status: '0' }
  if (hostFilter.value === 'disabled') return { status: '1' }
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
      selectedHosts.value = []
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

function goAsset(ciId: string) {
  if (!ciId) return
  router.push(`/assets/${ciId}`)
}

async function doSync() {
  if (!instanceCode.value) return
  syncing.value = true
  try {
    const res = await syncHosts(instanceCode.value)
    ElMessage.success(`已按 IP 关联 ${res.synced} 台主机与配置项`)
  } finally {
    syncing.value = false
  }
}

function onSelection(rows: MonitorHost[]) {
  selectedHosts.value = rows
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

function defaultPort(type: number) {
  if (type === 2) return '161'
  if (type === 3) return '623'
  if (type === 4) return '12345'
  return '10050'
}

function resetCreateForm() {
  editingHostId.value = ''
  createForm.hostname = ''
  createForm.displayName = ''
  createForm.status = 0
  createForm.groupIds = groupId.value ? [groupId.value] : []
  createForm.templateIds = []
  createForm.channel = 'direct'
  createForm.proxyId = ''
  createForm.proxyGroupId = ''
  createForm.ciId = ''
  createForm.interfaces = [blankIface(true)]
  createForm.macros = []
  createForm.tags = []
  createForm.inventoryMode = -1
  createForm.inventory = {}
  createTab.value = 'host'
}

async function loadCreateProxies() {
  if (!instanceCode.value) return
  createProxyLoading.value = true
  try {
    const [px, pg] = await Promise.all([
      listProxies(instanceCode.value),
      listProxyGroups(instanceCode.value),
    ])
    createProxies.value = px
    createProxyGroups.value = pg
  } catch {
    createProxies.value = []
    createProxyGroups.value = []
  } finally {
    createProxyLoading.value = false
  }
}

async function openCreateHost() {
  if (!instanceCode.value) {
    ElMessage.warning('请先选择 Zabbix 实例')
    return
  }
  resetCreateForm()
  createVisible.value = true
  if (!schemeGroups.value.length || !schemeTemplates.value.length) {
    await loadSchemeCatalog()
  }
  await loadCreateProxies()
}

async function openEditHost() {
  if (!instanceCode.value || selectedIds.value.length !== 1) {
    ElMessage.warning('请只选择一台主机进行编辑')
    return
  }
  const hostId = selectedIds.value[0]
  if (!schemeGroups.value.length || !schemeTemplates.value.length) {
    await loadSchemeCatalog()
  }
  creating.value = true
  try {
    const detail = await getMonitorHost(instanceCode.value, hostId)
    editingHostId.value = hostId
    createForm.hostname = detail.hostname || ''
    createForm.displayName = detail.displayName || ''
    createForm.status = detail.status === '1' ? 1 : 0
    createForm.groupIds = [...(detail.groupIds || [])]
    createForm.templateIds = [...(detail.templateIds || [])]
    createForm.channel =
      detail.channel === 'proxy' || detail.channel === 'proxyGroup' ? detail.channel : 'direct'
    createForm.proxyId = detail.proxyId || ''
    createForm.proxyGroupId = detail.proxyGroupId || ''
    createForm.ciId = ''
    createForm.interfaces = (detail.interfaces || []).length
      ? detail.interfaces.map((iface) => ({
          interfaceId: iface.interfaceId || '',
          type: Number(iface.type) || 1,
          main: iface.main === true || iface.main === 1 || String(iface.main) === '1',
          useip: Number(iface.useip ?? 1) === 0 ? 0 : 1,
          ip: iface.ip || '',
          dns: iface.dns || '',
          port: String(iface.port || ''),
          snmpVersion: Number(iface.snmpVersion ?? 2),
          community: iface.community || '{$SNMP_COMMUNITY}',
          contextName: iface.contextName || '',
          securityName: iface.securityName || '',
          securityLevel: Number(iface.securityLevel ?? 0),
          authPassphrase: iface.authPassphrase || '',
          privPassphrase: iface.privPassphrase || '',
        }))
      : [blankIface(true)]
    createForm.macros = (detail.macros || []).map((m) => ({
      macro: m.macro,
      value: m.secret ? '' : m.value,
      description: m.description || '',
    }))
    createForm.tags = (detail.tags || []).map((t) => ({ tag: t.tag, value: t.value || '' }))
    createForm.inventoryMode = detail.inventoryMode ?? -1
    createForm.inventory = { ...(detail.inventory || {}) }
    createTab.value = 'host'
    createVisible.value = true
    await loadCreateProxies()
  } finally {
    creating.value = false
  }
}

async function setHostsStatus(status: 0 | 1) {
  if (!instanceCode.value || !selectedIds.value.length) return
  const label = status === 0 ? '启用' : '禁用'
  await ElMessageBox.confirm(`确认${label}选中的 ${selectedIds.value.length} 台主机？`, '提示', {
    type: 'warning',
  })
  await massUpdateMonitorHosts(instanceCode.value, {
    hostIds: [...selectedIds.value],
    status,
  })
  ElMessage.success(`已${label} ${selectedIds.value.length} 台主机`)
  await loadHosts(true)
}

async function deleteSelectedHosts() {
  if (!instanceCode.value || !selectedIds.value.length) return
  await ElMessageBox.confirm(
    `确认从 Zabbix 删除选中的 ${selectedIds.value.length} 台主机？此操作不可恢复。`,
    '删除主机',
    { type: 'warning', confirmButtonText: '删除', confirmButtonClass: 'el-button--danger' },
  )
  await deleteMonitorHosts(instanceCode.value, [...selectedIds.value])
  ElMessage.success('已删除')
  selected.value = null
  selectedIds.value = []
  selectedHosts.value = []
  await loadHosts()
}

function openMassUpdate() {
  if (!instanceCode.value || !selectedIds.value.length) return
  massForm.useStatus = false
  massForm.status = 0
  massForm.useGroups = false
  massForm.groupIds = []
  massForm.useTemplates = false
  massForm.templateAction = 'replace'
  massForm.templateIds = []
  massForm.useChannel = false
  massForm.channel = 'direct'
  massForm.proxyId = ''
  massForm.proxyGroupId = ''
  massVisible.value = true
  void loadCreateProxies()
  if (!schemeGroups.value.length || !schemeTemplates.value.length) {
    void loadSchemeCatalog()
  }
}

async function submitMassUpdate() {
  if (!instanceCode.value || !selectedIds.value.length) return
  const payload: MassUpdateHostsInput = { hostIds: [...selectedIds.value] }
  if (massForm.useStatus) payload.status = massForm.status
  if (massForm.useGroups) {
    if (!massForm.groupIds.length) {
      ElMessage.warning('请选择主机组')
      return
    }
    payload.groupIds = [...massForm.groupIds]
  }
  if (massForm.useTemplates) {
    payload.templateIds = [...massForm.templateIds]
    payload.templateAction = massForm.templateAction
  }
  if (massForm.useChannel) {
    payload.channel = massForm.channel
    if (massForm.channel === 'proxy') {
      if (!massForm.proxyId) {
        ElMessage.warning('请选择 Proxy')
        return
      }
      payload.proxyId = massForm.proxyId
    }
    if (massForm.channel === 'proxyGroup') {
      if (!massForm.proxyGroupId) {
        ElMessage.warning('请选择 Proxy Group')
        return
      }
      payload.proxyGroupId = massForm.proxyGroupId
    }
  }
  if (!massForm.useStatus && !massForm.useGroups && !massForm.useTemplates && !massForm.useChannel) {
    ElMessage.warning('请至少勾选一项要更新的内容')
    return
  }
  massSaving.value = true
  try {
    await massUpdateMonitorHosts(instanceCode.value, payload)
    ElMessage.success(`已更新 ${selectedIds.value.length} 台主机`)
    massVisible.value = false
    await loadHosts(true)
  } finally {
    massSaving.value = false
  }
}

function addIface() {
  createForm.interfaces.push(blankIface(false))
}

function removeIface(idx: number) {
  if (createForm.interfaces.length <= 1) return
  const wasMain = createForm.interfaces[idx].main
  createForm.interfaces.splice(idx, 1)
  if (wasMain && createForm.interfaces.length) {
    createForm.interfaces[0].main = true
  }
}

function setMainIface(idx: number) {
  createForm.interfaces.forEach((iface, i) => {
    iface.main = i === idx
  })
}

function onIfaceTypeChange(iface: CreateIface) {
  if (!iface.port) iface.port = defaultPort(iface.type)
}

function buildCreatePayload(): UpdateMonitorHostInput {
  const interfaces = createForm.interfaces.map((iface, idx) => {
    const row: NonNullable<CreateMonitorHostInput['interfaces']>[number] = {
      interfaceId: iface.interfaceId || undefined,
      type: iface.type,
      main: (iface.main || idx === 0) ? 1 : 0,
      useip: iface.useip,
      ip: iface.ip.trim(),
      dns: iface.dns.trim(),
      port: iface.port.trim() || defaultPort(iface.type),
    }
    if (iface.type === 2) {
      row.details = {
        version: iface.snmpVersion,
        community: iface.community || '{$SNMP_COMMUNITY}',
        contextname: iface.contextName,
        securityname: iface.securityName,
        securitylevel: iface.securityLevel,
        authpassphrase: iface.authPassphrase,
        privpassphrase: iface.privPassphrase,
      }
      if (iface.snmpVersion !== 3) {
        row.community = iface.community
        row.snmpVersion = iface.snmpVersion
      }
    }
    return row
  })
  const payload: UpdateMonitorHostInput = {
    hostname: createForm.hostname.trim(),
    displayName: createForm.displayName.trim() || undefined,
    status: createForm.status,
    groupIds: createForm.groupIds,
    templateIds: createForm.templateIds,
    interfaces,
  }
  payload.channel = createForm.channel
  if (createForm.channel === 'proxy' && createForm.proxyId) {
    payload.proxyId = createForm.proxyId
  }
  if (createForm.channel === 'proxyGroup' && createForm.proxyGroupId) {
    payload.proxyGroupId = createForm.proxyGroupId
  }
  if (!editingHostId.value && createForm.ciId) payload.ciId = createForm.ciId
  const macros = createForm.macros
    .filter((m) => m.macro.trim())
    .map((m) => ({
      macro: m.macro.trim(),
      value: m.value,
      description: m.description || undefined,
    }))
  if (macros.length || editingHostId.value) payload.macros = macros
  const tags = createForm.tags
    .filter((t) => t.tag.trim())
    .map((t) => ({ tag: t.tag.trim(), value: t.value || '' }))
  if (tags.length || editingHostId.value) payload.tags = tags
  payload.inventoryMode = createForm.inventoryMode
  if (createForm.inventoryMode !== -1) {
    const inv: Record<string, string> = {}
    for (const f of inventoryFields) {
      const v = (createForm.inventory[f.key] || '').trim()
      if (v) inv[f.key] = v
    }
    payload.inventory = inv
  }
  return payload
}

async function submitCreateHost() {
  if (!instanceCode.value) return
  if (!createForm.hostname.trim()) {
    ElMessage.warning('请填写主机名')
    createTab.value = 'host'
    return
  }
  if (!createForm.groupIds.length) {
    ElMessage.warning('请选择主机组')
    createTab.value = 'host'
    return
  }
  for (let i = 0; i < createForm.interfaces.length; i++) {
    const iface = createForm.interfaces[i]
    if (iface.useip === 1 && !iface.ip.trim()) {
      ElMessage.warning(`接口 ${i + 1} 请填写 IP`)
      createTab.value = 'ifaces'
      return
    }
    if (iface.useip === 0 && !iface.dns.trim()) {
      ElMessage.warning(`接口 ${i + 1} 请填写 DNS`)
      createTab.value = 'ifaces'
      return
    }
  }
  if (!createForm.interfaces.some((i) => i.main)) {
    createForm.interfaces[0].main = true
  }
  creating.value = true
  try {
    const payload = buildCreatePayload()
    if (editingHostId.value) {
      await updateMonitorHost(instanceCode.value, editingHostId.value, payload)
      ElMessage.success('主机已保存')
    } else {
      const res = await registerMonitorHost(instanceCode.value, payload)
      ElMessage.success(res.created ? `已创建主机 ${res.hostId}` : `主机已存在，已更新关联 ${res.hostId}`)
    }
    createVisible.value = false
    editingHostId.value = ''
    await loadHosts(true)
  } finally {
    creating.value = false
  }
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

function attrText(attrs: Record<string, unknown>, keys: string[]): string {
  for (const k of keys) {
    const v = attrs[k]
    if (typeof v === 'string' && v.trim()) return v.trim()
    if (typeof v === 'number') return String(v)
  }
  return ''
}

async function onCreateCiChange(ciId: string | null | undefined) {
  if (!ciId) return
  ciLoading.value = true
  try {
    const ci = await getCiInstance(ciId)
    const attrs = ci.attributes || {}
    const hostname = attrText(attrs, ['hostname']) || ci.name
    const ip = attrText(attrs, ['ip', 'mgmt_ip', 'bk_host_innerip'])
    if (!createForm.hostname.trim()) createForm.hostname = hostname
    if (!createForm.displayName.trim()) createForm.displayName = ci.name
    const main = createForm.interfaces[0]
    if (main && main.useip === 1 && !main.ip.trim() && ip) {
      main.ip = ip
    }
    ElMessage.success('已从配置项带入主机名/IP，可继续改接口与模板')
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : '读取配置项失败'
    ElMessage.error(msg)
  } finally {
    ciLoading.value = false
  }
}

async function runAction(action: MonitorAction) {
  const targets = resolveAgentTargets()
  if (!targets) return
  const label = AGENT_ACTION_LABEL[action] || action
  try {
    await ElMessageBox.confirm(
      `将对 ${targets.length} 台主机执行「${label}」。操作前会创建 Zabbix 维护期。`,
      '确认操作',
      { type: 'warning' },
    )
  } catch {
    return
  }
  await submitTask(
    action,
    targets.map((h) => h.hostId),
  )
}

async function openUpgrade() {
  const targets = resolveAgentTargets()
  if (!targets) return
  upgradeAction.value = 'upgrade'
  upgradePackageId.value = ''
  try {
    agentPackages.value = (await listAgentPackages()).filter((p) => p.enabled)
    upgradePackageId.value = agentPackages.value[0]?.id || ''
  } catch {
    agentPackages.value = []
  }
  if (!agentPackages.value.length) {
    ElMessage.warning('无可用 Agent 包，请先到部署配置上传')
    return
  }
  upgradeVisible.value = true
}

async function confirmUpgrade() {
  if (!upgradePackageId.value) {
    ElMessage.warning('请选择 Agent 包')
    return
  }
  const targets = resolveAgentTargets()
  if (!targets) return
  const label = AGENT_ACTION_LABEL[upgradeAction.value] || upgradeAction.value
  try {
    await ElMessageBox.confirm(
      `将对 ${targets.length} 台主机执行「${label}」。操作前会创建 Zabbix 维护期。`,
      '确认操作',
      { type: 'warning' },
    )
  } catch {
    return
  }
  const ok = await submitTask(
    upgradeAction.value,
    targets.map((h) => h.hostId),
    undefined,
    upgradePackageId.value,
  )
  if (ok) upgradeVisible.value = false
}

/** @returns 是否创建成功 */
async function submitTask(
  action: MonitorAction,
  hostIds: string[],
  version?: string,
  packageId?: string,
): Promise<boolean> {
  if (!hostIds.length) {
    ElMessage.warning('主机列表为空')
    return false
  }
  taskSubmitting.value = true
  try {
    const res = await createTask({
      instanceCode: instanceCode.value,
      action,
      hostIds,
      targetVersion: version,
      packageId,
    })
    ElMessage.success('任务已创建')
    await refreshTask(res.id)
    if (pollTimer) window.clearInterval(pollTimer)
    pollTimer = window.setInterval(() => refreshTask(res.id), 3000)
    return true
  } catch {
    // request 拦截器已提示错误
    return false
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
.zbx-avail-empty {
  color: var(--el-text-color-placeholder);
  font-size: 13px;
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
.cmdb-link {
  cursor: pointer;
}
.cmdb-link:hover {
  opacity: 0.85;
}
.field-hint {
  margin-top: 6px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
  line-height: 1.4;
}
.create-toolbar {
  margin-bottom: 10px;
}
.iface-card {
  border: 1px solid var(--el-border-color-lighter);
  border-radius: 6px;
  padding: 10px 12px 4px;
  margin-bottom: 12px;
}
.iface-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
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
