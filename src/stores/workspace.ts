import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface TabState {
  key: string
  title: string
  type: 'data' | 'design' | 'query' | 'redis'
  connectionId?: string
  database?: string
  schema?: string
  content?: string
  filePath?: string
  readOnly?: boolean
}

export interface SessionState {
  open_tabs: any[]
  active_tab_key: string
}

export const useWorkspaceStore = defineStore('workspace', () => {
  const isRestoring = ref(false)

  async function saveSession(tabs: TabState[], activeKey: string) {
    if (isRestoring.value) return
    
    try {
      // 过滤掉临时且不需要持久化的标签页
      const persistableTabs = tabs.filter(t => t.type === 'query' || t.type === 'redis')
      
      await invoke('save_session', {
        state: {
          open_tabs: persistableTabs.map(t => ({
            key: t.key,
            title: t.title,
            type: t.type,
            connection_id: t.connectionId,
            database: t.database,
            schema: t.schema,
            content: t.content,
            file_path: t.filePath,
            read_only: t.readOnly
          })),
          active_tab_key: activeKey
        }
      })
    } catch (e) {
      console.error('保存会话失败:', e)
    }
  }

  async function loadSession(): Promise<{ open_tabs: TabState[], active_tab_key: string } | null> {
    try {
      const session = await invoke<any | null>('load_session')
      if (!session) return null
      
      return {
        open_tabs: session.open_tabs.map((t: any) => ({
          key: t.key,
          title: t.title,
          type: t.type,
          connectionId: t.connection_id,
          database: t.database,
          schema: t.schema,
          content: t.content,
          filePath: t.file_path,
          readOnly: t.read_only
        })),
        active_tab_key: session.active_tab_key
      }
    } catch (e) {
      console.error('加载会话失败:', e)
      return null
    }
  }

  return {
    isRestoring,
    saveSession,
    loadSession
  }
})
