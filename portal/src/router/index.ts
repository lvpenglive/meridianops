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
    redirect: '/overview',
    children: [
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
        meta: { title: '资产管理', icon: 'Server' }
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
        path: 'audit',
        name: 'Audit',
        component: () => import('../views/audit/AuditPage.vue'),
        meta: { title: '审计中心', icon: 'Notebook', permission: 'audit:read' }
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
      next('/overview')
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
  // 路由级权限校验：meta.permission 存在时检查是否拥有该权限码
  const perm = to.meta.permission as string | undefined
  if (perm && userStore.isAuthenticated && !userStore.hasPermission(perm)) {
    next('/overview')
    return
  }
  next()
})

export default router
