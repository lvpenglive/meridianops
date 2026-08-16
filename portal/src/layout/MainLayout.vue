<template>
  <el-container class="main-layout">
    <el-aside :width="isCollapse ? '64px' : '220px'" class="sidebar">
      <div class="logo">
        <span v-if="!isCollapse" class="logo-text">MeridianOps</span>
        <span v-else class="logo-icon">M</span>
      </div>
      <div class="menu-scroll">
        <el-menu
          :default-active="activeMenu"
          :collapse="isCollapse"
          :collapse-transition="false"
          :default-openeds="defaultOpeneds"
          background-color="#001529"
          text-color="#c9d1d9"
          active-text-color="#409EFF"
          router
        >
          <template v-for="g in menuGroups" :key="g.type === 'item' ? g.path : g.title">
            <!-- 独立一级菜单 -->
            <el-menu-item v-if="g.type === 'item'" :index="g.path">
              <el-icon><component :is="g.icon" /></el-icon>
              <template #title>{{ g.title }}</template>
            </el-menu-item>
            <!-- 二级菜单分组 -->
            <el-sub-menu v-else :index="g.title">
              <template #title>
                <el-icon><component :is="g.icon" /></el-icon>
                <span>{{ g.title }}</span>
              </template>
              <el-menu-item
                v-for="c in g.children"
                :key="c.path"
                :index="c.path"
              >
                <el-icon><component :is="c.icon" /></el-icon>
                <template #title>{{ c.title }}</template>
              </el-menu-item>
            </el-sub-menu>
          </template>
        </el-menu>
      </div>
    </el-aside>

    <el-container>
      <el-header class="header">
        <div class="header-left">
          <el-icon class="collapse-btn" @click="toggleCollapse">
            <Fold v-if="!isCollapse" />
            <Expand v-else />
          </el-icon>
          <el-breadcrumb separator="/">
            <el-breadcrumb-item :to="{ path: '/dashboard' }">首页</el-breadcrumb-item>
            <el-breadcrumb-item>{{ currentTitle }}</el-breadcrumb-item>
          </el-breadcrumb>
        </div>

        <div class="header-right">
          <el-input
            v-model="searchQuery"
            placeholder="搜索服务、主机、告警..."
            prefix-icon="Search"
            class="search-input"
          />
          <el-badge :value="alertCount" :hidden="alertCount === 0" class="alert-badge">
            <el-icon class="header-icon" :size="20"><Bell /></el-icon>
          </el-badge>
          <el-dropdown @command="handleCommand">
            <span class="user-info">
              <el-avatar :size="32">{{ username.charAt(0).toUpperCase() }}</el-avatar>
              <span class="username">{{ username }}</span>
              <el-icon><ArrowDown /></el-icon>
            </span>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="profile">个人中心</el-dropdown-item>
                <el-dropdown-item command="logout" divided>退出登录</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </el-header>

      <!-- 授权预警横幅 -->
      <div v-if="licenseBannerProps" :class="['license-banner', `license-banner--${licenseBannerProps.level}`]">
        <el-icon><WarningFilled /></el-icon>
        <span class="license-banner-text">{{ licenseBannerProps.text }}</span>
        <el-button
          v-if="userStore.hasPermission('system:read')"
          link
          type="primary"
          size="small"
          @click="router.push('/system/license')"
        >
          前往授权管理
        </el-button>
      </div>

      <el-main class="main-content">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
        <!-- 页脚授权标识 -->
        <div class="license-footer">
          <span>© {{ new Date().getFullYear() }} MeridianOps</span>
          <span class="license-footer-divider">·</span>
          <el-tag :type="footerEditionTagType" size="small" effect="plain">{{ footerEditionLabel }}</el-tag>
          <template v-if="userStore.license?.customer">
            <span class="license-footer-divider">·</span>
            <span>{{ userStore.license.customer }}</span>
          </template>
          <template v-if="footerExpiryText">
            <span class="license-footer-divider">·</span>
            <span :class="{ 'license-footer-expired': userStore.license?.isExpired }">
              {{ footerExpiryText }}
            </span>
          </template>
        </div>
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useUserStore, PERPETUAL_DAYS } from '../stores/user'
import { ElMessageBox } from 'element-plus'
import { WarningFilled } from '@element-plus/icons-vue'

const route = useRoute()
const router = useRouter()
const userStore = useUserStore()

const isCollapse = ref(false)
const searchQuery = ref('')
const alertCount = ref(5)
const username = computed(() => userStore.user?.displayName || userStore.user?.username || 'Admin')

// 菜单分组：态势中心独立一级，其余按运维场景分 4 组（资产/监控/运维/后台）
type MenuItem = { path: string; title: string; icon: string; permission?: string }
type MenuGroup =
  | ({ type: 'item' } & MenuItem)
  | { type: 'sub'; title: string; icon: string; children: MenuItem[] }

