import { invoke } from '@tauri-apps/api/core'

export const workspaceApi = {
  /**
   * 保存当前工作区会话
   */
  async saveSession(state: any): Promise<void> {
    return invoke('save_session', { state })
  },

  /**
   * 加载上次工作区会话
   */
  async loadSession(): Promise<any | null> {
    return invoke<any | null>('load_session')
  },

  /**
   * 列出数据库相关的脚本文件
   */
  async listDbScripts(connectionId: string, database: string): Promise<any[]> {
    return invoke('list_db_scripts', { connectionId, database })
  },

  /**
   * 创建新的数据库脚本文件
   */
  async createDbScript(connectionId: string, database: string, content: string): Promise<any> {
    return invoke('create_db_script', { connectionId, database, content })
  }
}
