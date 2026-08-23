import { ref, type Ref } from 'vue'
import { type DictItem } from '../api/dict'
import { loadDictType, labelOf } from './useSystemDicts'

export type { DictItem }
export { labelOf }

export interface TicketDicts {
  statuses: Ref<DictItem[]>
  priorities: Ref<DictItem[]>
  categories: Ref<DictItem[]>
  nodeKinds: Ref<DictItem[]>
  nodeStatuses: Ref<DictItem[]>
  actions: Ref<DictItem[]>
  decisions: Ref<DictItem[]>
  ticketTypes: Ref<DictItem[]>
  approverSelectors: Ref<DictItem[]>
  loading: Ref<boolean>
  load: () => Promise<void>
}

export function useTicketDicts(): TicketDicts {
  const statuses = ref<DictItem[]>([])
  const priorities = ref<DictItem[]>([])
  const categories = ref<DictItem[]>([])
  const nodeKinds = ref<DictItem[]>([])
  const nodeStatuses = ref<DictItem[]>([])
  const actions = ref<DictItem[]>([])
  const decisions = ref<DictItem[]>([])
  const ticketTypes = ref<DictItem[]>([])
  const approverSelectors = ref<DictItem[]>([])
  const loading = ref(false)

  async function load() {
    loading.value = true
    try {
      const [
        s, p, c, nk, ns, a, d, tt, asel
      ] = await Promise.all([
        loadDictType('ticket_status'),
        loadDictType('ticket_priority'),
        loadDictType('ticket_category'),
        loadDictType('workflow_node_kind'),
        loadDictType('workflow_node_status'),
        loadDictType('workflow_action'),
        loadDictType('workflow_decision'),
        loadDictType('workflow_ticket_type'),
        loadDictType('workflow_approver_selector'),
      ])
      statuses.value = s
      priorities.value = p
      categories.value = c
      nodeKinds.value = nk
      nodeStatuses.value = ns
      actions.value = a
      decisions.value = d
      ticketTypes.value = tt
      approverSelectors.value = asel
    } finally {
      loading.value = false
    }
  }

  return {
    statuses, priorities, categories, nodeKinds, nodeStatuses,
    actions, decisions, ticketTypes, approverSelectors,
    loading, load,
  }
}
