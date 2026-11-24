import request from './request'

export interface MonitorInstance {
  code: string
  name: string
  tokenConfigured: boolean
  protected?: boolean
}

export interface MonitorGroupRef {
  groupId: string
  name: string
  hostCount?: number
}

/** 代理生命周期状态（一期） */
export type AgentLifecycleStatus = 'running' | 'stopped' | 'absent' | 'unknown'

export interface AgentStatusSummary {
  running: number
  stopped: number
  absent: number
  unknown: number
}

export interface HostRoster {
  total: number
  matched: number
  page: number
  pageSize: number
  groups: MonitorGroupRef[]
  agentSummary?: AgentStatusSummary
  items: MonitorHost[]
}

export interface MonitorTemplateRef {
  templateId: string
  name: string
}

export interface MonitorHost {
  hostId: string
  hostname: string
  displayName: string
  /** Zabbix status：0 启用 / 1 禁用 */
  status?: string
  ip: string
  /** Agent 接口端口（Zabbix host interface） */
  port?: string | null
  /** Agent 通断：以 agent.ping 为准 */
  agentAvailable: boolean | null
  /**
   * 代理生命周期：absent 无接口 / running 运行中 / stopped 未运行 / unknown 未知
   * stopped 一期含停服与链路不可达，二期再拆
   */
  agentStatus?: AgentLifecycleStatus | null
  /** Zabbix 接口可用性汇总（主接口） */
  interfaceAvailable?: boolean | null
  installPath?: string | null
  packageId?: string | null
  /**
   * Zabbix 同款按类型可用性；null 表示无该类型接口。
   * status：0 未知 / 1 可用 / 2 不可用；error / errorsFrom 用于悬停提示。
   */
  availability?: {
    zbx?: HostInterfaceAvailability | null
    snmp?: HostInterfaceAvailability | null
    ipmi?: HostInterfaceAvailability | null
    jmx?: HostInterfaceAvailability | null
  } | null
  agentVersion: string | null
  ciId?: string | null
  cmdbLinked?: boolean
  /** 关联配置项是否已绑定 SSH 凭据 */
  sshCredentialBound?: boolean
  sshCredentialId?: number | null
  /** 资产 SSH 通道：auto|direct */
  sshExecMode?: string | null
  /** Zabbix 监控通道：direct|proxy|proxyGroup */
  channel?: string | null
  proxyId?: string | null
  proxyGroupId?: string | null
  /** SSH 运维路径：direct 直连 / jump 经 ProxyJump */
  sshPath?: 'direct' | 'jump' | string | null
  /** 是否已配置可用 SSH 跳板 */
  sshJumpConfigured?: boolean
  groups?: MonitorGroupRef[]
}

export interface HostInterfaceAvailability {
  status: number
  error?: string
  errorsFrom?: number
  disableUntil?: number
}

export interface MonitorProblem {
  eventId: string
  name: string
  severity: string
  clock: string
}

export interface MetricPoint {
  clock: string
  value: string
}

export interface MetricSeries {
  name: string
  key: string | null
  points: MetricPoint[]
}

export type MonitorAction = 'start' | 'stop' | 'restart' | 'upgrade' | 'rollback'

export interface MonitorTaskHost {
  hostId: string
  ciId?: string | null
  jobRunId?: number | null
  versionBefore?: string | null
  versionAfter?: string | null
  status: string
  error?: string | null
}

export interface MonitorTask {
  id: string
  instanceCode: string
  action: string
  targetVersion?: string | null
  concurrency: number
  maintenanceId?: string | null
  status: string
  requestedBy: string
  error?: string | null
  hosts: MonitorTaskHost[]
}

export function listInstances(): Promise<MonitorInstance[]> {
  return request.get('/monitor/instances')
}

