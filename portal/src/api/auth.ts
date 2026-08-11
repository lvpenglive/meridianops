import request from './request'
import type { LoginRequest, LoginResponse, UserInfo, ChangePasswordRequest } from './types'

// 响应拦截器已拆包（返回 body.data），这里直接声明返回类型供调用方获得类型提示。
export function login(data: LoginRequest): Promise<LoginResponse> {
  return request.post('/auth/login', data)
}

export function getMe(): Promise<UserInfo> {
  return request.get('/auth/me')
}

export function logout(): Promise<void> {
  return request.post('/auth/logout')
}

/** 用户自助修改密码（校验旧密码 + 新密码强度策略）。 */
export function changePassword(data: ChangePasswordRequest): Promise<void> {
  return request.post('/auth/change-password', data)
}

export function listUsers(): Promise<UserInfo[]> {
  return request.get('/users')
}
