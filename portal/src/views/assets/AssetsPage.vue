<template>
  <div class="assets-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <span>🌐 资产管理</span>
          <el-button type="primary" :icon="Plus">新增 Agent</el-button>
        </div>
      </template>
      <el-table :data="agents" stripe>
        <el-table-column prop="hostname" label="主机名" min-width="140" />
        <el-table-column prop="ip" label="IP 地址" width="140" />
        <el-table-column width="100" label="状态">
          <template #default="{ row }">
            <el-tag :type="row.status === 'online' ? 'success' : 'info'" size="small">
              {{ row.status === 'online' ? '在线' : '离线' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="services" label="服务数" width="100" />
        <el-table-column width="150" label="CPU">
          <template #default="{ row }">
            <el-progress :percentage="row.cpu" :stroke-width="8" />
          </template>
        </el-table-column>
        <el-table-column width="150" label="内存">
          <template #default="{ row }">
            <el-progress :percentage="row.memory" :stroke-width="8" />
          </template>
        </el-table-column>
        <el-table-column prop="uptime" label="运行时长" width="120" />
        <el-table-column width="150" label="操作" fixed="right">
          <template #default>
            <el-button size="small" link type="primary">详情</el-button>
            <el-button size="small" link type="primary">配置</el-button>
            <el-button size="small" link type="danger">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { mockAgents } from '../../mock/data'

const agents = ref(mockAgents)
</script>

<style scoped>
.page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-weight: 600;
}
</style>
