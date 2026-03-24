import { invoke } from '@tauri-apps/api/core'
import type { QueryResult } from '@/types/database'

export const exportApi = {
  toCsv(data: QueryResult, filePath: string) {
    return invoke<boolean>('export_to_csv', { data, filePath })
  },
  toJson(data: QueryResult, filePath: string) {
    return invoke<boolean>('export_to_json', { data, filePath })
  },
  toSql(data: QueryResult, tableName: string, filePath: string) {
    return invoke<boolean>('export_to_sql', { data, tableName, filePath })
  },
  tableDdl(connectionId: string, database: string, table: string) {
    return invoke<string>('export_table_ddl', { connectionId, database, table })
  },
}
