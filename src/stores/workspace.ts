import { defineStore } from 'pinia'
import { ref } from 'vue'
import { workspaceApi } from '@/api'
import { withErrorHandler } from '@/utils/errorHandler'
import { TabType, type TabState } from '@/types/workspace'

export type { TabState }

export const useWorkspaceStore = defineStore('workspace', () => {
  const isRestoring = ref(false)

  async function saveSession(tabs: TabState[], activeKey: string) {
    if (isRestoring.value) return
    
    // 过滤掉临时且不需要持久化的标签页
    const persistableTabs = tabs.filter(t => t.type === TabType.Query || t.type === TabType.Redis)
    
    await workspaceApi.saveSession({
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
    })
  }

  async function loadSession(): Promise<{ open_tabs: TabState[], active_tab_key: string } | null> {
    const result = await withErrorHandler(async () => {
      const session = await workspaceApi.loadSession()
      if (!session) return null
      
      return {
        open_tabs: session.open_tabs.map((t) => ({
          key: t.key,
          title: t.title,
          type: t.type as TabType,
          connectionId: t.connection_id,
          database: t.database,
          schema: t.schema,
          content: t.content,
          filePath: t.file_path,
          readOnly: t.read_only
        })),
        active_tab_key: session.active_tab_key
      }
    }, { messagePrefix: '加载工作区会话失败' })

    return result || null
  }

  return {
    isRestoring,
    saveSession,
    loadSession
  }
})
