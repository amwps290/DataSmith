import { invoke } from '@tauri-apps/api/core'

export const dataApi = {
  async insertTableData(params: {
    connectionId: string,
    database: string,
    table: string,
    schema?: string,
    data: Record<string, any>
  }): Promise<void> {
    return invoke<void>('insert_table_data', params)
  },

  async updateTableData(params: {
    connectionId: string,
    database: string,
    table: string,
    schema?: string | null,
    column: string,
    value: string | null,
    whereConditions: Record<string, any>
  }): Promise<void> {
    return invoke<void>('update_table_data', params)
  },

  async deleteTableData(params: {
    connectionId: string,
    database: string,
    table: string,
    schema?: string | null,
    whereConditions: Record<string, any>
  }): Promise<void> {
    return invoke<void>('delete_table_data', params)
  },
}
