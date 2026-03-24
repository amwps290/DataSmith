import { ref, computed, reactive } from 'vue'
import type { TabState } from '@/types/workspace'

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
      }
    }
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
    setSqlEditorRef,
    callActiveEditor,
    closeTab,
    findTabByKey,
    tabExists,
    addTab,
    handleContentChange,
    handleFileSaved,
  }
}
