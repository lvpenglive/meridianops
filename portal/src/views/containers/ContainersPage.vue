<template>
  <div class="placeholder-page">
    <el-card shadow="never">
      <el-empty description="容器管理模块">
        <template #image>
          <el-icon :size="80" color="#c0c4cc"><Box /></el-icon>
        </template>
        <p class="placeholder-desc">对接 Kubernetes API，统一管理容器集群</p>
        <el-tag type="info">规划中</el-tag>
      </el-empty>
    </el-card>
  </div>
</template>

<script setup lang="ts">
</script>

<style scoped>
.placeholder-page {
  min-height: 400px;
}

.placeholder-desc {
  color: #909399;
  margin: 12px 0;
}
</style>
