<template>
  <div class="audit-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <span class="title">📝 审计中心</span>
          <div class="filters">
            <el-select
              v-model="filters.action"
              placeholder="操作类型"
              clearable
              style="width: 140px"
            >
              <el-option label="登录" value="login" />
              <el-option label="创建" value="create" />
              <el-option label="更新" value="update" />
              <el-option label="启用" value="enable" />
              <el-option label="禁用" value="disable" />
              <el-option label="重置密码" value="reset_password" />
            </el-select>
            <el-select
              v-model="filters.status"
              placeholder="状态"
              clearable
              style="width: 120px"
            >
              <el-option label="成功" value="success" />
              <el-option label="失败" value="failure" />
            </el-select>
            <el-input
              v-model="filters.actor"
              placeholder="操作人"
              clearable
              style="width: 160px"
            />
            <el-date-picker
              v-model="dateRange"
              type="datetimerange"
              range-separator="至"
              start-placeholder="开始时间"
              end-placeholder="结束时间"
              value-format="YYYY-MM-DDTHH:mm:ssZ"
              style="width: 340px"
            />
            <el-button type="primary" @click="loadLogs(1)">查询</el-button>
            <el-button @click="resetFilters">重置</el-button>
          </div>
        </div>
      </template>

      <el-table :data="items" v-loading="loading" stripe>
        <el-table-column prop="actorUsername" label="操作人" width="120" />
        <el-table-column prop="action" label="操作类型" width="120">
          <template #default="{ row }">
            <el-tag :type="getActionColor(row.action)" size="small">
              {{ getActionLabel(row.action) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作对象" min-width="200">
          <template #default="{ row }">
            <span class="target-cell">
              <el-tag type="info" size="small">{{ row.targetType }}</el-tag>
              <span class="target-id">{{ row.targetId }}</span>
            </span>
          </template>
        </el-table-column>
        <el-table-column prop="ip" label="IP 地址" width="140" />
        <el-table-column prop="status" label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="row.status === 'success' ? 'success' : 'danger'" size="small">
              {{ row.status === 'success' ? '成功' : '失败' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="createdAt" label="时间" width="180" />
        <el-table-column label="操作" width="100" fixed="right">
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
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="loadLogs(1)"
          @current-change="loadLogs"
        />
      </div>
    </el-card>

    <!-- 审计详情抽屉 -->
    <el-drawer v-model="detailVisible" title="审计详情" size="500px">
      <template v-if="currentDetail">
        <el-descriptions :column="1" border>
          <el-descriptions-item label="ID">{{ currentDetail.id }}</el-descriptions-item>
          <el-descriptions-item label="操作人">{{ currentDetail.actorUsername }}</el-descriptions-item>
          <el-descriptions-item label="操作类型">{{ getActionLabel(currentDetail.action) }}</el-descriptions-item>
          <el-descriptions-item label="对象类型">{{ currentDetail.targetType }}</el-descriptions-item>
          <el-descriptions-item label="对象 ID">{{ currentDetail.targetId }}</el-descriptions-item>
          <el-descriptions-item label="IP 地址">{{ currentDetail.ip }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="currentDetail.status === 'success' ? 'success' : 'danger'">
              {{ currentDetail.status === 'success' ? '成功' : '失败' }}
            </el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="时间">{{ currentDetail.createdAt }}</el-descriptions-item>
          <el-descriptions-item v-if="currentDetail.detail" label="详情">
            <pre class="detail-json">{{ formatDetail(currentDetail.detail) }}</pre>
          </el-descriptions-item>
        </el-descriptions>
      </template>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { listAuditLogs } from '../../api/audit'
import type { AuditLog, AuditQueryParams } from '../../api/types'

const loading = ref(false)
const items = ref<AuditLog[]>([])
const total = ref(0)
const page = ref(1)
const pageSize = ref(20)
const dateRange = ref<[string, string] | null>(null)

const filters = reactive<{
  actor: string
  action: string
  status: string
}>({
  actor: '',
  action: '',
  status: '',
})

// 详情抽屉
const detailVisible = ref(false)
const currentDetail = ref<AuditLog | null>(null)

async function loadLogs(p?: number) {
  loading.value = true
  try {
    const params: AuditQueryParams = {
      page: p ?? page.value,
      pageSize: pageSize.value,
    }
    if (filters.actor) params.actor = filters.actor
    if (filters.action) params.action = filters.action
    if (filters.status) params.status = filters.status
    if (dateRange.value && dateRange.value[0]) {
      params.startFrom = dateRange.value[0]
    }
    const data = await listAuditLogs(params)
    items.value = data.items
    total.value = data.total
    page.value = data.page
  } catch (e: any) {
    if (e?.message !== '无权限访问') {
      ElMessage.error('加载审计日志失败')
    }
  } finally {
    loading.value = false
  }
}

function resetFilters() {
  filters.actor = ''
  filters.action = ''
  filters.status = ''
  dateRange.value = null
  loadLogs(1)
}

function showDetail(row: AuditLog) {
  currentDetail.value = row
  detailVisible.value = true
}

function formatDetail(detail: string): string {
  try {
    return JSON.stringify(JSON.parse(detail), null, 2)
  } catch {
    return detail
  }
}

function getActionLabel(action: string): string {
  const map: Record<string, string> = {
    login: '登录',
    create: '创建',
    update: '更新',
    enable: '启用',
    disable: '禁用',
    reset_password: '重置密码',
  }
  return map[action] || action
}

function getActionColor(action: string): string {
  const map: Record<string, string> = {
    login: '',
    create: 'success',
    update: 'warning',
    enable: 'success',
    disable: 'danger',
    reset_password: 'warning',
  }
  return map[action] || 'info'
}

onMounted(() => loadLogs())
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}
.page-header .title {
  font-weight: 600;
  font-size: 16px;
}
.filters {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}
.pagination-wrap {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
.target-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}
.target-id {
  font-family: monospace;
  font-size: 12px;
  color: #666;
}
.detail-json {
  background: #f5f5f5;
  padding: 12px;
  border-radius: 4px;
  font-size: 12px;
  max-height: 400px;
  overflow: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>