import request from './request'

/** 任何已登录用户可查的授权状态摘要（用于页脚、到期预警横幅） */
export interface LicenseStatus {
  edition: string
  customer: string
  expiresAt: string
  activatedAt: string
  daysRemaining: number
  isExpired: boolean
  warnLevel: 'none' | 'soon' | 'urgent' | 'expired'
}

/** 管理员视角的完整授权信息（额外包含激活码脱敏值和机器指纹） */
export interface LicenseAdminInfo extends LicenseStatus {
  licenseKey: string
  /** 当前机器指纹（SHA256(server_uuid+hostname)[:12]） */
  fingerprint: string
}

/** 查询授权状态摘要（任意已登录用户） */
export function getLicenseStatus(): Promise<LicenseStatus> {
  return request.get('/license/status')
}

/** 查询管理员视角完整授权信息（需要 system:read） */
export function getLicenseAdmin(): Promise<LicenseAdminInfo> {
  return request.get('/license/admin')
}

/** 管理员更新授权（需要 system:update）。字段均为可选，传什么改什么。
 * expiresAt 传空字符串 => 永不到期。支持格式 RFC3339 / YYYY-MM-DD HH:MM:SS / YYYY-MM-DD
 */
export function updateLicenseAdmin(payload: {
  edition?: string
  customer?: string
  expiresAt?: string
  licenseKey?: string
}): Promise<LicenseAdminInfo & { message?: string }> {
  return request.put('/license/admin', payload)
}
