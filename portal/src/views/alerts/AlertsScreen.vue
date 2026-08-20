<template>
  <div class="alert-screen" ref="screenRef">
    <!-- 顶栏 -->
    <header class="screen-header">
      <div class="header-left">
        <div class="brand">
          <el-icon class="brand-icon"><Bell /></el-icon>
          <div>
            <div class="brand-title">告警事件监控大屏</div>
            <div class="brand-sub">MERIDIANOPS · ALERT MONITORING CENTER</div>
          </div>
        </div>
      </div>
      <div class="header-center">
        <div class="clock">{{ nowText }}</div>
        <div class="clock-sub">{{ nowDate }}</div>
      </div>
      <div class="header-right">
        <div class="stat-cards">
          <div class="stat-card stat-current">
            <el-icon class="stat-icon"><Warning /></el-icon>
            <div class="stat-body">
              <div class="stat-label">当前告警</div>
              <div class="stat-value">{{ criticalActive }}</div>
            </div>
          </div>
          <div class="stat-card stat-today">
            <el-icon class="stat-icon"><Bell /></el-icon>
            <div class="stat-body">
              <div class="stat-label">今日告警</div>
              <div class="stat-value">{{ criticalToday }}</div>
            </div>
          </div>
        </div>
        <el-icon class="fs-btn" :title="isFullscreen ? '退出全屏' : '全屏'" @click="toggleFullscreen">
          <Aim v-if="isFullscreen" /><FullScreen v-else />
        </el-icon>
      </div>
    </header>

    <!-- 工具条 -->
    <div class="toolbar">
      <div class="toolbar-left">
        <span class="live-dot" :class="{ online: !busy && online, busy: busy }" />
        <span class="toolbar-title">实时事件列表</span>
        <span class="toolbar-count">共 {{ totalCount }} 条 · 当前 {{ filteredList.length }} 条</span>
      </div>
      <div class="toolbar-right">
        <div class="pager">
          <button class="pager-btn" :disabled="currentPage <= 1 || busy" @click="prevPage">‹</button>
          <span class="pager-info">{{ currentPage }} / {{ totalPages }}</span>
          <button class="pager-btn" :disabled="currentPage >= totalPages || busy" @click="nextPage">›</button>
        </div>
      </div>
    </div>

    <!-- 事件列表（Tivoli 风格密集表格） -->
    <section class="event-table-area">
      <!-- 固定表头 -->
      <div class="event-thead">
        <div class="th th-idx">序号</div>
        <div class="th th-sev">级别</div>
        <div class="th th-status">状态</div>
        <div class="th th-title">告警标题</div>
        <div class="th th-desc">告警描述</div>
        <div class="th th-ip">告警IP</div>
        <div class="th th-host">主机名/资产</div>
        <div class="th th-src">来源</div>
        <div class="th th-channel">接入方式</div>
        <div class="th th-actor">接入者</div>
        <div class="th th-contact">联系人</div>
        <div class="th th-count">次数</div>
        <div class="th th-time">触发时间</div>
        <div class="th th-ack">认领人</div>
      </div>

      <!-- 滚动表体 -->
      <div ref="tableBodyRef" class="event-tbody"
        @mouseenter="paused = true" @mouseleave="paused = false">
        <div
          v-for="(row, idx) in displayList"
          :key="`${row.id}-${idx}`"
          class="event-tr"
          :class="[`row-level-${normalizeAlertLevel(row.severity)}`, { 'row-new': row._isNew }]"
        >
          <div class="td td-idx mono">{{ idx < filteredList.length ? idx + 1 : idx - filteredList.length + 1 }}</div>
          <div class="td td-sev">
            <span class="sev-badge" :style="{ background: ALERT_LEVEL_META[normalizeAlertLevel(row.severity)].color }">
              {{ normalizeAlertLevel(row.severity) }} {{ alertLevelShortName(normalizeAlertLevel(row.severity)) }}
            </span>
          </div>
          <div class="td td-status">
            <span class="status-dot" :class="`dot-${row.status}`" />
            {{ statusLabel(row.status) }}
          </div>
          <div class="td td-title" :title="row.title">{{ row.title || 'N/A' }}</div>
          <div class="td td-desc" :title="alertSummary(row)">{{ alertSummary(row) }}</div>
          <div class="td td-ip mono">{{ alertIp(row) }}</div>
          <div class="td td-host" :title="alertHostname(row)">{{ alertHostname(row) }}</div>
          <div class="td td-src">{{ sourceLabel(row.source) }}</div>
          <div class="td td-channel">{{ ingressChannelLabel(row.ingressChannel ?? '') }}</div>
          <div class="td td-actor">{{ row.ingressActor ?? 'N/A' }}</div>
          <div class="td td-contact">{{ row.contactName ?? 'N/A' }}</div>
          <div class="td td-count mono">
            <span v-if="row.fireCount > 1" class="count-badge">{{ row.fireCount }}</span>
            <span v-else>1</span>
          </div>
          <div class="td td-time mono">{{ formatTime(row.firedAt) }}</div>
          <div class="td td-ack">{{ row.acknowledgedBy ?? 'N/A' }}</div>
        </div>

        <!-- 空状态 -->
        <div v-if="!filteredList.length && !busy" class="empty-state">
          <el-icon :size="56"><CircleCheck /></el-icon>
          <div class="empty-text">无灾难/重要告警</div>
          <div class="empty-sub">当前 4-5 级无活跃事件</div>
        </div>

        <!-- 加载中 -->
        <div v-if="busy && !filteredList.length" class="loading-state">
          <el-icon :size="32" class="is-loading"><Loading /></el-icon>
          <span>加载中…</span>
        </div>
      </div>
    </section>

    <!-- 底部跑马灯 -->
    <footer class="screen-footer">
      <div class="ticker-wrap">
        <span class="ticker-label">
          <el-icon class="ticker-bell"><Bell /></el-icon>
          实时播报
        </span>
        <div class="ticker-track">
          <div class="ticker-content" ref="tickerRef">
            <span v-for="(row, idx) in tickerList" :key="`tk-${row.id}-${idx}`" class="ticker-item">
              <span class="ticker-sev" :style="{ color: ALERT_LEVEL_META[normalizeAlertLevel(row.severity)].color }">
                [{{ normalizeAlertLevel(row.severity) }} {{ alertLevelShortName(normalizeAlertLevel(row.severity)) }}]
              </span>
              <span class="ticker-text">{{ row.title || 'N/A' }}</span>
              <span class="ticker-ip">{{ alertIp(row) }}</span>
              <span class="ticker-host">{{ alertHostname(row) }}</span>
              <span class="ticker-time mono">{{ formatTime(row.firedAt) }}</span>
              <span class="ticker-sep">|</span>
            </span>
            <span v-if="!tickerList.length" class="ticker-empty">暂无告警播报</span>
          </div>
        </div>
        <span class="footer-info">
          <span>数据来源：MeridianOps Alert Center</span>
          <span class="footer-sep">·</span>
          <span>刷新间隔 15s</span>
          <span class="footer-sep">·</span>
          <span>最后刷新：{{ lastRefreshTime }}</span>
        </span>
      </div>
    </footer>

    <!-- 新事件弹窗推送动画 -->
    <Transition name="popup">
      <div v-if="popupEvent" class="popup-alert"
        :style="{ '--popup-color': ALERT_LEVEL_META[normalizeAlertLevel(popupEvent.severity)].color, '--popup-glow': ALERT_LEVEL_META[normalizeAlertLevel(popupEvent.severity)].glow }">
        <div class="popup-header">
          <span class="popup-sev-badge" :style="{ background: ALERT_LEVEL_META[normalizeAlertLevel(popupEvent.severity)].color }">
            {{ normalizeAlertLevel(popupEvent.severity) }} {{ alertLevelShortName(normalizeAlertLevel(popupEvent.severity)) }}
          </span>
          <span class="popup-title">新告警事件</span>
        </div>
        <div class="popup-body">
          <div class="popup-event-title">{{ popupEvent.title || 'N/A' }}</div>
          <div class="popup-event-msg">{{ alertSummary(popupEvent) }}</div>
          <div class="popup-event-meta">
            <span>IP: {{ alertIp(popupEvent) }}</span>
            <span>·</span>
            <span>主机: {{ alertHostname(popupEvent) }}</span>
            <span>·</span>
            <span>{{ sourceLabel(popupEvent.source) }}</span>
            <span>·</span>
            <span>联系人: {{ popupEvent.contactName ?? 'N/A' }}</span>
            <span>·</span>
            <span class="mono">{{ formatTime(popupEvent.firedAt) }}</span>
          </div>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Bell, Warning, CircleCheck, Loading, FullScreen, Aim,
} from '@element-plus/icons-vue'
import {
  listAlertEvents, getAlertStats,
  type AlertEvent, type AlertStats,
} from '../../api/alert'
import {
  normalizeAlertLevel, alertLevelShortName,
  ALERT_LEVEL_META,
} from '../../utils/alert-level'

