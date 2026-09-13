<template>
  <div class="sync-page">
    <!-- 顶部：数据源卡片 -->
    <div class="page-header">
      <div class="header-title">
        <h2>数据源同步</h2>
        <span class="header-desc">管理外部数据源拉取，以及向 Eventide / 优云 / AxleOps / 理想自动化推送主机投影</span>
      </div>
      <div class="header-actions">
        <el-button v-if="hasPermission('system:update')" type="primary" :icon="Plus" @click="openCreate">
          新增数据源
        </el-button>
        <el-button :icon="Refresh" circle @click="refreshAll" />
      </div>
    </div>

    <div class="section-head">
      <h3>同步通道</h3>
      <span class="section-desc">HTTP 推送到 Eventide 或其它平台，也可从外部 CMDB 拉取资产</span>
    </div>
    <div v-loading="sourcesLoading" class="sources-row">
      <div class="source-card eventide-card">
        <div class="source-head">
          <div class="source-name">
            <el-icon :size="22" :color="lookupStatus?.enabled ? '#67c23a' : '#909399'">
              <Promotion />
            </el-icon>
            <span>Eventide 外表同步</span>
          </div>
          <div class="source-tags">
            <el-tag size="small" type="success" effect="plain">推送</el-tag>
            <el-tag v-if="lookupStatus?.enabled" size="small" type="primary">定时 {{ lookupStatus.intervalSecs }}s</el-tag>
            <el-tag v-else size="small" type="info">未启用</el-tag>
          </div>
        </div>
        <div class="source-meta">
          <div class="meta-row">
            <span class="meta-label">目标</span>
            <span class="meta-value">{{ lookupStatus?.baseUrl || '—' }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">外表</span>
            <span class="meta-value">{{ lookupTargetText }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">Token</span>
            <span class="meta-value">{{ lookupStatus?.tokenSet ? (lookupStatus.tokenMasked || '已配置') : '未配置' }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">最后同步</span>
            <span class="meta-value">
              <template v-if="lookupStatus?.last">
                <el-tag size="small" :type="lookupStatus.last.ok ? 'success' : 'danger'">
                  {{ lookupStatus.last.ok ? '成功' : '失败' }}
                </el-tag>
                <span class="sync-time">
                  {{ formatTime(lookupStatus.last.finishedAt) }}
                  · {{ lastLookupText }}
                </span>
              </template>
              <span v-else class="text-muted">从未同步</span>
            </span>
          </div>
        </div>
        <div class="source-actions">
          <el-button
            v-if="hasPermission('asset:create')"
            type="primary"
            size="small"
            :loading="lookupPushing"
            :disabled="!lookupStatus?.enabled || !lookupStatus?.configured"
            @click="onPushLookup"
          >
            <el-icon><Upload /></el-icon>&nbsp;立即推送
          </el-button>
          <el-button size="small" :loading="previewLoading" @click="openPreview">
            查看内容
          </el-button>
          <el-button
            v-if="hasPermission('system:update')"
            size="small"
            @click="openLookupConfig"
          >
            <el-icon><Setting /></el-icon>&nbsp;配置
          </el-button>
          <el-button size="small" link @click="viewLogs('eventide_lookup')">
            <el-icon><Document /></el-icon>&nbsp;日志
          </el-button>
        </div>
      </div>
      <div
        v-for="src in sources"
        :key="src.code"
        class="source-card"
        :class="{ 'eventide-card': isOutbound(src) }"
      >
        <div class="source-head">
          <div class="source-name">
            <el-icon :size="22" :color="src.enabled ? '#67c23a' : '#909399'">
              <component :is="sourceIcon(src)" />
            </el-icon>
            <span>{{ src.name }}</span>
          </div>
          <div class="source-tags">
            <el-tag size="small" :type="src.sourceType === 'pull' ? 'warning' : 'success'" effect="plain">
              {{ sourceTypeLabel(src) }}
            </el-tag>
            <el-tag v-if="isSample(src)" size="small" type="warning" effect="plain">样例</el-tag>
            <el-tag v-if="!src.enabled" size="small" type="info">已禁用</el-tag>
            <el-tag v-if="src.pullEnabled && !isOutbound(src)" size="small" type="primary">
              定时 {{ pullInterval(src) }}s
            </el-tag>
            <el-tag v-if="isOutbound(src) && outboundScheduleEnabled(src)" size="small" type="primary">
              定时 {{ outboundInterval(src) }}s
            </el-tag>
          </div>
        </div>

        <div class="source-meta">
          <div class="meta-row">
            <span class="meta-label">{{ isOutbound(src) ? '目标' : 'API 地址' }}</span>
            <span class="meta-value">{{ src.apiUrl || '—' }}</span>
          </div>
          <div v-if="isOutbound(src)" class="meta-row">
            <span class="meta-label">外表</span>
            <span class="meta-value">{{ outboundTargetText(src) }}</span>
          </div>
          <div v-else class="meta-row">
            <span class="meta-label">拉取路径</span>
            <span class="meta-value">{{ pullPath(src) || '—' }}</span>
          </div>
          <div v-if="isOutbound(src)" class="meta-row">
            <span class="meta-label">定时间隔</span>
            <span class="meta-value">{{ outboundScheduleEnabled(src) ? `${outboundInterval(src)} 秒` : '未启用' }}</span>
          </div>
          <div v-else class="meta-row">
            <span class="meta-label">定时间隔</span>
            <span class="meta-value">{{ src.pullEnabled ? `${pullInterval(src)} 秒` : '未启用' }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">最后同步</span>
            <span class="meta-value">
              <el-tag v-if="src.lastSyncStatus" size="small" :type="syncStatusType(src.lastSyncStatus)">
                {{ syncStatusLabel(src.lastSyncStatus) }}
              </el-tag>
              <span v-if="src.lastSyncAt" class="sync-time">{{ formatTime(src.lastSyncAt) }}（{{ src.lastSyncCount }} 条）</span>
              <span v-else class="text-muted">从未同步</span>
            </span>
          </div>
        </div>

        <div class="source-actions">
          <el-button
            v-if="isOutbound(src) && hasPermission('asset:create')"
            type="primary"
            size="small"
            :loading="pushingCode === src.code"
            :disabled="!src.enabled || !src.apiUrl"
            @click="onPushOut(src)"
          >
            <el-icon><Upload /></el-icon>&nbsp;立即推送
          </el-button>
          <el-button
            v-else-if="hasPermission('asset:create')"
            type="primary"
            size="small"
            :loading="pullingCode === src.code"
            :disabled="!src.enabled || !src.apiUrl"
            @click="onPull(src)"
          >
            <el-icon><Download /></el-icon>&nbsp;手动拉取
          </el-button>
          <el-button
            v-if="isOutbound(src)"
            size="small"
            :loading="outboundPreviewCode === src.code"
            @click="openOutboundPreview(src)"
          >
            查看内容
          </el-button>
          <el-button
            v-if="hasPermission('system:update')"
            size="small"
            @click="openConfig(src)"
          >
            <el-icon><Setting /></el-icon>&nbsp;配置
          </el-button>
          <el-button size="small" link @click="viewLogs(src.code)">
            <el-icon><Document /></el-icon>&nbsp;日志
          </el-button>
          <el-button
            v-if="hasPermission('system:update')"
            size="small"
            type="danger"
            link
            @click="onDelete(src)"
          >
            <el-icon><Delete /></el-icon>&nbsp;删除
          </el-button>
        </div>
      </div>
      <el-empty v-if="!sourcesLoading && !sources.length" description="暂无数据源" />
    </div>

    <el-drawer v-model="previewVisible" :title="previewTitle" size="72%">
      <div class="preview-head">
        <span>当前投影 {{ preview?.rowCount ?? 0 }} 行</span>
        <span class="text-muted">{{ previewHint }}</span>
      </div>
      <div v-for="t in preview?.targets || []" :key="t.lookupId" class="preview-target">
        <div class="preview-target-title">
          {{ t.name || 'hosts' }} · {{ t.source === 'sql' ? 'SQL' : '主机' }} · {{ t.rowCount }} 行
        </div>
        <el-table :data="t.rows" stripe size="small" max-height="560" empty-text="没有可推送的行">
          <el-table-column
            v-for="col in previewColumns(t.rows)"
            :key="col"
            :prop="col"
            :label="col === '_key' ? 'KEY' : col"
            :min-width="col === 'ip' || col === '_key' ? 130 : 120"
            show-overflow-tooltip
          />
        </el-table>
      </div>
    </el-drawer>

    <div class="section-head">
      <h3>同步日志</h3>
      <span class="section-desc">每次推送、拉取的结果与内容快照，点左侧箭头展开</span>
    </div>
    <el-card shadow="never" class="logs-card">
      <template #header>
        <div class="logs-header">
          <span class="logs-title">最近记录</span>
          <div class="logs-filter">
            <el-select v-model="logQuery.sourceCode" placeholder="数据源" clearable style="width: 160px" @change="fetchLogs">
              <el-option label="Eventide 外表" value="eventide_lookup" />
              <el-option v-for="s in sources" :key="s.code" :label="s.name" :value="s.code" />
            </el-select>
            <el-select v-model="logQuery.status" placeholder="状态" clearable style="width: 120px" @change="fetchLogs">
              <el-option label="成功" value="success" />
              <el-option label="失败" value="failed" />
              <el-option label="跳过" value="skipped" />
            </el-select>
            <el-button :icon="Refresh" circle @click="fetchLogs" />
          </div>
        </div>
      </template>

      <el-table :data="logs.items" v-loading="logsLoading" stripe size="default">
        <el-table-column type="expand">
          <template #default="{ row }">
            <div class="log-payload">
              <div v-if="payloadHint(row) === '—'" class="text-muted">这次没有保存内容快照</div>
              <template v-else-if="payloadRows(row).length">
                <div v-if="payloadTruncated(row)" class="payload-tip">仅展示前 {{ payloadRows(row).length }} 行</div>
                <el-table :data="payloadRows(row)" stripe size="small" max-height="360" empty-text="无行">
                  <el-table-column
                    v-for="col in payloadRowColumns(row)"
                    :key="col"
                    :prop="col"
                    :label="col"
                    :min-width="col === 'ip' ? 130 : 120"
                    show-overflow-tooltip
                  />
                </el-table>
              </template>
              <el-descriptions v-else-if="payloadFields(row).length" :column="2" border size="small">
                <el-descriptions-item v-for="[k, v] in payloadFields(row)" :key="k" :label="k">
                  <span class="payload-val">{{ v }}</span>
                </el-descriptions-item>
              </el-descriptions>
              <pre v-else class="payload-json">{{ prettyPayload(row) }}</pre>
            </div>
          </template>
        </el-table-column>
        <el-table-column prop="createdAt" label="时间" width="170">
          <template #default="{ row }">{{ formatTime(row.createdAt) }}</template>
        </el-table-column>
        <el-table-column label="数据源" width="110">
          <template #default="{ row }">{{ sourceName(row.sourceCode) }}</template>
        </el-table-column>
        <el-table-column prop="action" label="动作" width="100">
          <template #default="{ row }">
            <el-tag size="small" :type="actionTagType(row.action)">{{ actionLabel(row.action) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="modelCode" label="模型" width="100" />
        <el-table-column prop="externalId" label="外部ID" width="130" show-overflow-tooltip />
        <el-table-column prop="instanceName" label="实例名" min-width="150" show-overflow-tooltip />
        <el-table-column label="内容" width="100">
          <template #default="{ row }">
            <span :class="payloadHint(row) === '—' ? 'text-muted' : ''">{{ payloadHint(row) }}</span>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag size="small" :type="logStatusType(row.status)">{{ row.status }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="message" label="消息" min-width="200" show-overflow-tooltip />
      </el-table>

      <div class="pagination-row">
        <el-pagination
          v-model:current-page="logQuery.page"
          v-model:page-size="logQuery.pageSize"
          :total="logs.total"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next"
          @size-change="fetchLogs"
          @current-change="fetchLogs"
        />
      </div>
    </el-card>

    <el-dialog v-model="lookupConfigVisible" title="Eventide 外表同步配置" width="760px">
      <el-form :model="lookupForm" label-width="120px">
        <el-form-item label="启用">
          <el-switch v-model="lookupForm.enabled" />
        </el-form-item>
        <el-form-item label="Eventide 地址">
          <el-input v-model="lookupForm.baseUrl" placeholder="http://127.0.0.1:8080" />
        </el-form-item>
        <el-form-item label="同步 Token">
          <el-input
            v-model="lookupForm.lookupSyncToken"
            show-password
            :placeholder="lookupStatus?.tokenSet ? `已配置 ${lookupStatus.tokenMasked}，留空不改` : 'lks_ 开头的外表同步 Token'"
          />
        </el-form-item>
        <el-form-item label="定时间隔">
          <el-input-number v-model="lookupForm.intervalSecs" :min="30" :max="86400" :step="30" />
          <span class="form-inline-tip">秒，最小 30</span>
        </el-form-item>
        <el-form-item label="变更防抖">
          <el-input-number v-model="lookupForm.debounceSecs" :min="5" :max="300" :step="5" />
          <span class="form-inline-tip">秒，资产变更后延迟再推</span>
        </el-form-item>
        <el-form-item label="外表清单">
          <div class="target-list">
            <div v-for="(t, idx) in lookupForm.targets" :key="idx" class="target-card">
              <div class="target-card-head">
                <span>外表 {{ idx + 1 }}</span>
                <el-button
                  v-if="lookupForm.targets.length > 1"
                  type="danger"
                  link
                  size="small"
                  @click="lookupForm.targets.splice(idx, 1)"
                >删除</el-button>
              </div>
              <el-form-item label="内容来源" label-width="88px">
                <el-radio-group v-model="t.source">
                  <el-radio value="hosts">主机投影</el-radio>
                  <el-radio value="sql">自定义 SQL</el-radio>
                </el-radio-group>
              </el-form-item>
              <el-form-item label="lookup_id" label-width="88px">
                <el-input v-model="t.lookupId" placeholder="Eventide 外表 UUID" />
              </el-form-item>
              <el-form-item label="名称" label-width="88px">
                <el-input v-model="t.name" :placeholder="t.source === 'sql' ? '如 biz_owners' : 'hosts'" />
              </el-form-item>
              <template v-if="t.source === 'sql'">
                <el-form-item label="KEY 列" label-width="88px">
                  <el-input v-model="t.keyColumn" placeholder="空则用第一列，通常填 ip" />
                </el-form-item>
                <el-form-item label="SQL" label-width="88px">
                  <el-input
                    v-model="t.sql"
                    type="textarea"
                    :rows="6"
                    placeholder="SELECT ip, hostname AS `主机名`, owner AS `联系人` FROM ..."
                  />
                  <div class="form-tip">只允许一条 SELECT / WITH。列别名即 Eventide 属性名。最多 3000 行。</div>
                </el-form-item>
                <el-button size="small" :loading="sqlPreviewing === idx" @click="onTrySql(idx)">试跑 SQL</el-button>
              </template>
            </div>
            <el-button size="small" @click="addLookupTarget">增加外表</el-button>
          </div>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="lookupConfigVisible = false">取消</el-button>
        <el-button type="primary" :loading="lookupConfigSaving" @click="onSaveLookupConfig">保存配置</el-button>
      </template>
    </el-dialog>

    <!-- 数据源配置对话框 -->
    <el-dialog v-model="configVisible" :title="configForm.sourceType === 'http_push' ? 'HTTP 出站配置' : '数据源拉取配置'" :width="configForm.sourceType === 'http_push' ? '760px' : '640px'">
      <el-form :model="configForm" label-width="120px">
        <el-form-item label="数据源">
          <el-input :model-value="configForm.name" disabled />
        </el-form-item>
        <el-form-item label="启用">
          <el-switch v-model="configForm.enabled" />
        </el-form-item>
        <el-form-item label="API 地址">
          <el-input v-model="configForm.apiUrl" :placeholder="configForm.sourceType === 'http_push' ? 'http://youyun-cmdb.example.local' : 'https://bk.example.com'" />
        </el-form-item>
        <el-form-item label="API Token">
          <el-input v-model="configForm.apiToken" placeholder="外部系统访问令牌" show-password />
        </el-form-item>
        <template v-if="configForm.sourceType === 'http_push'">
          <el-form-item label="认证方式">
            <el-select v-model="configForm.authStyle" style="width: 100%">
              <el-option value="bearer" label="Bearer Token" />
              <el-option value="axleops" label="Bearer + X-AxleOps-Token" />
              <el-option value="none" label="不带认证" />
            </el-select>
          </el-form-item>
          <el-form-item label="定时推送">
            <el-switch v-model="configForm.scheduleEnabled" />
            <span class="form-inline-tip">按间隔把全部外表推到对端，默认关闭</span>
          </el-form-item>
          <el-form-item v-if="configForm.scheduleEnabled" label="定时间隔">
            <el-input-number v-model="configForm.intervalSecs" :min="30" :max="86400" :step="30" />
            <span class="form-inline-tip">秒，最小 30</span>
          </el-form-item>
          <el-form-item label="外表清单">
            <div class="target-list">
              <div v-for="(t, idx) in configForm.targets" :key="idx" class="target-card">
                <div class="target-card-head">
                  <span>外表 {{ idx + 1 }}</span>
                  <el-button
                    v-if="configForm.targets.length > 1"
                    type="danger"
                    link
                    size="small"
                    @click="configForm.targets.splice(idx, 1)"
                  >删除</el-button>
                </div>
                <el-form-item label="名称" label-width="88px">
                  <el-input v-model="t.name" :placeholder="t.contentSource === 'sql' ? '如 biz_owners' : 'hosts'" />
                </el-form-item>
                <el-form-item label="推送路径" label-width="88px">
                  <el-input v-model="t.path" placeholder="/openapi/v1/cmdb/ci/sync" />
                </el-form-item>
                <el-form-item label="请求方法" label-width="88px">
                  <el-radio-group v-model="t.method">
                    <el-radio value="POST">POST</el-radio>
                    <el-radio value="PUT">PUT</el-radio>
                  </el-radio-group>
                </el-form-item>
                <el-form-item label="报文格式" label-width="88px">
                  <el-select v-model="t.bodyMode" style="width: 100%">
                    <el-option value="items" label="items：优云风格 { source, items[] }" />
                    <el-option value="rows" label="rows：AxleOps / Eventide 风格 { rows: { ip: {} } }" />
                    <el-option value="hosts" label="hosts：理想自动化 { source, hosts[] }" />
                  </el-select>
                </el-form-item>
                <el-form-item label="内容来源" label-width="88px">
                  <el-radio-group v-model="t.contentSource">
                    <el-radio value="hosts">主机投影</el-radio>
                    <el-radio value="sql">自定义 SQL</el-radio>
                  </el-radio-group>
                </el-form-item>
                <template v-if="t.contentSource === 'sql'">
                  <el-form-item label="KEY 列" label-width="88px">
                    <el-input v-model="t.keyColumn" placeholder="空则用第一列，通常填 ip" />
                  </el-form-item>
                  <el-form-item label="SQL" label-width="88px">
                    <el-input
                      v-model="t.sql"
                      type="textarea"
                      :rows="6"
                      placeholder="SELECT ip, hostname AS `主机名`, owner AS `联系人` FROM ..."
                    />
                    <div class="form-tip">只允许一条 SELECT / WITH。列别名即对外属性名。最多 3000 行。</div>
                  </el-form-item>
                  <el-button size="small" :loading="outboundSqlPreviewing === `config-${idx}`" @click="onTryOutboundSql('config', idx)">试跑 SQL</el-button>
                </template>
              </div>
              <el-button size="small" @click="addPushTarget(configForm.targets)">增加外表</el-button>
            </div>
          </el-form-item>
        </template>
        <template v-else>
          <el-form-item label="拉取路径">
            <el-input v-model="configForm.path" placeholder="/api/v3/host/list" />
          </el-form-item>
          <el-form-item label="请求方法">
            <el-radio-group v-model="configForm.method">
              <el-radio value="GET">GET</el-radio>
              <el-radio value="POST">POST</el-radio>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="响应数据路径">
            <el-input v-model="configForm.responsePath" placeholder="data.info" />
            <div class="form-tip">JSON path 到数据项数组，如 data.info / data.list，留空表示响应根为数组</div>
          </el-form-item>
          <el-form-item label="CI 模型编码">
            <el-input v-model="configForm.modelCode" placeholder="host" />
          </el-form-item>
          <el-form-item label="定时拉取">
            <el-switch v-model="configForm.pullEnabled" />
            <span class="form-inline-tip">按间隔去对端拉资产，默认关闭</span>
          </el-form-item>
          <el-form-item v-if="configForm.pullEnabled" label="定时间隔">
            <el-input-number v-model="configForm.intervalSecs" :min="30" :max="86400" :step="30" />
            <span class="form-inline-tip">秒，最小 30</span>
          </el-form-item>
        </template>
      </el-form>
      <template #footer>
        <el-button @click="configVisible = false">取消</el-button>
        <el-button type="primary" :loading="configSaving" @click="onSaveConfig">保存配置</el-button>
      </template>
    </el-dialog>

    <!-- 新增数据源对话框 -->
    <el-dialog v-model="createVisible" title="新增数据源" :width="createForm.sourceType === 'http_push' ? '760px' : '640px'">
      <el-form :model="createForm" label-width="120px">
        <el-form-item label="编码 code" required>
          <el-input v-model="createForm.code" placeholder="如 zabbix / prometheus / custom_cmdb" />
          <div class="form-tip">唯一标识，仅小写字母/数字/下划线，最长 32 字符，创建后不可修改</div>
        </el-form-item>
        <el-form-item label="名称" required>
          <el-input v-model="createForm.name" placeholder="如 Zabbix 监控" />
        </el-form-item>
        <el-form-item label="接入方式">
          <el-radio-group v-model="createForm.sourceType" @change="onCreateTypeChange">
            <el-radio value="webhook">Webhook 接入</el-radio>
            <el-radio value="pull">主动拉取</el-radio>
            <el-radio value="http_push">HTTP 出站</el-radio>
          </el-radio-group>
          <div class="form-tip">webhook：外部推给我们；pull：我们去拉；http_push：把主机投影或自定义 SQL HTTP 推出去</div>
        </el-form-item>
        <el-form-item label="API 地址">
          <el-input v-model="createForm.apiUrl" placeholder="https://example.com（pull 模式必填）" />
        </el-form-item>
        <el-form-item label="API Token">
          <el-input v-model="createForm.apiToken" placeholder="外部系统访问令牌" show-password />
        </el-form-item>
        <el-form-item label="Webhook 密钥">
          <el-input v-model="createForm.webhookSecret" placeholder="webhook 签名校验密钥（可选）" show-password />
        </el-form-item>
        <template v-if="createForm.sourceType === 'http_push'">
          <el-divider content-position="left">出站推送</el-divider>
          <el-form-item label="认证方式">
            <el-select v-model="createForm.authStyle" style="width: 100%">
              <el-option value="bearer" label="Bearer Token" />
              <el-option value="axleops" label="Bearer + X-AxleOps-Token" />
              <el-option value="none" label="不带认证" />
            </el-select>
          </el-form-item>
          <el-form-item label="定时推送">
            <el-switch v-model="createForm.scheduleEnabled" />
            <span class="form-inline-tip">按间隔把全部外表推到对端，默认关闭</span>
          </el-form-item>
          <el-form-item v-if="createForm.scheduleEnabled" label="定时间隔">
            <el-input-number v-model="createForm.intervalSecs" :min="30" :max="86400" :step="30" />
            <span class="form-inline-tip">秒，最小 30</span>
          </el-form-item>
          <el-form-item label="外表清单">
            <div class="target-list">
              <div v-for="(t, idx) in createForm.targets" :key="idx" class="target-card">
                <div class="target-card-head">
                  <span>外表 {{ idx + 1 }}</span>
                  <el-button
                    v-if="createForm.targets.length > 1"
                    type="danger"
                    link
                    size="small"
                    @click="createForm.targets.splice(idx, 1)"
                  >删除</el-button>
                </div>
                <el-form-item label="名称" label-width="88px">
                  <el-input v-model="t.name" :placeholder="t.contentSource === 'sql' ? '如 biz_owners' : 'hosts'" />
                </el-form-item>
                <el-form-item label="推送路径" label-width="88px">
                  <el-input v-model="t.path" placeholder="/openapi/v1/cmdb/ci/sync" />
                </el-form-item>
                <el-form-item label="请求方法" label-width="88px">
                  <el-radio-group v-model="t.method">
                    <el-radio value="POST">POST</el-radio>
                    <el-radio value="PUT">PUT</el-radio>
                  </el-radio-group>
                </el-form-item>
                <el-form-item label="报文格式" label-width="88px">
                  <el-select v-model="t.bodyMode" style="width: 100%">
                    <el-option value="items" label="items：优云风格" />
                    <el-option value="rows" label="rows：AxleOps / Eventide 风格" />
                    <el-option value="hosts" label="hosts：理想自动化" />
                  </el-select>
                </el-form-item>
                <el-form-item label="内容来源" label-width="88px">
                  <el-radio-group v-model="t.contentSource">
                    <el-radio value="hosts">主机投影</el-radio>
                    <el-radio value="sql">自定义 SQL</el-radio>
                  </el-radio-group>
                </el-form-item>
                <template v-if="t.contentSource === 'sql'">
                  <el-form-item label="KEY 列" label-width="88px">
                    <el-input v-model="t.keyColumn" placeholder="空则用第一列，通常填 ip" />
                  </el-form-item>
                  <el-form-item label="SQL" label-width="88px">
                    <el-input
                      v-model="t.sql"
                      type="textarea"
                      :rows="6"
                      placeholder="SELECT ip, hostname AS `主机名`, owner AS `联系人` FROM ..."
                    />
                    <div class="form-tip">只允许一条 SELECT / WITH。列别名即对外属性名。最多 3000 行。</div>
                  </el-form-item>
                  <el-button size="small" :loading="outboundSqlPreviewing === `create-${idx}`" @click="onTryOutboundSql('create', idx)">试跑 SQL</el-button>
                </template>
              </div>
              <el-button size="small" @click="addPushTarget(createForm.targets)">增加外表</el-button>
            </div>
          </el-form-item>
        </template>
        <template v-else-if="createForm.sourceType === 'pull'">
          <el-divider content-position="left">拉取配置</el-divider>
          <el-form-item label="拉取路径">
            <el-input v-model="createForm.path" placeholder="/api/v3/host/list" />
          </el-form-item>
          <el-form-item label="请求方法">
            <el-radio-group v-model="createForm.method">
              <el-radio value="GET">GET</el-radio>
              <el-radio value="POST">POST</el-radio>
            </el-radio-group>
          </el-form-item>
          <el-form-item label="响应数据路径">
            <el-input v-model="createForm.responsePath" placeholder="data.info" />
            <div class="form-tip">JSON path 到数据项数组，留空表示响应根为数组</div>
          </el-form-item>
          <el-form-item label="CI 模型编码">
            <el-input v-model="createForm.modelCode" placeholder="host" />
          </el-form-item>
          <el-form-item label="定时拉取">
            <el-switch v-model="createForm.pullEnabled" />
            <span class="form-inline-tip">按间隔去对端拉资产，默认关闭</span>
          </el-form-item>
          <el-form-item v-if="createForm.pullEnabled" label="定时间隔">
            <el-input-number v-model="createForm.intervalSecs" :min="30" :max="86400" :step="30" />
            <span class="form-inline-tip">秒，最小 30</span>
          </el-form-item>
        </template>
      </el-form>
      <template #footer>
        <el-button @click="createVisible = false">取消</el-button>
        <el-button type="primary" :loading="createSaving" @click="onCreate">创建</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Refresh, Download, Setting, Document, Connection, Promotion, Plus, Delete, Upload } from '@element-plus/icons-vue'
import {
  listSyncSources,
  createSyncSource,
  deleteSyncSource,
  pullInstances,
  pushOutInstances,
  previewHttpPush,
  updateSyncSource,
  listSyncLogs,
  getEventideLookupStatus,
  runEventideLookupSync,
  previewEventideLookup,
  previewEventideLookupSql,
  updateEventideLookupConfig,
  type EventideLookupStatus,
  type EventideLookupPreview,
  type EventideLookupTargetConfig,
} from '../../api/cmdb'
import { useSystemDicts, labelOf } from '../../composables/useSystemDicts'

const dicts = useSystemDicts()
import type { SyncSource, SyncLog, SyncLogPage, HttpPushTarget } from '../../api/types'
import { useUserStore } from '../../stores/user'

const userStore = useUserStore()
const hasPermission = (code: string) => userStore.hasPermission(code)

// ---- 数据源列表 ----
const sources = ref<SyncSource[]>([])
const sourcesLoading = ref(false)

async function fetchSources() {
  sourcesLoading.value = true
  try {
    sources.value = await listSyncSources()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '加载数据源失败')
  } finally {
    sourcesLoading.value = false
  }
}

function sourceName(code: string): string {
  if (code === 'eventide_lookup') return 'Eventide 外表'
  return sources.value.find(s => s.code === code)?.name ?? code
}

const SAMPLE_CODES = ['youyun_cmdb', 'axleops_push', 'ideal_auto']

function isOutbound(src: SyncSource) {
  return src.sourceType === 'http_push'
}
function isSample(src: SyncSource) {
  return SAMPLE_CODES.includes(src.code)
}
function sourceTypeLabel(src: SyncSource) {
  if (src.sourceType === 'pull') return '拉取'
  if (src.sourceType === 'http_push') return '出站'
  return '接入'
}
function sourceCfg(src: SyncSource): Record<string, unknown> {
  if (!src.pullConfig) return {}
  return typeof src.pullConfig === 'string'
    ? safeParse(src.pullConfig)
    : src.pullConfig as Record<string, unknown>
}

interface HttpPushTargetForm {
  name: string
  path: string
  method: string
  bodyMode: string
  contentSource: 'hosts' | 'sql'
  sql: string
  keyColumn: string
}

function emptyPushTarget(): HttpPushTargetForm {
  return {
    name: 'hosts',
    path: '/openapi/v1/cmdb/ci/sync',
    method: 'POST',
    bodyMode: 'items',
    contentSource: 'hosts',
    sql: '',
    keyColumn: 'ip',
  }
}

function normalizePushTarget(raw: Record<string, unknown>): HttpPushTargetForm {
  return {
    name: String(raw.name || (raw.contentSource === 'sql' ? 'sql_lookup' : 'hosts')),
    path: String(raw.path || ''),
    method: String(raw.method || 'POST').toUpperCase() === 'PUT' ? 'PUT' : 'POST',
    bodyMode: String(raw.bodyMode || 'items'),
    contentSource: raw.contentSource === 'sql' ? 'sql' : 'hosts',
    sql: String(raw.sql || ''),
    keyColumn: String(raw.keyColumn || ''),
  }
}

function outboundTargets(src: SyncSource): HttpPushTargetForm[] {
  const cfg = sourceCfg(src)
  const raw = cfg.targets
  if (Array.isArray(raw) && raw.length) {
    return raw.map((t) => normalizePushTarget((t || {}) as Record<string, unknown>))
  }
  return [normalizePushTarget(cfg)]
}

function outboundTargetText(src: SyncSource) {
  const ts = outboundTargets(src)
  if (!ts.length) return '未配置外表'
  return ts.map((t) => `${t.name || 'hosts'}·${t.contentSource === 'sql' ? 'SQL' : '主机'}`).join('、')
}

function outboundScheduleEnabled(src: SyncSource) {
  return Boolean(sourceCfg(src).scheduleEnabled)
}

function outboundInterval(src: SyncSource) {
  const n = Number(sourceCfg(src).intervalSecs)
  return Number.isFinite(n) && n >= 30 ? Math.floor(n) : 300
}

function pullInterval(src: SyncSource) {
  return outboundInterval(src)
}

function addPushTarget(list: HttpPushTargetForm[]) {
  list.push({
    name: '',
    path: '',
    method: 'POST',
    bodyMode: 'items',
    contentSource: 'sql',
    sql: '',
    keyColumn: 'ip',
  })
}

function serializePushTargets(list: HttpPushTargetForm[]): HttpPushTarget[] {
  return list.map((t) => ({
    name: (t.name || '').trim() || (t.contentSource === 'sql' ? 'sql_lookup' : 'hosts'),
    path: (t.path || '').trim(),
    method: t.method === 'PUT' ? 'PUT' : 'POST',
    bodyMode: t.bodyMode || 'items',
    contentSource: t.contentSource === 'sql' ? 'sql' : 'hosts',
    sql: t.contentSource === 'sql' ? t.sql : '',
    keyColumn: t.contentSource === 'sql' ? t.keyColumn : '',
    rejectEmpty: true,
  }))
}

const lookupStatus = ref<EventideLookupStatus | null>(null)
const lookupPushing = ref(false)
const pushingCode = ref('')
const outboundPreviewCode = ref('')
const previewVisible = ref(false)
const previewLoading = ref(false)
const previewTitle = ref('Eventide 外表推送内容')
const previewHint = ref('主机投影 + 自定义 SQL 外表都会列在下面')
const preview = ref<EventideLookupPreview | null>(null)
const lookupTargetText = computed(() => {
  const ts = lookupStatus.value?.targets || []
  if (!ts.length) return '未配置 lookup_id'
  return ts.map((t) => {
    const id = t.lookupId ? t.lookupId.slice(0, 8) : ''
    const kind = t.source === 'sql' ? 'SQL' : '主机'
    const label = `${t.name || 'hosts'}·${kind}`
    return id ? `${label}（${id}…）` : label
  }).join('、')
})
const lookupConfigVisible = ref(false)
const lookupConfigSaving = ref(false)
const lookupForm = reactive({
  enabled: false,
  baseUrl: '',
  lookupSyncToken: '',
  intervalSecs: 300,
  debounceSecs: 45,
  targets: [] as EventideLookupTargetConfig[],
})
const sqlPreviewing = ref<number | null>(null)

function emptyLookupTarget(): EventideLookupTargetConfig {
  return { lookupId: '', name: 'hosts', source: 'hosts', sql: '', keyColumn: 'ip' }
}

function openLookupConfig() {
  const s = lookupStatus.value
  lookupForm.enabled = Boolean(s?.enabled)
  lookupForm.baseUrl = s?.baseUrl || ''
  lookupForm.lookupSyncToken = ''
  lookupForm.intervalSecs = s?.intervalSecs || 300
  lookupForm.debounceSecs = s?.debounceSecs || 45
  lookupForm.targets = (s?.targets || []).map((t) => ({
    lookupId: t.lookupId || '',
    name: t.name || 'hosts',
    source: t.source === 'sql' ? 'sql' : 'hosts',
    sql: t.sql || '',
    keyColumn: t.keyColumn || '',
  }))
  if (!lookupForm.targets.length) lookupForm.targets.push(emptyLookupTarget())
  lookupConfigVisible.value = true
}

function addLookupTarget() {
  lookupForm.targets.push({ lookupId: '', name: '', source: 'sql', sql: '', keyColumn: 'ip' })
}

const outboundSqlPreviewing = ref<string | null>(null)

async function onTryOutboundSql(which: 'config' | 'create', idx: number) {
  const t = which === 'config' ? configForm.targets[idx] : createForm.targets[idx]
  if (!t?.sql?.trim()) {
    ElMessage.warning('请先填写 SQL')
    return
  }
  outboundSqlPreviewing.value = `${which}-${idx}`
  try {
    const data = await previewEventideLookupSql({
      sql: t.sql,
      keyColumn: t.keyColumn || undefined,
    })
    ElMessage.success(`试跑成功：${data.rowCount} 行，列 ${data.columns.join('、') || '—'}`)
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '试跑失败')
  } finally {
    outboundSqlPreviewing.value = null
  }
}

async function onTrySql(idx: number) {
  const t = lookupForm.targets[idx]
  if (!t?.sql?.trim()) {
    ElMessage.warning('请先填写 SQL')
    return
  }
  sqlPreviewing.value = idx
  try {
    const data = await previewEventideLookupSql({
      sql: t.sql,
      keyColumn: t.keyColumn || undefined,
    })
    ElMessage.success(`试跑成功：${data.rowCount} 行，列 ${data.columns.join('、') || '—'}`)
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '试跑失败')
  } finally {
    sqlPreviewing.value = null
  }
}

async function onSaveLookupConfig() {
  const targets = lookupForm.targets
    .map((t) => ({
      lookupId: t.lookupId.trim(),
      name: (t.name || '').trim() || (t.source === 'sql' ? 'sql_lookup' : 'hosts'),
      source: t.source === 'sql' ? 'sql' as const : 'hosts' as const,
      sql: t.sql || '',
      keyColumn: t.keyColumn || '',
    }))
    .filter((t) => t.lookupId)
  if (lookupForm.enabled && !targets.length) {
    ElMessage.warning('启用前请至少填写一张外表的 lookup_id')
    return
  }
  if (targets.some((t) => t.source === 'sql' && !t.sql.trim())) {
    ElMessage.warning('SQL 外表必须填写查询语句')
    return
  }
  lookupConfigSaving.value = true
  try {
    lookupStatus.value = await updateEventideLookupConfig({
      enabled: lookupForm.enabled,
      baseUrl: lookupForm.baseUrl.trim(),
      lookupSyncToken: lookupForm.lookupSyncToken.trim() || undefined,
      intervalSecs: lookupForm.intervalSecs,
      debounceSecs: lookupForm.debounceSecs,
      targets,
    })
    ElMessage.success('外表同步配置已保存')
    lookupConfigVisible.value = false
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '保存失败')
  } finally {
    lookupConfigSaving.value = false
  }
}

function previewColumns(rows: Record<string, string>[]): string[] {
  const preferred = ['ip', '_key', '主机名', '机房', '联系人', '电话', '邮箱', '业务线']
  const keys = new Set<string>()
  for (const r of rows.slice(0, 8)) Object.keys(r).forEach((k) => keys.add(k))
  const head = preferred.filter((k) => keys.has(k))
  const rest = [...keys].filter((k) => !head.includes(k)).sort()
  return [...head, ...rest]
}
const lastLookupText = computed(() => {
  const last = lookupStatus.value?.last
  if (!last) return ''
  const rows = (last.targets || []).reduce((n, t) => n + (t.rowCount || 0), 0)
  if (rows > 0) return `已推送 ${rows} 行`
  return last.message || ''
})

async function fetchLookupStatus() {
  try {
    lookupStatus.value = await getEventideLookupStatus()
  } catch (e: unknown) {
    lookupStatus.value = null
    ElMessage.error(e instanceof Error ? e.message : '加载 Eventide 同步状态失败')
  }
}

async function openPreview() {
  previewLoading.value = true
  try {
    previewTitle.value = 'Eventide 外表推送内容'
    previewHint.value = '主机投影 + 自定义 SQL 外表都会列在下面'
    preview.value = await previewEventideLookup()
    previewVisible.value = true
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '加载推送内容失败')
  } finally {
    previewLoading.value = false
  }
}

async function openOutboundPreview(src: SyncSource) {
  outboundPreviewCode.value = src.code
  try {
    const data = await previewHttpPush(src.code)
    const targets = (data.targets && data.targets.length)
      ? data.targets
      : [{
          name: src.name,
          method: data.method || 'POST',
          url: data.url || src.apiUrl,
          bodyMode: data.bodyMode || 'items',
          contentSource: data.contentSource === 'sql' ? 'sql' as const : 'hosts' as const,
          rowCount: data.rowCount,
          rows: data.rows || [],
        }]
    previewTitle.value = `${src.name} 推送内容`
    previewHint.value = `${targets.length} 张外表`
    preview.value = {
      rowCount: data.rowCount,
      targets: targets.map((t, i) => ({
        lookupId: `${src.code}-${i}-${t.name}`,
        name: t.error ? `${t.name}（${t.error}）` : t.name,
        source: t.contentSource === 'sql' ? 'sql' : 'hosts',
        rowCount: t.rowCount,
        rows: t.rows,
      })),
    }
    previewVisible.value = true
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '加载推送内容失败')
  } finally {
    outboundPreviewCode.value = ''
  }
}

