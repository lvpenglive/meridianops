import * as XLSX from 'xlsx'

/**
 * 导出数据到 Excel 文件。
 * @param data 数据数组（每项为一个对象，键作为表头）
 * @param filename 文件名（不含扩展名）
 */
export function exportToExcel<T extends Record<string, unknown>>(
  data: T[],
  filename: string,
): void {
  const ws = XLSX.utils.json_to_sheet(data)
  const wb = XLSX.utils.book_new()
  XLSX.utils.book_append_sheet(wb, ws, 'Sheet1')
  XLSX.writeFile(wb, `${filename}.xlsx`)
}
