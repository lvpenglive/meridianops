<template>
  <div class="report-index">
    <div class="page-header">
      <div class="header-title">
        <el-icon class="header-icon"><TrendCharts /></el-icon>
        <div>
          <h2>报表中心</h2>
          <p class="sub-title">银行运维一体化平台 · 多维度统计报表矩阵</p>
        </div>
      </div>
      <div class="header-stats">
        <span class="stat-badge available">{{ availableCount }} 项可用</span>
        <span class="stat-badge planned">{{ plannedCount }} 项规划中</span>
      </div>
    </div>

    <div v-for="group in reportGroups" :key="group.name" class="report-group">
      <div class="group-header">
        <el-icon class="group-icon"><component :is="group.icon" /></el-icon>
        <h3>{{ group.name }}</h3>
        <el-tag size="small" :type="group.reports.every(r => r.available) ? 'success' : 'warning'">
          {{ group.reports.filter(r => r.available).length }} / {{ group.reports.length }} 可用
        </el-tag>
      </div>
      <div class="card-grid">
        <div
          v-for="r in group.reports"
          :key="r.path"
          class="report-card"
          :class="{ 'is-available': r.available, 'is-planned': !r.available }"
          @click="r.available && router.push(r.path)"
        >
          <div class="card-icon-wrap">
            <el-icon class="card-icon"><component :is="r.icon" /></el-icon>
          </div>
          <div class="card-body">
            <div class="card-title">
              {{ r.title }}
              <el-tag v-if="!r.available" size="small" type="info" effect="plain">规划中</el-tag>
              <el-tag v-else size="small" type="success" effect="plain">可用</el-tag>
            </div>
            <p class="card-desc">{{ r.desc }}</p>
          </div>
          <el-icon v-if="r.available" class="card-arrow"><ArrowRight /></el-icon>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import {
  TrendCharts, ArrowRight,
  Lock, WarningFilled, Checked,
  Bell, DataAnalysis, Histogram, Timer,
  Tickets, List, DocumentChecked,
  Monitor, RefreshRight, Connection,
  Box, Coin, Calendar,
} from '@element-plus/icons-vue'

const router = useRouter()

interface ReportItem {
  title: string
  desc: string
  path: string
  icon: string
  available: boolean
}
interface ReportGroup {
  name: string
  icon: string
  reports: ReportItem[]
}

// 报表矩阵：按业务域分组。已实现 available=true，未实现 available=false（灰色预留位）。
const reportGroups: ReportGroup[] = [
  {
    name: '安全审计',
    icon: 'Lock',
    reports: [
      {
        title: '登录安全分析',
        desc: '登录成功/失败趋势、失败登录 TOP 用户、当前锁定账号',
        path: '/report/login-security',
        icon: 'WarningFilled',
        available: true,
      },
      {
        title: '敏感操作审计',
        desc: '敏感操作趋势、TOP 操作人、敏感操作明细（删用户/改权限/改配置）',
        path: '/report/sensitive-ops',
        icon: 'Checked',
        available: true,
      },
      {
        title: '合规健康度',
        desc: '弱密码/密码过期/长期未登录用户、角色权限分配统计',
        path: '/report/compliance-health',
        icon: 'Checked',
        available: true,
      },
      {
        title: '审计操作趋势',
        desc: '近 N 天审计操作量趋势，按操作类型堆叠',
        path: '/report/audit-trend',
        icon: 'DataAnalysis',
        available: true,
      },
    ],
  },
  {
    name: '告警分析',
    icon: 'Bell',
    reports: [
      { title: '告警趋势', desc: '近 30 天告警量趋势、分级分布', path: '/report/alert-trend', icon: 'DataAnalysis', available: false },
      { title: 'MTTR 平均恢复时间', desc: '告警认领/恢复时长统计', path: '/report/mttr', icon: 'Timer', available: false },
      { title: '告警来源 TOP', desc: '按系统/服务维度统计告警来源', path: '/report/alert-source', icon: 'Histogram', available: false },
      { title: '告警分级分布', desc: 'P0/P1/P2/P3 告警占比', path: '/report/alert-level', icon: 'Bell', available: false },
    ],
  },
  {
    name: '工单分析',
    icon: 'Tickets',
    reports: [
      { title: '工单趋势', desc: '工单创建/完成趋势', path: '/report/ticket-trend', icon: 'DataAnalysis', available: false },
      { title: '工单类型分布', desc: '故障/变更/巡检/需求占比', path: '/report/ticket-type', icon: 'List', available: false },
      { title: 'SLA 达标率', desc: '工单响应/处理时长合规率', path: '/report/sla', icon: 'DocumentChecked', available: false },
      { title: '处理人 TOP', desc: '工单处理量排行', path: '/report/ticket-handler', icon: 'Histogram', available: false },
    ],
  },
  {
    name: '系统健康',
    icon: 'Monitor',
    reports: [
      { title: '核心系统可用性', desc: '核心系统 SLA 可用性统计', path: '/report/availability', icon: 'Monitor', available: false },
      { title: '巡检完成率', desc: '周期性巡检任务完成情况', path: '/report/inspection', icon: 'Checked', available: false },
      { title: '变更成功率', desc: '变更工单成功率统计', path: '/report/change-success', icon: 'RefreshRight', available: false },
      { title: '作业执行趋势', desc: '近 N 天执行量/成功/失败趋势', path: '/report/job-trend', icon: 'DataAnalysis', available: true },
      { title: '作业执行统计', desc: '按作业定义汇总成功率与耗时', path: '/report/job-summary', icon: 'Histogram', available: true },
    ],
  },
  {
    name: '资产 / CMDB',
    icon: 'Connection',
    reports: [
      { title: '资产总数趋势', desc: '主机/设备资产数量变化', path: '/report/asset-trend', icon: 'DataAnalysis', available: false },
      { title: '资产分类统计', desc: '按类型/状态分布，环形图+明细', path: '/report/asset-category', icon: 'Box', available: true },
      { title: '到期设备清单', desc: '保修/维护即将到期的设备', path: '/report/asset-expiry', icon: 'Calendar', available: false },
      { title: '数据库实例统计', desc: 'DB 实例分布与容量', path: '/report/db-instances', icon: 'Coin', available: false },
      { title: '知识库分类统计', desc: '按分类统计文章数、查看量、有用数', path: '/report/knowledge-stats', icon: 'DocumentChecked', available: true },
    ],
  },
]