async function onPushOut(src: SyncSource) {
  try {
    pushingCode.value = src.code
    const report = await pushOutInstances(src.code)
    const failed = (report.targets || []).find((t) => !t.ok && !t.skipped)
    if (report.ok) {
      ElMessage.success(report.message || `已推送 ${report.rowCount} 行`)
    } else if (report.skipped) {
      ElMessage.warning(report.message || '已跳过')
    } else {
      ElMessage.error(failed?.message || report.message || '推送失败')
    }
    fetchSources()
    fetchLogs()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '推送失败')
  } finally {
    pushingCode.value = ''
  }
}

async function onPushLookup() {
  lookupPushing.value = true
  try {
    const report = await runEventideLookupSync()
    lookupStatus.value = {
      ...(lookupStatus.value as EventideLookupStatus),
      last: report,
    }
    if (report.ok) {
      ElMessage.success(report.message || '已推送到 Eventide')
    } else {
      ElMessage.error(report.message || '推送失败')
    }
    fetchLogs()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '推送失败')
  } finally {
    lookupPushing.value = false
    fetchLookupStatus()
  }
}

function refreshAll() {
  fetchSources()
  fetchLookupStatus()
  fetchLogs()
}

function sourceIcon(src: SyncSource) {
  return src.sourceType === 'pull' ? Connection : Promotion
}

