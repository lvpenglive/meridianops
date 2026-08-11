<template>
  <div class="config-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <span>⚙️ 配置中心</span>
          <el-button type="primary" plain>新建配置</el-button>
        </div>
      </template>
      <el-tabs v-model="activeTab">
        <el-tab-pane label="Agent 配置" name="agent">
          <el-table :data="agentConfigs" stripe>
            <el-table-column prop="name" label="配置项" min-width="200" />
            <el-table-column prop="value" label="当前值" min-width="200" />
            <el-table-column prop="agent" label="所属 Agent" width="150" />
            <el-table-column width="200" label="操作">
              <template #default>
                <el-button size="small" link type="primary">编辑</el-button>
                <el-button size="small" link type="primary">热更</el-button>
                <el-button size="small" link type="primary">历史</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
        <el-tab-pane label="服务规格" name="service">
          <el-table :data="serviceConfigs" stripe>
            <el-table-column prop="name" label="服务名" width="180" />
            <el-table-column prop="type" label="类型" width="100" />
            <el-table-column prop="version" label="版本" width="100" />
            <el-table-column prop="healthUrl" label="健康检查地址" min-width="200" />
            <el-table-column prop="watchdog" label="看门狗" width="100">
              <template #default="{ row }">
                <el-tag :type="row.watchdog === 'enabled' ? 'success' : 'info'" size="small">
                  {{ row.watchdog === 'enabled' ? '已启用' : '未启用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column width="150" label="操作">
              <template #default>
                <el-button size="small" link type="primary">编辑</el-button>
                <el-button size="small" link type="primary">启停</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>
      </el-tabs>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'

const activeTab = ref('agent')

const agentConfigs = ref([
  { name: 'bind', value: '0.0.0.0:9100', agent: 'agent-01' },
  { name: 'watchdog_enabled', value: 'true', agent: 'agent-01' },
  { name: 'watchdog_interval_secs', value: '15', agent: 'agent-01' },
  { name: 'data_dir', value: './data', agent: 'agent-02' },
  { name: 'token', value: '********', agent: 'agent-02' }
])

const serviceConfigs = ref([
  { name: 'order-service', type: 'jar', version: 'v1.2.3', healthUrl: 'http://localhost:8080/actuator/health', watchdog: 'enabled' },
  { name: 'payment-service', type: 'jar', version: 'v1.1.0', healthUrl: 'http://localhost:8081/health', watchdog: 'enabled' },
  { name: 'user-service', type: 'jar', version: 'v2.0.1', healthUrl: 'http://localhost:8082/api/health', watchdog: 'enabled' },
  { name: 'notification-service', type: 'python', version: 'v0.9.5', healthUrl: 'http://localhost:9000/health', watchdog: 'disabled' }
])
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  font-weight: 600;
}
</style>
