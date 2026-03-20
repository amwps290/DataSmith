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
  functions: FunctionSuggestion[]
  keywords: string[]
}

export interface TableSuggestion {
  name: string
  schema?: string
  database: string
  columns: ColumnSuggestion[]
}

export interface FunctionSuggestion {
  name: string
  schema?: string
  database: string
  return_type?: string
  arguments?: string
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
   * 将函数参数字符串转换为 Monaco Snippet 格式
   * 例如: "id integer, name text" -> "(${1:id}, ${2:name})"
   */
  private generateFunctionSnippet(name: string, args: string | undefined): string {
    if (!args || args.trim() === '' || args.trim() === '()') {
      return `${name}($0)`
    }

    // 移除外层括号并按逗号分割
    const cleanArgs = args.replace(/^\(|\)$/g, '').trim()
    if (!cleanArgs) return `${name}($0)`

    const argList = cleanArgs.split(',').map(s => s.trim())
    const placeholders = argList.map((arg, index) => {
      // 尝试提取参数名 (通常是第一个单词)
      // 处理 "name type", "IN name type", 或者只有 "type" 的情况
      const parts = arg.split(/\s+/)
      let paramName = parts[0]
      
      // 如果第一个单词是 IN/OUT/INOUT，取第二个
      if (['IN', 'OUT', 'INOUT', 'VARIADIC'].includes(paramName.toUpperCase()) && parts.length > 1) {
        paramName = parts[1]
      }

      // 如果提取出的参数名看起来像类型名，或者包含特殊字符，使用通用占位符
      if (paramName.toLowerCase() === 'integer' || paramName.toLowerCase() === 'text' || /[^a-zA-Z0-9_]/.test(paramName)) {
        paramName = `param${index + 1}`
      }

      return `\${${index + 1}:${paramName}}`
    })

    return `${name}(${placeholders.join(', ')})$0`
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

    // 4. 列建议
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

    // 5. 函数建议
    const isFuncContext = lastToken === 'SELECT' || lastToken === '(' || lastToken === ',' || upperText.includes('SELECT')
    if (isFuncContext) {
      for (const func of data.functions) {
        const quotedName = this.quoteIdentifier(func.name, context.dbType)
        const quotedSchema = func.schema ? this.quoteIdentifier(func.schema, context.dbType) : ''
        
        // 格式化参数签名，如果参数带括号就原样使用，否则补齐
        const args = func.arguments || ''
        const signature = args.startsWith('(') ? args : `(${args})`

        // 构造 label 对象：主标签是函数名(参数)，辅助标签是返回类型
        const labelObj = {
          label: func.name + signature,
          description: func.return_type || 'void',
          detail: func.schema ? ` [${func.schema}]` : ''
        }

        // 生成带参数占位符的 Snippet
        const functionNameWithSchema = quotedSchema ? `${quotedSchema}.${quotedName}` : quotedName
        const insertText = this.generateFunctionSnippet(functionNameWithSchema, func.arguments)

        suggestions.push({
          label: labelObj,
          kind: monaco.languages.CompletionItemKind.Function,
          detail: `函数: ${func.schema ? func.schema + '.' : ''}${func.name}${signature}`,
          documentation: {
            value: `### ${func.name}\n\n**签名:** \`${signature}\`\n\n**返回:** \`${func.return_type || 'void'}\`\n\n**路径:** \`${func.database}.${func.schema || 'public'}\``,
            isTrusted: true
          },
          insertText,
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          range,
          sortText: `4_${func.name}`,
        })

      }

      // 通用内置函数 (如果没有拉取到)
      if (data.functions.length < 10) {
        const builtInFuncs = [
          { name: 'COUNT', detail: '计数', snippet: 'COUNT($0)' },
          { name: 'SUM', detail: '求和', snippet: 'SUM($0)' },
          { name: 'AVG', detail: '平均值', snippet: 'AVG($0)' },
          { name: 'MAX', detail: '最大值', snippet: 'MAX($0)' },
          { name: 'MIN', detail: '最小值', snippet: 'MIN($0)' },
          { name: 'COALESCE', detail: '返回第一个非空值', snippet: 'COALESCE($1, $2)' },
          { name: 'NOW', detail: '当前时间', snippet: 'NOW()' },
        ]
        for (const f of builtInFuncs) {
          suggestions.push({
            label: f.name,
            kind: monaco.languages.CompletionItemKind.Function,
            detail: `内置函数: ${f.detail}`,
            insertText: f.snippet,
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            range,
            sortText: `5_${f.name}`,
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
