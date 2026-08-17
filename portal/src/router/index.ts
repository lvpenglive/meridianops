import { createRouter, createWebHistory } from 'vue-router'
import MainLayout from '../layout/MainLayout.vue'
import { useUserStore } from '../stores/user'

const routes = [
  {
    path: '/login',
    name: 'Login',
    component: () => import('../views/login/LoginPage.vue'),
    meta: { requiresAuth: false }
  },
  {
    path: '/',
    component: MainLayout,
    redirect: '/dashboard',
    children: [
      {
        path: 'dashboard',
        name: 'Dashboard',
        component: () => import('../views/dashboard/DashboardPage.vue'),
        meta: { title: '个人工作台', icon: 'HomeFilled' }
      },
      {
        path: 'overview',
        name: 'Overview',
        component: () => import('../views/overview/OverviewPage.vue'),
        meta: { title: '态势中心', icon: 'Monitor' }
      },
      {
        path: 'assets',
        name: 'Assets',
        component: () => import('../views/assets/AssetsPage.vue'),
        meta: { title: '资产管理', icon: 'Service' }
      },
      {
        path: 'assets/:id',
        name: 'AssetDetail',
        component: () => import('../views/assets/AssetDetailPage.vue'),
        meta: { title: '资产详情', icon: 'Service', permission: 'asset:read' }
      },
      {
        path: 'cmdb/models',
        name: 'CiModels',
        component: () => import('../views/cmdb/ModelsPage.vue'),
        meta: { title: 'CI 模型', icon: 'Files', permission: 'asset:read' }
      },
      {
        path: 'cmdb/relation-types',
        name: 'CiRelationTypes',
        component: () => import('../views/cmdb/RelationTypesPage.vue'),
        meta: { title: '关系类型', icon: 'Connection', permission: 'asset:read' }
      },
      {
        path: 'topology',
        name: 'Topology',
        component: () => import('../views/topology/TopologyPage.vue'),
        meta: { title: '拓扑视图', icon: 'Share', permission: 'asset:read' }
      },
      {
        path: 'sync-sources',
        name: 'SyncSources',
        component: () => import('../views/sync/SyncSourcesPage.vue'),
        meta: { title: '数据源同步', icon: 'Connection', permission: 'asset:read' }
      },
      {
        path: 'containers',
        name: 'Containers',
        component: () => import('../views/containers/ContainersPage.vue'),
        meta: { title: '容器管理', icon: 'Box' }
      },
      {
        path: 'database',
        name: 'Database',
        component: () => import('../views/database/DatabasePage.vue'),
        meta: { title: 'DB数据库', icon: 'Coin' }
      },
      {
        path: 'alerts',
        name: 'Alerts',
        component: () => import('../views/alerts/AlertsPage.vue'),
        meta: { title: '告警中心', icon: 'BellFilled' }
      },
      {
        path: 'logs',
        name: 'Logs',
        component: () => import('../views/logs/LogsPage.vue'),
        meta: { title: '日志中心', icon: 'Document' }
      },
      {
        path: 'config',
        name: 'Config',
        component: () => import('../views/config/ConfigPage.vue'),
        meta: { title: '配置中心', icon: 'Setting' }
      },
      {
        path: 'cost',
        name: 'Cost',
        component: () => import('../views/cost/CostPage.vue'),
        meta: { title: '费用中心', icon: 'Money' }
      },
      {
        path: 'aiops',
        name: 'AIOps',
        component: () => import('../views/aiops/AIOpsPage.vue'),
        meta: { title: 'AIOps运维', icon: 'Cpu' }
      },
      {
        path: 'jobs',
        name: 'Jobs',
        component: () => import('../views/jobs/JobsPage.vue'),
        meta: { title: '作业中心', icon: 'Operation' }
      },
      {
        path: 'tickets',
        name: 'Tickets',
        component: () => import('../views/tickets/TicketsPage.vue'),
        meta: { title: '工单系统', icon: 'Tickets' }
      },
      {
        path: 'knowledge',
        name: 'Knowledge',
        component: () => import('../views/knowledge/KnowledgePage.vue'),
        meta: { title: '知识库', icon: 'Collection', permission: 'knowledge:read' }
      },
      {
        path: 'audit',
        name: 'Audit',
        component: () => import('../views/audit/AuditPage.vue'),
        meta: { title: '审计中心', icon: 'Notebook', permission: 'audit:read' }
      },
      {
        path: 'report',
        name: 'ReportIndex',
        component: () => import('../views/report/ReportIndexPage.vue'),
        meta: { title: '报表中心', icon: 'TrendCharts', permission: 'audit:read' }
      },
      {
        path: 'report/login-security',
        name: 'ReportLoginSecurity',
        component: () => import('../views/report/LoginSecurityReport.vue'),
        meta: { title: '登录安全分析', icon: 'TrendCharts', permission: 'audit:read' }
      },
      {
        path: 'report/sensitive-ops',
        name: 'ReportSensitiveOps',
        component: () => import('../views/report/SensitiveOpsReport.vue'),
        meta: { title: '敏感操作审计', icon: 'TrendCharts', permission: 'audit:read' }
      },
      {
        path: 'report/compliance-health',
        name: 'ReportComplianceHealth',
        component: () => import('../views/report/ComplianceHealthReport.vue'),
        meta: { title: '合规健康度', icon: 'TrendCharts', permission: 'audit:read' }
      },
      {
        path: 'report/asset-category',
        name: 'ReportAssetCategory',
        component: () => import('../views/report/AssetCategoryReport.vue'),
        meta: { title: '资产分类统计', icon: 'TrendCharts', permission: 'audit:read' }
      },
      {
        path: 'report/job-trend',
        name: 'ReportJobTrend',
        component: () => import('../views/report/JobRunTrendReport.vue'),
        meta: { title: '作业执行趋势', icon: 'TrendCharts', permission: 'audit:read' }
      },
      {
        path: 'report/job-summary',
        name: 'ReportJobSummary',
        component: () => import('../views/report/JobDefSummaryReport.vue'),
        meta: { title: '作业执行统计', icon: 'TrendCharts', permission: 'audit:read' }
      },
      {
        path: 'report/knowledge-stats',
        name: 'ReportKnowledgeStats',
        component: () => import('../views/report/KnowledgeCategoryReport.vue'),
        meta: { title: '知识库分类统计', icon: 'TrendCharts', permission: 'audit:read' }
      },
      {
        path: 'report/audit-trend',
        name: 'ReportAuditTrend',
        component: () => import('../views/report/AuditTrendReport.vue'),
        meta: { title: '审计操作趋势', icon: 'TrendCharts', permission: 'audit:read' }
      },
      {
        path: 'system/users',
        name: 'SystemUsers',
        component: () => import('../views/system/UsersPage.vue'),
        meta: { title: '用户管理', icon: 'User', permission: 'user:read' }
      },
      {
        path: 'system/roles',
        name: 'SystemRoles',
        component: () => import('../views/system/RolesPage.vue'),
        meta: { title: '角色管理', icon: 'UserFilled', permission: 'role:read' }
      },
      {
        path: 'system/departments',
        name: 'SystemDepartments',
        component: () => import('../views/system/DepartmentsPage.vue'),
        meta: { title: '部门管理', icon: 'OfficeBuilding', permission: 'dept:read' }
      },
      {
        path: 'profile',
        name: 'Profile',
        component: () => import('../views/system/ProfilePage.vue'),
        meta: { title: '个人中心', icon: 'User', requiresAuth: true }
      },
      {
        path: 'system',
        name: 'System',
        component: () => import('../views/system/SystemPage.vue'),
        meta: { title: '系统设置', icon: 'Tools', permission: 'system:read' }
      },
      {
        path: 'system/api-tokens',
        name: 'SystemApiTokens',
        component: () => import('../views/system/ApiTokensPage.vue'),
        meta: { title: 'API 令牌', icon: 'Key', permission: 'system:read' }
      },
      {
        path: 'system/dict',
        name: 'SystemDict',
        component: () => import('../views/system/DictPage.vue'),
        meta: { title: '字典管理', icon: 'Collection', permission: 'dict:read' }
      },
      {
        path: 'system/license',
        name: 'SystemLicense',
        component: () => import('../views/system/LicensePage.vue'),
        meta: { title: '授权管理', icon: 'Key', permission: 'system:read' }
      },
      {
        path: 'system/credentials',
        name: 'SystemCredentials',
        component: () => import('../views/system/CredentialsPage.vue'),
        meta: { title: 'SSH 凭据', icon: 'Key', permission: 'credential:read' }
      }
    ]
  }
]