export function listHosts(
  code: string,
  params: {
    search?: string
    page?: number
    pageSize?: number
    groupId?: string
    agent?: 'online' | 'offline' | 'unknown'
    agentStatus?: AgentLifecycleStatus
    availability?: 'unavailable' | 'available' | 'unknown'
    cmdb?: 'linked' | 'unlinked'
    status?: '0' | '1' | 'enabled' | 'disabled'
  } = {},
): Promise<HostRoster> {
  return request.get(`/monitor/instances/${code}/hosts`, {
    params: {
      search: params.search || undefined,
      page: params.page,
      pageSize: params.pageSize,
      groupId: params.groupId || undefined,
      agent: params.agent,
      agentStatus: params.agentStatus,
      availability: params.availability,
      cmdb: params.cmdb,
      status: params.status,
    },
  })
}

export function hostProblems(code: string, hostId: string): Promise<MonitorProblem[]> {
  return request.get(`/monitor/instances/${code}/hosts/${hostId}/problems`)
}

export function hostMetrics(code: string, hostId: string, hours = 24): Promise<MetricSeries[]> {
  return request.get(`/monitor/instances/${code}/hosts/${hostId}/metrics`, { params: { hours } })
}

export interface HostItem {
  itemId: string
  name: string
  key: string
  lastValue: string
  lastClock: string
  units: string
  status: string
  state: string
  valueType: string
  inherited: boolean
  templateId?: string
  templateName?: string
  error: string
}

export interface HostTrigger {
  triggerId: string
  name: string
  severity: string
  value: string
  status: string
  state: string
  lastChange: string
  inherited: boolean
  templateId?: string
  templateName?: string
  error: string
}

export function hostItems(code: string, hostId: string, search?: string): Promise<HostItem[]> {
  return request.get(`/monitor/instances/${code}/hosts/${hostId}/items`, {
    params: { search: search || undefined },
  })
}

export function hostTriggers(code: string, hostId: string): Promise<HostTrigger[]> {
  return request.get(`/monitor/instances/${code}/hosts/${hostId}/triggers`)
}

export function syncHosts(code: string, hostIds?: string[]): Promise<{ synced: number }> {
  // 全量对照可能较慢；传 hostIds 时只同步勾选主机
  return request.post(
    `/monitor/instances/${code}/sync`,
    hostIds?.length ? { hostIds } : {},
    { timeout: 300000 },
  )
}

export interface GovernanceProxy {
  proxyId: string
  name: string
  address?: string
  lastAccess: number
  ageSecs: number | null
  online: boolean
}

export interface MonitorProxy {
  proxyId: string
  name: string
  address: string
  lastAccess: number
  ageSecs: number | null
  online: boolean
}

export function listProxies(code: string): Promise<MonitorProxy[]> {
  return request.get(`/monitor/instances/${code}/proxies`)
}

export interface MonitorProxyGroupMember {
  proxyId: string
  name: string
  address: string
  online: boolean
}

export interface MonitorProxyGroup {
  proxyGroupId: string
  name: string
  state: number
  online: boolean
  proxies: MonitorProxyGroupMember[]
  addresses: string[]
  serverCsv: string
  serverActiveSemicolon: string
}

export function listProxyGroups(code: string): Promise<MonitorProxyGroup[]> {
  return request.get(`/monitor/instances/${code}/proxy-groups`)
}

export interface GovernanceDrift {
  hostId: string
  hostname: string
  displayName: string
  templates: string[]
  missing: string[]
  extra: string[]
  drifted: boolean
}

export interface GovernanceQueue {
  available: boolean
  count: number
  warn: number
  backedUp: boolean
  source?: string
  message?: string
}

export interface GovernanceData {
  proxies: GovernanceProxy[]
  offlineProxyCount: number
  templateDrift: GovernanceDrift[]
  driftCount: number
  queue: GovernanceQueue
  standardTemplates: string[]
}

export function getGovernance(code: string): Promise<GovernanceData> {
  return request.get(`/monitor/instances/${code}/governance`)
}

