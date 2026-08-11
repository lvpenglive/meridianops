import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import * as authApi from '../api/auth'
import type { UserInfo, LoginRequest, ChangePasswordRequest } from '../api/types'

const TOKEN_KEY = 'meridianops_token'
const USER_KEY = 'meridianops_user'
const LEGACY_USERNAME_KEY = 'meridianops_username' // 旧版遗留，启动时清理

function loadUser(): UserInfo | null {
  try {
    return JSON.parse(localStorage.getItem(USER_KEY) || 'null')
  } catch {
    return null
  }
}

/** 解析 JWT payload，提取 exp（毫秒）与 permissions（权限码数组）。
 * 失败返回 null。不引入 jwt-decode 依赖，手写 base64url 解码。 */
interface JwtClaims {
  exp?: number
  permissions?: string[]
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

/** 从 JWT 中提取权限码列表，失败返回空数组。 */
function parsePermissions(jwt: string): string[] {
  return parseClaims(jwt)?.permissions ?? []
}

export const useUserStore = defineStore('user', () => {
  // 清理旧版遗留 key（曾用 meridianops_username 存纯用户名）
  localStorage.removeItem(LEGACY_USERNAME_KEY)

  const token = ref(localStorage.getItem(TOKEN_KEY) || '')
  const user = ref<UserInfo | null>(loadUser())
  // 权限码列表（从 JWT claims 解析）。含 '*' 表示通配（开发模式匿名用户）。
  const permissions = ref<string[]>(parsePermissions(token.value))

  const isAuthenticated = computed(() => {
    if (!token.value) return false
    const exp = parseExp(token.value)
    if (!exp) return false
    return Date.now() < exp
  })

  const role = computed(() => user.value?.role || null)

  /** 检查是否拥有某权限码。permissions 含 '*' 时通配放行。 */
  function hasPermission(code: string): boolean {
    return permissions.value.some((p) => p === code || p === '*')
  }

  async function login(req: LoginRequest) {
    const resp = await authApi.login(req)
    token.value = resp.token
    user.value = resp.user
    permissions.value = parsePermissions(resp.token)
    localStorage.setItem(TOKEN_KEY, resp.token)
    localStorage.setItem(USER_KEY, JSON.stringify(resp.user))
  }

  async function fetchMe() {
    const u = await authApi.getMe()
    user.value = u
    // 重新解析权限（token 可能未变，但确保与最新 JWT 一致）
    permissions.value = parsePermissions(token.value)
    localStorage.setItem(USER_KEY, JSON.stringify(u))
  }

  async function logout() {
    try {
      await authApi.logout()
    } catch {
      // 忽略 API 失败，前端仍清凭据
    }
    token.value = ''
    user.value = null
    permissions.value = []
    localStorage.removeItem(TOKEN_KEY)
    localStorage.removeItem(USER_KEY)
  }

  /** 用户自助修改密码。成功后不强制登出（用户可继续使用旧 token）。 */
  async function changePassword(req: ChangePasswordRequest) {
    await authApi.changePassword(req)
  }

  return { token, user, role, permissions, isAuthenticated, hasPermission, login, logout, fetchMe, changePassword }
})
