/**
 * SQL 自动补全服务 (重构版)
 * 实现全局单例、模型感知、智能缓存
 */

import * as monaco from 'monaco-editor'
import { invoke } from '@tauri-apps/api/core'
import type { DatabaseType } from '@/types/database'

export interface AutoCompleteData {
  databases: string[]
  tables: TableSuggestion[]
  keywords: string[]
}

export interface TableSuggestion {
  name: string
  schema?: string
  database: string
  columns: ColumnSuggestion[]
}

export interface ColumnSuggestion {
  name: string
  data_type: string
}

interface ModelContext {
  connectionId: string
  database: string | null
  dbType: DatabaseType | null
}

/**
 * 全局 SQL 自动补全管理器
 */
export class SqlAutocompleteManager implements monaco.languages.CompletionItemProvider {
  private static instance: SqlAutocompleteManager
  private contextMap = new Map<string, ModelContext>() // modelId -> context
  private dataCache = new Map<string, AutoCompleteData>() // "connId:db" -> data
  private loadingMap = new Map<string, Promise<AutoCompleteData>>() // 防止重复请求

  private constructor() {
    // 注册到 Monaco
    monaco.languages.registerCompletionItemProvider('sql', this)
  }

  public static getInstance(): SqlAutocompleteManager {
    if (!SqlAutocompleteManager.instance) {
      SqlAutocompleteManager.instance = new SqlAutocompleteManager()
    }
    return SqlAutocompleteManager.instance
  }

  /**
   * 为模型绑定上下文
   */
  public bindModel(model: monaco.editor.ITextModel, context: ModelContext) {
    this.contextMap.set(model.id, context)
    // 预加载数据
    this.fetchData(context.connectionId, context.database)
  }

  /**
   * 解绑模型
   */
  public unbindModel(model: monaco.editor.ITextModel) {
    this.contextMap.delete(model.id)
  }

  /**
   * 获取/刷新数据
   */
  private async fetchData(connectionId: string, database: string | null): Promise<AutoCompleteData | null> {
    const cacheKey = `${connectionId}:${database || ''}`
    
    // 如果已有缓存，直接返回
    if (this.dataCache.has(cacheKey)) {
      return this.dataCache.get(cacheKey)!
    }

    // 如果正在加载，等待
    if (this.loadingMap.has(cacheKey)) {
      return this.loadingMap.get(cacheKey)!
    }

    const promise = (async () => {
      try {
        const data = await invoke<AutoCompleteData>('get_autocomplete_data', {
          connectionId,
          database,
        })
        this.dataCache.set(cacheKey, data)
        return data
      } catch (error) {
        console.error('加载补全数据失败:', error)
        throw error
      } finally {
        this.loadingMap.delete(cacheKey)
      }
    })()

    this.loadingMap.set(cacheKey, promise)
    return promise
  }

  private quoteIdentifier(name: string, dbType: DatabaseType | null): string {
    if (dbType === 'postgresql') {
      if (/[A-Z]/.test(name) || /[^a-z0-9_]/.test(name)) {
        return `"${name}"`
      }
    }
    return name
  }