export function createTask(data: {
  instanceCode: string
  action: MonitorAction
  hostIds: string[]
  targetVersion?: string
  packageId?: string
  concurrency?: number
}): Promise<{ id: string }> {
  return request.post('/monitor/tasks', data)
}

export function getTask(id: string): Promise<MonitorTask> {
  return request.get(`/monitor/tasks/${id}`)
}

export function listTemplateGroups(code: string): Promise<MonitorGroupRef[]> {
  return request.get(`/monitor/instances/${code}/template-groups`)
}

export function listHostGroups(code: string): Promise<MonitorGroupRef[]> {
  return request.get(`/monitor/instances/${code}/host-groups`)
}

export function hostsInGroup(code: string, groupId: string): Promise<MonitorHost[]> {
  return request.get(`/monitor/instances/${code}/host-groups/${groupId}/hosts`)
}

export interface MonitorTemplate {
  templateId: string
  host: string
  name: string
  groups: MonitorGroupRef[]
  parentTemplates: MonitorTemplateRef[]
}

export function listTemplates(code: string, groupId?: string): Promise<MonitorTemplate[]> {
  return request.get(`/monitor/instances/${code}/templates`, {
    params: { groupId: groupId || undefined },
  })
}

export interface HostLinkage {
  groups: MonitorGroupRef[]
  templates: MonitorTemplateRef[]
}

export function hostLinkage(code: string, hostId: string): Promise<HostLinkage> {
  return request.get(`/monitor/instances/${code}/hosts/${hostId}/linkage`)
}

export function linkHostTemplates(
  code: string,
  hostId: string,
  action: 'link' | 'unlink',
  templateIds: string[],
): Promise<HostLinkage> {
  return request.post(`/monitor/instances/${code}/hosts/${hostId}/templates`, { action, templateIds })
}

export function linkHostGroups(
  code: string,
  hostId: string,
  action: 'link' | 'unlink',
  groupIds: string[],
): Promise<HostLinkage> {
  return request.post(`/monitor/instances/${code}/hosts/${hostId}/groups`, { action, groupIds })
}

export interface HostMacro {
  hostMacroId: string
  macro: string
  value: string
  description: string
  secret: boolean
}

export function hostMacros(code: string, hostId: string): Promise<HostMacro[]> {
  return request.get(`/monitor/instances/${code}/hosts/${hostId}/macros`)
}

export function saveHostMacros(
  code: string,
  hostId: string,
  macros: Array<{ macro: string; value: string; description?: string }>,
): Promise<HostMacro[]> {
  return request.put(`/monitor/instances/${code}/hosts/${hostId}/macros`, { macros })
}

export interface HostInterfaceInput {
  interfaceId?: string
  type: number // 1 Agent 2 SNMP 3 IPMI 4 JMX
  main?: number | boolean
  useip?: number | boolean
  ip?: string
  dns?: string
  port?: string
  details?: Record<string, unknown>
  community?: string
  snmpVersion?: number
}

export interface CreateMonitorHostInput {
  hostname?: string
  displayName?: string
  status?: number | string
  ip?: string
  port?: string
  groupIds: string[]
  templateIds?: string[]
  /** direct | proxy | proxyGroup */
  channel?: 'direct' | 'proxy' | 'proxyGroup' | string
  proxyId?: string
  proxyGroupId?: string
  ciId?: string
  interfaces?: HostInterfaceInput[]
  macros?: Array<{ macro: string; value: string; type?: number; description?: string }>
  tags?: Array<{ tag: string; value?: string }>
  inventoryMode?: number
  inventory?: Record<string, string>
}

export function registerMonitorHost(
  code: string,
  data: CreateMonitorHostInput | { ciId: string; groupIds: string[]; templateIds?: string[] },
): Promise<{ hostId: string; created: boolean; ip?: string }> {
  return request.post(`/monitor/instances/${code}/hosts`, data)
}

