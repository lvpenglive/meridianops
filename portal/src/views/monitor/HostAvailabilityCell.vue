<template>
  <div class="zbx-avail">
    <template v-if="badges.length">
      <el-tooltip
        v-for="item in badges"
        :key="item.key"
        placement="top"
        :show-after="200"
        raw-content
        :content="item.tipHtml"
      >
        <span class="zbx-avail-badge" :class="item.cls">{{ item.label }}</span>
      </el-tooltip>
    </template>
    <span v-else class="zbx-avail-empty">—</span>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { HostInterfaceAvailability, MonitorHost } from '../../api/monitor'

const props = defineProps<{
  host: MonitorHost | null | undefined
}>()

const AVAIL_META = [
  { key: 'zbx', label: 'ZBX', name: 'Zabbix agent' },
  { key: 'snmp', label: 'SNMP', name: 'SNMP' },
  { key: 'ipmi', label: 'IPMI', name: 'IPMI' },
  { key: 'jmx', label: 'JMX', name: 'JMX' },
] as const

function escapeHtml(s: string) {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

function formatZbxTime(ts?: number | null) {
  if (!ts || ts <= 0) return ''
  const d = new Date(ts * 1000)
  if (Number.isNaN(d.getTime())) return ''
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
}

function normalizeAvail(
  raw: HostInterfaceAvailability | number | null | undefined,
): HostInterfaceAvailability | null {
  if (raw === null || raw === undefined) return null
  if (typeof raw === 'number') return { status: raw, error: '', errorsFrom: 0, disableUntil: 0 }
  return raw
}

function tipHtml(name: string, detail: HostInterfaceAvailability) {
  const status = detail.status
  const stateLabel = status === 1 ? '可用' : status === 2 ? '不可用' : '未知'
  const lines: string[] = [
    `<div class="zbx-avail-tip-title">${escapeHtml(name)}：${stateLabel}</div>`,
  ]
  const since = formatZbxTime(detail.errorsFrom)
  if (status === 2 && since) {
    lines.push(`<div>不可用起始时间：${escapeHtml(since)}</div>`)
  }
  const until = formatZbxTime(detail.disableUntil)
  if (status === 2 && until) {
    lines.push(`<div>下次探测：${escapeHtml(until)}</div>`)
  }
  const err = (detail.error || '').trim()
  if (err) {
    lines.push(`<div class="zbx-avail-tip-error">错误：${escapeHtml(err)}</div>`)
  }
  return `<div class="zbx-avail-tip">${lines.join('')}</div>`
}

const badges = computed(() => {
  const avail = props.host?.availability
  if (!avail) return [] as { key: string; label: string; cls: string; tipHtml: string }[]
  return AVAIL_META.flatMap((meta) => {
    const detail = normalizeAvail(avail[meta.key])
    if (!detail) return []
    const state =
      detail.status === 1 ? 'available' : detail.status === 2 ? 'unavailable' : 'unknown'
    return [
      {
        key: meta.key,
        label: meta.label,
        cls: `is-${state}`,
        tipHtml: tipHtml(meta.name, detail),
      },
    ]
  })
})
</script>

<style scoped>
.zbx-avail {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 2px;
  align-items: center;
}
.zbx-avail-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 34px;
  height: 18px;
  padding: 0 4px;
  border-radius: 2px;
  font-size: 11px;
  font-weight: 600;
  line-height: 1;
  color: #fff;
  letter-spacing: 0.02em;
  cursor: default;
}
.zbx-avail-badge.is-available {
  background: #28a745;
}
.zbx-avail-badge.is-unavailable {
  background: #e33734;
}
.zbx-avail-badge.is-unknown {
  background: #768d99;
}
.zbx-avail-empty {
  color: var(--el-text-color-placeholder);
  font-size: 13px;
}
</style>

<style>
.zbx-avail-tip {
  max-width: 420px;
  text-align: left;
  line-height: 1.45;
  white-space: normal;
  word-break: break-word;
}
.zbx-avail-tip-title {
  font-weight: 600;
  margin-bottom: 4px;
}
.zbx-avail-tip-error {
  margin-top: 4px;
  color: #ffb4b0;
}
</style>