const allMenuGroups: MenuGroup[] = [
  {
    type: 'sub', title: '工作台', icon: 'HomeFilled', children: [
      { path: '/dashboard', title: '个人工作台', icon: 'HomeFilled' },
    ]
  },
  {
    type: 'sub', title: '运营分析', icon: 'DataAnalysis', children: [
      { path: '/overview', title: '态势中心', icon: 'Monitor' },
      { path: '/report', title: '报表中心', icon: 'TrendCharts', permission: 'audit:read' },
      { path: '/audit', title: '审计中心', icon: 'Notebook', permission: 'audit:read' },
      { path: '/cost', title: '费用中心', icon: 'Money' },
    ]
  },
  {
    type: 'sub', title: '资产管理', icon: 'Platform', children: [
      { path: '/assets', title: '资产清单', icon: 'Connection', permission: 'asset:read' },
      { path: '/cmdb/models', title: 'CI 模型', icon: 'Files', permission: 'asset:read' },
      { path: '/cmdb/relation-types', title: '关系类型', icon: 'Link', permission: 'asset:read' },
      { path: '/topology', title: '拓扑视图', icon: 'Share', permission: 'asset:read' },
      { path: '/sync-sources', title: '数据源同步', icon: 'Promotion', permission: 'asset:read' },
      { path: '/containers', title: '容器管理', icon: 'Box' },
      { path: '/database', title: 'DB数据库', icon: 'Coin' },
      { path: '/config', title: '配置中心', icon: 'Setting' },
    ]
  },
  {
    type: 'sub', title: '监控告警', icon: 'Bell', children: [
      { path: '/alerts', title: '告警中心', icon: 'BellFilled' },
      { path: '/logs', title: '日志中心', icon: 'Document' },
      { path: '/aiops', title: 'AIOps运维', icon: 'Cpu' },
    ]
  },
  {
    type: 'sub', title: '运维流程', icon: 'Operation', children: [
      { path: '/jobs', title: '作业中心', icon: 'List' },
      { path: '/system/credentials', title: 'SSH 凭据', icon: 'Key', permission: 'credential:read' },
      { path: '/tickets', title: '工单系统', icon: 'Tickets' },
      { path: '/knowledge', title: '知识库', icon: 'Collection', permission: 'knowledge:read' },
    ]
  },
  {
    type: 'sub', title: '后台管理', icon: 'User', children: [
      { path: '/system/users', title: '用户管理', icon: 'User', permission: 'user:read' },
      { path: '/system/roles', title: '角色管理', icon: 'UserFilled', permission: 'role:read' },
      { path: '/system/departments', title: '部门管理', icon: 'OfficeBuilding', permission: 'dept:read' },
      { path: '/system', title: '系统设置', icon: 'Tools', permission: 'system:read' },
      { path: '/system/api-tokens', title: 'API 令牌', icon: 'Key', permission: 'system:read' },
      { path: '/system/dict', title: '字典管理', icon: 'Collection', permission: 'dict:read' },
      { path: '/system/license', title: '授权管理', icon: 'Key', permission: 'system:read' },
    ]
  },
]

// 按权限过滤菜单：无 permission 字段的项始终显示；有则检查 hasPermission。
// 分组内子项全部被过滤后，整个分组隐藏。
const menuGroups = computed<MenuGroup[]>(() => {
  return allMenuGroups
    .map((g) => {
      if (g.type === 'item') return g
      const children = g.children.filter((c) => !c.permission || userStore.hasPermission(c.permission))
      return { ...g, children }
    })
    .filter((g) => g.type === 'item' || g.children.length > 0)
})

// 默认展开所有分组：运维平台菜单项不多，全展开方便快速导航，用户可手动折叠
const defaultOpeneds = computed(() =>
  menuGroups.value
    .filter((g): g is Extract<MenuGroup, { type: 'sub' }> => g.type === 'sub')
    .map((g) => g.title),
)

const activeMenu = computed(() => route.path)
const currentTitle = computed(() => route.meta.title as string || '')

