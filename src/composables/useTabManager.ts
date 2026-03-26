import { ref, computed, reactive } from 'vue'
import type { TabState } from '@/types/workspace'
import type { SqlExecutionState } from '@/types/sqlExecution'

export interface DataTab extends TabState {
  table?: string
  closable?: boolean
}

/** SqlEditor 组件暴露的方法接口 */
export interface SqlEditorExposed {
  executeQuery: () => void
  explainQuery: () => void
  stopExecution: () => void
  handleSave: () => void
  formatSql: () => void
  clearEditor: () => void
  openHistory: () => void
  openSnippets: () => void
  refreshAutocomplete: () => void
  handleDatabaseChange: (db: string) => void
  setSelectedDatabase: (db: string) => void
  executing: boolean
  executionState: SqlExecutionState
  [key: string]: unknown
}

export function useTabManager() {
  const dataTabs = ref<DataTab[]>([])
  const mainTabKey = ref('')
  const sqlEditorRefs = reactive<Record<string, SqlEditorExposed>>({})

  const activeTabType = computed(() =>
    dataTabs.value.find(t => t.key === mainTabKey.value)?.type
  )

  const activeTabDatabase = computed({
    get: () => dataTabs.value.find(t => t.key === mainTabKey.value)?.database || '',
    set: (val: string) => {
      const tab = dataTabs.value.find(t => t.key === mainTabKey.value)
      if (tab) tab.database = val
    }
  })

  const activeEditorExecuting = computed(() =>
    sqlEditorRefs[mainTabKey.value]?.executing || false
  )

  const activeEditorExecutionState = computed<SqlExecutionState | null>(() =>
    sqlEditorRefs[mainTabKey.value]?.executionState || null
  )

  function setSqlEditorRef(el: unknown, key: string) {
    if (el) sqlEditorRefs[key] = el as SqlEditorExposed
    else delete sqlEditorRefs[key]
  }

  function callActiveEditor(method: string, ...args: unknown[]) {
    const editor = sqlEditorRefs[mainTabKey.value]
    if (editor && typeof editor[method] === 'function') {
      (editor[method] as (...a: unknown[]) => void)(...args)
    }
  }

  function closeTab(key: string) {
    const index = dataTabs.value.findIndex(t => t.key === key)
    if (index >= 0) {
      dataTabs.value.splice(index, 1)
      if (mainTabKey.value === key && dataTabs.value.length > 0) {
        mainTabKey.value = dataTabs.value[Math.min(index, dataTabs.value.length - 1)].key
      } else if (dataTabs.value.length === 0) {
        mainTabKey.value = ''
      }
    }
  }

  function applyTabRemoval(keysToRemove: Set<string>, fallbackActiveKey?: string) {
    if (keysToRemove.size === 0) return

    const remainingTabs = dataTabs.value.filter(tab => !keysToRemove.has(tab.key))
    dataTabs.value = remainingTabs

    if (remainingTabs.length === 0) {
      mainTabKey.value = ''
      return
    }

    if (!keysToRemove.has(mainTabKey.value) && remainingTabs.some(tab => tab.key === mainTabKey.value)) {
      return
    }

    if (fallbackActiveKey && remainingTabs.some(tab => tab.key === fallbackActiveKey)) {
      mainTabKey.value = fallbackActiveKey
      return
    }

    mainTabKey.value = remainingTabs[Math.max(remainingTabs.length - 1, 0)].key
  }

  function closeTabsLeftOf(key: string) {
    const anchorIndex = dataTabs.value.findIndex(tab => tab.key === key)
    if (anchorIndex <= 0) return

    const keysToRemove = new Set(
      dataTabs.value
        .slice(0, anchorIndex)
        .filter(tab => tab.closable !== false)
        .map(tab => tab.key)
    )

    applyTabRemoval(keysToRemove, key)
  }

  function closeTabsRightOf(key: string) {
    const anchorIndex = dataTabs.value.findIndex(tab => tab.key === key)
    if (anchorIndex < 0 || anchorIndex >= dataTabs.value.length - 1) return

    const keysToRemove = new Set(
      dataTabs.value
        .slice(anchorIndex + 1)
        .filter(tab => tab.closable !== false)
        .map(tab => tab.key)
    )

    applyTabRemoval(keysToRemove, key)
  }

  function closeOtherTabs(key: string) {
    const keysToRemove = new Set(
      dataTabs.value
        .filter(tab => tab.key !== key && tab.closable !== false)
        .map(tab => tab.key)
    )

    applyTabRemoval(keysToRemove, key)
  }

  function closeSavedTabs(fallbackActiveKey?: string) {
    const keysToRemove = new Set(
      dataTabs.value
        .filter(tab => tab.closable !== false && Boolean(tab.filePath))
        .map(tab => tab.key)
    )

    applyTabRemoval(keysToRemove, fallbackActiveKey)
  }

  function findTabByKey(key: string): DataTab | undefined {
    return dataTabs.value.find(t => t.key === key)
  }

  function tabExists(key: string): boolean {
    return dataTabs.value.some(t => t.key === key)
  }

  function addTab(tab: DataTab) {
    dataTabs.value.push(tab)
    mainTabKey.value = tab.key
  }

  function handleContentChange(key: string, val: string) {
    const tab = dataTabs.value.find(t => t.key === key)
    if (tab) tab.content = val
  }

  function handleFileSaved(key: string, path: string, title: string) {
    const tab = dataTabs.value.find(t => t.key === key)
    if (tab) {
      tab.filePath = path
      tab.title = title
    }
  }

  return {
    dataTabs,
    mainTabKey,
    sqlEditorRefs,
    activeTabType,
    activeTabDatabase,
    activeEditorExecuting,
    activeEditorExecutionState,
    setSqlEditorRef,
    callActiveEditor,
    closeTab,
    closeTabsLeftOf,
    closeTabsRightOf,
    closeOtherTabs,
    closeSavedTabs,
    findTabByKey,
    tabExists,
    addTab,
    handleContentChange,
    handleFileSaved,
  }
}
