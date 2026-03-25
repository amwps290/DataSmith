export type SqlDangerType =
  | 'update_without_where'
  | 'delete_without_where'
  | 'truncate'
  | 'drop'
  | 'batch_write'

export interface SqlDangerIssue {
  type: SqlDangerType
  statement: string
}

export interface SqlSafetyAnalysis {
  issues: SqlDangerIssue[]
  requiresConfirmation: boolean
}

export interface SqlWriteAnalysis {
  writeStatements: string[]
  hasWrites: boolean
}

const WRITE_PREFIXES = ['INSERT', 'UPDATE', 'DELETE', 'TRUNCATE', 'DROP', 'ALTER', 'CREATE', 'REPLACE', 'MERGE']

function stripLeadingComments(sql: string): string {
  let value = sql.trim()

  while (value) {
    if (value.startsWith('/*')) {
      const endIndex = value.indexOf('*/')
      if (endIndex === -1) return ''
      value = value.slice(endIndex + 2).trim()
      continue
    }

    if (value.startsWith('--')) {
      const endIndex = value.indexOf('\n')
      if (endIndex === -1) return ''
      value = value.slice(endIndex + 1).trim()
      continue
    }

    if (value.startsWith('#')) {
      const endIndex = value.indexOf('\n')
      if (endIndex === -1) return ''
      value = value.slice(endIndex + 1).trim()
      continue
    }

    break
  }

  return value
}

function normalizeSql(sql: string): string {
  return stripLeadingComments(sql)
    .replace(/\/\*[\s\S]*?\*\//g, ' ')
    .replace(/(--|#).*$/gm, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .toUpperCase()
}

function isWriteStatement(normalizedSql: string): boolean {
  return WRITE_PREFIXES.some((prefix) => normalizedSql.startsWith(prefix))
}

export function analyzeSqlWrites(statements: string[]): SqlWriteAnalysis {
  const writeStatements: string[] = []

  for (const statement of statements) {
    const normalizedSql = normalizeSql(statement)
    if (!normalizedSql) continue

    if (isWriteStatement(normalizedSql)) {
      writeStatements.push(statement)
    }
  }

  return {
    writeStatements,
    hasWrites: writeStatements.length > 0,
  }
}

export function analyzeSqlSafety(statements: string[]): SqlSafetyAnalysis {
  const issues: SqlDangerIssue[] = []
  const { writeStatements } = analyzeSqlWrites(statements)

  for (const statement of statements) {
    const normalizedSql = normalizeSql(statement)
    if (!normalizedSql) continue

    if (normalizedSql.startsWith('UPDATE ') && !/\bWHERE\b/.test(normalizedSql)) {
      issues.push({ type: 'update_without_where', statement })
    } else if (normalizedSql.startsWith('DELETE ') && !/\bWHERE\b/.test(normalizedSql)) {
      issues.push({ type: 'delete_without_where', statement })
    } else if (normalizedSql.startsWith('TRUNCATE ')) {
      issues.push({ type: 'truncate', statement })
    } else if (normalizedSql.startsWith('DROP ')) {
      issues.push({ type: 'drop', statement })
    }
  }

  if (writeStatements.length > 1) {
    issues.push({
      type: 'batch_write',
      statement: writeStatements.slice(0, 3).join('\n\n')
    })
  }

  return {
    issues,
    requiresConfirmation: issues.length > 0,
  }
}
