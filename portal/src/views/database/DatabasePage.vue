<template>
  <div class="database-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <span>🗄️ DB 数据库</span>
          <el-button type="primary" plain>新增实例</el-button>
        </div>
      </template>
      <el-row :gutter="16">
        <el-col :span="8" v-for="db in databases" :key="db.id">
          <el-card class="db-card" shadow="hover">
            <div class="db-header">
              <div class="db-type" :class="db.type">
                <el-icon :size="24"><Coin /></el-icon>
              </div>
              <div class="db-info">
                <div class="db-name">{{ db.name }}</div>
                <div class="db-meta">{{ db.type }} · {{ db.version }}</div>
              </div>
            </div>
            <div class="db-body">
              <div class="db-item">
                <span class="label">状态</span>
                <el-tag :type="db.status === 'running' ? 'success' : 'danger'" size="small">
                  {{ db.status === 'running' ? '运行中' : '异常' }}
                </el-tag>
              </div>
              <div class="db-item">
                <span class="label">连接数</span>
                <span class="value">{{ db.connections }}</span>
              </div>
              <div class="db-item">
                <span class="label">QPS</span>
                <span class="value">{{ db.qps }}</span>
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

const databases = ref([
  { id: 'db-1', name: '订单主库', type: 'MySQL', version: '8.0.34', status: 'running', connections: 156, qps: 2340 },
  { id: 'db-2', name: '订单从库', type: 'MySQL', version: '8.0.34', status: 'running', connections: 45, qps: 890 },
  { id: 'db-3', name: '用户库', type: 'MySQL', version: '8.0.34', status: 'running', connections: 89, qps: 1200 },
  { id: 'db-4', name: '缓存主节点', type: 'Redis', version: '7.2', status: 'running', connections: 234, qps: 15600 },
  { id: 'db-5', name: '消息队列', type: 'Kafka', version: '3.6', status: 'running', connections: 56, qps: 3400 },
  { id: 'db-6', name: '分析库', type: 'PostgreSQL', version: '16.1', status: 'error', connections: 0, qps: 0 }
])
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  font-weight: 600;
}

.db-card {
  margin-bottom: 16px;
  border-radius: 8px;
}

.db-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-bottom: 12px;
  border-bottom: 1px solid #f0f0f0;
}

.db-type {
  width: 44px;
  height: 44px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #e6f7ec;
  color: #67C23A;
}

.db-type.Redis {
  background: #fef0f0;
  color: #F56C6C;
}

.db-type.Kafka {
  background: #fdf6ec;
  color: #E6A23C;
}

.db-type.PostgreSQL {
  background: #ecf5ff;
  color: #409EFF;
}

.db-name {
  font-size: 16px;
  font-weight: 600;
}

.db-meta {
  font-size: 12px;
  color: #909399;
  margin-top: 2px;
}

.db-body {
  padding-top: 12px;
}

.db-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 0;
}

.db-item .label {
  color: #909399;
  font-size: 13px;
}

.db-item .value {
  font-weight: 600;
}
</style>
