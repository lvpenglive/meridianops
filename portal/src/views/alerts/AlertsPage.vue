<template>
  <div class="alerts-page">
    <!-- Tab 切换：告警事件 / 静默规则 -->
    <el-tabs v-model="activeTab" class="page-tabs">
      <el-tab-pane label="告警事件" name="events">
        <!-- 统计卡片（活跃 / 今日新增 + 6 级别 0-5） -->
        <el-row :gutter="12" class="stats-row stats-row-8cols" v-loading="statsLoading">
          <el-col :xs="12" :sm="6" :md="3">
            <div class="stat-card stat-active">
              <div class="stat-icon"><el-icon><Bell /></el-icon></div>
              <div class="stat-body">
                <div class="stat-label">活跃告警</div>
                <div class="stat-value">{{ stats?.activeTotal ?? 0 }}</div>
                <div class="stat-sub">未解决告警总数</div>
              </div>
            </div>
          </el-col>
          <el-col :xs="12" :sm="6" :md="3">
            <div class="stat-card stat-today">
              <div class="stat-icon"><el-icon><Plus /></el-icon></div>
              <div class="stat-body">
                <div class="stat-label">今日新增</div>
                <div class="stat-value">{{ stats?.todayNew ?? 0 }}</div>
                <div class="stat-sub">今日触发的告警</div>
              </div>
            </div>
          </el-col>
          <el-col v-for="lvl in ALERT_LEVEL_ORDER" :key="lvl" :xs="12" :sm="8" :md="4" :lg="4" :xl="3">
            <div class="stat-card" :class="`stat-level-${lvl}`" :style="{ boxShadow: `0 0 0 1px ${ALERT_LEVEL_META[lvl].glow}` }">
              <div class="stat-icon" :style="{ color: ALERT_LEVEL_META[lvl].color }"><el-icon><Warning /></el-icon></div>
              <div class="stat-body">
                <div class="stat-label">{{ lvl }} {{ ALERT_LEVEL_META[lvl].shortName }}</div>
                <div class="stat-value" :style="{ color: ALERT_LEVEL_META[lvl].color }">{{ stats?.bySeverity?.[lvl] ?? 0 }}</div>
                <div class="stat-sub">当前未解决</div>
              </div>
            </div>
          </el-col>
        </el-row>

        <!-- 筛选区 -->
        <el-card shadow="never" class="filter-card">
          <el-form :inline="true" :model="filter" @submit.prevent>
            <el-form-item label="级别">
              <el-select v-model="filter.severity" placeholder="全部" clearable style="width: 160px" @change="onFilter">
                <el-option v-for="o in ALERT_LEVEL_FILTER_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
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
              <el-button :icon="Monitor" @click="router.push('/alerts/screen')" title="打开大屏模式">告警大屏</el-button>
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
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="statusTagType(row.status)" effect="plain" size="small">
                  {{ statusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="title" label="告警标题" min-width="200" show-overflow-tooltip>
              <template #default="{ row }">
                {{ orNA(row.title) }}
              </template>
            </el-table-column>
            <el-table-column label="告警描述" min-width="200" show-overflow-tooltip>
              <template #default="{ row }">
                {{ alertSummary(row) }}
              </template>
            </el-table-column>
            <el-table-column label="告警IP" width="140" show-overflow-tooltip>
              <template #default="{ row }">
                {{ alertIp(row) }}
              </template>
            </el-table-column>
            <el-table-column label="主机名/资产" min-width="160" show-overflow-tooltip>
              <template #default="{ row }">
                {{ alertHostname(row) }}
              </template>
            </el-table-column>
            <el-table-column label="来源" width="110">
              <template #default="{ row }">
                <el-tag size="small" type="info" effect="plain">{{ sourceLabel(row.source) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="接入方式" min-width="160" show-overflow-tooltip>
              <template #default="{ row }">
                <el-tag size="small" :type="ingressChannelTagType(row.ingressChannel)" effect="plain">
                  {{ ingressChannelLabel(row.ingressChannel) }}
                </el-tag>
                <span class="ingress-actor">{{ orNA(row.ingressActor) }}</span>
              </template>
            </el-table-column>
            <el-table-column label="联系人" width="100" show-overflow-tooltip>
              <template #default="{ row }">
                {{ orNA(row.contactName) }}
              </template>
            </el-table-column>
            <el-table-column label="触发次数" width="80" align="center">
              <template #default="{ row }">
                <el-badge v-if="row.fireCount > 1" :value="row.fireCount" type="warning" />
                <span v-else>1</span>
              </template>
            </el-table-column>
            <el-table-column label="触发时间" width="170">
              <template #default="{ row }">{{ orNA(row.firedAt) }}</template>
            </el-table-column>
            <el-table-column label="认领人" width="100">
              <template #default="{ row }">
                {{ orNA(row.acknowledgedBy) }}
              </template>
            </el-table-column>
            <el-table-column label="操作" width="340" fixed="right">
              <template #default="{ row }">
                <el-button v-if="hasPermission('alert:update') && row.status === 'firing'" link type="primary"
                  size="small" @click.stop="ackEvent(row)">认领</el-button>
                <el-button v-if="hasPermission('alert:update') && row.status !== 'resolved'" link type="success"
                  size="small" @click.stop="openResolveDialog(row)">解决</el-button>
                <el-button v-if="hasPermission('alert:update') && row.status !== 'resolved' && row.status !== 'suppressed'" link type="warning"
                  size="small" :icon="Lock" @click.stop="suppressEvent(row)">静默</el-button>
                <el-button link type="primary" size="small" :icon="View" @click.stop="openDetail(row)">详情</el-button>
                <el-button v-if="row.ingressChannel === 'webhook' || (row.fingerprint && row.fingerprint.startsWith('eventide:'))"
                  link size="small" :icon="Open" @click.stop="openInEventide(row)">Eventide</el-button>
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

      <el-tab-pane label="维护窗口登记" name="silences">
        <el-alert type="info" :closable="false" show-icon style="margin-bottom: 12px">
          <template #title>与 Eventide 的关系</template>
          本页仅用于<b>登记维护窗口 / 变更冻结期</b>，方便排班和审计；<b>不会自动匹配静默告警</b>。
          告警静默请在事件列表中对单条告警使用「静默」按钮；Eventide 侧配置的静默规则以 Eventide 为准。
        </el-alert>
        <el-card shadow="never" class="list-card">
          <template #header>
            <div class="card-header-row">
              <span>维护窗口 / 变更冻结 登记</span>
              <el-button v-if="hasPermission('alert:update')" type="primary" :icon="Plus" size="small"
                @click="openSilenceDialog(null)">新建登记</el-button>
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

      <!-- Tab: 接入来源 -->
      <el-tab-pane label="接入来源" name="ingress">
        <!-- 渠道概览卡片 -->
        <el-row :gutter="12" class="stats-row" v-loading="ingressLoading">
          <el-col v-for="ch in ingressChannels" :key="ch.key" :xs="12" :sm="6">
            <div class="stat-card" :class="ch.cardClass">
              <div class="stat-icon"><el-icon><component :is="ch.icon" /></el-icon></div>
              <div class="stat-body">
                <div class="stat-label">{{ ch.label }}</div>
                <div class="stat-value">{{ ch.count }}</div>
                <div class="stat-sub">{{ ch.desc }}</div>
              </div>
            </div>
          </el-col>
        </el-row>

        <!-- 接入明细表格 -->
        <el-card shadow="never" class="list-card">
          <template #header>
            <div class="card-header-row">
              <span>接入者明细</span>
              <el-button :icon="Refresh" link @click="loadIngress">刷新</el-button>
            </div>
          </template>
          <el-table v-loading="ingressLoading" :data="ingressData?.items ?? []" stripe>
            <el-table-column label="接入渠道" width="130">
              <template #default="{ row }">
                <el-tag :type="ingressChannelTagType(row.ingressChannel)" effect="plain" size="small">
                  {{ ingressChannelLabel(row.ingressChannel) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="接入者" min-width="140">
              <template #default="{ row }">
                <span v-if="row.ingressActor">{{ row.ingressActor }}</span>
                <span v-else class="text-muted">—</span>
              </template>
            </el-table-column>
            <el-table-column label="告警总数" width="90" align="center">
              <template #default="{ row }">{{ row.totalCount }}</template>
            </el-table-column>
            <el-table-column label="触发中" width="80" align="center">
              <template #default="{ row }">
                <el-badge v-if="row.firingCount > 0" :value="row.firingCount" type="danger" />
                <span v-else class="text-muted">0</span>
              </template>
            </el-table-column>
            <el-table-column label="已认领" width="80" align="center">
              <template #default="{ row }">
                <span v-if="row.acknowledgedCount > 0">{{ row.acknowledgedCount }}</span>
                <span v-else class="text-muted">0</span>
              </template>
            </el-table-column>
            <el-table-column label="已解决" width="80" align="center">
              <template #default="{ row }">
                <span v-if="row.resolvedCount > 0" style="color: #67c23a">{{ row.resolvedCount }}</span>
                <span v-else class="text-muted">0</span>
              </template>
            </el-table-column>
            <el-table-column label="首次接入" width="170">
              <template #default="{ row }">{{ formatTime(row.firstFiredAt) }}</template>
            </el-table-column>
            <el-table-column label="最近接入" width="170">
              <template #default="{ row }">{{ formatTime(row.lastFiredAt) }}</template>
            </el-table-column>

            <!-- Token 信息展开行 -->
            <el-table-column v-if="hasTokenRows" type="expand" width="50">
              <template #default="{ row }">
                <div v-if="row.tokenInfo" class="token-detail">
                  <el-descriptions :column="3" border size="small">
                    <el-descriptions-item label="令牌名称">{{ row.tokenInfo.name }}</el-descriptions-item>
                    <el-descriptions-item label="角色">
                      <el-tag size="small" :type="row.tokenInfo.role === 'admin' ? 'danger' : row.tokenInfo.role === 'operator' ? 'warning' : 'info'">
                        {{ row.tokenInfo.role }}
                      </el-tag>
                    </el-descriptions-item>
                    <el-descriptions-item label="创建者">{{ row.tokenInfo.ownerName ?? '—' }}</el-descriptions-item>
                    <el-descriptions-item label="权限范围" :span="3">
                      <el-tag v-for="scope in row.tokenInfo.scopes" :key="scope" size="small" class="scope-tag">{{ scope }}</el-tag>
                      <span v-if="!row.tokenInfo.scopes.length" class="text-muted">无</span>
                    </el-descriptions-item>
                    <el-descriptions-item label="创建时间">{{ formatTime(row.tokenInfo.createdAt) }}</el-descriptions-item>
                    <el-descriptions-item label="最近使用">{{ formatTime(row.tokenInfo.lastUsedAt) }}</el-descriptions-item>
                    <el-descriptions-item label="状态">
                      <el-tag v-if="row.tokenInfo.revoked" size="small" type="danger">已吊销</el-tag>
                      <el-tag v-else-if="row.tokenInfo.expired" size="small" type="warning">已过期</el-tag>
                      <el-tag v-else size="small" type="success">有效</el-tag>
                      <span v-if="row.tokenInfo.expiresAt" class="text-muted" style="margin-left: 6px">
                        过期于 {{ formatTime(row.tokenInfo.expiresAt) }}
                      </span>
                      <span v-else class="text-muted" style="margin-left: 6px">永不过期</span>
                    </el-descriptions-item>
                  </el-descriptions>
                </div>
                <div v-else class="text-muted" style="padding: 12px">
                  此接入渠道无关联令牌信息
                </div>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <!-- Tab: 接入帮助 -->
      <el-tab-pane label="接入帮助" name="guide">
        <div class="guide-container">
          <!-- 顶部引导 -->
          <el-alert
            type="info"
            :closable="false"
            show-icon
            class="guide-intro"
          >
            <template #title>接入帮助总览</template>
            <div class="guide-intro-body">
              本页指导<b>外部系统、脚本或人工</b>如何将告警推送到 MeridianOps 告警中心。MeridianOps 支持 3 种接入方式，下方按推荐度从高到低排列，下方表格为各方式的对比概览。
              <ul class="guide-intro-list">
                <li><b>方式一 · API 令牌</b>：程序化调用，权限可裁剪、可审计、可吊销，适合作业脚本/外部 API 对接</li>
                <li><b>方式二 · Eventide Webhook</b>：共享密钥鉴权，接收 Alertmanager 风格 payload，适合 Eventide / Prometheus 等外部告警推送</li>
                <li><b>方式三 · 人工上报</b>：在本页「告警事件」Tab 点击「新建告警」直接创建，无需对接，适合值班时手动登记</li>
              </ul>
            </div>
          </el-alert>

          <!-- 接入渠道对照表（概览，置顶） -->
          <el-card shadow="never" class="guide-card">
            <template #header>
              <span class="guide-title">接入渠道概览</span>
            </template>
            <el-table :data="channelCompare" border size="small" class="guide-table">
              <el-table-column prop="channel" label="接入渠道" width="120">
                <template #default="{ row }">
                  <el-tag :type="ingressChannelTagType(row.channel)" effect="plain" size="small">
                    {{ ingressChannelLabel(row.channel) }}
                  </el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="endpoint" label="API 端点" width="240" />
              <el-table-column prop="auth" label="鉴权方式" width="140" />
              <el-table-column prop="actor" label="接入者显示" />
              <el-table-column prop="useCase" label="适用场景" />
            </el-table>
          </el-card>

          <!-- Webhook 共享密钥管理（属于方式二的鉴权配置） -->
          <el-card shadow="never" class="guide-card" v-loading="ingressCfgLoading">
            <template #header>
              <div class="guide-header">
                <el-icon><Lock /></el-icon>
                <span class="guide-title">Webhook 共享密钥</span>
                <el-tag type="primary" effect="plain" size="small">方式二专用</el-tag>
                <el-tag v-if="ingressCfg?.ingressEnabled" type="success" effect="dark" size="small">已启用</el-tag>
                <el-tag v-else type="info" effect="dark" size="small">未启用</el-tag>
                <el-tag v-if="ingressCfg?.source === 'database'" type="warning" effect="plain" size="small">数据库</el-tag>
                <el-tag v-else effect="plain" size="small">配置文件</el-tag>
              </div>
            </template>
            <el-alert type="info" :closable="false" show-icon style="margin-bottom: 12px">
              此密钥用于<b>方式二（Eventide Webhook）的鉴权</b>。修改后即时生效，无需重启服务。密钥仅明文返回一次（重新生成时），请妥善保存。
            </el-alert>

            <el-alert
              v-if="ingressCfg?.isDefault"
              type="warning"
              :closable="false"
              show-icon
              style="margin-bottom: 12px"
            >
              当前密钥仍是默认值或为空，Eventide Webhook 推送将无法鉴权通过，请点击「重新生成」设置真实密钥。
            </el-alert>

            <el-form label-width="100px" label-position="right">
              <el-form-item label="启用接入">
                <div class="switch-row">
                  <el-switch
                    v-model="ingressEnabledLocal"
                    :disabled="!hasPermission('system:update')"
                  />
                  <el-tag v-if="ingressEnabledLocal" type="success" size="small" effect="plain">接收外部推送</el-tag>
                  <el-tag v-else type="info" size="small" effect="plain">关闭（端点返回 404）</el-tag>
                  <span class="text-muted switch-tip">
                    关闭后 <code class="code-inline">/api/alerts/ingress/eventide</code> 端点返回 404
                  </span>
                </div>
              </el-form-item>

              <el-form-item label="接入密钥">
                <div class="token-row">
                  <el-input
                    v-model="ingressTokenDisplay"
                    :type="showIngressToken ? 'text' : 'password'"
                    readonly
                    placeholder="尚未配置"
                    style="width: 380px"
                  >
                    <template #append>
                      <el-button :icon="showIngressToken ? CircleClose : Connection" @click="showIngressToken = !showIngressToken" />
                    </template>
                  </el-input>
                  <el-button
                    v-if="hasPermission('system:update')"
                    type="primary"
                    :icon="RefreshRight"
                    @click="regenerateToken"
                    :loading="ingressSaving"
                  >重新生成</el-button>
                  <el-button
                    v-if="hasPermission('system:update')"
                    :icon="EditPen"
                    @click="openEditToken"
                  >自定义密钥</el-button>
                  <el-button
                    v-if="ingressCfg && !ingressCfg.isDefault"
                    :icon="CopyDocument"
                    @click="copyTokenPlaceholder"
                  >复制提示</el-button>
                </div>
                <div class="text-muted" style="font-size: 12px; margin-top: 4px">
                  当前长度 {{ ingressCfg?.tokenLength ?? 0 }} 位 · 最近更新：
                  <span v-if="ingressCfg?.updatedAt">{{ ingressCfg.updatedBy }} · {{ formatTime(ingressCfg.updatedAt) }}</span>
                  <span v-else>—</span>
                </div>
              </el-form-item>
            </el-form>

            <el-alert
              v-if="lastGeneratedToken"
              type="success"
              :closable="false"
              show-icon
              style="margin-bottom: 12px"
            >
              <div class="generated-token-box">
                <div><b>新密钥已生成（仅此一次显示）：</b></div>
                <code class="code-inline generated-token">{{ lastGeneratedToken }}</code>
                <el-button size="small" :icon="CopyDocument" @click="copyGeneratedToken">复制</el-button>
              </div>
            </el-alert>

            <div v-if="hasPermission('system:update')" class="cfg-actions">
              <el-button type="primary" @click="saveIngressEnabled" :loading="ingressSaving">保存启用状态</el-button>
            </div>
          </el-card>

          <!-- 方式一：API 令牌 -->
          <el-card shadow="never" class="guide-card">
            <template #header>
              <div class="guide-header">
                <el-tag type="danger" effect="dark" size="small">方式一</el-tag>
                <span class="guide-title">API 令牌接入</span>
                <el-tag effect="plain" size="small">POST /api/alerts/events</el-tag>
              </div>
            </template>
            <el-steps direction="vertical" :active="3" simple>
              <el-step title="创建 API 令牌" status="process">
                <template #description>
                  进入「后台管理 → API 令牌」，点击「新建令牌」，填写名称，<b>权限范围勾选 alert:create</b>，创建后复制明文令牌（仅显示一次）。
                </template>
              </el-step>
              <el-step title="调用接口" status="process">
                <template #description>
                  请求头携带 <code class="code-inline">Authorization: Bearer mk-你的令牌</code>
                </template>
              </el-step>
              <el-step title="查看接入效果" status="success">
                <template #description>
                  在「接入来源」Tab 可看到渠道=<b>API 令牌</b>，接入者=<b>令牌名称</b>。
                </template>
              </el-step>
            </el-steps>
            <el-divider content-position="left">请求示例</el-divider>
            <pre class="code-block">curl -X POST http://&lt;服务器地址&gt;:8000/api/alerts/events \
  -H "Authorization: Bearer mk-&lt;你的令牌明文&gt;" \
  -H "Content-Type: application/json" \
  -d '{
    "source": "manual",
    "severity": "5",
    "title": "核心数据库 CPU 100%",
    "message": "/data 使用率 95%，需要立即处理",
    "ci_name_snapshot": "核心数据库-01"
  }'</pre>
            <el-divider content-position="left">请求参数说明</el-divider>
            <el-table :data="apiTokenFields" border size="small" class="guide-table">
              <el-table-column prop="field" label="字段" width="120" />
              <el-table-column prop="required" label="必填" width="60" align="center">
                <template #default="{ row }">
                  <el-tag :type="row.required ? 'danger' : 'info'" size="small">{{ row.required ? '是' : '否' }}</el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="desc" label="说明" />
              <el-table-column prop="example" label="示例" width="180" />
            </el-table>
          </el-card>

          <!-- 方式二：Eventide Webhook -->
          <el-card shadow="never" class="guide-card">
            <template #header>
              <div class="guide-header">
                <el-tag type="primary" effect="dark" size="small">方式二</el-tag>
                <span class="guide-title">Eventide Webhook 接入</span>
                <el-tag effect="plain" size="small">POST /api/alerts/ingress/eventide</el-tag>
              </div>
            </template>
            <el-alert type="info" :closable="false" show-icon style="margin-bottom: 12px">
              使用上方「Webhook 共享密钥」面板中的密钥进行 Bearer Token 鉴权，接收 Alertmanager 风格 payload（labels/annotations/startsAt/fingerprint），支持自动状态转换和去重。适合 Eventide / Alertmanager / Prometheus Alertmanager 等外部告警推送。
            </el-alert>
            <el-steps direction="vertical" :active="3" simple>
              <el-step title="获取密钥并启用" status="process">
                <template #description>
                  滚动到上方「Webhook 共享密钥」面板：打开「启用接入」开关 → 点击「重新生成」获取明文密钥（仅显示一次，请立即保存）。修改即时生效，无需重启。
                </template>
              </el-step>
              <el-step title="调用接口" status="process">
                <template #description>
                  请求头携带 <code class="code-inline">Authorization: Bearer &lt;你的接入密钥&gt;</code>
                </template>
              </el-step>
              <el-step title="查看接入效果" status="success">
                <template #description>
                  在「接入来源」Tab 可看到渠道=<b>Webhook 推送</b>，接入者=<b>Eventide/来源名</b>。
                </template>
              </el-step>
            </el-steps>
            <el-divider content-position="left">请求示例</el-divider>
            <pre class="code-block">curl -X POST http://&lt;服务器地址&gt;:8000/api/alerts/ingress/eventide \
  -H "Authorization: Bearer &lt;你的接入密钥&gt;" \
  -H "Content-Type: application/json" \
  -d '{
    "severity": "5",
    "labels": {
      "alertname": "磁盘空间不足",
      "source": "zabbix",
      "ip": "10.0.1.100"
    },
    "annotations": {
      "summary": "磁盘 /data 使用率 95%",
      "description": "主机 10.0.1.100 磁盘即将满"
    },
    "startsAt": "2026-08-18T10:00:00Z"
  }'</pre>
            <el-divider content-position="left">关键字段说明</el-divider>
            <el-table :data="webhookFields" border size="small" class="guide-table">
              <el-table-column prop="field" label="字段" width="120" />
              <el-table-column prop="required" label="必填" width="60" align="center">
                <template #default="{ row }">
                  <el-tag :type="row.required ? 'danger' : 'info'" size="small">{{ row.required ? '是' : '否' }}</el-tag>
                </template>
              </el-table-column>
              <el-table-column prop="desc" label="说明" />
              <el-table-column prop="example" label="示例" width="180" />
            </el-table>
          </el-card>

          <!-- 方式三：人工上报 -->
          <el-card shadow="never" class="guide-card">
            <template #header>
              <div class="guide-header">
                <el-tag type="success" effect="dark" size="small">方式三</el-tag>
                <span class="guide-title">人工上报</span>
                <el-tag effect="plain" size="small">前端页面操作</el-tag>
            </div>
            </template>
            <el-alert type="info" :closable="false" show-icon style="margin-bottom: 12px">
              无需对接 API 或密钥，使用已登录用户的 JWT 鉴权，直接在告警中心页面创建。适合值班人员手动登记非自动化的告警或事件备注。
            </el-alert>
            <el-steps direction="vertical" :active="2" simple>
              <el-step title="切换到「告警事件」Tab" status="process">
                <template #description>
                  点击本页顶部的「告警事件」Tab，再点击右上角「新建告警」按钮。
                </template>
              </el-step>
              <el-step title="填写告警表单" status="process">
                <template #description>
                  填写级别（5 灾难 / 4 重要 / 3 一般 / 2 警告 / 1 信息 / 0 未分类）、标题、详情、关联资产（可选）后提交。提交后该告警的接入渠道自动标记为<b>人工上报</b>，接入者显示为<b>当前登录用户名</b>。
                </template>
              </el-step>
            </el-steps>
            <el-alert type="warning" :closable="false" show-icon>
              人工上报需要 <code class="code-inline">alert:create</code> 权限，通常分配给 operator/admin 角色。
            </el-alert>
          </el-card>
        </div>
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
          <el-descriptions-item label="接入渠道">
            <el-tag :type="ingressChannelTagType(detail.ingressChannel)" effect="plain" size="small">
              {{ ingressChannelLabel(detail.ingressChannel) }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="接入者">
            <span v-if="detail.ingressActor">{{ detail.ingressActor }}</span>
            <span v-else class="text-muted">—</span>
          </el-descriptions-item>
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

        <div class="detail-actions" style="display:flex;gap:8px;flex-wrap:wrap">
          <template v-if="hasPermission('alert:update') && detail.status !== 'resolved'">
            <el-button v-if="detail.status === 'firing'" type="primary" :icon="Check" @click="ackEvent(detail, true)">认领告警</el-button>
            <el-button type="success" :icon="CircleCheck" @click="openResolveDialog(detail)">解决告警</el-button>
            <el-button v-if="detail.status !== 'suppressed'" type="warning" :icon="Lock" @click="suppressEvent(detail)">手动静默</el-button>
            <el-button :icon="EditPen" @click="openNoteDialog(detail)">更新备注</el-button>
          </template>
          <el-button v-if="detail.ingressChannel === 'webhook' || (detail.fingerprint && detail.fingerprint.startsWith('eventide:'))"
            :icon="Open" @click.stop="openInEventide(detail)">在 Eventide 查看</el-button>
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
            <el-option v-for="o in ALERT_LEVEL_OPTIONS" :key="o.value" :label="o.label" :value="o.value">
              <div style="display:flex;align-items:center;gap:8px">
                <span :style="{
                  display:'inline-block',width:10,height:10,borderRadius:'50%',
                  background: ALERT_LEVEL_META[o.value as keyof typeof ALERT_LEVEL_META].color,
                }" />
                <span>{{ o.label }}</span>
              </div>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item label="标题" prop="title">
          <el-input v-model="createForm.title" placeholder="如：核心数据库 CPU 100%" />
        </el-form-item>
        <el-form-item label="详情" prop="message">
          <el-input v-model="createForm.message" type="textarea" :rows="3" placeholder="告警详情描述" />
        </el-form-item>
        <el-form-item label="关联资产">
          <el-select v-model="createForm.ci_id" filterable remote reserve-keyword
            placeholder="按名称 / IP / 资产编号搜索，留空可手动填写名称"
            style="width: 100%"
            :remote-method="searchCi"
            :loading="ciSearchLoading"
            :no-match-text="createForm.ci_name_snapshot ? '使用下方填写的名称' : '未找到资产，可在下方手动填名称'"
            @change="onCiSelected">
            <el-option v-for="ci in ciCandidates" :key="ci.id" :label="formatCiOption(ci)" :value="ci.id">
              <span style="float:left">{{ ci.name }}</span>
              <span style="float:right;color:var(--el-text-color-secondary);font-size:12px">
                <span v-if="(ci.attributes as any)?.model_name">【{{ (ci.attributes as any).model_name }}】</span>
                <span v-else-if="(ci.attributes as any)?.model">【{{ (ci.attributes as any).model }}】</span>
                <span v-if="(ci.attributes as any)?.ip">{{ (ci.attributes as any).ip }}</span>
                <span v-else-if="(ci.attributes as any)?.ip_addresses?.[0]">{{ (ci.attributes as any).ip_addresses[0] }}</span>
                <span v-else-if="(ci.attributes as any)?.asset_no">{{ (ci.attributes as any).asset_no }}</span>
              </span>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item label="资产名称">
          <el-input v-model="createForm.ci_name_snapshot" placeholder="手动填写资产名称（可选，未选 CMDB 时使用）" />
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
            <div class="match-tip">常用字段：source / severity / ciId；severity 使用 0-5 数字（Zabbix 体系），也可填 P0,P5 等别名（自动归一化）</div>
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
import { computed, onMounted, reactive, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, ElMessageBox, type FormInstance } from 'element-plus'
import {
  Bell, Warning, CircleClose, Plus, Search, Refresh, Check, CircleCheck,
  EditPen, Delete, Connection, Key, DataLine, Monitor, CopyDocument, RefreshRight, Lock,
  Open, View,
} from '@element-plus/icons-vue'
import { useUserStore } from '../../stores/user'
import {
  listAlertEvents, getAlertEvent, createAlertEvent, acknowledgeAlert, resolveAlert, suppressAlert,
  updateAlertNote, deleteAlertEvent, getAlertStats,
  listAlertSilences, createAlertSilence, updateAlertSilence, deleteAlertSilence,
  fetchIngressOverview, getAlertIngress, updateAlertIngress,
  type AlertEvent, type AlertStats, type AlertSilence, type IngressOverview,
  type AlertIngressConfig,
} from '../../api/alert'
import { listCiInstances } from '../../api/cmdb'
import type { CiInstance } from '../../api/types'
import {
  ALERT_LEVEL_OPTIONS,
  ALERT_LEVEL_FILTER_OPTIONS,
  ALERT_LEVEL_META,
  ALERT_LEVEL_ORDER,
  alertLevelColor,
  alertLevelName,
  alertLevelShortName,
  alertLevelTagType,
  normalizeAlertLevel,
} from '../../utils/alert-level'

const router = useRouter()
const userStore = useUserStore()
const hasPermission = (perm: string) => userStore.hasPermission(perm)

/** Eventide 外部链接 base：默认回退 mock 中的值，可通过 VITE_EVENTIDE_BASE_URL 覆盖 */
const EVENTIDE_BASE_URL = (import.meta as unknown as { env?: Record<string, string> }).env?.VITE_EVENTIDE_BASE_URL
  ?? 'http://eventide:8080'

/** 拼接 Eventide 告警详情页链接。拿不到 fingerprint 时跳告警中心首页。 */
function eventideAlertUrl(row: { fingerprint?: string | null }) {
  const fp = row.fingerprint?.replace(/^eventide:/, '')
  if (!fp) return `${EVENTIDE_BASE_URL}/alerts`
  // Eventide 的路由按实际部署可能调整，这里给一个保守实现：跳搜索页
  return `${EVENTIDE_BASE_URL}/alerts?fingerprint=${encodeURIComponent(fp)}`
}

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
  severity: '2' as string,
  title: '',
  message: '',
  ci_id: '' as string,
  ci_name_snapshot: '' as string,
})
const createRules = {
  title: [{ required: true, message: '请输入告警标题', trigger: 'blur' }],
  severity: [{ required: true, message: '请选择级别', trigger: 'change' }],
  source: [{ required: true, message: '请选择来源', trigger: 'change' }],
}

/** 新建告警里 CMDB 关联资产选择 */
const ciSearchLoading = ref(false)
const ciCandidates = ref<CiInstance[]>([])
let ciSearchTimer: ReturnType<typeof setTimeout> | null = null
async function searchCi(keyword: string) {
  if (ciSearchTimer) clearTimeout(ciSearchTimer)
  const kw = keyword?.trim()
  ciSearchTimer = setTimeout(async () => {
    ciSearchLoading.value = true
    try {
      const res = await listCiInstances({ page: 1, pageSize: 20, keyword: kw || undefined })
      ciCandidates.value = res.items ?? []
    } catch (e: unknown) {
      ciCandidates.value = []
    } finally {
      ciSearchLoading.value = false
    }
  }, 250)
}
function onCiSelected(ciId: string) {
  const ci = ciCandidates.value.find(c => c.id === ciId)
  if (ci) {
    createForm.ci_id = ci.id
    if (!createForm.ci_name_snapshot) {
      createForm.ci_name_snapshot = ci.name
    }
  }
}
function formatCiOption(ci: CiInstance) {
  const attr = (ci.attributes ?? {}) as Record<string, unknown>
  const extras: string[] = []
  const modelName = typeof attr.model_name === 'string' ? attr.model_name : (typeof attr.model === 'string' ? attr.model : undefined)
  if (modelName) extras.push(`【${modelName}】`)
  const ip = typeof attr.ip === 'string' ? attr.ip
    : Array.isArray(attr.ip_addresses) && attr.ip_addresses.length ? String(attr.ip_addresses[0])
    : undefined
  if (ip) extras.push(ip)
  else {
    const assetNo = typeof attr.asset_no === 'string' ? attr.asset_no
      : typeof attr.serial_number === 'string' ? attr.serial_number
      : undefined
    if (assetNo) extras.push(assetNo)
  }
  return `${ci.name}${extras.length ? ' ' + extras.join(' ') : ''}`
}

function openCreateDialog() {
  createForm.source = 'manual'
  createForm.severity = '2'
  createForm.title = ''
  createForm.message = ''
  createForm.ci_id = ''
  createForm.ci_name_snapshot = ''
  ciCandidates.value = []
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
    if (createForm.ci_id) payload.ci_id = createForm.ci_id
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

// ============ 手动静默（值班压制单条告警） ============
async function suppressEvent(row: AlertEvent | null) {
  if (!row) return
  try {
    await ElMessageBox.confirm(
      `确认将告警「${row.title}」标记为静默？标记后仍保留记录，但不计入活跃统计。建议同时在 Eventide 侧确认处置方案。`,
      '手动标记静默',
      { type: 'warning', confirmButtonText: '确认静默', cancelButtonText: '取消' },
    )
  } catch {
    return
  }
  try {
    await suppressAlert(row.id)
    ElMessage.success('已标记为静默')
    if (detail.value && detail.value.id === row.id) {
      detail.value = { ...detail.value, status: 'suppressed', suppressedBy: userStore.user?.username ?? '', suppressedAt: new Date().toISOString() }
    }
    loadEvents()
    loadStats()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  }
}

// ============ 跳转到 Eventide ============
function openInEventide(row: { fingerprint?: string | null; ingressChannel?: string | null }) {
  const url = eventideAlertUrl(row as AlertEvent)
  window.open(url, '_blank', 'noopener,noreferrer')
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

// ============ 接入帮助 ============
const apiTokenFields = [
  { field: 'source', required: false, desc: '告警来源（manual/zabbix/prometheus/job/system）', example: 'manual' },
  { field: 'severity', required: true, desc: '告警级别（Zabbix 体系）：0 未分类 / 1 信息 / 2 警告 / 3 一般 / 4 重要 / 5 灾难（也接受 P0-P5 / disaster / critical 等，会自动归一化）', example: '5' },
  { field: 'title', required: true, desc: '告警标题', example: 'CPU 100%' },
  { field: 'message', required: false, desc: '告警详情描述', example: '使用率 95%' },
  { field: 'ci_id', required: false, desc: '关联 CMDB 资产 ID（UUID）', example: 'abc-123' },
  { field: 'ci_name_snapshot', required: false, desc: '关联资产名称快照（无需 ci_id 时可用）', example: '核心数据库-01' },
  { field: 'labels', required: false, desc: '标签 JSON（含 metric 字段可参与去重）', example: '{"metric":"cpu"}' },
  { field: 'fired_at', required: false, desc: '触发时间（RFC3339），缺省取当前', example: '2026-08-18T10:00:00Z' },
]

const webhookFields = [
  { field: 'severity', required: false, desc: '告警级别：0-5 数字（Zabbix 体系），也接受 critical / warning 等英文，自动归一化', example: '5' },
  { field: 'labels', required: true, desc: '标签 JSON，alertname 为告警名，source 为来源，ip 用于关联 CMDB', example: '{"alertname":"磁盘满","source":"zabbix","ip":"10.0.1.100"}' },
  { field: 'annotations', required: false, desc: '注解 JSON，summary/description 作为告警详情', example: '{"summary":"磁盘 95%"}' },
  { field: 'startsAt', required: false, desc: '触发时间（RFC3339）', example: '2026-08-18T10:00:00Z' },
  { field: 'endsAt', required: false, desc: '结束时间（有此字段自动标记已解决）', example: '2026-08-18T11:00:00Z' },
  { field: 'transition', required: false, desc: '状态转换（became_resolved/became_acknowledged）', example: 'became_resolved' },
  { field: 'fingerprint', required: false, desc: '去重指纹（缺省由系统计算）', example: 'a1b2c3d4e5f6a7b8' },
  { field: 'tally', required: false, desc: '累计触发次数（Eventide 批量推送时使用）', example: '5' },
]

const channelCompare = [
  { channel: 'webhook', endpoint: 'POST /api/alerts/ingress/eventide', auth: '共享 Bearer Token', actor: 'Eventide/来源名', useCase: 'Eventide / Alertmanager 外部推送' },
  { channel: 'manual', endpoint: 'POST /api/alerts/events', auth: 'JWT 登录', actor: '操作用户名', useCase: '前端页面人工创建告警' },
  { channel: 'api_token', endpoint: 'POST /api/alerts/events', auth: 'API 令牌（mk- 前缀）', actor: '令牌名称', useCase: '外部程序/API 调用创建告警' },
]

// ============ 接入来源 ============
const ingressLoading = ref(false)
const ingressData = ref<IngressOverview | null>(null)

async function loadIngress() {
  ingressLoading.value = true
  try {
    ingressData.value = await fetchIngressOverview()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    ingressLoading.value = false
  }
}

const ingressChannels = computed(() => {
  const summary = ingressData.value?.channelSummary ?? {}
  const channels = [
    { key: 'webhook', label: 'Webhook 推送', desc: 'Eventide 等外部推送', icon: Connection, cardClass: 'stat-webhook' },
    { key: 'manual', label: '人工上报', desc: '前端操作创建', icon: Bell, cardClass: 'stat-manual' },
    { key: 'api_token', label: 'API 令牌', desc: '程序化 API 调用', icon: Key, cardClass: 'stat-token' },
    { key: 'job', label: '作业执行', desc: '作业触发告警', icon: DataLine, cardClass: 'stat-job' },
    { key: 'system', label: '系统内置', desc: '系统自动生成', icon: Monitor, cardClass: 'stat-system' },
  ]
  return channels.map(ch => ({
    ...ch,
    count: summary[ch.key] ?? 0,
  }))
})

const hasTokenRows = computed(() => {
  return (ingressData.value?.items ?? []).some(item => item.ingressChannel === 'api_token')
})

// 切换 Tab 时按需加载
watch(activeTab, (tab) => {
  if (tab === 'ingress' && !ingressData.value) {
    loadIngress()
  }
  if (tab === 'guide' && !ingressCfg.value) {
    loadIngressCfg()
  }
})

// ============ 告警接入配置（共享密钥管理） ============
const ingressCfgLoading = ref(false)
const ingressCfg = ref<AlertIngressConfig | null>(null)
const ingressEnabledLocal = ref(false)
const showIngressToken = ref(false)
const ingressSaving = ref(false)
const lastGeneratedToken = ref('')

const ingressTokenDisplay = computed(() => {
  if (showIngressToken.value && lastGeneratedToken.value) return lastGeneratedToken.value
  return ingressCfg.value?.ingressTokenMasked ?? ''
})

async function loadIngressCfg() {
  ingressCfgLoading.value = true
  try {
    const cfg = await getAlertIngress()
    ingressCfg.value = cfg
    ingressEnabledLocal.value = cfg.ingressEnabled
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    ingressCfgLoading.value = false
  }
}

async function saveIngressEnabled() {
  if (!hasPermission('system:update')) {
    ElMessage.error('无 system:update 权限')
    return
  }
  ingressSaving.value = true
  try {
    await updateAlertIngress({ ingressEnabled: ingressEnabledLocal.value })
    ElMessage.success('启用状态已保存')
    await loadIngressCfg()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    ingressSaving.value = false
  }
}

async function regenerateToken() {
  try {
    await ElMessageBox.confirm(
      '重新生成密钥后，原密钥立即失效。所有正在使用旧密钥的外部推送方需更新配置。是否继续？',
      '确认重新生成密钥',
      { type: 'warning' }
    )
  } catch {
    return
  }
  ingressSaving.value = true
  try {
    const resp = await updateAlertIngress({ regenerate: true })
    if (resp.ingressToken) {
      lastGeneratedToken.value = resp.ingressToken
      showIngressToken.value = true
      ElMessage.success('新密钥已生成（仅此一次显示，请立即复制保存）')
    }
    await loadIngressCfg()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    ingressSaving.value = false
  }
}

async function openEditToken() {
  let inputRef: { value: string } = { value: '' }
  try {
    const { value } = await ElMessageBox.prompt('请输入自定义密钥（至少 8 位，不能以 change-me 开头）', '自定义密钥', {
      inputType: 'password',
      inputPlaceholder: '至少 8 位',
      inputValidator: (v: string) => {
        if (!v) return '密钥不能为空'
        if (v.length < 8) return '密钥长度至少 8 位'
        if (v.startsWith('change-me')) return '密钥不能以 change-me 开头'
        return true
      },
    })
    inputRef.value = value
  } catch {
    return
  }
  ingressSaving.value = true
  try {
    await updateAlertIngress({ ingressToken: inputRef.value })
    lastGeneratedToken.value = inputRef.value
    showIngressToken.value = true
    ElMessage.success('密钥已更新')
    await loadIngressCfg()
  } catch (e: unknown) {
    ElMessage.error(errMsg(e))
  } finally {
    ingressSaving.value = false
  }
}

function copyGeneratedToken() {
  if (!lastGeneratedToken.value) return
  navigator.clipboard.writeText(lastGeneratedToken.value).then(() => {
    ElMessage.success('密钥已复制到剪贴板')
  }).catch(() => {
    ElMessage.warning('复制失败，请手动选择复制')
  })
}

function copyTokenPlaceholder() {
  // 提供给外部对接方的占位提示（不包含真实密钥，仅是配置说明）
  const text = `MeridianOps 告警接入 - Webhook 端点
端点: POST http://{服务器地址}:8000/api/alerts/ingress/eventide
鉴权: Authorization: Bearer <你的接入密钥>
密钥获取方式: 联系系统管理员在「告警中心 → 接入帮助 → 告警接入配置」中查看或重新生成`
  navigator.clipboard.writeText(text).then(() => {
    ElMessage.success('对接说明已复制，可发送给外部对接方')
  }).catch(() => {
    ElMessage.warning('复制失败')
  })
}

// ============ 工具函数 ============
function errMsg(e: unknown): string {
  if (e && typeof e === 'object' && 'message' in e) return (e as { message: string }).message
  return String(e)
}

function severityTagType(s: string | null | undefined): 'danger' | 'warning' | 'info' | 'success' | 'primary' {
  return alertLevelTagType(s)
}
function severityLabel(s: string | null | undefined): string {
  // 格式：数字 + 短名，紧凑 → "5 灾难" "0 未分类"
  const lvl = normalizeAlertLevel(s)
  return `${lvl} ${alertLevelShortName(lvl)}`
}

/** 从 labels JSON 提取告警 IP */
function alertIp(row: AlertEvent): string {
  if (!row.labels) return 'N/A'
  const lbl = row.labels as Record<string, unknown>
  const raw = lbl.alertIp || lbl.ip || lbl.instance || lbl.host_ip || lbl.manageIp || lbl.alert_ip || lbl.target_ip || lbl.src_ip
  if (typeof raw !== 'string' || !raw) return 'N/A'
  // instance 格式可能是 host:port，只取 host 部分
  return raw.includes(':') ? raw.split(':')[0] : raw
}

/** 从 labels JSON 提取主机名 */
function alertHostname(row: AlertEvent): string {
  if (row.ciName) return row.ciName
  if (!row.labels) return 'N/A'
  const lbl = row.labels as Record<string, unknown>
  const raw = lbl.hostname || lbl.host || lbl.host_name || lbl.hostName || lbl.trap_hosts || lbl.target || lbl.name
  return typeof raw === 'string' && raw ? raw : 'N/A'
}

/** 从 labels JSON 提取告警项名称 */
function alertName(row: AlertEvent): string {
  if (!row.labels) return 'N/A'
  const lbl = row.labels as Record<string, unknown>
  const raw = lbl.alertname || lbl.alert_name || lbl.alertName || lbl.rule || lbl.rule_name
  return typeof raw === 'string' && raw ? raw : 'N/A'
}

/** 从 labels/annotations 提取告警摘要 */
function alertSummary(row: AlertEvent): string {
  if (row.message) return row.message
  if (!row.labels) return 'N/A'
  const lbl = row.labels as Record<string, unknown>
  const raw = lbl.summary || lbl.description || lbl.detail || lbl.hint || lbl.annotations
  return typeof raw === 'string' && raw ? raw : 'N/A'
}

/** 安全取值：有值显示值，无值显示 N/A */
function orNA(val: string | null | undefined): string {
  return val && val.trim() ? val : 'N/A'
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

function ingressChannelLabel(s: string): string {
  const map: Record<string, string> = {
    webhook: 'Webhook 推送',
    manual: '人工上报',
    job: '作业执行',
    api_token: 'API 令牌',
    system: '系统内置',
  }
  return map[s] ?? s
}

function ingressChannelTagType(s: string): 'success' | 'warning' | 'primary' | 'info' | 'danger' {
  if (s === 'webhook') return 'primary'
  if (s === 'manual') return 'success'
  if (s === 'job') return 'warning'
  if (s === 'api_token') return 'danger'
  return 'info'
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
.stats-row-8cols .el-col { margin-bottom: 12px; }
.stat-card {
  display: flex; align-items: center; gap: 12px;
  padding: 14px 16px; border-radius: 8px; background: #fff;
  border-left: 4px solid #dcdfe6; box-shadow: 0 1px 3px rgba(0, 0, 0, 0.04);
  transition: transform 0.12s ease, box-shadow 0.12s ease;
}
.stat-card:hover { transform: translateY(-1px); box-shadow: 0 4px 10px rgba(0, 0, 0, 0.06); }
.stat-icon {
  width: 44px; height: 44px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  background: #f4f4f5; color: #909399; font-size: 20px;
}
.stat-active { border-left-color: #f56c6c; }
.stat-active .stat-icon { background: #fef0f0; color: #f56c6c; }
.stat-today { border-left-color: #409eff; }
.stat-today .stat-icon { background: #ecf5ff; color: #409eff; }

/* 0-5 级别卡片（Zabbix 体系：0=最低灰，5=最高深红） */
.stat-level-0 { border-left-color: #64748b; }
.stat-level-1 { border-left-color: #0ea5e9; }
.stat-level-2 { border-left-color: #ca8a04; }
.stat-level-3 { border-left-color: #ea580c; }
.stat-level-4 { border-left-color: #dc2626; }
.stat-level-5 { border-left-color: #7f1d1d; }
.stat-level-0 .stat-icon { background: rgba(100, 116, 139, 0.1); color: #64748b; }
.stat-level-1 .stat-icon { background: rgba(14, 165, 233, 0.1); color: #0ea5e9; }
.stat-level-2 .stat-icon { background: rgba(202, 138, 4, 0.1); color: #ca8a04; }
.stat-level-3 .stat-icon { background: rgba(234, 88, 12, 0.1); color: #ea580c; }
.stat-level-4 .stat-icon { background: rgba(220, 38, 38, 0.1); color: #dc2626; }
.stat-level-5 .stat-icon { background: rgba(127, 29, 29, 0.1); color: #7f1d1d; }

.stat-label { font-size: 13px; color: #606266; font-weight: 500; }
.stat-value { font-size: 24px; font-weight: 700; color: #1f2937; line-height: 1.2; margin: 2px 0; }
.stat-sub { font-size: 12px; color: #909399; }

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
.ingress-actor {
  margin-left: 6px;
  font-size: 12px;
  color: #606266;
}
:deep(.clickable-row) { cursor: pointer; }
:deep(.clickable-row:hover) { background: #f5f7fa !important; }

/* 接入来源卡片 */
.stat-webhook { border-left-color: #409eff; }
.stat-webhook .stat-icon { background: #ecf5ff; color: #409eff; }
.stat-manual { border-left-color: #67c23a; }
.stat-manual .stat-icon { background: #f0f9eb; color: #67c23a; }
.stat-token { border-left-color: #f56c6c; }
.stat-token .stat-icon { background: #fef0f0; color: #f56c6c; }
.stat-job { border-left-color: #e6a23c; }
.stat-job .stat-icon { background: #fdf6ec; color: #e6a23c; }
.stat-system { border-left-color: #909399; }
.stat-system .stat-icon { background: #f4f4f5; color: #909399; }

/* Token 详情展开区 */
.token-detail { padding: 12px 16px; background: #fafafa; }
.scope-tag { margin: 2px; }

/* 接入帮助 */
.guide-container { max-width: 960px; }
.guide-card { margin-bottom: 16px; }
.guide-header { display: flex; align-items: center; gap: 8px; }
.guide-title { font-size: 15px; font-weight: 600; color: #303133; }
.guide-intro { margin-bottom: 16px; }
.guide-intro-body { font-size: 13px; line-height: 1.7; color: #606266; }
.guide-intro-list { margin: 8px 0 0 0; padding-left: 20px; }
.guide-intro-list li { margin-bottom: 4px; }
.code-inline {
  background: #f5f7fa; padding: 2px 6px; border-radius: 3px;
  font-family: 'Cascadia Code', Consolas, monospace; font-size: 12px; color: #e6a23c;
}
.code-block {
  background: #1e1e1e; color: #d4d4d4; padding: 14px 16px; border-radius: 6px;
  font-family: 'Cascadia Code', Consolas, monospace; font-size: 13px;
  line-height: 1.6; overflow-x: auto; white-space: pre; margin: 0;
}
.guide-table { margin-top: 4px; }

/* 告警接入配置 */
.token-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.switch-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.switch-tip {
  font-size: 12px;
  line-height: 1.5;
}
.cfg-actions {
  margin-top: 8px;
  display: flex;
  justify-content: flex-end;
}
.generated-token-box {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
.generated-token {
  background: #f0f9eb;
  color: #67c23a;
  padding: 6px 10px;
  border-radius: 4px;
  font-size: 13px;
  word-break: break-all;
  flex: 1;
  min-width: 200px;
}
.text-muted { color: #909399; }
</style>
