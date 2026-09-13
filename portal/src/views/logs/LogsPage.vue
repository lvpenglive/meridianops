<template>
  <div class="logs-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <span class="title">📄 日志中心</span>
          <el-button :icon="Link" @click="openGrafana">跳转 Grafana</el-button>
        </div>
      </template>

      <el-tabs v-model="activeTab" @tab-change="onTabChange">
        <el-tab-pane label="基础检索" name="search" />
        <el-tab-pane label="聚合统计" name="stats" />
        <el-tab-pane label="模式挖掘" name="patterns" />
      </el-tabs>

      <div class="filters">
        <el-input v-model="filter.hostname" placeholder="主机" clearable style="width: 150px" />
        <el-input v-model="filter.service" placeholder="服务" clearable style="width: 150px" />
        <el-select v-model="filter.levels" multiple collapse-tags placeholder="级别" clearable style="width: 200px">
          <el-option v-for="l in LOG_LEVELS" :key="l.value" :label="l.label" :value="l.value" />
        </el-select>
        <el-input
          v-if="activeTab === 'search'"
          v-model="filter.keyword"
          placeholder="关键字"
          clearable
          style="width: 200px"
          @keyup.enter="runQuery"
        />
        <el-select v-if="activeTab === 'stats'" v-model="filter.groupBy" style="width: 140px">
          <el-option label="按服务" value="service" />
          <el-option label="按级别" value="level" />
          <el-option label="按主机" value="hostname" />
        </el-select>
        <el-date-picker
          v-model="dateRange"
          type="datetimerange"
          range-separator="至"
          start-placeholder="开始时间"
          end-placeholder="结束时间"
          value-format="YYYY-MM-DDTHH:mm:ssZ"
          style="width: 360px"
        />
        <el-button type="primary" :icon="Search" @click="runQuery">查询</el-button>
        <el-button :icon="Refresh" @click="resetFilters">重置</el-button>
      </div>

      <!-- L1 -->
      <template v-if="activeTab === 'search'">
        <el-table :data="rows" v-loading="loading" stripe max-height="560">
          <el-table-column prop="timestamp" label="时间" width="180" />
          <el-table-column label="级别" width="100" align="center">
            <template #default="{ row }">
              <el-tag size="small" :type="(levelTagType(row.level) || undefined) as any" effect="dark">
                {{ levelLabel(row.level) }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="hostname" label="主机" width="140" show-overflow-tooltip />
          <el-table-column prop="service" label="服务" width="140" show-overflow-tooltip />
          <el-table-column prop="message" label="消息" min-width="320" show-overflow-tooltip />
          <el-table-column label="操作" width="80" fixed="right">
            <template #default="{ row }">
              <el-button size="small" link type="primary" @click="showDetail(row)">详情</el-button>
            </template>
          </el-table-column>
        </el-table>
        <div class="pagination-wrap">
          <el-pagination
            v-model:current-page="page"
            v-model:page-size="pageSize"
            :total="total"
            :page-sizes="[20, 50, 100, 200]"
            layout="total, sizes, prev, pager, next"
            @size-change="loadSearch(1)"
            @current-change="loadSearch"
          />
        </div>
      </template>

      <!-- L2 -->
      <el-table v-else-if="activeTab === 'stats'" :data="statRows" v-loading="loading" stripe>
        <el-table-column prop="bucket" :label="statsBucketLabel" min-width="200" />
        <el-table-column label="级别" width="120" align="center">
          <template #default="{ row }">
            <el-tag v-if="row.level" size="small" :type="(levelTagType(row.level) || undefined) as any" effect="dark">
              {{ levelLabel(row.level) }}
            </el-tag>
            <span v-else>—</span>
          </template>
        </el-table-column>
        <el-table-column prop="count" label="条数" width="140" sortable />
      </el-table>

      <!-- L3 -->
      <el-table v-else :data="patternRows" v-loading="loading" stripe>
        <el-table-column prop="count" label="次数" width="100" sortable />
        <el-table-column prop="template" label="模板" min-width="280" show-overflow-tooltip />
        <el-table-column prop="sample" label="样例" min-width="320" show-overflow-tooltip />
      </el-table>
    </el-card>

    <el-drawer v-model="detailVisible" title="日志详情" size="560px">
      <template v-if="current">
        <el-descriptions :column="1" border>
          <el-descriptions-item label="时间">{{ current.timestamp }}</el-descriptions-item>
          <el-descriptions-item label="级别">
            <el-tag size="small" :type="(levelTagType(current.level) || undefined) as any" effect="dark">
              {{ levelLabel(current.level) }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="主机">{{ current.hostname || '—' }}</el-descriptions-item>
          <el-descriptions-item label="服务">{{ current.service || '—' }}</el-descriptions-item>
          <el-descriptions-item label="来源">{{ current.source || '—' }}</el-descriptions-item>
          <el-descriptions-item label="IP">{{ current.ip || '—' }}</el-descriptions-item>
          <el-descriptions-item label="Trace ID">{{ current.traceId || '—' }}</el-descriptions-item>
          <el-descriptions-item label="消息">
            <pre class="detail-pre">{{ current.message }}</pre>
          </el-descriptions-item>
          <el-descriptions-item v-if="current.extra" label="扩展">
            <pre class="detail-pre">{{ formatExtra(current.extra) }}</pre>
          </el-descriptions-item>
        </el-descriptions>
      </template>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { Link, Refresh, Search } from '@element-plus/icons-vue'
import {
  LOG_LEVELS,
  getLogPatterns,
  getLogStats,
  getGrafanaLink,
  levelLabel,
  levelTagType,
  listLogs,
  type LogRow,
  type PatternRow,
  type StatRow,
} from '../../api/logs'

const activeTab = ref<'search' | 'stats' | 'patterns'>('search')
const loading = ref(false)
const rows = ref<LogRow[]>([])
const statRows = ref<StatRow[]>([])
const patternRows = ref<PatternRow[]>([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(20)
const detailVisible = ref(false)
const current = ref<LogRow | null>(null)

const filter = reactive({
  hostname: '',
  service: '',
  levels: [] as string[],
  keyword: '',
  groupBy: 'service' as 'service' | 'level' | 'hostname',
})
const dateRange = ref<[string, string] | null>(defaultRange())

function defaultRange(): [string, string] {
  const end = new Date()
  const start = new Date(end.getTime() - 24 * 3600 * 1000)
  return [start.toISOString(), end.toISOString()]
}

const statsBucketLabel = computed(() => {
  if (filter.groupBy === 'level') return '级别'
  if (filter.groupBy === 'hostname') return '主机'
  return '服务'
})

function commonParams() {
  return {
    hostname: filter.hostname || undefined,
    service: filter.service || undefined,
    level: filter.levels.length ? filter.levels.join(',') : undefined,
    startTime: dateRange.value?.[0],
    endTime: dateRange.value?.[1],
  }
}

async function loadSearch(p = page.value) {
  page.value = p
  loading.value = true
  try {
    const data = await listLogs({
      ...commonParams(),
      keyword: filter.keyword || undefined,
      limit: pageSize.value,
      offset: (page.value - 1) * pageSize.value,
    })
    rows.value = data.items ?? []
    total.value = data.total ?? 0
  } catch {
    rows.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

async function loadStats() {
  loading.value = true
  try {
    const data = await getLogStats({
      ...commonParams(),
      groupBy: filter.groupBy,
      top: 20,
    })
    statRows.value = data.items ?? []
  } catch {
    statRows.value = []
  } finally {
    loading.value = false
  }
}

async function loadPatterns() {
  loading.value = true
  try {
    const data = await getLogPatterns({
      ...commonParams(),
      top: 30,
    })
    patternRows.value = data.items ?? []
  } catch {
    patternRows.value = []
  } finally {
    loading.value = false
  }
}

function runQuery() {
  if (activeTab.value === 'search') return loadSearch(1)
  if (activeTab.value === 'stats') return loadStats()
  return loadPatterns()
}

function onTabChange() {
  runQuery()
}

function resetFilters() {
  filter.hostname = ''
  filter.service = ''
  filter.levels = []
  filter.keyword = ''
  filter.groupBy = 'service'
  dateRange.value = defaultRange()
  runQuery()
}

function showDetail(row: LogRow) {
  current.value = row
  detailVisible.value = true
}

function formatExtra(extra: unknown) {
  try {
    return JSON.stringify(extra, null, 2)
  } catch {
    return String(extra)
  }
}

async function openGrafana() {
  try {
    const data = await getGrafanaLink({
      hostname: filter.hostname || undefined,
      level: filter.levels[0] || 'error',
      keyword: filter.keyword || undefined,
      startTime: dateRange.value?.[0],
      endTime: dateRange.value?.[1],
    })
    if (data?.url) {
      window.open(data.url, '_blank')
    } else {
      ElMessage.warning('未生成 Grafana 链接，请检查日志平台配置')
    }
  } catch {
    // 拦截器已提示
  }
}

onMounted(() => loadSearch(1))
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.title {
  font-weight: 600;
}
.filters {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-bottom: 16px;
}
.pagination-wrap {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
.detail-pre {
  margin: 0;
  white-space: pre-wrap;
  word-break: break-all;
  font-size: 12px;
  line-height: 1.5;
}
</style>