export interface MonitorHostDetail {
  hostId: string
  hostname: string
  displayName: string
  status: string
  description?: string
  groupIds: string[]
  groups?: MonitorGroupRef[]
  templateIds: string[]
  templates?: MonitorTemplateRef[]
  channel: 'direct' | 'proxy' | 'proxyGroup' | string
  proxyId?: string
  proxyGroupId?: string
  interfaces: Array<HostInterfaceInput & {
    interfaceId?: string
    main?: boolean | number
    snmpVersion?: number
    community?: string
    contextName?: string
    securityName?: string
    securityLevel?: number
    authPassphrase?: string
    privPassphrase?: string
  }>
  tags: Array<{ tag: string; value?: string }>
  inventoryMode: number
  inventory: Record<string, string>
  macros: HostMacro[]
}

export type UpdateMonitorHostInput = CreateMonitorHostInput & {
  status?: number | string
  description?: string
}

export function getMonitorHost(code: string, hostId: string): Promise<MonitorHostDetail> {
  return request.get(`/monitor/instances/${code}/hosts/${hostId}`)
}

export function updateMonitorHost(
  code: string,
  hostId: string,
  data: UpdateMonitorHostInput,
): Promise<MonitorHostDetail> {
  return request.put(`/monitor/instances/${code}/hosts/${hostId}`, data)
}

export function deleteMonitorHosts(
  code: string,
  hostIds: string[],
): Promise<{ hostIds: string[]; deleted: number }> {
  return request.post(`/monitor/instances/${code}/hosts/delete`, { hostIds })
}

export interface MassUpdateHostsInput {
  hostIds: string[]
  status?: number | string
  groupIds?: string[]
  templateIds?: string[]
  /** replace | link | unlink */
  templateAction?: 'replace' | 'link' | 'unlink'
  tags?: Array<{ tag: string; value?: string }>
  inventoryMode?: number
  inventory?: Record<string, string>
  channel?: 'direct' | 'proxy' | 'proxyGroup'
  proxyId?: string
  proxyGroupId?: string
}

export function massUpdateMonitorHosts(
  code: string,
  data: MassUpdateHostsInput,
): Promise<{ hostIds: string[]; updated: number }> {
  return request.post(`/monitor/instances/${code}/hosts/massupdate`, data)
}

export function installAgents(data: {
  instanceCode: string
  ciIds: string[]
  groupIds: string[]
  templateIds: string[]
  server?: string
  agentPort?: string
  serverActive?: string
  proxyId?: string
  proxyGroupId?: string
  packageId: string
  installPath?: string
  confSuffix?: string
  /** 验证 agent.ping 通过后禁用主机，避免立刻告警 */
  disableAfterInstall?: boolean
  concurrency?: number
}): Promise<{ id: string }> {
  return request.post('/monitor/tasks', { ...data, action: 'install' })
}

export interface AgentPackage {
  id: string
  name: string
  version: string
  osArch?: string | null
  agentKind?: string | null
  filename: string
  fileSize: number
  sha256?: string | null
  description?: string | null
  enabled: boolean
  createdBy?: string | null
  createdAt?: string | null
  defaultInstallPath?: string
}

export function listAgentPackages(): Promise<AgentPackage[]> {
  return request.get('/monitor/agent-packages')
}

/** 安装页默认目录，来自 Gateway toml `[zabbix_ctl].default_install_path`，与具体包无关。 */
export interface AgentInstallDefaults {
  defaultInstallPath: string
}

export function getAgentInstallDefaults(): Promise<AgentInstallDefaults> {
  return request.get('/monitor/agent-install-defaults')
}

export function uploadAgentPackage(form: FormData): Promise<AgentPackage> {
  return request.post('/monitor/agent-packages', form, { timeout: 300000 })
}

export function updateAgentPackage(
  id: string,
  data: Partial<Pick<AgentPackage, 'name' | 'version' | 'osArch' | 'agentKind' | 'description' | 'enabled'>>,
): Promise<AgentPackage> {
  return request.put(`/monitor/agent-packages/${id}`, data)
}

