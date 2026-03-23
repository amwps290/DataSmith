import { invoke } from '@tauri-apps/api/core'
import type { QueryResult } from '@/types/database'

export const queryApi = {
  /**
   * 执行 SQL 查询
   */
  async executeQuery(
    connectionId: string, 
    sql: string, 
    database?: string | null
  ): Promise<QueryResult[]> {
    return invoke<QueryResult[]>('execute_query', { 
      connectionId, 
      sql, 
      database: database || null 
    })
  },

  /**
   * 解释 SQL 查询 (EXPLAIN)
   */
  async explainQuery(
    connectionId: string, 
    sql: string, 
    database?: string | null
  ): Promise<QueryResult[]> {
    return invoke<QueryResult[]>('explain_query', { 
      connectionId, 
      sql, 
      database: database || null 
    })
  },

  /**
   * 批量执行 SQL
   */
  async executeQueryBatch(connectionId: string, sqls: string[]): Promise<QueryResult[]> {
    return invoke<QueryResult[]>('execute_query_batch', { connectionId, sqls })
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