// ---- 授权信息展示 ----
const footerEditionLabel = computed(() => {
  const v = userStore.license?.edition
  if (v === 'Community') return 'Community 社区版'
  if (v === 'Enterprise') return 'Enterprise 企业版'
  if (v === 'Ultimate') return 'Ultimate 旗舰版'
  return v || '未授权'
})
const footerEditionTagType = computed<'success' | 'warning' | 'info' | 'primary' | 'danger'>(() => {
  const v = userStore.license?.edition
  if (v === 'Ultimate') return 'warning'
  if (v === 'Enterprise') return 'primary'
  if (v === 'Community') return 'info'
  return 'info'
})
const footerExpiryText = computed(() => {
  const lic = userStore.license
  if (!lic) return ''
  if (!lic.expiresAt || lic.daysRemaining >= PERPETUAL_DAYS) return '永久授权'
  if (lic.isExpired) return `授权已于 ${formatDate(lic.expiresAt)} 过期`
  return `授权至 ${formatDate(lic.expiresAt)}（剩余 ${lic.daysRemaining} 天）`
})
function formatDate(s: string): string {
  const d = new Date(s)
  if (isNaN(d.getTime())) return s
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

/** 顶部预警横幅：仅 soon/urgent/expired 显示。none 不展示。 */
const licenseBannerProps = computed<{ level: 'soon' | 'urgent' | 'expired'; text: string } | null>(() => {
  const lic = userStore.license
  if (!lic) return null
  if (lic.isExpired) {
    return {
      level: 'expired',
      text: `产品授权已于 ${formatDate(lic.expiresAt)} 过期，业务功能已暂停。请联系管理员续期。`,
    }
  }
  if (!lic.expiresAt || lic.daysRemaining >= PERPETUAL_DAYS) return null
  if (lic.daysRemaining <= 7) {
    return {
      level: 'urgent',
      text: `授权仅剩 ${lic.daysRemaining} 天到期，请立即联系管理员续期避免业务中断。`,
    }
  }
  if (lic.daysRemaining <= 30) {
    return {
      level: 'soon',
      text: `授权将于 ${lic.daysRemaining} 天后到期，建议提前续期。`,
    }
  }
  return null
})

function toggleCollapse() {
  isCollapse.value = !isCollapse.value
}

onMounted(async () => {
  // 已登录则拉取最新用户信息，覆盖本地缓存（角色/启用状态可能被管理员变更）
  if (userStore.token) {
    try {
      await userStore.fetchMe()
    } catch {
      // 401 等错误由 request 拦截器统一处理
    }
    // 启动会话 idle 计时（从 localStorage 恢复 sessionTimeoutMinutes）
    userStore.startIdleTimer()
    userStore.updateLastActivity()
  }
  // 密码过期强制跳改密页
  if (userStore.passwordExpired && route.path !== '/profile') {
    router.replace('/profile?forceChange=1')
  }
})

async function handleCommand(command: string) {
  if (command === 'profile') {
    router.push('/profile')
  } else if (command === 'logout') {
    await ElMessageBox.confirm('确定要退出登录吗？', '提示', { type: 'warning' })
    await userStore.logout()
    router.push('/login')
  }
}
</script>

<style scoped>
.main-layout {
  height: 100vh;
}

.sidebar {
  background: #001529;
  transition: width 0.3s;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.logo {
  height: 60px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #002140;
  color: #fff;
  font-size: 18px;
  font-weight: bold;
  white-space: nowrap;
}

.logo-icon {
  font-size: 24px;
}

/* 菜单滚动容器：logo 固定在顶部，菜单区域独立纵向滚动 */
.menu-scroll {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

/* WebKit 滚动条美化 */
.menu-scroll::-webkit-scrollbar {
  width: 6px;
}
.menu-scroll::-webkit-scrollbar-track {
  background: transparent;
}
.menu-scroll::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.15);
  border-radius: 3px;
}
.menu-scroll::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.3);
}

.sidebar :deep(.el-menu) {
  border-right: none;
}

.sidebar :deep(.el-menu-item) {
  height: 50px;
  line-height: 50px;
}

.header {
  background: #fff;
  border-bottom: 1px solid #e6e6e6;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  box-shadow: 0 1px 4px rgba(0, 21, 41, 0.08);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.collapse-btn {
  font-size: 20px;
  cursor: pointer;
  color: #606266;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 20px;
}

.search-input {
  width: 280px;
}

.alert-badge {
  cursor: pointer;
}

.header-icon {
  color: #606266;
  cursor: pointer;
}

.user-info {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.username {
  font-size: 14px;
  color: #606266;
}

.main-content {
  background: #f0f2f5;
  padding: 16px;
  overflow-y: auto;
}

/* 授权预警横幅 */
.license-banner {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 20px;
  font-size: 13px;
  border-bottom: 1px solid transparent;
}
.license-banner--soon {
  background: #fdf6ec;
  color: #e6a23c;
  border-bottom-color: #f5dab1;
}
.license-banner--urgent,
.license-banner--expired {
  background: #fef0f0;
  color: #f56c6c;
  border-bottom-color: #fbc4c4;
}
.license-banner-text {
  flex: 1;
}

/* 页脚授权标识 */
.license-footer {
  margin-top: 16px;
  padding: 12px 0;
  text-align: center;
  font-size: 12px;
  color: #909399;
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}
.license-footer-divider {
  color: #c0c4cc;
}
.license-footer-expired {
  color: #f56c6c;
  font-weight: 500;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
