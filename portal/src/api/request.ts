import axios from 'axios'
import { ElMessage } from 'element-plus'

const TOKEN_KEY = 'meridianops_token'
const USER_KEY = 'meridianops_user'

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
    if (status === 403) {
      ElMessage.error(error.response?.data?.message || '无权限访问')
      return Promise.reject(error)
    }
    const msg = error.response?.data?.message || error.message || '请求失败'
    ElMessage.error(msg)
    return Promise.reject(error)
  },
)

export default request