/** 从 pullConfig 中提取拉取路径（兼容对象/字符串） */
function pullPath(src: SyncSource): string {
  if (!src.pullConfig) return ''
  const cfg = typeof src.pullConfig === 'string'
    ? safeParse(src.pullConfig)
    : src.pullConfig as Record<string, unknown>
  return (cfg.path as string) || ''
}

function safeParse(s: string): Record<string, unknown> {
  try { return JSON.parse(s) } catch { return {} }
}

// ---- 状态辅助 ----
function syncStatusType(s: string): string {
  if (s === 'success') return 'success'
  if (s === 'partial') return 'warning'
  if (s === 'failed') return 'danger'
  return 'info'
}
function syncStatusLabel(s: string): string {
  return labelOf(dicts.syncStatuses.value, s)
}
function actionTagType(a: string): string {
  if (a === 'webhook' || a === 'push') return 'success'
  if (a === 'pull') return 'warning'
  return 'info'
}
function actionLabel(a: string): string {
  if (a === 'push') return labelOf(dicts.syncActions.value, a) || '出站推送'
  return labelOf(dicts.syncActions.value, a)
}
function logStatusType(s: string): string {
  if (s === 'success') return 'success'
  if (s === 'failed') return 'danger'
  if (s === 'skipped') return 'info'
  return 'info'
}
function formatTime(t: string): string {
  if (!t) return '—'
  try { return new Date(t).toLocaleString('zh-CN', { hour12: false }) } catch { return t }
}

