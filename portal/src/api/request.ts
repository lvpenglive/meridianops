import axios from 'axios'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useUserStore } from '../stores/user'

const TOKEN_KEY = 'meridianops_token'
const USER_KEY = 'meridianops_user'
const LAST_ACTIVITY_KEY = 'meridianops_last_activity'

/** 用来防止短时间内出现多个 402 时反复弹窗。 */
let _licenseExpiredBoxShown = false

/** 处理产品授权过期：刷新 license 状态并弹续期模态框。 */
async function handleLicenseExpired(message: string) {
  const user = useUserStore()
  // 即使 token 不在也要尝试拿到 store 的 setLicense 能力，同步刷新缓存
  try {
    await user.refreshLicense()
  } catch {
    /* ignore */
  }
  if (_licenseExpiredBoxShown) return
  _licenseExpiredBoxShown = true
  const isAdmin = user.hasPermission('system:update')
  try {
    const title = '产品授权已过期'
    const content =
      (message || '当前产品授权已到期，业务功能已暂停使用。') +
      (isAdmin ? '\n您可以前往「授权管理」完成续期。' : '\n请联系系统管理员续期。')
    if (isAdmin) {
      await ElMessageBox.alert(content, title, {
        confirmButtonText: '前往授权管理',
        type: 'warning',
        dangerouslyUseHTMLString: false,
        closeOnClickModal: false,
        closeOnPressEscape: false,
        showClose: false,
      })
      if (window.location.pathname !== '/system/license') {
        window.location.href = '/system/license'
      }
    } else {
      await ElMessageBox.alert(content, title, {
        confirmButtonText: '知道了',
        type: 'warning',
        showClose: false,
      })
    }
  } finally {
    _licenseExpiredBoxShown = false
  }
}

const request = axios.create({
  baseURL: '/api',
  timeout: 15000,
})

request.interceptors.request.use((config) => {
  const token = localStorage.getItem(TOKEN_KEY)
  if (token) {
    config.headers['Authorization'] = `Bearer ${token}`
  }
  return config
})

request.interceptors.response.use(
  (response) => {
    // 任何成功响应都刷新 lastActivity（用于 idle 计时）
    localStorage.setItem(LAST_ACTIVITY_KEY, String(Date.now()))
    // 后端统一响应：{ code: 0, data: ... } 成功；{ code: <non-zero>, message: "..." } 业务错误
    const body = response.data
    if (body && typeof body === 'object' && 'code' in body) {
      if (body.code === 0) {
        return body.data
      }
      ElMessage.error(body.message || '请求失败')
      return Promise.reject(new Error(body.message || '请求失败'))
    }
    return body
  },
  (error) => {
    const status = error.response?.status
    if (status === 401) {
      if (window.location.pathname === '/login') {
        // 登录页的 401 是用户名密码错误
        ElMessage.error(error.response?.data?.message || '用户名或密码错误')
      } else {
        // 非登录页的 401 是 token 过期/无效，清凭据并跳登录
        localStorage.removeItem(TOKEN_KEY)
        localStorage.removeItem(USER_KEY)
        ElMessage.error('登录已过期，请重新登录')
        window.location.href = '/login'
      }
      return Promise.reject(error)
    }
    if (status === 402) {
      const msg = error.response?.data?.message || '产品授权已过期'
      void handleLicenseExpired(msg)
      return Promise.reject(error)
    }
    if (status === 403) {
      const msg = error.response?.data?.message || '无权限访问'
      // 密码过期的 403 单独提示，引导用户去改密
      if (msg.includes('密码已过期')) {
        ElMessage.warning(msg)
        if (window.location.pathname !== '/profile') {
          window.location.href = '/profile?forceChange=1'
        }
      } else {
        ElMessage.error(msg)
      }
      return Promise.reject(error)
    }
    const msg = error.response?.data?.message || error.message || '请求失败'
    ElMessage.error(msg)
    return Promise.reject(error)
  },
)

export default request