/** 大屏展示用告警事件，扩展 _isNew 标记新事件闪烁 */
interface ScreenAlertEvent extends AlertEvent {
  _isNew?: boolean
}

const busy = ref(false)
const online = ref(true)
const stats = ref<AlertStats | null>(null)
const feedList = ref<ScreenAlertEvent[]>([])
const nowText = ref('')
const nowDate = ref('')
const lastRefreshTime = ref('—')
const paused = ref(false)
const isFullscreen = ref(false)
const screenRef = ref<HTMLDivElement | null>(null)
// 分页
const currentPage = ref(1)
const pageSize = 50
const totalCount = ref(0)
const totalPages = computed(() => Math.max(1, Math.ceil(totalCount.value / pageSize)))
let prevIds = new Set<string>()

// 全屏切换（仅全屏大屏页面本身）
function toggleFullscreen() {
  const el = screenRef.value
  if (!el) return
  if (!document.fullscreenElement) {
    el.requestFullscreen?.().catch(() => ElMessage.error('当前浏览器不支持全屏'))
  } else {
    document.exitFullscreen?.()
  }
}
function onFsChange() { isFullscreen.value = !!document.fullscreenElement }

// 时钟
function tickClock() {
  const d = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  const days = ['日', '一', '二', '三', '四', '五', '六']
  nowText.value = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  nowDate.value = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} 星期${days[d.getDay()]}`
}
let clockTimer: ReturnType<typeof setInterval> | null = null

// 自动滚动
const tableBodyRef = ref<HTMLDivElement | null>(null)
const tickerRef = ref<HTMLDivElement | null>(null)
let scrollTimer: ReturnType<typeof setInterval> | null = null
let tickerTimer: ReturnType<typeof setInterval> | null = null
let refreshTimer: ReturnType<typeof setInterval> | null = null
let popupTimer: ReturnType<typeof setTimeout> | null = null
const popupEvent = ref<AlertEvent | null>(null)

// 固定只展示 4 重要 / 5 灾难 级别（符合银行大屏只上重大事件的做法）
const filteredList = computed(() => {
  if (!feedList.value.length) return []
  return feedList.value.filter(ev => {
    const lvl = normalizeAlertLevel(ev.severity)
    return lvl === '4' || lvl === '5'
  })
})
const displayList = computed(() => {
  if (!filteredList.value.length) return []
  // 正常银行大屏不会有 >100 条 4/5 级活跃告警，仅在超大量时复制实现无缝滚动
  // 避免中小数据量时出现重复显示
  if (filteredList.value.length <= 100) return filteredList.value
  return [...filteredList.value, ...filteredList.value]
})

// 当前 4-5 级活跃告警数（取后端按级别聚合统计）
const criticalActive = computed(() => {
  const bs = stats.value?.bySeverity
  if (!bs) return 0
  return (bs['4'] ?? 0) + (bs['5'] ?? 0)
})
// 今日 4-5 级新增告警数（从已加载事件列表中统计当日 4-5 级）
const criticalToday = computed(() => {
  const d = new Date()
  const pad = (n: number) => String(n).padStart(2, '0')
  const todayStr = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
  return feedList.value.filter(ev => {
    const lvl = normalizeAlertLevel(ev.severity)
    if (lvl !== '4' && lvl !== '5') return false
    const fd = new Date(ev.firedAt)
    if (isNaN(fd.getTime())) return false
    const pad2 = (n: number) => String(n).padStart(2, '0')
    return `${fd.getFullYear()}-${pad2(fd.getMonth() + 1)}-${pad2(fd.getDate())}` === todayStr
  }).length
})

// 跑马灯列表（最多 15 条）
const tickerList = computed(() => filteredList.value.slice(0, 15))

function startScroll() {
  stopScroll()
  scrollTimer = setInterval(() => {
    if (paused.value || !tableBodyRef.value) return
    const el = tableBodyRef.value
    const duplicated = displayList.value.length > filteredList.value.length
    if (duplicated) {
      el.scrollTop += 1
      if (el.scrollTop >= el.scrollHeight / 2) {
        el.scrollTop = 0
      }
    }
  }, 40)
}
function stopScroll() {
  if (scrollTimer) { clearInterval(scrollTimer); scrollTimer = null }
}

function startTicker() {
  stopTicker()
  let offset = 0
  tickerTimer = setInterval(() => {
    if (!tickerRef.value) return
    offset -= 1
    const w = tickerRef.value.scrollWidth / 2
    if (-offset >= w) offset = 0
    tickerRef.value.style.transform = `translateX(${offset}px)`
  }, 30)
}
function stopTicker() {
  if (tickerTimer) { clearInterval(tickerTimer); tickerTimer = null }
}

async function loadAll() {
  busy.value = true
  try {
    const [st, ev] = await Promise.all([
      getAlertStats(),
      listAlertEvents({ page: currentPage.value, pageSize: pageSize, status: 'firing,acknowledged' }),
    ])
    stats.value = st
    const evPage = ev as { items: AlertEvent[]; total: number }
    totalCount.value = evPage.total ?? 0
    // 越界保护：当前页超出总页数时回到末页
    if (currentPage.value > totalPages.value) {
      currentPage.value = totalPages.value
    }
    const items = evPage.items ?? []

    // 检测新事件
    const newIds: string[] = []
    const currentIds = new Set<string>()
    for (const item of items) {
      currentIds.add(item.id)
      if (!prevIds.has(item.id) && prevIds.size > 0) {
        newIds.push(item.id)
      }
    }
    prevIds = currentIds

    feedList.value = items.map(item => ({
      ...item,
      _isNew: newIds.includes(item.id) && prevIds.size > 0,
    }))

    // 弹窗推送最新新事件（仅 4 重要 / 5 灾难）
    if (newIds.length > 0 && prevIds.size > 0) {
      const newest = items.find(i => i.id === newIds[0] && ['4', '5'].includes(normalizeAlertLevel(i.severity)))
      if (newest) {
        popupEvent.value = newest
        if (popupTimer) clearTimeout(popupTimer)
        popupTimer = window.setTimeout(() => { popupEvent.value = null }, 8000)
      }
    }

    // 切页后重置滚动位置
    if (tableBodyRef.value) tableBodyRef.value.scrollTop = 0

    const d = new Date()
    const pad = (n: number) => String(n).padStart(2, '0')
    lastRefreshTime.value = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
    online.value = true
  } catch (e) {
    console.error('loadAll error', e)
    online.value = false
    ElMessage.error('数据加载失败')
  } finally {
    busy.value = false
  }
}

// 分页切换：停滚动 → 加载 → 重启滚动
async function goToPage(p: number) {
  const target = Math.min(Math.max(1, p), totalPages.value)
  if (target === currentPage.value) return
  stopScroll()
  currentPage.value = target
  await loadAll()
  startScroll()
}
async function prevPage() { if (currentPage.value > 1) await goToPage(currentPage.value - 1) }
async function nextPage() { if (currentPage.value < totalPages.value) await goToPage(currentPage.value + 1) }

onMounted(() => {
  tickClock()
  clockTimer = setInterval(tickClock, 1000)
  loadAll().then(() => {
    startScroll()
    startTicker()
  })
  refreshTimer = setInterval(() => { loadAll() }, 15000)
  document.addEventListener('fullscreenchange', onFsChange)
})

onBeforeUnmount(() => {
  if (clockTimer) clearInterval(clockTimer)
  stopScroll()
  stopTicker()
  if (refreshTimer) clearInterval(refreshTimer)
  if (popupTimer) clearTimeout(popupTimer)
  document.removeEventListener('fullscreenchange', onFsChange)
})

// ============ 工具函数 ============

function sourceLabel(s: string | null | undefined): string {
  if (!s) return 'N/A'
  const map: Record<string, string> = {
    zabbix: 'Zabbix', prometheus: 'Prometheus', manual: '人工', job: '作业', system: '系统',
    eventide: 'Eventide', snmptrap: 'SNMP', kafka: 'Kafka',
  }
  return map[s] ?? s
}
function ingressChannelLabel(s: string): string {
  return ({ webhook: 'Webhook', manual: '人工上报', api_token: 'API令牌', job: '作业', system: '系统' } as Record<string, string>)[s] ?? (s || 'N/A')
}
function statusLabel(s: string | null | undefined): string {
  return ({ firing: '触发中', acknowledged: '已认领', resolved: '已解决', suppressed: '已静默' } as Record<string, string>)[s ?? ''] ?? s ?? 'N/A'
}

/** 从 labels JSON 提取告警 IP */
function alertIp(row: AlertEvent): string {
  if (!row.labels) return 'N/A'
  const lbl = row.labels as Record<string, unknown>
  const raw = lbl.alertIp || lbl.ip || lbl.instance || lbl.host_ip || lbl.manageIp || lbl.alert_ip || lbl.target_ip || lbl.src_ip
  if (typeof raw !== 'string' || !raw) return 'N/A'
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

/** 从 labels/annotations 提取告警摘要 */
function alertSummary(row: AlertEvent): string {
  if (row.message) return row.message
  if (!row.labels) return 'N/A'
  const lbl = row.labels as Record<string, unknown>
  const raw = lbl.summary || lbl.description || lbl.detail || lbl.hint
  return typeof raw === 'string' && raw ? raw : 'N/A'
}

function formatTime(s: string | null | undefined): string {
  if (!s) return 'N/A'
  try {
    const d = new Date(s)
    if (isNaN(d.getTime())) return s
    const pad = (n: number) => String(n).padStart(2, '0')
    return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  } catch {
    return s
  }
}
</script>

<style scoped>
.alert-screen {
  width: 100vw; height: 100vh; overflow: hidden;
  background:
    radial-gradient(ellipse at 50% -10%, rgba(56,189,248,0.08) 0%, transparent 55%),
    radial-gradient(ellipse at 20% 100%, rgba(239,68,68,0.05) 0%, transparent 40%),
    radial-gradient(ellipse at 80% 100%, rgba(56,189,248,0.04) 0%, transparent 40%),
    linear-gradient(180deg, #080d1a 0%, #050810 100%);
  color: #cbd5e1;
  display: flex; flex-direction: column;
  font-family: 'Microsoft YaHei', 'Segoe UI', sans-serif;
  position: relative;
}
/* 全局微弱网格底纹 */
.alert-screen::before {
  content: ''; position: absolute; inset: 0; pointer-events: none; z-index: 0;
  background-image:
    linear-gradient(rgba(56,189,248,0.025) 1px, transparent 1px),
    linear-gradient(90deg, rgba(56,189,248,0.025) 1px, transparent 1px);
  background-size: 48px 48px;
}
/* 四角装饰括号 */
.alert-screen::after {
  content: ''; position: absolute; inset: 12px; pointer-events: none; z-index: 2;
  border: 1px solid rgba(56,189,248,0.08);
  border-radius: 2px;
}
.alert-screen > * { position: relative; z-index: 1; }

/* ====== 顶栏 ====== */
.screen-header {
  flex-shrink: 0; height: 52px;
  display: flex; align-items: center; justify-content: space-between;
  padding: 0 24px;
  background: linear-gradient(180deg, #0f172a 0%, #0a0e1a 100%);
  border-bottom: 1px solid rgba(56, 189, 248, 0.18);
  box-shadow: 0 2px 24px rgba(0,0,0,0.4), inset 0 -1px 0 rgba(56,189,248,0.08);
  position: relative;
}
/* 顶栏底部扫描线 */
.screen-header::after {
  content: ''; position: absolute; left: 0; right: 0; bottom: 0; height: 2px;
  background: linear-gradient(90deg, transparent 0%, rgba(56,189,248,0.6) 20%, rgba(56,189,248,0.8) 50%, rgba(56,189,248,0.6) 80%, transparent 100%);
  animation: header-scan 6s ease-in-out infinite;
}
@keyframes header-scan {
  0%,100% { opacity: 0.3; }
  50% { opacity: 1; }
}

.header-left { display: flex; align-items: center; flex: 0 0 auto; }
.brand { display: flex; align-items: center; gap: 12px; }
.brand-icon {
  font-size: 24px; color: #38bdf8;
  filter: drop-shadow(0 0 10px rgba(56,189,248,0.55));
}
.brand-title {
  font-size: 18px; font-weight: 800; color: #f1f5f9; letter-spacing: 2px;
  text-shadow: 0 0 12px rgba(56,189,248,0.35);
}
.brand-sub { font-size: 9px; color: rgba(148,163,184,0.55); letter-spacing: 3px; }

.header-center { text-align: center; flex: 1 1 auto; min-width: 0; overflow: hidden; }
.clock {
  font-size: 26px; font-weight: 800; color: #f1f5f9;
  font-family: 'Consolas', 'Courier New', monospace;
  letter-spacing: 3px;
  text-shadow: 0 0 14px rgba(56,189,248,0.3);
}
.clock-sub { font-size: 11px; color: rgba(148,163,184,0.55); margin-top: 1px; }

.header-right { display: flex; align-items: center; gap: 8px; flex: 0 0 auto; }
.fs-btn {
  font-size: 18px; color: rgba(56,189,248,0.7); cursor: pointer;
  padding: 4px; border-radius: 4px; transition: all 0.2s;
  border: 1px solid rgba(56,189,248,0.2); background: rgba(15,23,42,0.5);
}
.fs-btn:hover { color: #38bdf8; border-color: rgba(56,189,248,0.5); box-shadow: 0 0 10px rgba(56,189,248,0.3); }
.stat-cards { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }
.stat-card {
  display: flex; align-items: center; gap: 6px;
  padding: 3px 12px 3px 10px; border-radius: 5px;
  background: rgba(15,23,42,0.6);
  border: 1px solid rgba(56,189,248,0.12);
  white-space: nowrap; flex-shrink: 0;
  position: relative;
  overflow: hidden;
}
.stat-card::before {
  content: ''; position: absolute; left: 0; top: 0; bottom: 0; width: 3px;
}
.stat-current {
  border-color: rgba(239,68,68,0.4);
  box-shadow: 0 0 16px rgba(239,68,68,0.18), inset 0 0 12px rgba(239,68,68,0.06);
  animation: stat-pulse 3s ease-in-out infinite;
}
.stat-current::before { background: #ef4444; }
.stat-today {
  border-color: rgba(56,189,248,0.35);
  box-shadow: 0 0 16px rgba(56,189,248,0.18), inset 0 0 12px rgba(56,189,248,0.06);
}
.stat-today::before { background: #38bdf8; }
.stat-icon { font-size: 18px; position: relative; z-index: 1; }
.stat-current .stat-icon { color: #ef4444; filter: drop-shadow(0 0 6px rgba(239,68,68,0.6)); }
.stat-today .stat-icon { color: #38bdf8; filter: drop-shadow(0 0 6px rgba(56,189,248,0.6)); }
.stat-body { display: flex; flex-direction: column; gap: 0; position: relative; z-index: 1; }
.stat-label { font-size: 10px; color: rgba(148,163,184,0.7); letter-spacing: 1px; }
.stat-value {
  font-size: 20px; font-weight: 800; line-height: 1;
  font-family: 'Consolas', 'Courier New', monospace;
}
.stat-current .stat-value { color: #fca5a5; text-shadow: 0 0 10px rgba(239,68,68,0.5); }
.stat-today .stat-value { color: #7dd3fc; text-shadow: 0 0 10px rgba(56,189,248,0.5); }
@keyframes stat-pulse {
  0%,100% { box-shadow: 0 0 14px rgba(239,68,68,0.15), inset 0 0 12px rgba(239,68,68,0.06); }
  50% { box-shadow: 0 0 22px rgba(239,68,68,0.3), inset 0 0 16px rgba(239,68,68,0.12); }
}

/* ====== 工具条 ====== */
.toolbar {
  flex-shrink: 0; height: 34px;
  display: flex; align-items: center; justify-content: space-between;
  padding: 0 24px;
  background: linear-gradient(180deg, #0d1526 0%, #0a0f1d 100%);
  border-bottom: 1px solid rgba(56,189,248,0.1);
}
.toolbar-left { display: flex; align-items: center; gap: 10px; flex-shrink: 0; }
.toolbar-title { font-size: 13px; font-weight: 600; color: #e2e8f0; letter-spacing: 1px; }
.toolbar-count { font-size: 11px; color: rgba(148,163,184,0.55); }
.toolbar-right { display: flex; align-items: center; gap: 10px; flex-shrink: 0; }
.pager { display: flex; align-items: center; gap: 8px; }
.pager-info { font-size: 12px; color: #38bdf8; font-weight: 600; min-width: 42px; text-align: center; letter-spacing: 0.5px; }
.pager-btn {
  width: 28px; height: 24px; border-radius: 4px;
  background: rgba(15,23,42,0.8); color: rgba(56,189,248,0.9);
  border: 1px solid rgba(56,189,248,0.3); cursor: pointer;
  font-size: 16px; line-height: 1; transition: all 0.2s;
  display: flex; align-items: center; justify-content: center;
}
.pager-btn:hover:not(:disabled) {
  color: #38bdf8; border-color: rgba(56,189,248,0.6);
  box-shadow: 0 0 12px rgba(56,189,248,0.4); background: rgba(15,23,42,1);
}
.pager-btn:disabled { opacity: 0.55; cursor: not-allowed; color: rgba(148,163,184,0.6); border-color: rgba(100,116,139,0.25); }

.live-dot {
  width: 9px; height: 9px; border-radius: 50%;
  background: #22c55e; box-shadow: 0 0 8px rgba(34,197,94,0.6);
  animation: pulse 2s ease-in-out infinite;
}
.live-dot.busy { background: #f59e0b; box-shadow: 0 0 8px rgba(245,158,11,0.6); }
.live-dot:not(.online):not(.busy) { background: #ef4444; box-shadow: 0 0 8px rgba(239,68,68,0.6); }
@keyframes pulse { 0%,100% { opacity: 1 } 50% { opacity: 0.35 } }

/* ====== 事件列表（Tivoli 风格密集表格） ====== */
.event-table-area {
  flex: 1; overflow: hidden;
  display: flex; flex-direction: column;
  background: #04070f;
}

/* 表头 */
.event-thead {
  flex-shrink: 0;
  display: flex; align-items: center;
  height: 34px;
  background:
    linear-gradient(180deg, rgba(15,23,42,0.95) 0%, rgba(8,13,26,0.95) 100%);
  border-bottom: 2px solid rgba(56,189,248,0.3);
  font-size: 11px; font-weight: 700; color: rgba(203,213,225,0.9);
  letter-spacing: 1px;
  box-shadow: inset 0 -1px 0 rgba(56,189,248,0.15), 0 2px 8px rgba(0,0,0,0.3);
  position: relative;
}
/* 表头左侧级别色条渐变 */
.event-thead::before {
  content: ''; position: absolute; left: 0; top: 0; bottom: 0; width: 4px;
  background: linear-gradient(180deg, #38bdf8, #0ea5e9);
}
.th {
  padding: 0 10px; height: 100%;
  display: flex; align-items: center;
  white-space: nowrap; overflow: hidden;
  text-transform: uppercase;
  border-right: 1px solid rgba(56,189,248,0.07);
}
.th-idx { width: 48px; justify-content: center; }
.th-sev { width: 116px; }
.th-status { width: 92px; }
.th-title { flex: 2.5; min-width: 200px; }
.th-desc { flex: 2; min-width: 180px; }
.th-ip { width: 132px; }
.th-host { width: 150px; }
.th-src { width: 88px; }
.th-channel { width: 100px; }
.th-actor { width: 108px; }
.th-contact { width: 88px; }
.th-count { width: 58px; justify-content: center; }
.th-time { width: 152px; }
.th-ack { width: 88px; }

/* 表体 */
.event-tbody {
  flex: 1; overflow-y: auto; overflow-x: hidden;
  scrollbar-width: thin; scrollbar-color: rgba(56,189,248,0.22) transparent;
}
.event-tbody::-webkit-scrollbar { width: 5px; }
.event-tbody::-webkit-scrollbar-track { background: transparent; }
.event-tbody::-webkit-scrollbar-thumb { background: rgba(56,189,248,0.22); border-radius: 3px; }

.event-tr {
  display: flex; align-items: center;
  height: 38px; min-height: 38px;
  border-bottom: 1px solid rgba(30,41,59,0.45);
  font-size: 13px; color: rgba(203,213,225,0.88);
  transition: background 0.15s, box-shadow 0.15s;
  position: relative;
}
/* 斑马纹 */
.event-tr:nth-child(even) { background: rgba(15,23,42,0.35); }
.event-tr:hover { background: rgba(30,41,59,0.6); }

/* 级别左色条 */
.event-tr::before {
  content: ''; position: absolute; left: 0; top: 0; bottom: 0;
  width: 4px; background: #475569; z-index: 2;
}
.event-tr.row-level-0::before { background: #64748b; }
.event-tr.row-level-1::before { background: #0ea5e9; }
.event-tr.row-level-2::before { background: #ca8a04; }
.event-tr.row-level-3::before { background: #ea580c; }
.event-tr.row-level-4::before { background: #dc2626; box-shadow: 0 0 6px rgba(220,38,38,0.5); }
.event-tr.row-level-5::before { background: #ef4444; box-shadow: 0 0 10px rgba(239,68,68,0.8); }

/* 级别行背景微染色 + 灾难行呼吸 */
.event-tr.row-level-5 { background: rgba(127,29,29,0.15); animation: dis-breath 3s ease-in-out infinite; }
.event-tr.row-level-4 { background: rgba(220,38,38,0.06); }
.event-tr.row-level-5:nth-child(even) { background: rgba(127,29,29,0.2); }
.event-tr.row-level-4:nth-child(even) { background: rgba(220,38,38,0.08); }
.event-tr.row-level-5:hover { background: rgba(127,29,29,0.25); animation: none; }
.event-tr.row-level-4:hover { background: rgba(220,38,38,0.12); }
@keyframes dis-breath {
  0%,100% { background: rgba(127,29,29,0.12); box-shadow: inset 4px 0 14px rgba(239,68,68,0.12); }
  50% { background: rgba(185,28,28,0.25); box-shadow: inset 4px 0 20px rgba(239,68,68,0.28); }
}

/* 新事件行：左侧色条闪白+行整体闪烁 */
.event-tr.row-new::after {
  content: ''; position: absolute; left: 4px; top: 0; bottom: 0; width: 2px;
  background: #fff; z-index: 3;
  animation: new-col-flash 0.8s ease-in-out 5;
}
@keyframes new-col-flash {
  0%,100% { opacity: 0; }
  50% { opacity: 1; box-shadow: 0 0 8px #fff; }
}
.event-tr.row-new { animation: row-flash 0.8s ease-in-out 5; }
@keyframes row-flash {
  0%,100% { background: rgba(220,38,38,0.08); }
  50% { background: rgba(239,68,68,0.28); box-shadow: inset 0 0 16px rgba(239,68,68,0.3); }
}

.td {
  padding: 0 10px; height: 100%;
  display: flex; align-items: center;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  border-right: 1px solid rgba(30,41,59,0.3);
}
.td-idx { width: 48px; justify-content: center; color: rgba(148,163,184,0.45); font-size: 11px; }
.td-sev { width: 116px; }
.td-status { width: 92px; gap: 6px; }
.td-title { flex: 2.5; min-width: 200px; color: #e2e8f0; font-weight: 600; }
.td-desc { flex: 2; min-width: 180px; color: rgba(148,163,184,0.72); }
.td-ip { width: 132px; color: #60a5fa; }
.td-host { width: 150px; color: rgba(167,139,250,0.88); }
.td-src { width: 88px; }
.td-channel { width: 100px; }
.td-actor { width: 108px; color: rgba(203,213,225,0.75); }
.td-contact { width: 88px; color: rgba(203,213,225,0.75); }
.td-count { width: 58px; justify-content: center; }
.td-time { width: 152px; color: rgba(148,163,184,0.72); }
.td-ack { width: 88px; color: rgba(203,213,225,0.75); }

.sev-badge {
  font-size: 11px; font-weight: 700; color: #fff;
  padding: 3px 9px; border-radius: 3px;
  white-space: nowrap;
  box-shadow: 0 1px 3px rgba(0,0,0,0.3);
  letter-spacing: 0.5px;
}
/* 灾难级别徽章发光 */
.event-tr.row-level-5 .sev-badge {
  box-shadow: 0 0 14px rgba(239,68,68,0.7), 0 1px 3px rgba(0,0,0,0.3);
  animation: badge-glow 2s ease-in-out infinite;
}
.event-tr.row-level-4 .sev-badge {
  box-shadow: 0 0 8px rgba(220,38,38,0.5), 0 1px 3px rgba(0,0,0,0.3);
}
@keyframes badge-glow {
  0%,100% { box-shadow: 0 0 10px rgba(239,68,68,0.5); }
  50% { box-shadow: 0 0 20px rgba(239,68,68,0.9); }
}

.status-dot {
  width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0;
}
.dot-firing { background: #ef4444; box-shadow: 0 0 6px rgba(239,68,68,0.6); animation: pulse 1.6s ease-in-out infinite; }
.dot-acknowledged { background: #f59e0b; box-shadow: 0 0 4px rgba(245,158,11,0.4); }
.dot-resolved { background: #22c55e; }
.dot-suppressed { background: #64748b; }

.count-badge {
  background: rgba(245,158,11,0.18); color: #fbbf24;
  padding: 2px 7px; border-radius: 3px; font-size: 11px; font-weight: 700;
  border: 1px solid rgba(245,158,11,0.3);
}

/* 空状态 / 加载 */
.empty-state, .loading-state {
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  height: 240px; gap: 14px; color: rgba(148,163,184,0.5);
}
.empty-state {
  position: relative;
}
.empty-state::before {
  content: ''; position: absolute; width: 120px; height: 120px;
  border: 1px dashed rgba(56,189,248,0.15); border-radius: 50%;
  animation: empty-rotate 20s linear infinite;
}
.empty-state::after {
  content: ''; position: absolute; width: 80px; height: 80px;
  border: 1px solid rgba(34,197,94,0.2); border-radius: 50%;
}
@keyframes empty-rotate {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
.empty-state .el-icon {
  color: rgba(34,197,94,0.6);
  filter: drop-shadow(0 0 12px rgba(34,197,94,0.4));
  position: relative; z-index: 1;
}
.empty-text { font-size: 20px; font-weight: 600; color: rgba(34,197,94,0.75); letter-spacing: 1px; position: relative; z-index: 1; }
.empty-sub { font-size: 13px; position: relative; z-index: 1; }

/* ====== 底部跑马灯 ====== */
.screen-footer {
  flex-shrink: 0; height: 30px;
  background: linear-gradient(180deg, #0a0f1d 0%, #0d1424 100%);
  border-top: 1px solid rgba(56,189,248,0.18);
  display: flex; align-items: center;
  box-shadow: 0 -2px 16px rgba(0,0,0,0.3);
  position: relative;
}
/* 底部扫描线 */
.screen-footer::after {
  content: ''; position: absolute; left: 0; right: 0; bottom: 0; height: 1px;
  background: linear-gradient(90deg, transparent 0%, rgba(56,189,248,0.5) 50%, transparent 100%);
  animation: scan-line 4s linear infinite;
}
@keyframes scan-line {
  0% { transform: translateX(-100%); }
  100% { transform: translateX(100%); }
}
.ticker-wrap {
  display: flex; align-items: center; width: 100%;
  padding: 0 16px; gap: 12px;
}
.ticker-label {
  display: flex; align-items: center; gap: 5px;
  font-size: 12px; font-weight: 700; color: rgba(56,189,248,0.85);
  flex-shrink: 0; padding-right: 12px;
  border-right: 1px solid rgba(56,189,248,0.15);
  letter-spacing: 1px;
}
.ticker-bell { animation: bell-ring 2s ease-in-out infinite; }
@keyframes bell-ring { 0%,100% { transform: rotate(0) } 10% { transform: rotate(-12deg) } 20% { transform: rotate(12deg) } 30% { transform: rotate(-8deg) } 40% { transform: rotate(8deg) } 50% { transform: rotate(0) } }

.ticker-track { flex: 1; overflow: hidden; }
.ticker-content {
  display: flex; align-items: center; gap: 12px;
  white-space: nowrap; will-change: transform;
}
.ticker-item { display: flex; align-items: center; gap: 6px; font-size: 12px; }
.ticker-sev { font-weight: 700; }
.ticker-text { color: rgba(203,213,225,0.9); }
.ticker-ip { color: rgba(96,165,250,0.75); font-family: 'Courier New', monospace; }
.ticker-host { color: rgba(167,139,250,0.75); }
.ticker-time { color: rgba(148,163,184,0.6); }
.ticker-sep { color: rgba(148,163,184,0.2); }
.ticker-empty { font-size: 12px; color: rgba(148,163,184,0.4); }

.footer-info {
  display: flex; align-items: center; gap: 6px;
  font-size: 11px; color: rgba(148,163,184,0.4);
  flex-shrink: 0; padding-left: 12px;
  border-left: 1px solid rgba(56,189,248,0.1);
}
.footer-sep { color: rgba(148,163,184,0.15); }

/* ====== 弹窗 ====== */
.popup-alert {
  position: fixed; top: 68px; right: 24px; z-index: 1000;
  width: 440px;
  background: rgba(10,15,26,0.97); backdrop-filter: blur(14px);
  border: 1px solid var(--popup-color, #dc2626);
  border-radius: 8px;
  box-shadow: 0 0 40px var(--popup-glow, rgba(220,38,38,0.4)), 0 8px 32px rgba(0,0,0,0.6);
  overflow: hidden;
  animation: popup-float 3s ease-in-out infinite;
}
@keyframes popup-float {
  0%,100% { transform: translateY(0); }
  50% { transform: translateY(-2px); }
}
.popup-header {
  display: flex; align-items: center; gap: 8px;
  padding: 11px 16px;
  background: linear-gradient(90deg, var(--popup-color, #dc2626) 0%, transparent 100%);
}
.popup-sev-badge {
  font-size: 12px; font-weight: 700; color: #fff;
  padding: 3px 9px; border-radius: 3px;
}
.popup-title { font-size: 13px; font-weight: 600; color: #f1f5f9; letter-spacing: 1px; }
.popup-body { padding: 14px 16px; }
.popup-event-title { font-size: 16px; font-weight: 700; color: #f1f5f9; margin-bottom: 8px; }
.popup-event-msg { font-size: 13px; color: rgba(203,213,225,0.82); margin-bottom: 10px; line-height: 1.5; }
.popup-event-meta {
  display: flex; flex-wrap: wrap; gap: 6px;
  font-size: 11px; color: rgba(148,163,184,0.72);
}

.popup-enter-active, .popup-leave-active { transition: all 0.5s cubic-bezier(0.22, 1, 0.36, 1); }
.popup-enter-from, .popup-leave-to { transform: translateX(120%) scale(0.95); opacity: 0; }

.mono { font-family: 'Consolas', 'Courier New', monospace; }
</style>