// ---- 手动拉取 ----
const pullingCode = ref('')

async function onPull(src: SyncSource) {
  const modelCode = pullModelCode(src)
  if (!modelCode) {
    ElMessage.warning('数据源未配置 modelCode，无法拉取')
    return
  }
  try {
    pullingCode.value = src.code
    const result = await pullInstances({ source: src.code, modelCode })
    ElMessage.success(`拉取完成：共 ${result.total} 条，成功 ${result.success} 条，失败 ${result.failed} 条`)
    fetchSources()
    fetchLogs()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '拉取失败')
  } finally {
    pullingCode.value = ''
  }
}

function pullModelCode(src: SyncSource): string {
  if (!src.pullConfig) return ''
  const cfg = typeof src.pullConfig === 'string'
    ? safeParse(src.pullConfig)
    : src.pullConfig as Record<string, unknown>
  return (cfg.modelCode as string) || ''
}

// ---- 配置对话框 ----
const configVisible = ref(false)
const configSaving = ref(false)
const configForm = reactive({
  code: '',
  name: '',
  sourceType: 'pull',
  enabled: true,
  apiUrl: '',
  apiToken: '',
  path: '',
  method: 'GET',
  responsePath: 'data.info',
  modelCode: 'host',
  bodyMode: 'items',
  authStyle: 'bearer',
  platform: '',
  extraHeaders: null as unknown,
  targets: [] as HttpPushTargetForm[],
  scheduleEnabled: false,
  intervalSecs: 300,
  pullEnabled: false,
  pullCron: '0 */2 * * *',
})

