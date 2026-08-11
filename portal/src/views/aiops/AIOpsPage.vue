<template>
  <div class="aiops-page">
    <el-row :gutter="16">
      <el-col :span="16">
        <el-card shadow="never">
          <template #header>
            <div class="page-header">
              <span>🤖 AIOps 智能运维</span>
              <el-button type="primary" :icon="Cpu" plain>运行诊断</el-button>
            </div>
          </template>
          <div class="diagnosis-section">
            <h3>🔍 根因分析</h3>
            <el-alert
              type="error"
              :closable="false"
              show-icon
              title="检测到异常模式"
              description="最近 30 分钟内，order-service 相关告警 5 条，关联 MySQL 主从延迟。系统分析可能根因：主机 agent-03 磁盘 IO 瓶颈。"
            />
            <div class="diagnosis-tree">
              <el-tree :data="diagnosisTree" node-key="id" default-expand-all>
                <template #default="{ node, data }">
                  <span class="tree-node" :class="data.level">
                    <el-icon v-if="data.level === 'root'" color="#F56C6C"><Warning /></el-icon>
                    <el-icon v-else-if="data.level === 'cause'" color="#E6A23C"><InfoFilled /></el-icon>
                    <el-icon v-else color="#67C23A"><CircleCheck /></el-icon>
                    <span class="node-label">{{ node.label }}</span>
                    <el-tag v-if="data.confidence" size="small" class="confidence">置信度: {{ data.confidence }}%</el-tag>
                  </span>
                </template>
              </el-tree>
            </div>
          </div>

          <el-divider />

          <div class="suggestion-section">
            <h3>💡 处置建议</h3>
            <el-timeline>
              <el-timeline-item
                v-for="(suggestion, idx) in suggestions"
                :key="idx"
                :timestamp="suggestion.time"
                :type="suggestion.type"
              >
                <div class="suggestion-content">
                  <div class="suggestion-title">{{ suggestion.title }}</div>
                  <div class="suggestion-desc">{{ suggestion.desc }}</div>
                </div>
              </el-timeline-item>
            </el-timeline>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="never">
          <template #header><span>📚 知识档案匹配</span></template>
          <el-table :data="kbMatches" size="small" stripe>
            <el-table-column prop="id" label="ID" width="80" />
            <el-table-column prop="title" label="标题" min-width="150" />
            <el-table-column prop="score" label="匹配度" width="80">
              <template #default="{ row }">
                <el-tag :type="row.score >= 80 ? 'success' : 'warning'" size="small">{{ row.score }}%</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-card>

        <el-card shadow="never" style="margin-top: 16px">
          <template #header><span>📊 异常检测统计</span></template>
          <div class="stats">
            <div class="stat-item">
              <span class="stat-value">{{ anomalyStats.total }}</span>
              <span class="stat-label">检测事件</span>
            </div>
            <div class="stat-item">
              <span class="stat-value">{{ anomalyStats.correct }}</span>
              <span class="stat-label">准确识别</span>
            </div>
            <div class="stat-item">
              <span class="stat-value">{{ anomalyStats.saved }}</span>
              <span class="stat-label">节省时间(小时)</span>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const diagnosisTree = ref([
  {
    id: 'root',
    label: 'order-service CPU 高 (告警)',
    level: 'root',
    confidence: 92,
    children: [
      {
        id: 'cause1',
        label: 'MySQL 主从同步延迟',
        level: 'cause',
        confidence: 88,
        children: [
          { id: 'leaf1', label: '主机 agent-03 磁盘 IO > 90%', level: 'direct', confidence: 95 },
          { id: 'leaf2', label: 'binlog 写入瓶颈', level: 'direct', confidence: 82 }
        ]
      },
      {
        id: 'cause2',
        label: 'Redis 缓存穿透',
        level: 'cause',
        confidence: 45
      }
    ]
  }
])

const suggestions = ref([
  { time: '立即', type: 'danger', title: '检查磁盘 IO', desc: '登录 agent-03 执行 iostat -x 1 3，确认 IO 瓶颈' },
  { time: '5 分钟', type: 'warning', title: '考虑迁移 SSD', desc: 'order-service 主库磁盘 IO 持续高，建议迁移至 SSD 节点' },
  { time: '10 分钟', type: 'primary', title: '优化 binlog 配置', desc: '调整 sync_binlog 和 innodb_flush_log_at_trx_commit 参数' },
  { time: '长期', type: 'success', title: '设置 IO 告警阈值', desc: '在 Zabbix 中配置磁盘 IO > 80% 提前告警' }
])

const kbMatches = ref([
  { id: 'KB-087', title: 'MySQL 主从延迟导致订单系统雪崩', score: 92 },
  { id: 'KB-063', title: '磁盘 IO 瓶颈诊断与优化', score: 85 },
  { id: 'KB-041', title: '订单系统性能问题排查手册', score: 78 }
])

const anomalyStats = ref({ total: 156, correct: 138, saved: 42 })
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  font-weight: 600;
}

.diagnosis-section h3,
.suggestion-section h3 {
  margin: 0 0 16px 0;
  font-size: 15px;
}

.tree-node {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tree-node.root {
  color: #F56C6C;
  font-weight: 600;
}

.tree-node.cause {
  color: #E6A23C;
}

.confidence {
  margin-left: auto;
}

.stats {
  display: flex;
  justify-content: space-around;
}

.stat-item {
  text-align: center;
}

.stat-value {
  display: block;
  font-size: 24px;
  font-weight: bold;
  color: #303133;
}

.stat-label {
  font-size: 12px;
  color: #909399;
}
</style>