export function deleteAgentPackage(id: string): Promise<{ id: string }> {
  return request.delete(`/monitor/agent-packages/${id}`)
}

export interface ProxyConfMap {
  id: number
  instanceCode: string
  proxyGroupId: string
  confSuffix: string
  /** 组内用作 SSH 跳板的 Proxy ID */
  jumpProxyId?: string | null
  remark?: string | null
  createdAt?: string | null
}

export function listProxyConfMaps(): Promise<ProxyConfMap[]> {
  return request.get('/monitor/proxy-conf-maps')
}

export function upsertProxyConfMap(data: {
  instanceCode: string
  proxyGroupId: string
  confSuffix: string
  jumpProxyId?: string
  remark?: string
}): Promise<ProxyConfMap[]> {
  return request.post('/monitor/proxy-conf-maps', data)
}

export function deleteProxyConfMap(id: number): Promise<{ id: number }> {
  return request.delete(`/monitor/proxy-conf-maps/${id}`)
}

/** Proxy SSH 跳板：Gateway 经该主机再 SSH 到业务机 */
export interface MonitorSshJump {
  id: number
  instanceCode: string
  proxyId: string
  proxyName?: string | null
  jumpHost: string
  jumpPort: number
  credentialId: number
  credentialName?: string | null
  enabled: boolean
  remark?: string | null
  createdAt?: string | null
}

export function listSshJumps(): Promise<MonitorSshJump[]> {
  return request.get('/monitor/ssh-jumps')
}

export function upsertSshJump(data: {
  instanceCode: string
  proxyId: string
  proxyName?: string
  jumpHost: string
  jumpPort?: number
  credentialId: number
  enabled?: boolean
  remark?: string
}): Promise<MonitorSshJump[]> {
  return request.post('/monitor/ssh-jumps', data)
}

export function deleteSshJump(id: number): Promise<{ id: number }> {
  return request.delete(`/monitor/ssh-jumps/${id}`)
}

export function ackMonitorEvent(
  code: string,
  eventId: string,
  data: { message?: string; severity?: number; close?: boolean },
): Promise<{ eventId: string }> {
  return request.post(`/monitor/instances/${code}/events/${eventId}/ack`, data)
}

export interface MonitorCatalog {
  version: string
  itemTypes: { type: number; name: string; editable: boolean; fields: string[] }[]
  valueTypes: { value: number; name: string }[]
  preprocessing: { type: number; name: string; paramsHint: string }[]
  severities: { value: number; name: string }[]
  functions: { name: string; insert: string; hint: string }[]
}

export interface DesignerTemplate {
  templateId: string
  host: string
  name: string
  groups: MonitorGroupRef[]
  parentTemplates: MonitorTemplateRef[]
}

export interface DesignerItem {
  itemId: string
  name: string
  key: string
  type: number
  typeName: string
  valueType: number
  delay: string
  units: string
  status: number
  description: string
  inherited: boolean
  editable: boolean
  masterItemId: string
  snmpOid: string
  url: string
  requestMethod: number
  statusCodes: string
  params: string
  preprocessing: { type: number; params: string; errorHandler: number; errorHandlerParams: string }[]
}

export interface DesignerTrigger {
  triggerId: string
  description: string
  expression: string
  priority: number
  status: number
  recoveryMode: number
  recoveryExpression: string
  inherited: boolean
  editable: boolean
}

export interface DesignerMacro {
  hostMacroId: string
  macro: string
  value: string
  description: string
  secret: boolean
}

export interface TemplateChange {
  id: string
  ticketId: string
  title: string
  method: string
  path: string
  status: string
  requestedBy: string
  createdAt?: string | null
  appliedAt?: string | null
}

export function monitorCatalog(): Promise<MonitorCatalog> {
  return request.get('/monitor/catalog')
}

export function getDesignerTemplate(code: string, templateId: string): Promise<DesignerTemplate> {
  return request.get(`/monitor/instances/${code}/templates/${templateId}`)
}