  /**
   * Monaco 补全核心回调
   */
  async provideCompletionItems(
    model: monaco.editor.ITextModel,
    position: monaco.Position
  ): Promise<monaco.languages.CompletionList> {
    const context = this.contextMap.get(model.id)
    if (!context) return { suggestions: [] }

    const data = await this.fetchData(context.connectionId, context.database).catch(() => null)
    if (!data) return { suggestions: [] }

    const suggestions: monaco.languages.CompletionItem[] = []
    const word = model.getWordUntilPosition(position)
    const range = {
      startLineNumber: position.lineNumber,
      endLineNumber: position.lineNumber,
      startColumn: word.startColumn,
      endColumn: word.endColumn,
    }

    const textUntilPosition = model.getValueInRange({
      startLineNumber: position.lineNumber,
      startColumn: 1,
      endLineNumber: position.lineNumber,
      endColumn: position.column,
    })
    const upperText = textUntilPosition.toUpperCase()
    const tokens = textUntilPosition.trim().split(/\s+/)
    const lastToken = tokens[tokens.length - 1]?.toUpperCase() || ''
    const secondLastToken = tokens[tokens.length - 2]?.toUpperCase() || ''

    // 1. 关键字
    for (const keyword of data.keywords) {
      suggestions.push({
        label: keyword,
        kind: monaco.languages.CompletionItemKind.Keyword,
        insertText: keyword,
        range,
        sortText: `0_${keyword}`,
      })
    }

    // 2. 数据库
    const isDbContext = lastToken === 'FROM' || lastToken === 'USE' || lastToken === 'DATABASE' || upperText.includes('FROM')
    if (isDbContext) {
      for (const db of data.databases) {
        suggestions.push({
          label: db,
          kind: monaco.languages.CompletionItemKind.Module,
          insertText: this.quoteIdentifier(db, context.dbType),
          range,
          sortText: `1_${db}`,
        })
      }
    }

    // 3. 表建议
    const isTableContext = lastToken === 'FROM' || lastToken === 'JOIN' || lastToken === 'TABLE' || upperText.includes('FROM')
    if (isTableContext) {
      for (const table of data.tables) {
        const quotedName = this.quoteIdentifier(table.name, context.dbType)
        const quotedSchema = table.schema ? this.quoteIdentifier(table.schema, context.dbType) : ''
        
        let label = table.name
        if (table.schema && table.schema !== 'public') label = `${table.schema}.${table.name}`
        
        let insertText = quotedName
        if (quotedSchema) insertText = `${quotedSchema}.${quotedName}`

        suggestions.push({
          label,
          kind: monaco.languages.CompletionItemKind.Class,
          detail: `表 (${table.schema || 'public'})`,
          insertText,
          range,
          sortText: `2_${label}`,
        })
      }
    }

    // 4. 列建议 (逻辑保持，但使用 context)
    const isColContext = lastToken === 'SELECT' || lastToken === 'WHERE' || lastToken === 'SET' || lastToken === ',' || upperText.includes('SELECT')
    if (isColContext) {
      const tablesInQuery = this.extractTablesFromQuery(model.getValue())
      for (const table of data.tables) {
        const normalizedTableName = table.name.replace(/"/g, '').toLowerCase()
        if (tablesInQuery.length > 0 && !tablesInQuery.some(t => t.replace(/"/g, '').toLowerCase() === normalizedTableName)) {
          continue
        }

        for (const column of table.columns) {
          const quotedCol = this.quoteIdentifier(column.name, context.dbType)
          const label = tablesInQuery.length > 1 ? `${table.name}.${column.name}` : column.name
          const insertText = tablesInQuery.length > 1 ? `${this.quoteIdentifier(table.name, context.dbType)}.${quotedCol}` : quotedCol

          suggestions.push({
            label,
            kind: monaco.languages.CompletionItemKind.Field,
            detail: `${column.data_type} (${table.name})`,
            insertText,
            range,
            sortText: `3_${label}`,
          })
        }
      }
    }

    return { suggestions }
  }

  private extractTablesFromQuery(sql: string): string[] {
    const tables: string[] = []
    const tableRegex = /(?:FROM|JOIN|UPDATE|INTO|TABLE)\s+((?:"[^"]+")|(?:`[^`]+`)|(?:[a-zA-Z0-9_]+))/gi
    let match
    while ((match = tableRegex.exec(sql)) !== null) {
      if (match[1]) tables.push(match[1])
    }
    return tables
  }

  /**
   * 清理特定连接的所有缓存 (例如连接关闭时)
   */
  public clearCache(connectionId?: string) {
    if (!connectionId) {
      this.dataCache.clear()
    } else {
      for (const key of this.dataCache.keys()) {
        if (key.startsWith(`${connectionId}:`)) {
          this.dataCache.delete(key)
        }
      }
    }
  }
}

/**
 * 外部调用的单例获取函数
 */
export function getSqlAutocompleteManager(): SqlAutocompleteManager {
  return SqlAutocompleteManager.getInstance()
}