function openConfig(src: SyncSource) {
  configForm.code = src.code
  configForm.name = src.name
  configForm.sourceType = src.sourceType
  configForm.enabled = Boolean(src.enabled)
  configForm.apiUrl = src.apiUrl
  configForm.apiToken = src.apiToken
  configForm.pullEnabled = src.sourceType === 'http_push' ? false : src.pullEnabled
  configForm.pullCron = ''
  const cfg = sourceCfg(src)
  configForm.path = (cfg.path as string) || ''
  configForm.method = (cfg.method as string) || (src.sourceType === 'http_push' ? 'POST' : 'GET')
  configForm.responsePath = (cfg.responsePath as string) || 'data.info'
  configForm.modelCode = (cfg.modelCode as string) || 'host'
  configForm.bodyMode = (cfg.bodyMode as string) || 'items'
  configForm.authStyle = (cfg.authStyle as string) || 'bearer'
  configForm.platform = (cfg.platform as string) || ''
  configForm.extraHeaders = cfg.extraHeaders ?? null
  configForm.targets = outboundTargets(src)
  if (!configForm.targets.length) configForm.targets.push(emptyPushTarget())
  configForm.scheduleEnabled = Boolean(cfg.scheduleEnabled)
  configForm.intervalSecs = outboundInterval(src)
  configVisible.value = true
}

