<template>
  <div class="cost-page">
    <el-card shadow="never">
      <template #header>
        <div class="page-header">
          <span>💰 费用中心</span>
          <el-radio-group v-model="period" size="small">
            <el-radio-button label="week">本周</el-radio-button>
            <el-radio-button label="month">本月</el-radio-button>
            <el-radio-button label="year">本年</el-radio-button>
          </el-radio-group>
        </div>
      </template>
      <el-row :gutter="16">
        <el-col :span="8">
          <el-statistic title="总费用" :value="128650" precision="2" prefix="¥" />
        </el-col>
        <el-col :span="8">
          <el-statistic title="计算资源" :value="58400" precision="2" prefix="¥" />
        </el-col>
        <el-col :span="8">
          <el-statistic title="存储资源" :value="32100" precision="2" prefix="¥" />
        </el-col>
      </el-row>
      <el-divider />
      <el-row :gutter="16">
        <el-col :span="12">
          <div ref="chartRef" style="height: 300px"></div>
        </el-col>
        <el-col :span="12">
          <el-table :data="costBreakdown" stripe>
            <el-table-column prop="category" label="费用类型" />
            <el-table-column prop="amount" label="金额" width="120">
              <template #default="{ row }">¥{{ row.amount.toLocaleString() }}</template>
            </el-table-column>
            <el-table-column prop="percent" label="占比" width="100">
              <template #default="{ row }">{{ row.percent }}%</template>
            </el-table-column>
          </el-table>
        </el-col>
      </el-row>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import * as echarts from 'echarts'

const period = ref('month')
const chartRef = ref<HTMLElement>()
const costBreakdown = ref([
  { category: '云服务器 ECS', amount: 45200, percent: 35 },
  { category: '云数据库 RDS', amount: 28400, percent: 22 },
  { category: '对象存储 OSS', amount: 18900, percent: 15 },
  { category: '负载均衡 SLB', amount: 12300, percent: 10 },
  { category: 'CDN 加速', amount: 8700, percent: 7 },
  { category: '其他', amount: 15150, percent: 11 }
])

onMounted(async () => {
  await nextTick()
  if (chartRef.value) {
    const chart = echarts.init(chartRef.value)
    chart.setOption({
      tooltip: { trigger: 'axis' },
      xAxis: { type: 'category', data: ['周一', '周二', '周三', '周四', '周五', '周六', '周日'] },
      yAxis: { type: 'value', name: '费用 (¥)' },
      series: [{
        type: 'bar',
        data: [18200, 19300, 17600, 21200, 22800, 15400, 14150],
        itemStyle: { color: '#409EFF', borderRadius: [4, 4, 0, 0] }
      }]
    })
  }
})
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}
</style>
