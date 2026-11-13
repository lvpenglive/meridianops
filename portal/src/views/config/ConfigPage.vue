<template>
  <div class="placeholder-page">
    <el-card shadow="never">
      <el-empty description="配置中心">
        <p class="placeholder-desc">配置项管理尚未接入，Agent 配置请在监控模板和作业中心维护。</p>
        <el-tag type="info">规划中</el-tag>
      </el-empty>
    </el-card>
  </div>
</template>

<style scoped>
.placeholder-page { min-height: 400px; }
.placeholder-desc { color: #909399; margin: 12px 0; }
</style>