async function onSaveConfig() {
  if (configForm.sourceType === 'http_push') {
    if (!configForm.targets.length) {
      ElMessage.warning('请至少配置一张外表')
      return
    }
    if (configForm.targets.some((t) => t.contentSource === 'sql' && !t.sql.trim())) {
      ElMessage.warning('SQL 外表必须填写查询语句')
      return
    }
  }
  const pullConfig = configForm.sourceType === 'http_push'
    ? JSON.stringify({
        authStyle: configForm.authStyle,
        rejectEmpty: true,
        platform: configForm.platform,
        extraHeaders: configForm.extraHeaders,
        scheduleEnabled: configForm.scheduleEnabled,
        intervalSecs: Math.max(30, configForm.intervalSecs || 300),
        targets: serializePushTargets(configForm.targets),
      })
    : JSON.stringify({
        method: configForm.method,
        path: configForm.path,
        responsePath: configForm.responsePath,
        modelCode: configForm.modelCode,
        intervalSecs: Math.max(30, configForm.intervalSecs || 300),
      })
  try {
    configSaving.value = true
    await updateSyncSource(configForm.code, {
      apiUrl: configForm.apiUrl,
      apiToken: configForm.apiToken,
      pullConfig,
      pullCron: '',
      pullEnabled: configForm.sourceType === 'http_push' ? false : configForm.pullEnabled,
      enabled: configForm.enabled,
    })
    ElMessage.success('配置已保存')
    configVisible.value = false
    await fetchSources()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '保存失败')
  } finally {
    configSaving.value = false
  }
}