export function createDesignerTemplate(code: string, data: {
  host: string
  name: string
  groupIds: string[]
  parentTemplateIds: string[]
}): Promise<{ templateId: string }> {
  return request.post(`/monitor/instances/${code}/templates`, data)
}

export function updateDesignerTemplate(code: string, templateId: string, data: {
  host: string
  name: string
  groupIds: string[]
  parentTemplateIds: string[]
}): Promise<DesignerTemplate> {
  return request.put(`/monitor/instances/${code}/templates/${templateId}`, data)
}

export function deleteDesignerTemplate(code: string, templateId: string): Promise<{ templateId: string }> {
  return request.delete(`/monitor/instances/${code}/templates/${templateId}`)
}

export function createMonitorGroup(code: string, kind: 'template' | 'host', name: string): Promise<{ groupId: string }> {
  const path = kind === 'template' ? 'template-groups' : 'host-groups'
  return request.post(`/monitor/instances/${code}/${path}`, { name })
}

export function renameMonitorGroup(code: string, kind: 'template' | 'host', groupId: string, name: string) {
  const path = kind === 'template' ? 'template-groups' : 'host-groups'
  return request.put(`/monitor/instances/${code}/${path}/${groupId}`, { name })
}

export function deleteMonitorGroup(code: string, kind: 'template' | 'host', groupId: string) {
  const path = kind === 'template' ? 'template-groups' : 'host-groups'
  return request.delete(`/monitor/instances/${code}/${path}/${groupId}`)
}

export function listDesignerItems(code: string, templateId: string): Promise<DesignerItem[]> {
  return request.get(`/monitor/instances/${code}/templates/${templateId}/items`)
}

export function saveDesignerItem(code: string, templateId: string, item: Partial<DesignerItem> & { itemId?: string }) {
  const body = { ...item }
  if (item.itemId) {
    return request.put(`/monitor/instances/${code}/templates/${templateId}/items/${item.itemId}`, body)
  }
  return request.post(`/monitor/instances/${code}/templates/${templateId}/items`, body)
}

export function deleteDesignerItem(code: string, templateId: string, itemId: string) {
  return request.delete(`/monitor/instances/${code}/templates/${templateId}/items/${itemId}`)
}

export function listDesignerTriggers(code: string, templateId: string): Promise<DesignerTrigger[]> {
  return request.get(`/monitor/instances/${code}/templates/${templateId}/triggers`)
}

export function saveDesignerTrigger(code: string, templateId: string, trigger: Partial<DesignerTrigger> & { triggerId?: string }) {
  if (trigger.triggerId) {
    return request.put(`/monitor/instances/${code}/templates/${templateId}/triggers/${trigger.triggerId}`, trigger)
  }
  return request.post(`/monitor/instances/${code}/templates/${templateId}/triggers`, trigger)
}

export function deleteDesignerTrigger(code: string, templateId: string, triggerId: string) {
  return request.delete(`/monitor/instances/${code}/templates/${templateId}/triggers/${triggerId}`)
}

export function listDesignerMacros(code: string, templateId: string): Promise<DesignerMacro[]> {
  return request.get(`/monitor/instances/${code}/templates/${templateId}/macros`)
}

export function saveDesignerMacros(code: string, templateId: string, macros: DesignerMacro[]): Promise<DesignerMacro[]> {
  return request.put(`/monitor/instances/${code}/templates/${templateId}/macros`, { macros })
}

export function exportDesignerTemplates(code: string, templateIds: string[]): Promise<{ source: string }> {
  return request.post(`/monitor/instances/${code}/templates/export`, { templateIds }, { timeout: 60000 })
}

export function importDesignerTemplates(code: string, source: string): Promise<{ imported: boolean }> {
  return request.post(`/monitor/instances/${code}/templates/import`, { source }, { timeout: 60000 })
}

