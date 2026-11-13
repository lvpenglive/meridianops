<template>
  <div class="placeholder-page">
    <el-card shadow="never">
      <el-empty description="费用中心">
        <p class="placeholder-desc">费用数据尚未接入，这里不展示估算金额。</p>
        <el-tag type="info">规划中</el-tag>
      </el-empty>
    </el-card>
  </div>
</template>

<style scoped>
.placeholder-page { min-height: 400px; }
.placeholder-desc { color: #909399; margin: 12px 0; }
</style>
