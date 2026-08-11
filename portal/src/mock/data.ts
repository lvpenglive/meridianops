import type { OverviewData, AlertInfo, AgentInfo, ServiceInfo, SystemInfo } from '../api/types'

export const mockSystems: SystemInfo[] = [
  { id: 'axleops', name: 'AxleOps 服务管理', type: 'service-mgmt', baseUrl: 'http://axleops:9000', status: 'online', version: 'v0.4.1' },
  { id: 'eventide', name: 'Eventide 告警中心', type: 'alert-center', baseUrl: 'http://eventide:8080', status: 'online', version: 'v0.3.0' },
  { id: 'zabbix', name: 'Zabbix 监控', type: 'monitoring', baseUrl: 'http://zabbix:10051', status: 'online', version: 'v7.0' },
  { id: 'elk', name: 'ELK 日志', type: 'logging', baseUrl: 'http://elk:5601', status: 'online', version: 'v8.12' },
  { id: 'prometheus', name: 'Prometheus 指标', type: 'metrics', baseUrl: 'http://prometheus:9090', status: 'online', version: 'v2.50' },
  { id: 'skywalking', name: 'SkyWalking 链路', type: 'tracing', baseUrl: 'http://skywalking:8080', status: 'offline', version: 'v10.1' }
]

export const mockAgents: AgentInfo[] = [
  { id: 'agent-01', hostname: 'prod-node-01', ip: '10.0.1.11', status: 'online', services: 8, cpu: 45, memory: 62, uptime: '32天' },
  { id: 'agent-02', hostname: 'prod-node-02', ip: '10.0.1.12', status: 'online', services: 6, cpu: 23, memory: 48, uptime: '15天' },
  { id: 'agent-03', hostname: 'prod-node-03', ip: '10.0.1.13', status: 'online', services: 5, cpu: 78, memory: 85, uptime: '45天' },
  { id: 'agent-04', hostname: 'prod-node-04', ip: '10.0.1.14', status: 'online', services: 4, cpu: 12, memory: 35, uptime: '7天' },
  { id: 'agent-05', hostname: 'staging-node-01', ip: '10.0.2.11', status: 'online', services: 3, cpu: 34, memory: 56, uptime: '20天' },
  { id: 'agent-06', hostname: 'staging-node-02', ip: '10.0.2.12', status: 'offline', services: 2, cpu: 0, memory: 0, uptime: '—' },
  { id: 'agent-07', hostname: 'db-node-01', ip: '10.0.3.11', status: 'online', services: 3, cpu: 67, memory: 72, uptime: '60天' },
  { id: 'agent-08', hostname: 'db-node-02', ip: '10.0.3.12', status: 'online', services: 2, cpu: 41, memory: 55, uptime: '60天' },
  { id: 'agent-09', hostname: 'cache-node-01', ip: '10.0.4.11', status: 'online', services: 2, cpu: 28, memory: 90, uptime: '30天' },
  { id: 'agent-10', hostname: 'mq-node-01', ip: '10.0.5.11', status: 'online', services: 2, cpu: 55, memory: 68, uptime: '25天' }
]

export const mockServices: ServiceInfo[] = [
  { id: 'svc-001', name: 'order-service', type: 'jar', agentId: 'agent-01', status: 'running', cpu: 25, memory: 512, version: 'v1.2.3', startedAt: '2026-08-09 10:30' },
  { id: 'svc-002', name: 'payment-service', type: 'jar', agentId: 'agent-01', status: 'running', cpu: 18, memory: 256, version: 'v1.1.0', startedAt: '2026-08-09 10:31' },
  { id: 'svc-003', name: 'user-service', type: 'jar', agentId: 'agent-02', status: 'running', cpu: 12, memory: 192, version: 'v2.0.1', startedAt: '2026-08-08 15:20' },
  { id: 'svc-004', name: 'notification-service', type: 'python', agentId: 'agent-02', status: 'running', cpu: 8, memory: 64, version: 'v0.9.5', startedAt: '2026-08-08 15:22' },
  { id: 'svc-005', name: 'report-service', type: 'jar', agentId: 'agent-03', status: 'error', cpu: 0, memory: 0, version: 'v1.0.2', startedAt: '—' },
  { id: 'svc-006', name: 'batch-job', type: 'script', agentId: 'agent-03', status: 'stopped', cpu: 0, memory: 0, version: 'v0.3.0', startedAt: '—' },
  { id: 'svc-007', name: 'api-gateway', type: 'jar', agentId: 'agent-04', status: 'running', cpu: 5, memory: 128, version: 'v3.0.0', startedAt: '2026-08-10 08:00' },
  { id: 'svc-008', name: 'db-mysql-master', type: 'command', agentId: 'agent-07', status: 'running', cpu: 45, memory: 2048, version: 'v8.0.34', startedAt: '2026-06-01 00:00' },
  { id: 'svc-009', name: 'db-mysql-slave', type: 'command', agentId: 'agent-08', status: 'running', cpu: 38, memory: 2048, version: 'v8.0.34', startedAt: '2026-06-01 00:00' },
  { id: 'svc-010', name: 'cache-redis', type: 'command', agentId: 'agent-09', status: 'running', cpu: 15, memory: 4096, version: 'v7.2', startedAt: '2026-07-10 00:00' }
]

