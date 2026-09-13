/**
 * 权限展示：与侧栏一级 / 二级菜单对齐。
 */

export interface PermMenuNode {
  parent: string
  child: string
}

export interface MenuPermGroup<T> {
  parent: string
  children: { title: string; items: T[] }[]
}

const PREFIX_MENU: Record<string, PermMenuNode> = {
  audit: { parent: '运营分析', child: '审计中心' },
  asset: { parent: '资产管理', child: '资产清单' },
  alert: { parent: '监控告警', child: '告警中心' },
  notification: { parent: '监控告警', child: '通知通道' },
  sms_strategy: { parent: '监控告警', child: '通知策略' },
  alert_group: { parent: '监控告警', child: '告警组维护' },
  notification_log: { parent: '监控告警', child: '通知发送日志' },
  log: { parent: '监控告警', child: '日志中心' },
  job: { parent: '运维流程', child: '作业中心' },
  credential: { parent: '运维流程', child: 'SSH 凭据' },
  ticket: { parent: '运维流程', child: '工单系统' },
  workflow: { parent: '运维流程', child: '流程模板' },
  knowledge: { parent: '运维流程', child: '知识库' },
  user: { parent: '后台管理', child: '用户管理' },
  role: { parent: '后台管理', child: '角色管理' },
  dept: { parent: '后台管理', child: '部门管理' },
  system: { parent: '后台管理', child: '系统设置' },
  dict: { parent: '后台管理', child: '字典管理' },
}

const PARENT_ORDER = ['运营分析', '资产管理', '监控告警', '运维流程', '后台管理']

const CHILD_ORDER = [
  '态势中心', '报表中心', '审计中心', '费用中心',
  '资产清单', 'CI 模型', '关系类型', '拓扑视图', '数据源同步', '容器管理', 'DB数据库', '配置中心',
  '告警中心', '告警大屏', '通知通道', '通知策略', '告警组维护', '通知发送日志', '日志中心', 'AIOps运维',
  '作业中心', 'SSH 凭据', '工单系统', '工单统计', '流程模板', '知识库',
  '用户管理', '角色管理', '部门管理', '系统设置', '组件状态', 'API 令牌', '字典管理', '授权管理',
]

const PERM_LABELS: Record<string, string> = {
  'user:read': '查看用户管理',
  'user:create': '创建用户',
  'user:update': '编辑用户',
  'user:delete': '删除用户',
  'user:reset_password': '重置密码',
  'user:toggle_enable': '启用/禁用用户',
  'role:read': '查看角色管理',
  'role:create': '创建角色',
  'role:update': '编辑角色',
  'role:delete': '删除角色',
  'role:assign_permission': '分配权限',
  'dept:read': '查看部门管理',
  'dept:create': '创建部门',
  'dept:update': '编辑部门',
  'dept:delete': '删除部门',
  'audit:read': '查看审计中心',
  'system:read': '查看系统设置',
  'system:update': '修改系统设置',
  'asset:read': '查看资产清单',
  'asset:create': '创建资产',
  'asset:update': '修改资产',
  'asset:delete': '删除资产',
  'knowledge:read': '查看知识库',
  'knowledge:create': '创建知识',
  'knowledge:update': '编辑知识',
  'knowledge:delete': '删除知识',
  'dict:read': '查看字典管理',
  'dict:create': '创建字典',
  'dict:update': '编辑字典',
  'dict:delete': '删除字典',
  'job:read': '查看作业中心',
  'job:create': '创建作业',
  'job:execute': '执行作业',
  'job:admin': '管理作业中心',
  'credential:read': '查看 SSH 凭据',
  'credential:create': '创建 SSH 凭据',
  'credential:delete': '删除 SSH 凭据',
  'alert:read': '查看告警中心',
  'alert:create': '创建告警',
  'alert:update': '处置告警',
  'alert:delete': '删除告警',
  'notification:read': '查看通知通道',
  'notification:manage': '管理通知通道',
  'notification_log:read': '查看通知发送日志',
  'sms_strategy:read': '查看通知策略',
  'sms_strategy:manage': '管理通知策略',
  'alert_group:read': '查看告警组维护',
  'alert_group:manage': '管理告警组维护',
  'log:read': '查看日志中心',
  'ticket:read': '查看工单系统',
  'ticket:create': '创建工单',
  'ticket:update': '更新工单',
  'ticket:delete': '删除工单',
  'workflow:read': '查看流程模板',
  'workflow:admin': '管理流程模板',
}

