import { invoke } from '@tauri-apps/api/core'

export const redisApi = {
  executeCommand(connectionId: string, command: string, args: string[]) {
    return invoke<any>('execute_redis_command', { connectionId, command, args })
  },
  getInfo(connectionId: string) {
    return invoke<Record<string, string>>('get_redis_info', { connectionId })
  },
  getKeyValue(connectionId: string, key: string) {
    return invoke<any>('get_redis_key_value', { connectionId, key })
  },
  setKeyValue(connectionId: string, key: string, value: string, ttl?: number) {
    return invoke<void>('set_redis_key_value', { connectionId, key, value, ttl })
  },
  deleteKey(connectionId: string, key: string) {
    return invoke<void>('delete_redis_key', { connectionId, key })
  },
}
