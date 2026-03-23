import { invoke } from '@tauri-apps/api/core'
import type { 
  DatabaseInfo, TableInfo, SchemaInfo, ColumnInfo, IndexInfo, ForeignKeyInfo, FunctionInfo, ExtensionInfo 
} from '@/types/database'

export const metadataApi = {
  /**
   * 获取数据库列表
   */
  async getDatabases(connectionId: string): Promise<DatabaseInfo[]> {
    return invoke<DatabaseInfo[]>('get_databases', { connectionId })
  },

  /**
   * 获取表列表
   */
  async getTables(connectionId: string, database?: string | null): Promise<TableInfo[]> {
    return invoke<TableInfo[]>('get_tables', { connectionId, database })
  },

  /**
   * 获取视图列表
   */
  async getViews(connectionId: string, database?: string | null): Promise<TableInfo[]> {
    return invoke<TableInfo[]>('get_views', { connectionId, database })
  },

  /**
   * 获取 Schema 列表 (PostgreSQL)
   */
  async getSchemas(connectionId: string, database?: string | null): Promise<SchemaInfo[]> {
    return invoke<SchemaInfo[]>('get_schemas', { connectionId, database })
  },

  /**
   * 获取指定 Schema 下的表
   */
  async getSchemaTables(connectionId: string, database: string, schema: string): Promise<TableInfo[]> {
    return invoke<TableInfo[]>('get_schema_tables', { connectionId, database, schema })
  },

  /**
   * 获取指定 Schema 下的视图
   */
  async getSchemaViews(connectionId: string, database: string, schema: string): Promise<TableInfo[]> {
    return invoke<TableInfo[]>('get_schema_views', { connectionId, database, schema })
  },

  /**
   * 获取函数列表
   */
  async getSchemaFunctions(connectionId: string, database: string, schema: string): Promise<FunctionInfo[]> {
    return invoke<FunctionInfo[]>('get_schema_functions', { connectionId, database, schema })
  },

  /**
   * 获取聚合函数列表
   */
  async getSchemaAggregateFunctions(connectionId: string, database: string, schema: string): Promise<FunctionInfo[]> {
    return invoke<FunctionInfo[]>('get_schema_aggregate_functions', { connectionId, database, schema })
  },

  /**
   * 获取数据库扩展 (PostgreSQL)
   */
  async getDatabaseExtensions(connectionId: string, database: string): Promise<ExtensionInfo[]> {
    return invoke<ExtensionInfo[]>('get_database_extensions', { connectionId, database })
  },

  /**
   * 获取表结构
   */
  async getTableStructure(params: {
    connectionId: string,
    table: string,
    database?: string | null,
    schema?: string | null
  }): Promise<ColumnInfo[]> {
    return invoke<ColumnInfo[]>('get_table_structure', params)
  },

  /**
   * 获取表索引
   */
  async getTableIndexes(params: {
    connectionId: string,
    table: string,
    schema?: string | null
  }): Promise<IndexInfo[]> {
    return invoke<IndexInfo[]>('get_table_indexes', params)
  },

  /**
   * 获取表外键
   */
  async getTableForeignKeys(params: {
    connectionId: string,
    table: string,
    schema?: string | null
  }): Promise<ForeignKeyInfo[]> {
    return invoke<ForeignKeyInfo[]>('get_table_foreign_keys', params)
  },

  /**
   * 获取 Schema 下的所有索引
   */
  async getSchemaIndexes(connectionId: string, database: string, schema: string): Promise<IndexInfo[]> {
    return invoke<IndexInfo[]>('get_schema_indexes', { connectionId, database, schema })
  },

  /**
   * 获取创建表的 DDL
   */
  async getCreateTableDdl(params: {
    connectionId: string,
    table: string,
    database?: string | null,
    schema?: string | null
  }): Promise<string> {
    return invoke<string>('get_create_table_ddl', params)
  }
}