// ---- 同步日志 ----
const logs = reactive<SyncLogPage>({ total: 0, page: 1, pageSize: 20, items: [] as SyncLog[] })
const logsLoading = ref(false)
const logQuery = reactive({
  sourceCode: '' as string,
  status: '' as string,
  page: 1,
  pageSize: 20,
})

async function fetchLogs() {
  logsLoading.value = true
  try {
    const params = {
      sourceCode: logQuery.sourceCode || undefined,
      status: logQuery.status || undefined,
      page: logQuery.page,
      pageSize: logQuery.pageSize,
    }
    const res = await listSyncLogs(params)
    logs.items = res.items
    logs.total = res.total
    logs.page = res.page
    logs.pageSize = res.pageSize
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '加载日志失败')
  } finally {
    logsLoading.value = false
  }
}

function viewLogs(code: string) {
  logQuery.sourceCode = code
  logQuery.page = 1
  fetchLogs()
}

function parsePayload(row: SyncLog): unknown {
  const raw = row.payload
  if (raw == null || raw === '') return null
  if (typeof raw === 'string') {
    try { return JSON.parse(raw) } catch { return raw }
  }
  return raw
}

function asRecord(v: unknown): Record<string, unknown> | null {
  return v && typeof v === 'object' && !Array.isArray(v) ? v as Record<string, unknown> : null
}

function payloadRows(row: SyncLog): Record<string, string>[] {
  const obj = asRecord(parsePayload(row))
  const rows = obj?.rows
  if (!Array.isArray(rows)) return []
  return rows.map((item) => {
    const rec = asRecord(item) || {}
    const out: Record<string, string> = {}
    for (const [k, v] of Object.entries(rec)) out[k] = formatPayloadVal(v)
    return out
  })
}

function payloadRowColumns(row: SyncLog): string[] {
  const preferred = ['ip', '主机名', '机房', '联系人', '电话', '邮箱', '业务线']
  const keys = new Set<string>()
  for (const r of payloadRows(row).slice(0, 20)) {
    Object.keys(r).forEach((k) => keys.add(k))
  }
  const head = preferred.filter((k) => keys.has(k))
  const rest = [...keys].filter((k) => !head.includes(k)).sort()
  return [...head, ...rest]
}

function payloadTruncated(row: SyncLog): boolean {
  return asRecord(parsePayload(row))?.truncated === true
}

function payloadFields(row: SyncLog): [string, string][] {
  const raw = parsePayload(row)
  const obj = asRecord(raw)
  if (!obj || Array.isArray(obj.rows)) return []
  return Object.entries(obj).map(([k, v]) => [k, formatPayloadVal(v)])
}

function payloadHint(row: SyncLog): string {
  const raw = parsePayload(row)
  if (raw == null) return '—'
  const obj = asRecord(raw)
  if (obj) {
    if (Array.isArray(obj.rows)) {
      const n = typeof obj.rowCount === 'number' ? obj.rowCount : obj.rows.length
      return `${n} 行`
    }
    const keys = Object.keys(obj)
    if (keys.length) return `${keys.length} 字段`
  }
  if (Array.isArray(raw)) return `${raw.length} 条`
  return '有内容'
}

function prettyPayload(row: SyncLog): string {
  const raw = parsePayload(row)
  if (raw == null) return ''
  if (typeof raw === 'string') return raw
  try { return JSON.stringify(raw, null, 2) } catch { return String(raw) }
}

function formatPayloadVal(v: unknown): string {
  if (v == null) return ''
  if (typeof v === 'string') return v
  if (typeof v === 'number' || typeof v === 'boolean') return String(v)
  try { return JSON.stringify(v) } catch { return String(v) }
}

