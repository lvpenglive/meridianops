<template>
  <div class="dashboard-page">
    <!-- 欢迎卡 -->
    <el-card shadow="never" class="welcome-card">
      <div class="welcome">
        <el-avatar :size="56" class="welcome__avatar">{{ initial }}</el-avatar>
        <div class="welcome__text">
          <div class="welcome__hello">{{ greeting }}，{{ displayName }}</div>
          <div class="welcome__sub">
            <el-tag size="small" :type="roleTagType" effect="dark">{{ roleLabel }}</el-tag>
            <span class="welcome__time">{{ nowText }}</span>
          </div>
        </div>
        <div class="welcome__stats">
          <div class="welcome__stat">
            <div class="welcome__stat-value">{{ stats.todayOps }}</div>
            <div class="welcome__stat-label">今日操作</div>
          </div>
          <div class="welcome__stat">
            <div class="welcome__stat-value">{{ stats.todayLogins }}</div>
            <div class="welcome__stat-label">今日登录</div>
          </div>
        </div>
      </div>
    </el-card>

    <!-- 运维统计卡片 -->
    <el-row :gutter="16" class="stat-row">
      <el-col :xs="12" :sm="12" :md="6">
        <el-card shadow="hover" class="stat-card stat-card--assets" @click="go('/assets')">
          <div class="stat-card__body">
            <div class="stat-card__icon"><Connection :size="26" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ opsStats.totalAssets }}</div>
              <div class="stat-card__label">资产总数</div>
            </div>
          </div>
          <div class="stat-card__extra">{{ opsStats.totalModels }} 个 CI 模型</div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="12" :md="6">
        <el-card shadow="hover" class="stat-card stat-card--jobs" @click="go('/jobs')">
          <div class="stat-card__body">
            <div class="stat-card__icon"><Tools :size="26" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ opsStats.totalJobDefs }}</div>
              <div class="stat-card__label">作业定义</div>
            </div>
          </div>
          <div class="stat-card__extra">启用 {{ opsStats.enabledJobDefs }} 个</div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="12" :md="6">
        <el-card shadow="hover" class="stat-card stat-card--runs" @click="go('/jobs')">
          <div class="stat-card__body">
            <div class="stat-card__icon"><CaretRight :size="26" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ opsStats.todayJobRuns }}</div>
              <div class="stat-card__label">今日执行</div>
            </div>
          </div>
          <div class="stat-card__extra">成功 {{ opsStats.todayJobSuccess }} 次</div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="12" :md="6">
        <el-card shadow="hover" class="stat-card stat-card--sync" @click="go('/sync')">
          <div class="stat-card__body">
            <div class="stat-card__icon"><Refresh :size="26" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ opsStats.totalSyncSources }}</div>
              <div class="stat-card__label">同步数据源</div>
            </div>
          </div>
          <div class="stat-card__extra">启用 {{ opsStats.enabledSyncSources }} 个</div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 统计卡片 -->
    <el-row :gutter="16" class="stat-row">
      <el-col :xs="12" :sm="12" :md="6">
        <el-card shadow="hover" class="stat-card stat-card--users" @click="go('/system/users')">
          <div class="stat-card__body">
            <div class="stat-card__icon"><User :size="26" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.totalUsers }}</div>
              <div class="stat-card__label">用户总数</div>
            </div>
          </div>
          <div class="stat-card__extra">启用 {{ stats.enabledUsers }} 人</div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="12" :md="6">
        <el-card shadow="hover" class="stat-card stat-card--roles" @click="go('/system/roles')">
          <div class="stat-card__body">
            <div class="stat-card__icon"><UserFilled :size="26" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.totalRoles }}</div>
              <div class="stat-card__label">角色总数</div>
            </div>
          </div>
          <div class="stat-card__extra">RBAC 权限模型</div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="12" :md="6">
        <el-card shadow="hover" class="stat-card stat-card--depts" @click="go('/system/departments')">
          <div class="stat-card__body">
            <div class="stat-card__icon"><OfficeBuilding :size="26" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.totalDepartments }}</div>
              <div class="stat-card__label">部门总数</div>
            </div>
          </div>
          <div class="stat-card__extra">树形组织架构</div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="12" :md="6">
        <el-card shadow="hover" class="stat-card stat-card--ops" @click="go('/audit')">
          <div class="stat-card__body">
            <div class="stat-card__icon"><Notebook :size="26" /></div>
            <div class="stat-card__info">
              <div class="stat-card__value">{{ stats.todayOps }}</div>
              <div class="stat-card__label">今日操作</div>
            </div>
          </div>
          <div class="stat-card__extra">登录成功 {{ stats.todayLogins }} 次</div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="16" class="main-row">
      <!-- 快捷入口 -->
      <el-col :xs="24" :md="8">
        <el-card shadow="never" class="shortcut-card">
          <template #header>
            <div class="card-header">
              <span class="card-header__title">⚡ 快捷入口</span>
            </div>
          </template>
          <div class="shortcut-grid">
            <div
              v-for="s in visibleShortcuts"
              :key="s.path"
              class="shortcut-item"
              @click="go(s.path)"
            >
              <div class="shortcut-item__icon" :style="{ background: s.color }">
                <component :is="s.icon" :size="20" />
              </div>
              <div class="shortcut-item__label">{{ s.label }}</div>
            </div>
          </div>
        </el-card>

        <!-- 我的最近活动 -->
        <el-card shadow="never" class="mine-card">
          <template #header>
            <div class="card-header">
              <span class="card-header__title">🕐 我的最近活动</span>
              <el-button link type="primary" size="small" @click="go('/audit')">查看全部</el-button>
            </div>
          </template>
          <el-timeline v-if="myActivities.length > 0" class="mine-timeline">
            <el-timeline-item
              v-for="item in myActivities"
              :key="item.id"
              :timestamp="item.createdAt"
              :type="item.status === 'success' ? 'success' : 'danger'"
              placement="top"
            >
              <div class="mine-item">
                <el-tag :type="actionColor(item.action)" size="small">{{ actionLabel(item.action) }}</el-tag>
                <span class="mine-item__target">{{ item.targetType }} · {{ item.targetId }}</span>
              </div>
            </el-timeline-item>
          </el-timeline>
          <el-empty v-else description="暂无活动" :image-size="60" />
        </el-card>
      </el-col>

      <!-- 全局最近活动 -->
      <el-col :xs="24" :md="16">
        <!-- 资产模型分布 + 最近作业执行 -->
        <el-row :gutter="16" class="chart-row">
          <el-col :xs="24" :sm="10">
            <el-card shadow="never" class="chart-card">
              <template #header>
                <div class="card-header">
                  <span class="card-header__title">📊 资产模型分布</span>
                </div>
              </template>
              <div ref="modelChartRef" class="chart-container"></div>
              <el-empty v-if="!loading && modelStats.length === 0" description="暂无资产" :image-size="40" />
            </el-card>
          </el-col>
          <el-col :xs="24" :sm="14">
            <el-card shadow="never" class="job-runs-card">
              <template #header>
                <div class="card-header">
                  <span class="card-header__title">🔧 最近作业执行</span>
                  <el-button link type="primary" size="small" @click="go('/jobs')">作业中心</el-button>
                </div>
              </template>
              <el-table :data="recentJobRuns" size="small" stripe max-height="280">
                <el-table-column prop="jobName" label="作业名称" min-width="120" show-overflow-tooltip />
                <el-table-column label="状态" width="80">
                  <template #default="{ row }">
                    <el-tag :type="runStatusType(row.overallStatus)" size="small" effect="dark">
                      {{ runStatusLabel(row.overallStatus) }}
                    </el-tag>
                  </template>
                </el-table-column>
                <el-table-column label="目标/成功" width="90">
                  <template #default="{ row }">
                    <span style="font-family: monospace; font-size: 12px">{{ row.successCount }}/{{ row.targetCount }}</span>
                  </template>
                </el-table-column>
                <el-table-column prop="startedBy" label="执行人" width="80" show-overflow-tooltip />
                <el-table-column prop="startedAt" label="开始时间" width="150" />
              </el-table>
              <el-empty v-if="!loading && recentJobRuns.length === 0" description="暂无执行记录" :image-size="40" />
            </el-card>
          </el-col>
        </el-row>

        <el-card shadow="never" class="recent-card">
          <template #header>
            <div class="card-header">
              <span class="card-header__title">📋 最近平台活动</span>
              <el-button link type="primary" size="small" @click="go('/audit')">进入审计中心</el-button>
            </div>
          </template>
          <el-table :data="recentActivities" v-loading="loading" stripe size="small">
            <el-table-column prop="actorUsername" label="操作人" width="110" />
            <el-table-column label="操作" width="100">
              <template #default="{ row }">
                <el-tag :type="actionColor(row.action)" size="small">
                  {{ actionLabel(row.action) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="对象" min-width="180">
              <template #default="{ row }">
                <span class="target-cell">
                  <el-tag type="info" size="small">{{ row.targetType }}</el-tag>
                  <span class="target-id">{{ row.targetId }}</span>
                </span>
              </template>
            </el-table-column>
            <el-table-column prop="ip" label="IP" width="130" />
            <el-table-column label="状态" width="80">
              <template #default="{ row }">
                <el-tag :type="row.status === 'success' ? 'success' : 'danger'" size="small">
                  {{ row.status === 'success' ? '成功' : '失败' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="createdAt" label="时间" width="170" />
          </el-table>
          <el-empty v-if="!loading && recentActivities.length === 0" description="暂无活动" />
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { User, UserFilled, OfficeBuilding, Notebook, Tools, Tickets, Document, Connection, CaretRight, Refresh } from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import { useUserStore } from '../../stores/user'
import { getDashboard } from '../../api/dashboard'
import type { DashboardData, DashboardStats, OpsStats, ModelStatItem, JobRunSummaryItem, AuditLog } from '../../api/types'

const router = useRouter()
const userStore = useUserStore()

const loading = ref(false)
const stats = ref<DashboardStats>({
  totalUsers: 0,
  enabledUsers: 0,
  totalRoles: 0,
  totalDepartments: 0,
  todayOps: 0,
  todayLogins: 0,
})
const opsStats = ref<OpsStats>({
  totalAssets: 0,
  totalModels: 0,
  totalJobDefs: 0,
  enabledJobDefs: 0,
  todayJobRuns: 0,
  todayJobSuccess: 0,
  totalSyncSources: 0,
  enabledSyncSources: 0,
})
const modelStats = ref<ModelStatItem[]>([])
const recentJobRuns = ref<JobRunSummaryItem[]>([])
const recentActivities = ref<AuditLog[]>([])
const myActivities = ref<AuditLog[]>([])

const modelChartRef = ref<HTMLElement | null>(null)
let modelChart: echarts.ECharts | null = null

// 当前时间显示（每秒刷新）
const nowText = ref('')
let timer: number | undefined
function tick() {
  const d = new Date()
  const pad = (n: number) => n.toString().padStart(2, '0')
  const weekday = ['日', '一', '二', '三', '四', '五', '六'][d.getDay()]
  nowText.value = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} 周${weekday} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

const initial = computed(() => {
  const name = userStore.user?.displayName || userStore.user?.username || 'U'
  return name.charAt(0).toUpperCase()
})

const displayName = computed(() => userStore.user?.displayName || userStore.user?.username || '访客')

const roleLabel = computed(() => {
  const role = userStore.user?.role
  if (role === 'admin') return '管理员'
  if (role === 'operator') return '运维员'
  if (role === 'viewer') return '只读用户'
  return role || '用户'
})

const roleTagType = computed<'primary' | 'success' | 'warning' | 'info'>(() => {
  const role = userStore.user?.role
  if (role === 'admin') return 'primary'
  if (role === 'operator') return 'success'
  if (role === 'viewer') return 'info'
  return 'info'
})

const greeting = computed(() => {
  const h = new Date().getHours()
  if (h < 6) return '凌晨好'
  if (h < 9) return '早上好'
  if (h < 12) return '上午好'
  if (h < 14) return '中午好'
  if (h < 18) return '下午好'
  return '晚上好'
})

// 快捷入口（按权限过滤）
type Shortcut = { path: string; label: string; icon: any; color: string; permission?: string }
const allShortcuts: Shortcut[] = [
  { path: '/system/users', label: '用户管理', icon: User, color: '#409EFF', permission: 'user:read' },
  { path: '/system/roles', label: '角色管理', icon: UserFilled, color: '#67C23A', permission: 'role:read' },
  { path: '/system/departments', label: '部门管理', icon: OfficeBuilding, color: '#E6A23C', permission: 'dept:read' },
  { path: '/audit', label: '审计中心', icon: Notebook, color: '#F56C6C', permission: 'audit:read' },
  { path: '/system', label: '系统设置', icon: Tools, color: '#909399', permission: 'system:read' },
  { path: '/profile', label: '个人中心', icon: User, color: '#9C27B0' },
  { path: '/tickets', label: '工单系统', icon: Tickets, color: '#00BCD4' },
  { path: '/logs', label: '日志中心', icon: Document, color: '#FF9800' },
]
const visibleShortcuts = computed(() =>
  allShortcuts.filter((s) => !s.permission || userStore.hasPermission(s.permission)),
)

async function load() {
  loading.value = true
  try {
    const data: DashboardData = await getDashboard()
    stats.value = data.stats
    opsStats.value = data.opsStats
    modelStats.value = data.modelStats
    recentJobRuns.value = data.recentJobRuns
    recentActivities.value = data.recentActivities
    myActivities.value = data.myActivities
    await nextTick()
    initModelChart()
  } catch (e: any) {
    if (e?.message !== '无权限访问') {
      ElMessage.error('加载工作台数据失败')
    }
  } finally {
    loading.value = false
  }
}

function initModelChart() {
  if (!modelChartRef.value || modelStats.value.length === 0) return
  if (!modelChart) {
    modelChart = echarts.init(modelChartRef.value)
  }
  const total = modelStats.value.reduce((sum, m) => sum + m.count, 0)
  modelChart.setOption({
    tooltip: {
      trigger: 'item',
      formatter: '{b}: {c} ({d}%)',
    },
    legend: {
      orient: 'horizontal',
      bottom: 0,
      textStyle: { fontSize: 11 },
      itemWidth: 10,
      itemHeight: 10,
    },
    series: [
      {
        name: '资产分布',
        type: 'pie',
        radius: ['38%', '62%'],
        center: ['50%', '42%'],
        avoidLabelOverlap: true,
        label: {
          show: true,
          position: 'center',
          formatter: `{a|${total}}\n{b|资产总数}`,
          rich: {
            a: { fontSize: 24, fontWeight: 'bold', color: '#303133' },
            b: { fontSize: 12, color: '#909399', padding: [4, 0, 0, 0] },
          },
        },
        labelLine: { show: false },
        data: modelStats.value.map((m) => ({
          name: m.name,
          value: m.count,
        })),
      },
    ],
  })
}

function handleResize() {
  modelChart?.resize()
}

function runStatusLabel(s: string): string {
  const map: Record<string, string> = {
    running: '执行中', success: '成功', failed: '失败', partial: '部分成功', timeout: '超时', pending: '等待',
  }
  return map[s] || s
}

function runStatusType(s: string): '' | 'success' | 'warning' | 'danger' | 'info' {
  const map: Record<string, '' | 'success' | 'warning' | 'danger' | 'info'> = {
    running: 'warning', success: 'success', failed: 'danger', partial: 'warning', timeout: 'danger', pending: 'info',
  }
  return map[s] || 'info'
}

function go(path: string) {
  router.push(path)
}

function actionLabel(action: string): string {
  const map: Record<string, string> = {
    login: '登录',
    logout: '登出',
    create: '创建',
    update: '更新',
    enable: '启用',
    disable: '禁用',
    reset_password: '重置密码',
    delete: '删除',
  }
  return map[action] || action
}

function actionColor(action: string): '' | 'success' | 'warning' | 'danger' | 'info' {
  const map: Record<string, '' | 'success' | 'warning' | 'danger' | 'info'> = {
    login: '',
    logout: 'info',
    create: 'success',
    update: 'warning',
    enable: 'success',
    disable: 'danger',
    reset_password: 'warning',
    delete: 'danger',
  }
  return map[action] || 'info'
}

onMounted(() => {
  tick()
  timer = window.setInterval(tick, 1000)
  load()
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer)
  window.removeEventListener('resize', handleResize)
  modelChart?.dispose()
  modelChart = null
})
</script>

<style scoped>
.dashboard-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* 欢迎卡 */
.welcome-card :deep(.el-card__body) {
  padding: 20px 24px;
}
.welcome {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}
.welcome__avatar {
  background: linear-gradient(135deg, #409EFF, #67C23A);
  color: #fff;
  font-weight: bold;
  font-size: 22px;
  flex-shrink: 0;
}
.welcome__text {
  flex: 1;
  min-width: 200px;
}
.welcome__hello {
  font-size: 18px;
  font-weight: 600;
  color: #303133;
  margin-bottom: 6px;
}
.welcome__sub {
  display: flex;
  align-items: center;
  gap: 12px;
  color: #909399;
  font-size: 13px;
}
.welcome__stats {
  display: flex;
  gap: 24px;
}
.welcome__stat {
  text-align: center;
}
.welcome__stat-value {
  font-size: 24px;
  font-weight: 700;
  color: #409EFF;
}
.welcome__stat-label {
  font-size: 12px;
  color: #909399;
  margin-top: 2px;
}

/* 统计卡片 */
.stat-row {
  margin-bottom: 0;
}
.stat-card {
  cursor: pointer;
  transition: all 0.2s ease;
}
.stat-card:hover {
  transform: translateY(-2px);
}
.stat-card :deep(.el-card__body) {
  padding: 18px 20px;
}
.stat-card__body {
  display: flex;
  align-items: center;
  gap: 14px;
}
.stat-card__icon {
  width: 48px;
  height: 48px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  flex-shrink: 0;
}
.stat-card__value {
  font-size: 26px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}
.stat-card__label {
  font-size: 13px;
  color: #909399;
  margin-top: 2px;
}
.stat-card__extra {
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed #ebeef5;
  font-size: 12px;
  color: #909399;
}
.stat-card--users .stat-card__icon { background: linear-gradient(135deg, #409EFF, #66b1ff); }
.stat-card--roles .stat-card__icon { background: linear-gradient(135deg, #67C23A, #85ce61); }
.stat-card--depts .stat-card__icon { background: linear-gradient(135deg, #E6A23C, #ebb563); }
.stat-card--ops .stat-card__icon { background: linear-gradient(135deg, #F56C6C, #f78989); }
.stat-card--assets .stat-card__icon { background: linear-gradient(135deg, #5B8FF9, #5AD8A6); }
.stat-card--jobs .stat-card__icon { background: linear-gradient(135deg, #5D7092, #8B5CF6); }
.stat-card--runs .stat-card__icon { background: linear-gradient(135deg, #F6BD16, #E86452); }
.stat-card--sync .stat-card__icon { background: linear-gradient(135deg, #6DC8EC, #945FB9); }

/* 图表行 */
.chart-row {
  margin-bottom: 16px;
}
.chart-card :deep(.el-card__body) {
  padding: 12px 16px;
}
.chart-container {
  height: 280px;
}
.job-runs-card :deep(.el-card__body) {
  padding: 8px 12px;
}

/* 主行 */
.main-row {
  margin-bottom: 0;
}

/* 通用卡片头 */
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.card-header__title {
  font-weight: 600;
  font-size: 15px;
  color: #303133;
}

/* 快捷入口 */
.shortcut-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
}
.shortcut-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 14px 6px;
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.2s ease;
  text-align: center;
}
.shortcut-item:hover {
  background: #f5f7fa;
  transform: translateY(-1px);
}
.shortcut-item__icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  margin-bottom: 8px;
}
.shortcut-item__label {
  font-size: 12px;
  color: #606266;
}

/* 我的最近活动 */
.mine-card {
  margin-top: 16px;
}
.mine-timeline {
  padding: 8px 0 0 4px;
}
.mine-item {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.mine-item__target {
  font-size: 12px;
  color: #606266;
  font-family: monospace;
}

/* 最近平台活动表格 */
.target-cell {
  display: flex;
  align-items: center;
  gap: 6px;
}
.target-id {
  font-family: monospace;
  font-size: 12px;
  color: #666;
}

/* 响应式 */
@media (max-width: 768px) {
  .shortcut-grid {
    grid-template-columns: repeat(3, 1fr);
  }
  .welcome__stats {
    width: 100%;
    justify-content: space-around;
    margin-top: 8px;
  }
}
@media (max-width: 480px) {
  .shortcut-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
</style>
