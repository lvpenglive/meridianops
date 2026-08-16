import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import * as authApi from '../api/auth'
import type {
  UserInfo,
  LoginRequest,
  ChangePasswordRequest,
  LicenseStatus,
} from '../api/types'

const TOKEN_KEY = 'meridianops_token'
const USER_KEY = 'meridianops_user'
const PWD_EXP_KEY = 'meridianops_pwd_expired'
const SESSION_TIMEOUT_KEY = 'meridianops_session_timeout'
const LAST_ACTIVITY_KEY = 'meridianops_last_activity'
const LICENSE_KEY = 'meridianops_license'
const LEGACY_USERNAME_KEY = 'meridianops_username' // 旧版遗留，启动时清理

function loadUser(): UserInfo | null {
  try {
    return JSON.parse(localStorage.getItem(USER_KEY) || 'null')
  } catch {
    return null
  }
}
function loadLicense(): LicenseStatus | null {
  try {
    return JSON.parse(localStorage.getItem(LICENSE_KEY) || 'null')
  } catch {
    return null
  }
}

/** 后端返回永不到期时的 daysRemaining 哨兵值（i64::MAX）。前端统一以该阈值视为「永久」。 */
export const PERPETUAL_DAYS = 9000000000000000000

/** 从 JWT 中提取权限码列表，失败返回空数组。 */
function parsePermissions(jwt: string): string[] {
  return parseClaims(jwt)?.permissions ?? []
}

/** 从 JWT 中提取 pwd_exp 标志，失败返回 false。 */
function parsePwdExpired(jwt: string): boolean {
  return parseClaims(jwt)?.pwd_exp === true
}