// ---- 新增数据源 ----
const createVisible = ref(false)
const createSaving = ref(false)
const createForm = reactive({
  code: '',
  name: '',
  sourceType: 'pull' as 'webhook' | 'pull' | 'http_push',
  apiUrl: '',
  apiToken: '',
  webhookSecret: '',
  path: '',
  method: 'GET',
  responsePath: 'data.info',
  modelCode: 'host',
  bodyMode: 'items',
  authStyle: 'bearer',
  targets: [] as HttpPushTargetForm[],
  scheduleEnabled: false,
  intervalSecs: 300,
  pullEnabled: false,
  pullCron: '0 */2 * * *',
})

function openCreate() {
  // 重置表单到默认值
  createForm.code = ''
  createForm.name = ''
  createForm.sourceType = 'pull'
  createForm.apiUrl = ''
  createForm.apiToken = ''
  createForm.webhookSecret = ''
  createForm.path = ''
  createForm.method = 'GET'
  createForm.responsePath = 'data.info'
  createForm.modelCode = 'host'
  createForm.bodyMode = 'items'
  createForm.authStyle = 'bearer'
  createForm.targets = [emptyPushTarget()]
  createForm.scheduleEnabled = false
  createForm.intervalSecs = 300
  createForm.pullEnabled = false
  createForm.pullCron = '0 */2 * * *'
  createVisible.value = true
}

function onCreateTypeChange(v: string) {
  if (v === 'http_push') {
    createForm.method = 'POST'
    createForm.pullEnabled = false
    if (!createForm.targets.length) createForm.targets.push(emptyPushTarget())
  } else if (v === 'webhook') {
    createForm.pullEnabled = false
    if (createForm.method === 'PUT') createForm.method = 'GET'
  } else if (createForm.method === 'PUT') {
    createForm.method = 'GET'
  }
}

async function onCreate() {
  // 前端基础校验
  if (!createForm.code.trim() || !createForm.name.trim()) {
    ElMessage.warning('编码和名称不能为空')
    return
  }
  if (!/^[a-z0-9_]{1,32}$/.test(createForm.code)) {
    ElMessage.warning('编码只能包含小写字母、数字、下划线，最长 32 字符')
    return
  }
  if (createForm.sourceType === 'pull' && createForm.pullEnabled && !(createForm.intervalSecs >= 30)) {
    ElMessage.warning('启用定时拉取时间隔至少 30 秒')
    return
  }
  // pull / 出站必须有 apiUrl
  if ((createForm.sourceType === 'pull' || createForm.sourceType === 'http_push') && !createForm.apiUrl.trim()) {
    ElMessage.warning(createForm.sourceType === 'http_push' ? '出站推送必须填写 API 地址' : '拉取模式必须填写 API 地址')
    return
  }

  if (createForm.sourceType === 'http_push') {
    if (!createForm.targets.length) {
      ElMessage.warning('请至少配置一张外表')
      return
    }
    if (createForm.targets.some((t) => t.contentSource === 'sql' && !t.sql.trim())) {
      ElMessage.warning('SQL 外表必须填写查询语句')
      return
    }
  }

  const pullConfig = createForm.sourceType === 'http_push'
    ? JSON.stringify({
        authStyle: createForm.authStyle,
        rejectEmpty: true,
        scheduleEnabled: createForm.scheduleEnabled,
        intervalSecs: Math.max(30, createForm.intervalSecs || 300),
        targets: serializePushTargets(createForm.targets),
      })
    : JSON.stringify({
        method: createForm.method,
        path: createForm.path,
        responsePath: createForm.responsePath,
        modelCode: createForm.modelCode,
        intervalSecs: Math.max(30, createForm.intervalSecs || 300),
      })
  try {
    createSaving.value = true
    await createSyncSource({
      code: createForm.code.trim(),
      name: createForm.name.trim(),
      sourceType: createForm.sourceType,
      apiUrl: createForm.apiUrl,
      apiToken: createForm.apiToken,
      webhookSecret: createForm.webhookSecret,
      pullConfig,
      pullCron: '',
      pullEnabled: createForm.sourceType === 'pull' ? createForm.pullEnabled : false,
    })
    ElMessage.success('数据源已创建')
    createVisible.value = false
    await fetchSources()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '创建失败')
  } finally {
    createSaving.value = false
  }
}

// ---- 删除数据源 ----
async function onDelete(src: SyncSource) {
  try {
    await ElMessageBox.confirm(
      `确定要删除数据源「${src.name}」(${src.code}) 吗？\n若有关联的 CI 实例或同步日志将拒绝删除。`,
      '删除确认',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' },
    )
  } catch {
    return // 用户取消
  }
  try {
    await deleteSyncSource(src.code)
    ElMessage.success('数据源已删除')
    fetchSources()
    fetchLogs()
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : '删除失败')
  }
}

// ---- 初始化 ----
onMounted(async () => {
  await dicts.load('sync_action', 'sync_status')
  await Promise.all([fetchSources(), fetchLookupStatus(), fetchLogs()])
})
</script>

<style scoped>
.sync-page { padding: 0; }

/* 页头 */
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
.header-title h2 {
  margin: 0;
  font-size: 20px;
  color: #303133;
}
.header-desc {
  display: block;
  font-size: 13px;
  color: #909399;
  margin-top: 4px;
}
.section-head {
  display: flex;
  align-items: baseline;
  gap: 12px;
  margin: 4px 0 12px;
}
.section-head h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  color: #303133;
}
.section-desc {
  font-size: 12px;
  color: #909399;
}

/* 数据源卡片 */
.sources-row {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
  gap: 16px;
  margin-bottom: 20px;
}
.source-card {
  background: #fff;
  border-radius: 10px;
  box-shadow: 0 1px 4px rgba(0,0,0,0.06);
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-height: 248px;
}
.eventide-card {
  border: 1px solid #e8eef8;
}
.source-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.source-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}
.source-tags {
  display: flex;
  gap: 6px;
}
.source-meta {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.meta-row {
  display: flex;
  font-size: 13px;
  line-height: 1.5;
}
.meta-label {
  width: 80px;
  color: #909399;
  flex-shrink: 0;
}
.meta-value {
  color: #606266;
  word-break: break-all;
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.sync-time { font-size: 12px; color: #909399; }
.text-muted { color: #c0c4cc; font-size: 12px; }

.source-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding-top: 8px;
  border-top: 1px solid #f0f0f0;
  margin-top: auto;
}
.form-inline-tip {
  margin-left: 8px;
  font-size: 12px;
  color: #909399;
}
.target-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
  width: 100%;
}
.target-card {
  border: 1px solid #ebeef5;
  border-radius: 8px;
  padding: 12px 12px 8px;
  background: #fafbfc;
}
.target-card-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 8px;
}

/* 日志卡片 */
.logs-card { margin-top: 4px; }
.logs-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.logs-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
}
.logs-filter {
  display: flex;
  gap: 8px;
}

/* 分页 */
.pagination-row {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

/* 表单提示 */
.form-tip {
  font-size: 12px;
  color: #909399;
  line-height: 1.4;
  margin-top: 4px;
}
.preview-head {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 12px;
  font-size: 13px;
  color: #303133;
}
.preview-target { margin-bottom: 16px; }
.preview-target-title {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 8px;
}
.log-payload {
  padding: 4px 8px 8px 36px;
}
.payload-tip {
  font-size: 12px;
  color: #909399;
  margin-bottom: 8px;
}
.payload-val {
  word-break: break-all;
}
.payload-json {
  margin: 0;
  max-height: 360px;
  overflow: auto;
  padding: 10px 12px;
  background: #f5f7fa;
  border-radius: 6px;
  font-size: 12px;
  line-height: 1.5;
}
</style>
