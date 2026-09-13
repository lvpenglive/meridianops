import ExcelJS from 'exceljs'

function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
}

async function workbookToXlsxBlob(wb: ExcelJS.Workbook): Promise<Blob> {
  const buf = await wb.xlsx.writeBuffer()
  return new Blob([buf], {
    type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
  })
}

function addSheetFromObjects(
  wb: ExcelJS.Workbook,
  name: string,
  rows: Record<string, unknown>[],
  headers?: string[],
) {
  const sheet = wb.addWorksheet(name)
  const cols = headers ?? (rows[0] ? Object.keys(rows[0]) : [])
  if (!cols.length) return
  sheet.columns = cols.map((h) => ({
    header: h,
    key: h,
    width: Math.max(12, h.length + 4),
  }))
  for (const row of rows) {
    const out: Record<string, unknown> = {}
    for (const h of cols) {
      const v = row[h]
      out[h] = v === undefined || v === null ? '' : v
    }
    sheet.addRow(out)
  }
}

/** 导出数据到 Excel 文件。 */
export async function exportToExcel<T extends Record<string, unknown>>(
  data: T[],
  filename: string,
): Promise<void> {
  const wb = new ExcelJS.Workbook()
  addSheetFromObjects(wb, 'Sheet1', data)
  downloadBlob(await workbookToXlsxBlob(wb), `${filename}.xlsx`)
}

export async function exportSheetsToExcel(
  sheets: Array<{ name: string; rows: Record<string, unknown>[]; headers?: string[] }>,
  filename: string,
): Promise<void> {
  const wb = new ExcelJS.Workbook()
  for (const s of sheets) {
    addSheetFromObjects(wb, s.name, s.rows, s.headers)
  }
  const name = filename.endsWith('.xlsx') ? filename : `${filename}.xlsx`
  downloadBlob(await workbookToXlsxBlob(wb), name)
}

export function exportToCsv(rows: Record<string, unknown>[], headers: string[], filename: string) {
  const esc = (v: unknown) => {
    const s = v === undefined || v === null ? '' : String(v)
    if (/[",\n\r]/.test(s)) return `"${s.replace(/"/g, '""')}"`
    return s
  }
  const lines = [headers.map(esc).join(',')]
  for (const row of rows) {
    lines.push(headers.map((h) => esc(row[h])).join(','))
  }
  const blob = new Blob(['\uFEFF' + lines.join('\n')], { type: 'text/csv;charset=utf-8' })
  downloadBlob(blob, filename.endsWith('.csv') ? filename : `${filename}.csv`)
}

export async function parseSpreadsheet(file: File): Promise<Record<string, unknown>[]> {
  const name = file.name.toLowerCase()
  if (name.endsWith('.csv')) {
    return parseCsvText(await file.text())
  }
  const wb = new ExcelJS.Workbook()
  await wb.xlsx.load(await file.arrayBuffer())
  const sheet = wb.worksheets[0]
  if (!sheet) return []
  const headers: string[] = []
  const rows: Record<string, unknown>[] = []
  sheet.eachRow((row, idx) => {
    if (idx === 1) {
      row.eachCell((cell, col) => {
        headers[col] = String(cell.value ?? '').trim()
      })
      return
    }
    const obj: Record<string, unknown> = {}
    headers.forEach((h, col) => {
      if (!h) return
      const raw = row.getCell(col).value
      obj[h] = raw === null || raw === undefined ? '' : typeof raw === 'object' && raw && 'text' in raw ? String((raw as { text: string }).text) : raw
    })
    rows.push(obj)
  })
  return rows
}

function parseCsvText(text: string): Record<string, unknown>[] {
  const lines = text.split(/\r?\n/).filter((l) => l.trim())
  if (lines.length < 2) return []
  const headers = splitCsvLine(lines[0])
  return lines.slice(1).map((line) => {
    const cols = splitCsvLine(line)
    const row: Record<string, unknown> = {}
    headers.forEach((h, i) => {
      row[h] = cols[i] ?? ''
    })
    return row
  })
}

function splitCsvLine(line: string): string[] {
  const out: string[] = []
  let cur = ''
  let inQuotes = false
  for (let i = 0; i < line.length; i++) {
    const ch = line[i]
    if (inQuotes) {
      if (ch === '"' && line[i + 1] === '"') {
        cur += '"'
        i++
      } else if (ch === '"') {
        inQuotes = false
      } else {
        cur += ch
      }
    } else if (ch === '"') {
      inQuotes = true
    } else if (ch === ',') {
      out.push(cur.trim())
      cur = ''
    } else {
      cur += ch
    }
  }
  out.push(cur.trim())
  return out
}
