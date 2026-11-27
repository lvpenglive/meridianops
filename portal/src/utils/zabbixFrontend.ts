/** Zabbix 前端深链（浏览器打开，不经 API） */

export type ZabbixHostDeepLinkKind = 'dashboard' | 'problems' | 'edit'

/**
 * 拼主机深链。`frontendUrl` 为前端根（可含子路径，如 https://host/zabbix），不要带 api_jsonrpc.php。
 */
export function buildZabbixHostUrl(
  frontendUrl: string,
  hostId: string,
  kind: ZabbixHostDeepLinkKind = 'dashboard',
): string | null {
  const base = frontendUrl.trim().replace(/\/+$/, '')
  const id = hostId.trim()
  if (!base || !id) return null
  const q = encodeURIComponent(id)
  if (kind === 'problems') {
    return `${base}/zabbix.php?action=problem.view&hostids[0]=${q}`
  }
  if (kind === 'edit') {
    return `${base}/zabbix.php?action=host.edit&hostid=${q}`
  }
  return `${base}/zabbix.php?action=host.dashboard.view&hostid=${q}`
}

export function openZabbixHost(
  frontendUrl: string | null | undefined,
  hostId: string,
  kind: ZabbixHostDeepLinkKind = 'dashboard',
): boolean {
  const url = buildZabbixHostUrl(frontendUrl || '', hostId, kind)
  if (!url) return false
  window.open(url, '_blank', 'noopener,noreferrer')
  return true
}
