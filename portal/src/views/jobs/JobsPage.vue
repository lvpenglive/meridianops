<template>
  <div class="jobs-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <span>⚡ 作业中心</span>
          <el-button type="primary" :icon="Plus">新建作业</el-button>
        </div>
      </template>
      <el-row :gutter="16">
        <el-col :span="16">
          <el-table :data="jobs" stripe>
            <el-table-column prop="name" label="作业名称" min-width="220" />
            <el-table-column width="120" label="状态">
              <template #default="{ row }">
                <el-tag :type="getJobStatusType(row.status)" size="small">
                  {{ getJobStatusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="creator" label="创建人" width="100" />
            <el-table-column prop="createdAt" label="创建时间" width="170" />
            <el-table-column prop="duration" label="耗时" width="100" />
            <el-table-column label="目标 Agent" min-width="180">
              <template #default="{ row }">
                <el-tag v-for="t in row.targets" :key="t" size="small" style="margin-right: 4px">{{ t }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column width="150" label="操作" fixed="right">
              <template #default="{ row }">
                <el-button size="small" link type="primary" :disabled="row.status === 'running'">执行</el-button>
                <el-button size="small" link type="primary">日志</el-button>
                <el-button size="small" link type="danger">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-col>
        <el-col :span="8">
          <el-card shadow="never" style="margin-bottom: 16px">
            <template #header><span>📈 作业统计</span></template>
            <div class="job-stats">
              <div class="stat-block">
                <span class="stat-num">{{ stats.completed }}</span>
                <span class="stat-text">已完成</span>
              </div>
              <div class="stat-block">
                <span class="stat-num">{{ stats.running }}</span>
                <span class="stat-text">执行中</span>
              </div>
              <div class="stat-block">
                <span class="stat-num">{{ stats.failed }}</span>
                <span class="stat-text">失败</span>
              </div>
            </div>
          </el-card>
          <el-card shadow="never">
            <template #header><span>⚙️ 常用剧本</span></template>
            <div class="playbook-list">
              <div v-for="pb in playbooks" :key="pb.id" class="playbook-item">
                <el-icon :size="16" color="#409EFF"><Operation /></el-icon>
                <span>{{ pb.name }}</span>
                <el-button size="small" link type="primary">执行</el-button>
              </div>
            </div>
          </el-card>
        </el-col>
      </el-row>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { mockJobs } from '../../mock/data'

const jobs = ref(mockJobs)
const stats = ref({ completed: 45, running: 2, failed: 3 })

const playbooks = ref([
  { id: 'pb1', name: 'MySQL 主从延迟自动修复' },
  { id: 'pb2', name: '服务批量重启' },
  { id: 'pb3', name: '日志清理脚本' },
  { id: 'pb4', name: '磁盘空间检查与清理' }
])

function getJobStatusType(status: string) {
  return { completed: 'success', running: 'primary', pending: 'info', failed: 'danger' }[status] || 'info'
}

function getJobStatusLabel(status: string) {
  return { completed: '已完成', running: '执行中', pending: '等待中', failed: '失败' }[status] || status
}
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  font-weight: 600;
}

.job-stats {
  display: flex;
  justify-content: space-around;
}

.stat-block {
  text-align: center;
}

.stat-num {
  display: block;
  font-size: 24px;
  font-weight: bold;
  color: #303133;
}

.stat-text {
  font-size: 12px;
  color: #909399;
}

.playbook-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.playbook-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: #f8f9fb;
  border-radius: 6px;
  transition: all 0.2s;
}

.playbook-item:hover {
  background: #ecf5ff;
}

.playbook-item span {
  flex: 1;
  font-size: 13px;
}
</style>