export const useUserStore = defineStore('user', () => {
  // 清理旧版遗留 key（曾用 meridianops_username 存纯用户名）
  localStorage.removeItem(LEGACY_USERNAME_KEY)

  const token = ref(localStorage.getItem(TOKEN_KEY) || '')
  const user = ref<UserInfo | null>(loadUser())
  // 产品授权摘要（登录时由后端返回，也可调用 refreshLicense 主动刷新）
  const license = ref<LicenseStatus | null>(loadLicense())
  // 权限码列表（从 JWT claims 解析）。含 '*' 表示通配（开发模式匿名用户）。
  const permissions = ref<string[]>(parsePermissions(token.value))
  // 密码过期标志（从 JWT claims 解析）。true 时仅能访问改密端点。
  const passwordExpired = ref<boolean>(parsePwdExpired(token.value))
  // 会话超时分钟数（0=不超时），从 login 响应带入。
  const sessionTimeoutMinutes = ref<number>(
    Number(localStorage.getItem(SESSION_TIMEOUT_KEY) || '0'),
  )

  const isAuthenticated = computed(() => {
    if (!token.value) return false
    const exp = parseExp(token.value)
    if (!exp) return false
    return Date.now() < exp
  })

  const role = computed(() => user.value?.role || null)

  /** 是否需要显示「产品已过期」全局模态框。 */
  const licenseExpired = computed(() => license.value?.isExpired === true)

  /** 检查是否拥有某权限码。permissions 含 '*' 时通配放行。 */
  function hasPermission(code: string): boolean {
    return permissions.value.some((p) => p === code || p === '*')
  }

  // ---- 会话 idle 计时 ----
  let idleTimer: number | undefined

  function updateLastActivity() {
    localStorage.setItem(LAST_ACTIVITY_KEY, String(Date.now()))
  }

  function startIdleTimer() {
    stopIdleTimer()
    if (sessionTimeoutMinutes.value <= 0) return
    // 每 30s 检查一次是否超时
    idleTimer = window.setInterval(() => {
      const last = Number(localStorage.getItem(LAST_ACTIVITY_KEY) || '0')
      if (last === 0) return
      const elapsed = Date.now() - last
      if (elapsed >= sessionTimeoutMinutes.value * 60 * 1000) {
        stopIdleTimer()
        // 超时：静默登出并跳登录
        clearCredentials()
        ElMessage.warning('会话超时，请重新登录')
        window.location.href = '/login'
      }
    }, 30000)
    // 监听用户交互刷新 lastActivity
    window.addEventListener('mousedown', updateLastActivity, { passive: true })
    window.addEventListener('keydown', updateLastActivity, { passive: true })
  }

  function stopIdleTimer() {
    if (idleTimer !== undefined) {
      window.clearInterval(idleTimer)
      idleTimer = undefined
    }
    window.removeEventListener('mousedown', updateLastActivity)
    window.removeEventListener('keydown', updateLastActivity)
  }

  function clearCredentials() {
    token.value = ''
    user.value = null
    license.value = null
    permissions.value = []
    passwordExpired.value = false
    stopIdleTimer()
    localStorage.removeItem(TOKEN_KEY)
    localStorage.removeItem(USER_KEY)
    localStorage.removeItem(PWD_EXP_KEY)
    localStorage.removeItem(SESSION_TIMEOUT_KEY)
    localStorage.removeItem(LAST_ACTIVITY_KEY)
    localStorage.removeItem(LICENSE_KEY)
  }

  function setLicense(next: LicenseStatus | null) {
    license.value = next
    if (next) {
      localStorage.setItem(LICENSE_KEY, JSON.stringify(next))
    } else {
      localStorage.removeItem(LICENSE_KEY)
    }
  }

  async function login(req: LoginRequest) {
    const resp = await authApi.login(req)
    token.value = resp.token
    user.value = resp.user
    permissions.value = parsePermissions(resp.token)
    passwordExpired.value = !!resp.passwordExpired
    sessionTimeoutMinutes.value = resp.sessionTimeoutMinutes ?? 0
    setLicense(resp.license ?? null)
    localStorage.setItem(TOKEN_KEY, resp.token)
    localStorage.setItem(USER_KEY, JSON.stringify(resp.user))
    localStorage.setItem(SESSION_TIMEOUT_KEY, String(sessionTimeoutMinutes.value))
    if (passwordExpired.value) {
      localStorage.setItem(PWD_EXP_KEY, '1')
    } else {
      localStorage.removeItem(PWD_EXP_KEY)
    }
    updateLastActivity()
    startIdleTimer()
  }

  /** 主动刷新授权信息（管理员更新授权后调用，或路由进入时调用）。 */
  async function refreshLicense() {
    if (!token.value) return
    try {
      const { getLicenseStatus } = await import('../api/license')
      const data = await getLicenseStatus()
      setLicense(data)
    } catch {
      /* 忽略，保持旧值 */
    }
  }

  async function fetchMe() {
    const u = await authApi.getMe()
    user.value = u
    // 重新解析权限（token 可能未变，但确保与最新 JWT 一致）
    permissions.value = parsePermissions(token.value)
    localStorage.setItem(USER_KEY, JSON.stringify(u))
    // 顺便刷新授权信息，避免用户改完系统时间或服务端更新后前端仍显示旧
    void refreshLicense()
  }

  async function logout() {
    try {
      await authApi.logout()
    } catch {
      // 忽略 API 失败，前端仍清凭据
    }
    clearCredentials()
  }

  /** 用户自助修改密码。成功后清密码过期标志（token 仍是 pwd_exp 旧 token，需重新登录获取新 token）。 */
  async function changePassword(req: ChangePasswordRequest) {
    await authApi.changePassword(req)
    // 改密成功后旧 token 仍带 pwd_exp，需登出后重新登录拿新 token
    if (passwordExpired.value) {
      clearCredentials()
      ElMessage.success('密码修改成功，请重新登录')
      window.location.href = '/login'
    }
  }

  return {
    token, user, role, permissions, isAuthenticated, passwordExpired, sessionTimeoutMinutes,
    license, licenseExpired,
    hasPermission, login, logout, fetchMe, changePassword, refreshLicense, setLicense,
    startIdleTimer, stopIdleTimer, updateLastActivity,
  }
})

/** 解析 JWT payload，提取 exp（毫秒）与 permissions（权限码数组）。
 * 失败返回 null。不引入 jwt-decode 依赖，手写 base64url 解码。 */
interface JwtClaims {
  exp?: number
  permissions?: string[]
  pwd_exp?: boolean
}
function parseClaims(jwt: string): JwtClaims | null {
  try {
    const payload = jwt.split('.')[1]
    if (!payload) return null
    // base64url -> base64
    const b64 = payload.replace(/-/g, '+').replace(/_/g, '/')
    const json = JSON.parse(atob(b64)) as JwtClaims
    return json
  } catch {
    return null
  }
}

/** 从 JWT 中提取 exp（毫秒），失败返回 null。 */
function parseExp(jwt: string): number | null {
  const claims = parseClaims(jwt)
  if (!claims?.exp) return null
  return Number(claims.exp) * 1000
}

