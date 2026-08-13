<template>
  <div class="alerts-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <span>🔔 告警中心</span>
        </div>
      </template>
      <div class="placeholder">
        <el-icon class="placeholder-icon"><BellFilled /></el-icon>
        <h2 class="placeholder-title">功能开发中</h2>
        <p class="placeholder-desc">告警中心正在规划中，将提供统一告警聚合、分级处置、告警溯源与可视化大屏能力。</p>
        <p class="placeholder-sub">计划接入 Zabbix / Prometheus / ELK 等多源告警，由 Eventide 统一采集后汇聚至此。</p>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { BellFilled } from '@element-plus/icons-vue'
</script>

<style scoped>
.page-header {
  font-weight: 600;
}

.placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 80px 20px;
  text-align: center;
}

.placeholder-icon {
  font-size: 72px;
  color: #c0c4cc;
  margin-bottom: 24px;
}

.placeholder-title {
  font-size: 22px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}

.placeholder-desc {
  font-size: 14px;
  color: #606266;
  margin: 0 0 8px 0;
  max-width: 560px;
  line-height: 1.6;
}

.placeholder-sub {
  font-size: 13px;
  color: #909399;
  margin: 0;
  max-width: 560px;
  line-height: 1.6;
}
</style>
