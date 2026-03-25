import { invoke } from '@tauri-apps/api/core'

export const utilsApi = {
  /**
   * 读取文件内容
   */
  async readFile(path: string): Promise<string> {
    return invoke<string>('read_file', { path })
  },

  /**
   * 写入文件内容
   */
  async writeFile(path: string, content: string): Promise<void> {
    return invoke('write_file', { path, content })
  },

  /**
   * 更新运行时日志等级
   */
  async setLogLevel(level: string): Promise<string> {
    return invoke<string>('set_log_level', { level })
  }
}