export function listTemplateChanges(code: string): Promise<TemplateChange[]> {
  return request.get(`/monitor/instances/${code}/template-changes`)
}

export function submitTemplateChange(code: string, data: {
  title: string
  method: 'POST' | 'PUT' | 'DELETE'
  resource: string
  body?: unknown
}): Promise<{ id: string; ticketId: string; ticketNo: string }> {
  return request.post(`/monitor/instances/${code}/template-changes`, data)
}

export function applyTemplateChange(id: string): Promise<unknown> {
  return request.post(`/monitor/template-changes/${id}/apply`)
}

export interface LldFilter {
  macro: string
  operator: number
  value: string
}

export interface LldOperation {
  operationObject: number
  operator: number
  value: string
  status: number | null
  discover: number | null
}

export interface LldOverride {
  name: string
  step: number
  stop: boolean
  filterEvalType: number
  filter: LldFilter[]
  operations: LldOperation[]
}

export interface DiscoveryRule {
  ruleId: string
  name: string
  key: string
  type: number
  typeName: string
  delay: string
  status: number
  lifetimeType: number
  lifetime: string
  description: string
  inherited: boolean
  editable: boolean
  filterEvalType: number
  filter: LldFilter[]
  overrides: LldOverride[]
}

export function listDiscoveryRules(code: string, templateId: string): Promise<DiscoveryRule[]> {
  return request.get(`/monitor/instances/${code}/templates/${templateId}/discovery-rules`)
}

export function saveDiscoveryRule(code: string, templateId: string, rule: Partial<DiscoveryRule> & { ruleId?: string }): Promise<{ ruleId: string }> {
  if (rule.ruleId) {
    return request.put(`/monitor/instances/${code}/templates/${templateId}/discovery-rules/${rule.ruleId}`, rule)
  }
  return request.post(`/monitor/instances/${code}/templates/${templateId}/discovery-rules`, rule)
}

export function deleteDiscoveryRule(code: string, templateId: string, ruleId: string) {
  return request.delete(`/monitor/instances/${code}/templates/${templateId}/discovery-rules/${ruleId}`)
}

export function listPrototypeItems(code: string, templateId: string, ruleId: string): Promise<DesignerItem[]> {
  return request.get(`/monitor/instances/${code}/templates/${templateId}/discovery-rules/${ruleId}/items`)
}

export function savePrototypeItem(code: string, templateId: string, ruleId: string, item: Partial<DesignerItem> & { itemId?: string }) {
  if (item.itemId) {
    return request.put(`/monitor/instances/${code}/templates/${templateId}/discovery-rules/${ruleId}/items/${item.itemId}`, item)
  }
  return request.post(`/monitor/instances/${code}/templates/${templateId}/discovery-rules/${ruleId}/items`, item)
}

export function deletePrototypeItem(code: string, templateId: string, ruleId: string, itemId: string) {
  return request.delete(`/monitor/instances/${code}/templates/${templateId}/discovery-rules/${ruleId}/items/${itemId}`)
}

export function listPrototypeTriggers(code: string, templateId: string, ruleId: string): Promise<DesignerTrigger[]> {
  return request.get(`/monitor/instances/${code}/templates/${templateId}/discovery-rules/${ruleId}/triggers`)
}

export function savePrototypeTrigger(code: string, templateId: string, ruleId: string, trigger: Partial<DesignerTrigger> & { triggerId?: string }) {
  if (trigger.triggerId) {
    return request.put(`/monitor/instances/${code}/templates/${templateId}/discovery-rules/${ruleId}/triggers/${trigger.triggerId}`, trigger)
  }
  return request.post(`/monitor/instances/${code}/templates/${templateId}/discovery-rules/${ruleId}/triggers`, trigger)
}

export function deletePrototypeTrigger(code: string, templateId: string, ruleId: string, triggerId: string) {
  return request.delete(`/monitor/instances/${code}/templates/${templateId}/discovery-rules/${ruleId}/triggers/${triggerId}`)
}
