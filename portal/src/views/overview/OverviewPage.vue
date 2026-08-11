<template>
  <div class="overview-page">
    <el-row :gutter="16">
      <el-col :span="6">
        <el-card class="stat-card" shadow="hover">
          <div class="stat-content">
            <div class="stat-icon" style="background: #409EFF"><el-icon :size="28"><Server /></el-icon></div>
            <div class="stat-info">
              <div class="stat-value">{{ overview.agents.online }} / {{ overview.agents.total }}</div>
              <div class="stat-label">Agent 在线</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card class="stat-card" shadow="hover">
          <div class="stat-content">
            <div class="stat-icon" style="background: #67C23A"><el-icon :size="28"><Cpu /></el-icon></div>
            <div class="stat-info">
              <div class="stat-value">{{ overview.services.running }} / {{ overview.services.total }}</div>
              <div class="stat-label">服务运行中</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card class="stat-card" shadow="hover">
          <div class="stat-content">
            <div class="stat-icon" style="background: #F56C6C"><el-icon :size="28"><Warning /></el-icon></div>
            <div class="stat-info">
              <div class="stat-value">{{ overview.alerts.firing }}</div>
              <div class="stat-label">活动告警</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card class="stat-card" shadow="hover">
          <div class="stat-content">
            <div class="stat-icon" style="background: #E6A23C"><el-icon :size="28"><Monitor /></el-icon></div>
            <div class="stat-info">
              <div class="stat-value">{{ overview.hosts.healthy }} / {{ overview.hosts.total }}</div>
              <div class="stat-label">主机健康</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="16" style="margin-top: 16px">
      <el-col :span="16">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>🚨 实时告警流</span>
              <el-tag type="danger" effect="dark">{{ overview.alerts.firing }} 活动</el-tag>
            </div>
          </template>
          <el-table :data="overview.recentAlerts" stripe style="width: 100%" :row-style="{ height: '50px' }">
            <el-table-column width="100">
              <template #default="{ row }">
                <el-tag :type="getSeverityType(row.severity)" effect="dark">{{ getSeverityLabel(row.severity) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="title" label="告警标题" min-width="300" />
            <el-table-column prop="source" label="来源" width="120">
              <template #default="{ row }">
                <el-tag size="small">{{ getSourceLabel(row.source) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="agent" label="Agent" width="120" />
            <el-table-column prop="service" label="服务" width="160" />
            <el-table-column prop="createdAt" label="时间" width="170" />
            <el-table-column width="100">
              <template #default="{ row }">
                <el-tag :type="row.status === 'firing' ? 'danger' : row.status === 'acknowledged' ? 'warning' : 'success'" size="small">
                  {{ row.status === 'firing' ? '活动' : row.status === 'acknowledged' ? '处理中' : '已解决' }}
                </el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>📊 服务状态分布</span>
            </div>
          </template>
          <div ref="chartRef" style="height: 300px"></div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="16" style="margin-top: 16px">
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>🖥️ Agent 状态</span>
            </div>
          </template>
          <el-table :data="agents" stripe size="small" style="width: 100%">
            <el-table-column prop="hostname" label="主机" />
            <el-table-column prop="ip" label="IP" width="130" />
            <el-table-column width="100">
              <template #default="{ row }">
                <el-tag :type="row.status === 'online' ? 'success' : 'info'" size="small">
                  {{ row.status === 'online' ? '在线' : '离线' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="services" label="服务数" width="80" />
            <el-table-column width="120" label="CPU">
              <template #default="{ row }">
                <el-progress :percentage="row.cpu" :color="getProgressColor(row.cpu)" :stroke-width="8" />
              </template>
            </el-table-column>
            <el-table-column width="120" label="内存">
              <template #default="{ row }">
                <el-progress :percentage="row.memory" :color="getProgressColor(row.memory)" :stroke-width="8" />
              </template>
            </el-table-column>
            <el-table-column prop="uptime" label="运行时长" width="100" />
          </el-table>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>🔗 已接入系统</span>
            </div>
          </template>
          <div class="systems-grid">
            <div v-for="sys in systems" :key="sys.id" class="system-item">
              <div class="system-icon" :class="sys.status">
                <el-icon :size="24"><component :is="getSystemIcon(sys.type)" /></el-icon>
              </div>
              <div class="system-info">
                <div class="system-name">{{ sys.name }}</div>
                <div class="system-meta">
                  <el-tag :type="sys.status === 'online' ? 'success' : 'info'" size="small">
                    {{ sys.status === 'online' ? '在线' : '离线' }}
                  </el-tag>
                  <span v-if="sys.version" class="version">v{{ sys.version }}</span>
                </div>
              </div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import * as echarts from 'echarts'
import { mockOverview, mockAgents, mockSystems } from '../../mock/data'
import type { AgentInfo, SystemInfo } from '../../api/types'

const overview = ref(mockOverview)
const agents = ref<AgentInfo[]>(mockAgents)
const systems = ref<SystemInfo[]>(mockSystems)
const chartRef = ref<HTMLElement>()
let chart: echarts.ECharts | null = null

function getSeverityType(severity: string) {
  return { critical: 'danger', warning: 'warning', info: 'info', resolved: 'success' }[severity] || 'info'
}

function getSeverityLabel(severity: string) {
  return { critical: '严重', warning: '警告', info: '信息', resolved: '恢复' }[severity] || severity
}

function getSourceLabel(source: string) {
  return { eventide: 'Eventide', zabbix: 'Zabbix', axleops: 'AxleOps', prometheus: 'Prometheus', elk: 'ELK' }[source] || source
}

function getProgressColor(percent: number) {
  if (percent >= 80) return '#F56C6C'
  if (percent >= 60) return '#E6A23C'
  return '#67C23A'
}

function getSystemIcon(type: string) {
  return { 'service-mgmt': 'Operation', 'alert-center': 'Bell', 'monitoring': 'Monitor', 'logging': 'Document', 'metrics': 'TrendCharts', 'tracing': 'Position' }[type] || 'Link'
}

onMounted(async () => {
  await nextTick()
  if (chartRef.value) {
    chart = echarts.init(chartRef.value)
    chart.setOption({
      tooltip: { trigger: 'item' },
      legend: { bottom: '5%', left: 'center' },
      series: [{
        type: 'pie',
        radius: ['40%', '70%'],
        avoidLabelOverlap: false,
        itemStyle: { borderRadius: 6, borderColor: '#fff', borderWidth: 2 },
        label: { show: true, formatter: '{b}: {c}' },
        data: [
          { value: overview.value.services.running, name: '运行中', itemStyle: { color: '#67C23A' } },
          { value: overview.value.services.stopped, name: '已停止', itemStyle: { color: '#909399' } },
          { value: overview.value.services.error, name: '异常', itemStyle: { color: '#F56C6C' } }
        ]
      }]
    })
  }
})
</script>

<style scoped>
.stat-card {
  border-radius: 8px;
  border: none;
}

.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}

.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
}

.stat-value {
  font-size: 28px;
  font-weight: bold;
  color: #303133;
  line-height: 1;
}

.stat-label {
  font-size: 13px;
  color: #909399;
  margin-top: 4px;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-weight: 600;
}

.systems-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}

.system-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px;
  background: #f8f9fb;
  border-radius: 8px;
  transition: all 0.2s;
}

.system-item:hover {
  background: #ecf5ff;
  transform: translateY(-2px);
}

.system-icon {
  width: 44px;
  height: 44px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #e6f7ec;
  color: #67C23A;
}

.system-icon.offline {
  background: #f4f4f5;
  color: #909399;
}

.system-name {
  font-size: 14px;
  font-weight: 500;
  color: #303133;
}

.system-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
}

.version {
  font-size: 12px;
  color: #909399;
}
</style>