export const mockAlerts: AlertInfo[] = [
  { id: 'alert-001', severity: 'critical', title: 'order-service CPU 使用率超过95%', source: 'eventide', agent: 'agent-01', service: 'order-service', createdAt: '2026-08-10 14:30:00', status: 'firing' },
  { id: 'alert-002', severity: 'critical', title: 'MySQL 主从同步延迟超过120秒', source: 'zabbix', agent: 'agent-08', service: 'db-mysql-slave', createdAt: '2026-08-10 14:28:00', status: 'firing' },
  { id: 'alert-003', severity: 'warning', title: 'user-service 内存使用率超过80%', source: 'eventide', agent: 'agent-02', service: 'user-service', createdAt: '2026-08-10 14:25:00', status: 'firing' },
  { id: 'alert-004', severity: 'warning', title: 'prod-node-03 磁盘IO使用率超过90%', source: 'zabbix', agent: 'agent-03', service: '—', createdAt: '2026-08-10 14:20:00', status: 'firing' },
  { id: 'alert-005', severity: 'critical', title: 'payment-service 健康检查失败', source: 'axleops', agent: 'agent-01', service: 'payment-service', createdAt: '2026-08-10 14:15:00', status: 'firing' },
  { id: 'alert-006', severity: 'info', title: 'order-service 发布 v1.2.3 成功', source: 'axleops', agent: 'agent-01', service: 'order-service', createdAt: '2026-08-10 14:10:00', status: 'resolved' },
  { id: 'alert-007', severity: 'warning', title: 'Redis 内存使用率超过85%', source: 'prometheus', agent: 'agent-09', service: 'cache-redis', createdAt: '2026-08-10 14:05:00', status: 'acknowledged' },
  { id: 'alert-008', severity: 'resolved', title: 'API 网关响应时间恢复正常', source: 'eventide', agent: 'agent-04', service: 'api-gateway', createdAt: '2026-08-10 14:00:00', status: 'resolved' },
  { id: 'alert-009', severity: 'critical', title: 'staging-node-02 主机离线', source: 'zabbix', agent: 'agent-06', service: '—', createdAt: '2026-08-10 13:55:00', status: 'firing' },
  { id: 'alert-010', severity: 'info', title: 'payment-service 重启完成', source: 'axleops', agent: 'agent-01', service: 'payment-service', createdAt: '2026-08-10 13:50:00', status: 'resolved' }
]

export const mockOverview: OverviewData = {
  agents: { total: 10, online: 9, offline: 1 },
  services: { total: 45, running: 38, stopped: 4, error: 3 },
  alerts: { firing: 5, warning: 2, resolved: 3 },
  hosts: { total: 10, healthy: 8, warning: 1, critical: 1 },
  recentAlerts: mockAlerts.slice(0, 8)
}

export const mockAuditLogs = [
  { id: 'audit-001', user: 'admin', action: '登录', target: '控制台', ip: '10.0.0.100', createdAt: '2026-08-10 14:30:00' },
  { id: 'audit-002', user: 'zhangsan', action: '重启服务', target: 'order-service@agent-01', ip: '10.0.0.105', createdAt: '2026-08-10 14:25:00' },
  { id: 'audit-003', user: 'lisi', action: '发布制品', target: 'payment-service v1.2.0', ip: '10.0.0.110', createdAt: '2026-08-10 14:20:00' },
  { id: 'audit-004', user: 'admin', action: '修改配置', target: 'agent-03 watchdog_interval', ip: '10.0.0.100', createdAt: '2026-08-10 14:15:00' },
  { id: 'audit-005', user: 'wangwu', action: '登记Agent', target: 'agent-10', ip: '10.0.0.115', createdAt: '2026-08-10 14:10:00' },
  { id: 'audit-006', user: 'zhangsan', action: '查看日志', target: 'order-service@agent-01', ip: '10.0.0.105', createdAt: '2026-08-10 14:05:00' }
]

export const mockJobs = [
  { id: 'job-001', name: '批量重启订单服务', status: 'completed', creator: 'zhangsan', createdAt: '2026-08-10 14:25:00', duration: '30秒', targets: ['agent-01'] },
  { id: 'job-002', name: '发布支付服务 v1.2.0', status: 'completed', creator: 'lisi', createdAt: '2026-08-10 14:20:00', duration: '2分钟', targets: ['agent-01'] },
  { id: 'job-003', name: 'MySQL 主从同步检查', status: 'running', creator: 'system', createdAt: '2026-08-10 14:00:00', duration: '进行中', targets: ['agent-08'] },
  { id: 'job-004', name: '清理 Redis 过期键', status: 'pending', creator: 'wangwu', createdAt: '2026-08-10 13:55:00', duration: '—', targets: ['agent-09'] }
]
