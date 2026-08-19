/**
 * 告警级别：与 Zabbix 对齐的 0-5 六级数字体系。
 * 0=未分类(最低)  1=信息  2=警告  3=一般  4=重要  5=灾难(最高)
 *
 * 兼容读取 DB 历史值（P0-P5 / disaster-information 等别名），
 * 对外统一展示 canonical 中文名称，内部存储统一写入 "0"-"5"。
 */

export type AlertLevel = '0' | '1' | '2' | '3' | '4' | '5'

/** 级别元数据：展示中文名 + Element Plus Tag 类型 + 自定义 class（用于大屏荧光色） */
export interface AlertLevelMeta {
  level: AlertLevel
  name: string
  shortName: string
  /** Element Plus tag type：danger/warning/primary/info/success */
  tagType: 'danger' | 'warning' | 'primary' | 'info' | 'success'
  /** HEX 颜色（用于环形图/卡片/大屏荧光） */
  color: string
  /** 背景发光色（大屏 card 用） */
  glow: string
}

export const ALERT_LEVEL_META: Record<AlertLevel, AlertLevelMeta> = {
  '0': {
    level: '0',
    name: '未分类 Not Classified',
    shortName: '未分类',
    tagType: 'info',
    color: '#64748b',
    glow: 'rgba(100, 116, 139, 0.18)',
  },
  '1': {
    level: '1',
    name: '信息 Information',
    shortName: '信息',
    tagType: 'info',
    color: '#0ea5e9',
    glow: 'rgba(14, 165, 233, 0.22)',
  },
  '2': {
    level: '2',
    name: '警告 Warning',
    shortName: '警告',
    tagType: 'primary',
    color: '#ca8a04',
    glow: 'rgba(202, 138, 4, 0.22)',
  },
  '3': {
    level: '3',
    name: '一般 Average',
    shortName: '一般',
    tagType: 'warning',
    color: '#ea580c',
    glow: 'rgba(234, 88, 12, 0.25)',
  },
  '4': {
    level: '4',
    name: '重要 High',
    shortName: '重要',
    tagType: 'danger',
    color: '#dc2626',
    glow: 'rgba(220, 38, 38, 0.25)',
  },
  '5': {
    level: '5',
    name: '灾难 Disaster',
    shortName: '灾难',
    tagType: 'danger',
    color: '#7f1d1d',
    glow: 'rgba(220, 38, 38, 0.35)',
  },
}

/** severity 下拉/筛选项（前端表单用），按严重度从高到低排列 */
export const ALERT_LEVEL_OPTIONS: Array<{ value: string; label: string }> = [
  { value: '5', label: '5 灾难（Disaster）' },
  { value: '4', label: '4 重要（High）' },
  { value: '3', label: '3 一般（Average）' },
  { value: '2', label: '2 警告（Warning）' },
  { value: '1', label: '1 信息（Information）' },
  { value: '0', label: '0 未分类（Not Classified）' },
]

/** 只保留中文短名的筛选下拉（列表筛选用，更紧凑） */
export const ALERT_LEVEL_FILTER_OPTIONS: Array<{ value: string; label: string }> = [
  { value: '5', label: '5 灾难' },
  { value: '4', label: '4 重要' },
  { value: '3', label: '3 一般' },
  { value: '2', label: '2 警告' },
  { value: '1', label: '1 信息' },
  { value: '0', label: '0 未分类' },
]

/**
 * 把 DB 中任意 severity 原始值（"0"-"5" / "P0"-"P5" / disaster-information 等）
 * 归一化到 0-5 canonical 级别（Zabbix 体系：0=最低, 5=最高）。
 */
export function normalizeAlertLevel(raw: string | null | undefined): AlertLevel {
  if (!raw) return '0'
  const lower = String(raw).trim().toLowerCase()
  if (!lower) return '0'
  switch (lower) {
    // 0 = 未分类（最低）
    case '0':
    case 'p0':
    case 'notclassified':
    case 'not_classified':
    case 'not classified':
    case 'classified':
      return '0'
    // 1 = 信息
    case '1':
    case 'p1':
    case 'information':
    case 'info':
    case 'notice':
    case 'informational':
      return '1'
    // 2 = 警告
    case '2':
    case 'p2':
    case 'warning':
    case 'warn':
      return '2'
    // 3 = 一般
    case '3':
    case 'p3':
    case 'average':
    case 'avg':
    case 'medium':
      return '3'
    // 4 = 重要
    case '4':
    case 'p4':
    case 'high':
    case 'major':
    case 'critical':
    case 'crit':
      return '4'
    // 5 = 灾难（最高）
    case '5':
    case 'p5':
    case 'disaster':
    case 'dis':
      return '5'
    default:
      return '0'
  }
}

/** 获取级别中文名（兼容任意原始值） */
export function alertLevelName(raw: string | null | undefined): string {
  const lvl = normalizeAlertLevel(raw)
  return ALERT_LEVEL_META[lvl].name
}

/** 获取级别中文短名（用于标签，更紧凑） */
export function alertLevelShortName(raw: string | null | undefined): string {
  const lvl = normalizeAlertLevel(raw)
  return ALERT_LEVEL_META[lvl].shortName
}

/** 获取 tag type（Element Plus） */
export function alertLevelTagType(
  raw: string | null | undefined,
): AlertLevelMeta['tagType'] {
  const lvl = normalizeAlertLevel(raw)
  return ALERT_LEVEL_META[lvl].tagType
}

/** 获取颜色 HEX */
export function alertLevelColor(raw: string | null | undefined): string {
  const lvl = normalizeAlertLevel(raw)
  return ALERT_LEVEL_META[lvl].color
}

/** 以级别数字为 key 的顺序遍历（从高到低：5→0），适合渲染环形图、卡片列表 */
export const ALERT_LEVEL_ORDER: AlertLevel[] = ['5', '4', '3', '2', '1', '0']
