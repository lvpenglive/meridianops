import request from './request'

export interface NotificationItem {
  id: string
  type: string
  title: string
  content?: string
  link?: string
  isRead: boolean
  createdAt: string
  readAt?: string | null
}

export interface NotificationPage {
  total: number
  page: number
  pageSize: number
  list: NotificationItem[]
}

export function getNotifications(params?: {
  unreadOnly?: boolean
  page?: number
  pageSize?: number
}): Promise<NotificationPage> {
  return request.get('/notifications', { params })
}

export function getUnreadCount(): Promise<{ count: number }> {
  return request.get('/notifications/unread-count')
}

export function markRead(id: string): Promise<void> {
  return request.post(`/notifications/${id}/read`)
}

export function markAllRead(): Promise<{ updated: number }> {
  return request.post('/notifications/read-all')
}

export function deleteNotification(id: string): Promise<void> {
  return request.delete(`/notifications/${id}`)
}