/** 二级菜单下的动作文案（不再重复菜单名） */
const PERM_ACTION_LABELS: Record<string, string> = {
  'user:read': '查看',
  'user:create': '创建',
  'user:update': '编辑',
  'user:delete': '删除',
  'user:reset_password': '重置密码',
  'user:toggle_enable': '启用/禁用',
  'role:read': '查看',
  'role:create': '创建',
  'role:update': '编辑',
  'role:delete': '删除',
  'role:assign_permission': '分配权限',
  'dept:read': '查看',
  'dept:create': '创建',
  'dept:update': '编辑',
  'dept:delete': '删除',
  'audit:read': '查看',
  'system:read': '查看',
  'system:update': '修改',
  'asset:read': '查看',
  'asset:create': '创建',
  'asset:update': '修改',
  'asset:delete': '删除',
  'knowledge:read': '查看',
  'knowledge:create': '创建',
  'knowledge:update': '编辑',
  'knowledge:delete': '删除',
  'dict:read': '查看',
  'dict:create': '创建',
  'dict:update': '编辑',
  'dict:delete': '删除',
  'job:read': '查看',
  'job:create': '创建',
  'job:execute': '执行',
  'job:admin': '管理',
  'credential:read': '查看',
  'credential:create': '创建',
  'credential:delete': '删除',
  'alert:read': '查看',
  'alert:create': '创建',
  'alert:update': '处置',
  'alert:delete': '删除',
  'notification:read': '查看',
  'notification:manage': '管理',
  'notification_log:read': '查看',
  'sms_strategy:read': '查看',
  'sms_strategy:manage': '管理',
  'alert_group:read': '查看',
  'alert_group:manage': '管理',
  'log:read': '查看',
  'ticket:read': '查看',
  'ticket:create': '创建',
  'ticket:update': '更新',
  'ticket:delete': '删除',
  'workflow:read': '查看',
  'workflow:admin': '管理',
}

export function permissionPrefix(code: string): string {
  const i = code.indexOf(':')
  return i === -1 ? code : code.slice(0, i)
}

export function resolvePermissionMenu(code: string, fallbackModule?: string): PermMenuNode {
  const prefix = permissionPrefix(code)
  return PREFIX_MENU[prefix]
    ?? (fallbackModule ? PREFIX_MENU[fallbackModule] : undefined)
    ?? { parent: '其他', child: fallbackModule || prefix }
}

export function permissionGroupLabel(moduleOrPrefix: string): string {
  return PREFIX_MENU[moduleOrPrefix]?.child ?? moduleOrPrefix
}

export function permissionLabel(code: string, fallback?: string): string {
  return PERM_LABELS[code] ?? fallback ?? code
}

export function permissionActionLabel(code: string, fallback?: string): string {
  return PERM_ACTION_LABELS[code] ?? fallback ?? permissionLabel(code)
}

function indexOfOrLast(list: string[], name: string): number {
  const i = list.indexOf(name)
  return i === -1 ? list.length : i
}

export function groupByMenu<T>(
  items: T[],
  getCode: (item: T) => string,
  getModule?: (item: T) => string,
): MenuPermGroup<T>[] {
  const parents = new Map<string, Map<string, T[]>>()
  for (const item of items) {
    const code = getCode(item)
    const { parent, child } = resolvePermissionMenu(code, getModule?.(item))
    if (!parents.has(parent)) parents.set(parent, new Map())
    const children = parents.get(parent)!
    if (!children.has(child)) children.set(child, [])
    children.get(child)!.push(item)
  }
  return [...parents.entries()]
    .sort((a, b) => indexOfOrLast(PARENT_ORDER, a[0]) - indexOfOrLast(PARENT_ORDER, b[0]))
    .map(([parent, children]) => ({
      parent,
      children: [...children.entries()]
        .sort((a, b) => indexOfOrLast(CHILD_ORDER, a[0]) - indexOfOrLast(CHILD_ORDER, b[0]))
        .map(([title, childItems]) => ({ title, items: childItems })),
    }))
}
