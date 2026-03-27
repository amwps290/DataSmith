import type { ColumnInfo } from '@/types/database'

export type InsertFieldKind = 'text' | 'textarea' | 'json' | 'number' | 'datetime' | 'boolean'

function unwrapDefaultExpression(input: string): string {
  let value = input.trim()
  while (value.startsWith('(') && value.endsWith(')')) {
    value = value.slice(1, -1).trim()
  }
  return value.replace(/::[\w\s.[\]"]+$/g, '').trim()
}

export function normalizeColumnType(dataType: string): string {
  return dataType.trim().toLowerCase()
}

export function hasColumnDefault(column: Pick<ColumnInfo, 'default_value'>): boolean {
  return column.default_value !== undefined && column.default_value !== null && String(column.default_value).trim() !== ''
}

export function isBooleanColumn(dataType: string): boolean {
  const normalized = normalizeColumnType(dataType)
  return ['boolean', 'bool', 'tinyint(1)', 'bit(1)'].includes(normalized)
}

export function isNumericColumn(dataType: string): boolean {
  const normalized = normalizeColumnType(dataType)
  return [
    'int',
    'integer',
    'bigint',
    'smallint',
    'tinyint',
    'mediumint',
    'serial',
    'bigserial',
    'decimal',
    'numeric',
    'float',
    'double',
    'double precision',
    'real',
    'money',
  ].includes(normalized)
}

export function isDatetimeColumn(dataType: string): boolean {
  const normalized = normalizeColumnType(dataType)
  return [
    'date',
    'datetime',
    'timestamp',
    'timestamp without time zone',
    'timestamp with time zone',
    'time',
    'time without time zone',
    'time with time zone',
  ].includes(normalized)
}

export function isJsonColumn(dataType: string): boolean {
  const normalized = normalizeColumnType(dataType)
  return normalized === 'json' || normalized === 'jsonb'
}

export function isTextareaColumn(dataType: string): boolean {
  const normalized = normalizeColumnType(dataType)
  return [
    'text',
    'tinytext',
    'mediumtext',
    'longtext',
    'blob',
    'mediumblob',
    'longblob',
    'clob',
    'nclob',
    'xml',
  ].includes(normalized) || isJsonColumn(normalized)
}

export function getInsertFieldKind(column: Pick<ColumnInfo, 'data_type'>): InsertFieldKind {
  if (isBooleanColumn(column.data_type)) return 'boolean'
  if (isJsonColumn(column.data_type)) return 'json'
  if (isDatetimeColumn(column.data_type)) return 'datetime'
  if (isNumericColumn(column.data_type)) return 'number'
  if (isTextareaColumn(column.data_type)) return 'textarea'
  return 'text'
}

export function parseColumnDefaultValue(defaultValue?: string | null): any {
  if (defaultValue === undefined || defaultValue === null) return undefined

  const raw = unwrapDefaultExpression(String(defaultValue))
  if (!raw || /^null$/i.test(raw)) return undefined
  if (/^(current_timestamp|current_date|current_time|localtimestamp|localtime)$/i.test(raw)) return undefined
  if (/^(now|uuid_generate_v4|gen_random_uuid)\(\)$/i.test(raw)) return undefined

  const singleQuoted = raw.match(/^'(.*)'$/s)
  if (singleQuoted) {
    return singleQuoted[1].replace(/''/g, "'")
  }

  if (/^(true|false)$/i.test(raw)) {
    return /^true$/i.test(raw)
  }

  if (/^b'([01])'$/i.test(raw)) {
    return /1/.test(raw)
  }

  if (/^[+-]?\d+(\.\d+)?$/.test(raw)) {
    const numericValue = Number(raw)
    return Number.isNaN(numericValue) ? raw : numericValue
  }

  return undefined
}

export function buildInitialColumnValue(column: Pick<ColumnInfo, 'default_value' | 'is_auto_increment' | 'data_type' | 'name'>): any {
  if (column.is_auto_increment) return null

  const parsedDefault = parseColumnDefaultValue(column.default_value)
  return parsedDefault === undefined ? null : normalizeInsertValue(column, parsedDefault)
}

export function normalizeInsertValue(column: Pick<ColumnInfo, 'name' | 'data_type'>, value: any): any {
  if (value === null || value === undefined) return value

  if (isBooleanColumn(column.data_type)) {
    if (typeof value === 'boolean') return value
    if (typeof value === 'number') return value !== 0
    if (typeof value === 'string') {
      const normalized = value.trim().toLowerCase()
      if (['true', '1', 'yes', 'y', 'on'].includes(normalized)) return true
      if (['false', '0', 'no', 'n', 'off'].includes(normalized)) return false
    }
  }

  if (isJsonColumn(column.data_type) && typeof value === 'string') {
    const trimmed = value.trim()
    if (!trimmed) return ''
    try {
      return JSON.parse(trimmed)
    } catch {
      throw new Error(`INVALID_JSON:${column.name}`)
    }
  }

  return value
}
