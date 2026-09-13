<template>
  <div class="components-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <div>
            <div class="title">组件状态</div>
            <div class="sub">只读展示配置中的依赖连通情况，地址不可在此修改</div>
          </div>
          <div class="header-actions">
            <el-tag :type="summaryType" effect="plain">
              正常 {{ data?.ok ?? 0 }} / 异常 {{ data?.failed ?? 0 }} / 共 {{ data?.total ?? 0 }}
            </el-tag>
            <el-button :icon="Refresh" :loading="loading" @click="load">刷新</el-button>
          </div>
        </div>
      </template>

      <el-table :data="data?.items ?? []" v-loading="loading" stripe>
        <el-table-column label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'ok' ? 'success' : 'danger'" effect="dark" size="small">
              {{ row.status === 'ok' ? '正常' : '异常' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="name" label="组件" width="180" />
        <el-table-column prop="kind" label="类型" width="140">
          <template #default="{ row }">{{ kindLabel(row.kind) }}</template>
        </el-table-column>
        <el-table-column prop="address" label="地址" min-width="280">
          <template #default="{ row }">
            <code class="addr">{{ row.address }}</code>
          </template>
        </el-table-column>
        <el-table-column prop="latencyMs" label="耗时" width="100">
          <template #default="{ row }">{{ row.latencyMs }} ms</template>
        </el-table-column>
        <el-table-column prop="message" label="说明" min-width="220" show-overflow-tooltip />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Refresh } from '@element-plus/icons-vue'
import { listComponentStatus, type ComponentStatusResponse } from '../../api/system'

const loading = ref(false)
const data = ref<ComponentStatusResponse | null>(null)

const summaryType = computed(() => {
  if (!data.value) return 'info'
  return data.value.failed > 0 ? 'danger' : 'success'
})

function kindLabel(kind: string) {
  const map: Record<string, string> = {
    self: '本服务',
    database: '数据库',
    'log-store': '日志存储',
    'log-index': '日志索引',
    'service-mgmt': '服务管理',
    'alert-center': '告警中心',
    cmdb: 'CMDB',
    logging: '日志',
    metrics: '指标',
  }
  return map[kind] ?? kind
}

async function load() {
  loading.value = true
  try {
    data.value = await listComponentStatus()
  } catch {
    data.value = null
  } finally {
    loading.value = false
  }
}

onMounted(load)
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
}
.title {
  font-weight: 600;
}
.sub {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}
.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}
.addr {
  font-size: 12px;
  background: #f4f4f5;
  padding: 2px 6px;
  border-radius: 4px;
}
</style>