const availableCount = computed(() =>
  reportGroups.reduce((sum, g) => sum + g.reports.filter(r => r.available).length, 0),
)
const plannedCount = computed(() =>
  reportGroups.reduce((sum, g) => sum + g.reports.filter(r => !r.available).length, 0),
)
</script>

<style scoped>
.report-index {
  padding: 0;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 12px;
  color: #fff;
  margin-bottom: 24px;
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.2);
}
.header-title {
  display: flex;
  align-items: center;
  gap: 16px;
}
.header-icon {
  font-size: 42px;
}
.header-title h2 {
  margin: 0;
  font-size: 22px;
  font-weight: 600;
}
.sub-title {
  margin: 4px 0 0;
  font-size: 13px;
  opacity: 0.85;
}
.header-stats {
  display: flex;
  gap: 12px;
}
.stat-badge {
  padding: 6px 14px;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 500;
  background: rgba(255, 255, 255, 0.18);
}
.stat-badge.available { background: rgba(103, 194, 58, 0.35); }
.stat-badge.planned { background: rgba(230, 162, 60, 0.35); }

.report-group {
  margin-bottom: 28px;
}
.group-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
  padding: 0 4px;
}
.group-icon {
  font-size: 22px;
  color: #409eff;
}
.group-header h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
}
.report-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 18px 20px;
  background: #fff;
  border: 1px solid #ebeef5;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.25s;
  position: relative;
}
.report-card.is-available:hover {
  border-color: #409eff;
  box-shadow: 0 4px 16px rgba(64, 158, 255, 0.15);
  transform: translateY(-2px);
}
.report-card.is-planned {
  cursor: not-allowed;
  background: #fafafa;
  opacity: 0.65;
}
.card-icon-wrap {
  flex-shrink: 0;
  width: 48px;
  height: 48px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #409eff 0%, #66b1ff 100%);
}
.is-planned .card-icon-wrap {
  background: linear-gradient(135deg, #909399 0%, #b1b3b8 100%);
}
.card-icon {
  font-size: 24px;
  color: #fff;
}
.card-body {
  flex: 1;
  min-width: 0;
}
.card-title {
  font-size: 15px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 6px;
  display: flex;
  align-items: center;
  gap: 8px;
}
.card-desc {
  margin: 0;
  font-size: 12px;
  color: #909399;
  line-height: 1.5;
  overflow: hidden;
  text-overflow: ellipsis;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}
.card-arrow {
  flex-shrink: 0;
  color: #c0c4cc;
  font-size: 16px;
}
.is-available:hover .card-arrow {
  color: #409eff;
}

@media (max-width: 768px) {
  .page-header { flex-direction: column; gap: 14px; align-items: flex-start; }
  .card-grid { grid-template-columns: 1fr; }
}
</style>
