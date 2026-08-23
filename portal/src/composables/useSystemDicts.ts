import { ref, type Ref } from 'vue'
import { listDictItems, type DictItem } from '../api/dict'

export interface SystemDicts {
  auditActions: Ref<DictItem[]>
  auditResults: Ref<DictItem[]>
  alertSeverities: Ref<DictItem[]>
  alertStatuses: Ref<DictItem[]>
  alertSources: Ref<DictItem[]>
  syncActions: Ref<DictItem[]>
  syncStatuses: Ref<DictItem[]>
  cmdbActions: Ref<DictItem[]>
  jobRunStatuses: Ref<DictItem[]>
  agentStatuses: Ref<DictItem[]>
  sensitiveActions: Ref<DictItem[]>
  syncSourceTypes: Ref<DictItem[]>
  modelIcons: Ref<DictItem[]>
  loading: Ref<boolean>
  load: (...types: string[]) => Promise<void>
}

const cache: Record<string, DictItem[]> = {}

export async function loadDictType(code: string): Promise<DictItem[]> {
  if (cache[code]) return cache[code]
  const items = await listDictItems(code)
  cache[code] = items
  return items
}

export function clearDictCache(code?: string) {
  if (code) {
    delete cache[code]
  } else {
    for (const k of Object.keys(cache)) delete cache[k]
  }
}

export function labelOf(items: DictItem[], value: string): string {
  const item = items.find(i => i.value === value)
  return item?.label || value
}

const TYPE_MAP = {
  audit_action: 'auditActions',
  audit_result: 'auditResults',
  alert_severity: 'alertSeverities',
  alert_status: 'alertStatuses',
  alert_source: 'alertSources',
  sync_action: 'syncActions',
  sync_status: 'syncStatuses',
  cmdb_action: 'cmdbActions',
  job_run_status: 'jobRunStatuses',
  agent_status: 'agentStatuses',
  sensitive_action: 'sensitiveActions',
  sync_source_type: 'syncSourceTypes',
  cmdb_model_icon: 'modelIcons',
} as const

export function useSystemDicts(): SystemDicts {
  const auditActions = ref<DictItem[]>([])
  const auditResults = ref<DictItem[]>([])
  const alertSeverities = ref<DictItem[]>([])
  const alertStatuses = ref<DictItem[]>([])
  const alertSources = ref<DictItem[]>([])
  const syncActions = ref<DictItem[]>([])
  const syncStatuses = ref<DictItem[]>([])
  const cmdbActions = ref<DictItem[]>([])
  const jobRunStatuses = ref<DictItem[]>([])
  const agentStatuses = ref<DictItem[]>([])
  const sensitiveActions = ref<DictItem[]>([])
  const syncSourceTypes = ref<DictItem[]>([])
  const modelIcons = ref<DictItem[]>([])
  const loading = ref(false)

  const refs: Record<string, Ref<DictItem[]>> = {
    auditActions,
    auditResults,
    alertSeverities,
    alertStatuses,
    alertSources,
    syncActions,
    syncStatuses,
    cmdbActions,
    jobRunStatuses,
    agentStatuses,
    sensitiveActions,
    syncSourceTypes,
    modelIcons,
  }

  async function load(...types: string[]) {
    const toLoad = types.length > 0 ? types : Object.keys(TYPE_MAP)
    loading.value = true
    try {
      const results = await Promise.all(toLoad.map(t => loadDictType(t)))
      toLoad.forEach((code, i) => {
        const refKey = (TYPE_MAP as Record<string, string>)[code]
        if (refKey && refs[refKey]) {
          refs[refKey].value = results[i]
        }
      })
    } finally {
      loading.value = false
    }
  }

  return {
    auditActions,
    auditResults,
    alertSeverities,
    alertStatuses,
    alertSources,
    syncActions,
    syncStatuses,
    cmdbActions,
    jobRunStatuses,
    agentStatuses,
    sensitiveActions,
    syncSourceTypes,
    modelIcons,
    loading,
    load,
  }
}
