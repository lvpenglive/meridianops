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
}

export interface MonitorTemplateRef {
  templateId: string
  name: string
}

export interface MonitorHost {
  hostId: string
  hostname: string
  displayName: string
  ip: string
  agentAvailable: boolean | null
  agentVersion: string | null
  ciId?: string | null
  cmdbLinked?: boolean
  groups?: MonitorGroupRef[]
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

export function listHosts(code: string, search?: string): Promise<MonitorHost[]> {
  return request.get(`/monitor/instances/${code}/hosts`, {
    params: { search: search || undefined },
  })
}

export function hostProblems(code: string, hostId: string): Promise<MonitorProblem[]> {
  return request.get(`/monitor/instances/${code}/hosts/${hostId}/problems`)
}

export function hostMetrics(code: string, hostId: string, hours = 24): Promise<MetricSeries[]> {
  return request.get(`/monitor/instances/${code}/hosts/${hostId}/metrics`, { params: { hours } })
}

export function syncHosts(code: string): Promise<{ synced: number }> {
  return request.post(`/monitor/instances/${code}/sync`)
}

export interface GovernanceProxy {
  proxyId: string
  name: string
  lastAccess: number
  ageSecs: number | null
  online: boolean
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

export function registerMonitorHost(
  code: string,
  data: { ciId: string; groupIds: string[]; templateIds?: string[] },
): Promise<{ hostId: string; created: boolean }> {
  return request.post(`/monitor/instances/${code}/hosts`, data)
}

export function installAgents(data: {
  instanceCode: string
  ciIds: string[]
  groupIds: string[]
  templateIds: string[]
  server: string
  serverActive?: string
  concurrency?: number
}): Promise<{ id: string }> {
  return request.post('/monitor/tasks', { ...data, action: 'install' })
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
