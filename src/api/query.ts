import { invoke } from '@tauri-apps/api/core'
import type { QueryResult } from '@/types/database'

export interface PreparedSqlStatement {
  sql: string
  can_page: boolean
}

export const queryApi = {
  /**
   * 执行 SQL 查询
   */
  async executeQuery(
    connectionId: string, 
    sql: string, 
    database?: string | null,
    queryId?: number | null
  ): Promise<QueryResult[]> {
    return invoke<QueryResult[]>('execute_query', { 
      connectionId, 
      sql, 
      database: database || null,
      queryId: queryId ?? null,
    })
  },

  /**
   * 解释 SQL 查询 (EXPLAIN)
   */
  async explainQuery(
    connectionId: string, 
    sql: string, 
    database?: string | null,
    queryId?: number | null
  ): Promise<QueryResult[]> {
    return invoke<QueryResult[]>('explain_query', { 
      connectionId, 
      sql, 
      database: database || null,
      queryId: queryId ?? null,
    })
  },

  /**
   * 批量执行 SQL
   */
  async executeQueryBatch(
    connectionId: string,
    sqls: string[],
    database?: string | null,
    queryId?: number | null
  ): Promise<QueryResult[]> {
    return invoke<QueryResult[]>('execute_query_batch', {
      connectionId,
      sqls,
      database: database || null,
      queryId: queryId ?? null,
    })
  },

  /**
   * 取消正在执行的 SQL
   */
  async cancelQuery(
    connectionId: string,
    queryId: number
  ): Promise<boolean> {
    return invoke<boolean>('cancel_query', {
      connectionId,
      queryId,
    })
  },

  /**
   * 让后端按数据库方言解析 SQL 脚本
   */
  async prepareSqlScript(
    connectionId: string,
    sql: string
  ): Promise<PreparedSqlStatement[]> {
    return invoke<PreparedSqlStatement[]>('prepare_sql_script', { connectionId, sql })
  },

  /**
   * 格式化/美化 SQL
   */
  async beautifySql(connectionId: string, sql: string): Promise<string> {
    return invoke<string>('beautify_sql', { connectionId, sql })
  },

  /**
   * 更新表数据 (参数化)
   */
  async updateTableData(params: {
    connectionId: string,
    database: string,
    table: string,
    schema?: string | null,
    column: string,
    value: string | null,
    whereConditions: Record<string, any>
  }): Promise<void> {
    return invoke('update_table_data', params)
  },

  /**
   * 删除表数据 (参数化)
   */
  async deleteTableData(params: {
    connectionId: string,
    database: string,
    table: string,
    schema?: string | null,
    whereConditions: Record<string, any>
  }): Promise<void> {
    return invoke('delete_table_data', params)
  },

  /**
   * 变更表结构
   */
  async alterTableStructure(params: {
    connectionId: string,
    database: string,
    table: string,
    schema?: string | null,
    changes: Array<{ type: string, data: any }>
  }): Promise<void> {
    return invoke('alter_table_structure', params)
  }
}
