<template>
  <div class="tickets-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <span>📋 工单系统</span>
          <div class="header-actions">
            <el-tag type="danger">待处理: {{ pendingCount }}</el-tag>
            <el-button type="primary" :icon="Plus">新建工单</el-button>
          </div>
        </div>
      </template>
      <el-table :data="tickets" stripe>
        <el-table-column prop="id" label="工单号" width="120" />
        <el-table-column prop="title" label="标题" min-width="250" />
        <el-table-column width="100" label="类型">
          <template #default="{ row }">
            <el-tag :type="getTicketTypeColor(row.type)" size="small">{{ row.type }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column width="100" label="优先级">
          <template #default="{ row }">
            <el-tag :type="getPriorityColor(row.priority)" size="small">{{ row.priority }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column width="100" label="状态">
          <template #default="{ row }">
            <el-tag :type="getStatusColor(row.status)" size="small">{{ row.status }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="assignee" label="处理人" width="100" />
        <el-table-column prop="createdAt" label="创建时间" width="170" />
        <el-table-column width="150" label="操作" fixed="right">
          <template #default>
            <el-button size="small" link type="primary">处理</el-button>
            <el-button size="small" link type="primary">详情</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'

const tickets = ref([
  { id: 'T-001', title: 'order-service CPU 持续高，需排查', type: '告警处理', priority: 'P1', status: '待处理', assignee: '张三', createdAt: '2026-08-10 14:30' },
  { id: 'T-002', title: 'MySQL 主从延迟问题', type: '故障修复', priority: 'P1', status: '处理中', assignee: '李四', createdAt: '2026-08-10 14:28' },
  { id: 'T-003', title: '新增 Redis 集群节点', type: '变更申请', priority: 'P2', status: '待处理', assignee: '王五', createdAt: '2026-08-10 10:00' },
  { id: 'T-004', title: '支付系统 SSL 证书续期', type: '运维任务', priority: 'P3', status: '已完成', assignee: '赵六', createdAt: '2026-08-09 16:00' },
  { id: 'T-005', title: '用户反馈登录超时', type: '告警处理', priority: 'P2', status: '已完成', assignee: '张三', createdAt: '2026-08-09 11:30' }
])

const pendingCount = computed(() => tickets.value.filter(t => t.status === '待处理').length)

function getTicketTypeColor(type: string) {
  return { '告警处理': 'danger', '故障修复': 'warning', '变更申请': 'primary', '运维任务': 'info' }[type] || 'info'
}

function getPriorityColor(priority: string) {
  return { 'P0': 'danger', 'P1': 'warning', 'P2': 'primary', 'P3': 'info' }[priority] || 'info'
}

function getStatusColor(status: string) {
  return { '待处理': 'info', '处理中': 'warning', '已完成': 'success', '已关闭': 'danger' }[status] || 'info'
}
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 16px;
}
</style>
