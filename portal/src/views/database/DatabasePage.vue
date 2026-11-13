<template>
  <div class="placeholder-page">
    <el-card shadow="never">
      <el-empty description="DB 数据库">
        <p class="placeholder-desc">数据库实例仍以资产清单和配置项为准，这里不展示模拟连接数。</p>
        <el-tag type="info">规划中</el-tag>
      </el-empty>
    </el-card>
  </div>
</template>

<style scoped>
.placeholder-page { min-height: 400px; }
.placeholder-desc { color: #909399; margin: 12px 0; }
</style>
