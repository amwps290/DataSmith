import { invoke } from '@tauri-apps/api/core'
import type {
  ConnectionConfig,
  ConnectionTestResult,
  StoredConnection
} from '@/types/database'

export const connectionApi = {
  /**
   * 获取所有已保存的连接
   */
  async getConnections(): Promise<ConnectionConfig[]> {
    return invoke<ConnectionConfig[]>('get_connections')
  },

  /**
   * 测试数据库连接
   */
  async testConnection(config: ConnectionConfig): Promise<ConnectionTestResult> {
    return invoke<ConnectionTestResult>('test_connection', { config })
  },

  /**
   * 保存新连接
   */
  async saveConnection(connection: StoredConnection, password: string | null): Promise<StoredConnection> {
    return invoke<StoredConnection>('save_connection', { connection, password })
  },

  /**
   * 更新现有连接
   */
  async updateConnection(connection: StoredConnection, password: string | null): Promise<StoredConnection> {
    return invoke<StoredConnection>('update_connection', { connection, password })
  },

  /**
   * 删除连接
   */
  async deleteConnection(id: string): Promise<boolean> {
    return invoke<boolean>('delete_connection', { id })
  },

  /**
   * 物理建立数据库连接
   */
  async createConnection(connectionId: string): Promise<void> {
    return invoke('create_connection', { connectionId })
  },

  /**
   * 断开数据库连接
   */
  async disconnectDatabase(connectionId: string): Promise<void> {
    return invoke('disconnect_database', { connectionId })
  },

  /**
   * 创建 SQLite 数据库文件
   */
  async createSqliteDatabase(path: string): Promise<string> {
    return invoke<string>('create_sqlite_database', { path })
  }
}