const router = createRouter({
  history: createWebHistory(),
  routes
})

router.beforeEach((to, _from, next) => {
  const userStore = useUserStore()
  // 已登录访问 /login，重定向首页
  if (to.path === '/login') {
    if (userStore.isAuthenticated) {
      // 密码过期用户访问 /login 也跳改密页
      next(userStore.passwordExpired ? '/profile?forceChange=1' : '/dashboard')
    } else {
      next()
    }
    return
  }
  // requiresAuth !== false 的路由需要登录
  if (to.meta.requiresAuth !== false && !userStore.isAuthenticated) {
    next({ path: '/login', query: { redirect: to.fullPath } })
    return
  }
  // 密码过期强制：已登录但密码过期，除 /profile 外全部拦截到改密页
  if (userStore.isAuthenticated && userStore.passwordExpired && to.path !== '/profile') {
    next('/profile?forceChange=1')
    return
  }
  // 路由级权限校验：meta.permission 存在时检查是否拥有该权限码
  const perm = to.meta.permission as string | undefined
  if (perm && userStore.isAuthenticated && !userStore.hasPermission(perm)) {
    next('/dashboard')
    return
  }
  // 产品授权过期拦截：除登录/个人中心/授权管理外，其余页面跳转授权管理页
  // （仅当用户为管理员 system:read 时；普通用户允许浏览但请求会被 402 拦截）
  if (
    userStore.isAuthenticated &&
    userStore.licenseExpired &&
    !['/login', '/profile', '/system/license'].includes(to.path) &&
    userStore.hasPermission('system:read')
  ) {
    next('/system/license')
    return
  }
  next()
})

export default router
