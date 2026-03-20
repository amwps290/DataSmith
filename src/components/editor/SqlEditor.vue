<template>
  <div class="sql-editor-container">
    <!-- 编辑器区域 -->
    <div class="editor-section" :style="{ height: editorHeight + 'px' }">
      <div ref="editorContainer" class="monaco-container"></div>
    </div>

    <!-- 拖拽调整器 -->
    <div class="split-resizer" @mousedown="startResize">
      <div class="resizer-handle"></div>
    </div>

    <!-- 结果区域 -->
    <div class="result-section">
      <a-tabs v-model:activeKey="resultTabKey" size="small" class="result-tabs">
        <a-tab-pane key="result" tab="查询结果">
          <div class="result-content">
            <div v-if="executing" class="executing-overlay">
              <a-spin tip="正在执行查询..." />
              <a-button danger size="small" @click="stopExecution" style="margin-top: 16px">
                停止执行
              </a-button>
            </div>
            <div v-if="queryResults.length > 0" class="result-info">
              <a-space>
                <a-tag color="success">
                  {{ queryResults[currentResultIndex]?.affected_rows || 0 }} 行
                </a-tag>
                <a-tag color="processing">
                  {{ queryResults[currentResultIndex]?.execution_time_ms || 0 }} ms
                </a-tag>
                
                <a-divider type="vertical" />
                <a-button 
                  size="small" 
                  :disabled="currentPage <= 1 || executing"
                  @click="handlePageChange(currentPage - 1)"
                >
                  上一页
                </a-button>
                <span class="page-indicator">第 {{ currentPage }} 页</span>
                <a-button 
                  size="small" 
                  :disabled="(currentResult?.rows.length || 0) < pageSize || executing"
                  @click="handlePageChange(currentPage + 1)"
                >
                  下一页
                </a-button>
                <a-select 
                  v-model:value="pageSize" 
                  size="small" 
                  style="width: 100px"
                  @change="handlePageChange(1)"
                >
                  <a-select-option :value="100">100 / page</a-select-option>
                  <a-select-option :value="200">200 / page</a-select-option>
                  <a-select-option :value="500">500 / page</a-select-option>
                </a-select>

                <a-dropdown v-if="queryResults.length > 1">
                  <a-button size="small">
                    结果集 {{ currentResultIndex + 1 }}/{{ queryResults.length }}
                    <DownOutlined />
                  </a-button>
                  <template #overlay>
                    <a-menu @click="({ key }) => currentResultIndex = Number(key)">
                      <a-menu-item v-for="(res, idx) in queryResults" :key="idx">
                        结果集 {{ idx + 1 }} ({{ res.rows.length }} 行)
                      </a-menu-item>
                    </a-menu>
                  </template>
                </a-dropdown>
              </a-space>
            </div>
            
            <div class="table-wrapper">
              <a-table
                v-if="currentResult"
                :columns="resultColumns"
                :data-source="currentResult.rows"
                :scroll="{ x: 'max-content', y: resultTableHeight }"
                :pagination="false"
                size="small"
                bordered
              />
              <a-empty v-else description="暂无查询结果" />
            </div>
          </div>
        </a-tab-pane>
        <a-tab-pane key="messages" tab="消息">
          <div class="messages-content">
            <div v-for="(msg, index) in messages" :key="index" :class="['message-item', msg.type]">
              <span class="message-time">{{ msg.time }}</span>
              <span class="message-text">{{ msg.text }}</span>
            </div>
            <a-empty v-if="messages.length === 0" description="暂无消息" />
          </div>
        </a-tab-pane>
      </a-tabs>
    </div>

    <!-- 历史记录抽屉 -->
    <a-drawer
      title="查询历史"
      placement="right"
      :visible="showHistory"
      @close="showHistory = false"
      width="400"
    >
      <a-list :data-source="sqlHistory" size="small">
        <template #renderItem="{ item }">
          <a-list-item class="history-item" @click="useHistorySql(item.sql)">
            <a-list-item-meta>
              <template #title>
                <code class="history-sql">{{ item.sql.substring(0, 100) }}{{ item.sql.length > 100 ? '...' : '' }}</code>
              </template>
              <template #description>
                {{ new Date(item.timestamp).toLocaleString() }} • 
                {{ item.database || '默认' }}
              </template>
            </a-list-item-meta>
          </a-list-item>
        </template>
      </a-list>
    </a-drawer>

    <!-- 保存查询对话框 -->
    <SaveQueryDialog
      v-model="showSaveDialog"
      :sql="editor?.getValue() || ''"
      @saved="handleQuerySaved"
    />

    <!-- 代码片段管理器 -->
    <SqlSnippetsManager
      v-model:visible="showSnippets"
      @insert="insertSnippet"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, watch, ref, computed, onActivated, nextTick } from 'vue'
import * as monaco from 'monaco-editor'
// @ts-ignore
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
import { getSqlAutocompleteManager } from '@/services/sqlAutocomplete'
import { DownOutlined } from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { save } from '@tauri-apps/plugin-dialog'
import { writeTextFile } from '@tauri-apps/plugin-fs'
import type { QueryResult } from '@/types/database'
import { useConnectionStore } from '@/stores/connection'
import { useAppStore } from '@/stores/app'
import SaveQueryDialog from './SaveQueryDialog.vue'
import SqlSnippetsManager from './SqlSnippetsManager.vue'

// 1. 定义 Props 和 Emits
const props = defineProps<{
  connectionId?: string
  initialDatabase?: string
  initialValue?: string
  filePath?: string
}>()

const emit = defineEmits(['contentChange', 'fileSaved', 'databasesLoaded'])

// 2. 实例化 Store
const connectionStore = useConnectionStore()
const appStore = useAppStore()

// 3. 配置 Monaco 环境 (确保只在 setup 内部配置一次)
if (!(window as any).MonacoEnvironment) {
  (window as any).MonacoEnvironment = {
    getWorker(_: any, _label: string) {
      return new editorWorker()
    }
  }
}

// 4. 定义响应式变量
const editorContainer = ref<HTMLElement>()
let editor: monaco.editor.IStandaloneCodeEditor | null = null
const autocompleteManager = getSqlAutocompleteManager()

const editorHeight = ref(300) 
const isSplitResizing = ref(false)
const resultTableHeight = ref(300) 

const availableDatabases = ref<any[]>([])
const selectedDatabase = ref(props.initialDatabase || '')
const loadingDatabases = ref(false)

const executing = ref(false)
const queryResults = ref<QueryResult[]>([])
const currentResultIndex = ref(0)
const resultTabKey = ref('result')
const showHistory = ref(false)
const sqlHistory = ref<any[]>([])
const showSaveDialog = ref(false)
const showSnippets = ref(false)

const currentPage = ref(1)
const pageSize = ref(200)

interface Message {
  type: 'info' | 'success' | 'error' | 'warning'
  text: string
  time: string
}
const messages = ref<Message[]>([])

// 5. 计算属性
const currentResult = computed(() => queryResults.value[currentResultIndex.value])

const resultColumns = computed(() => {
  if (!currentResult.value) return []
  return currentResult.value.columns.map(col => ({
    title: col,
    dataIndex: col,
    key: col,
    ellipsis: true,
    width: 150,
    resizable: true
  }))
})

// 6. 核心逻辑方法
function addMessage(type: Message['type'], text: string) {
  messages.value.unshift({ type, text, time: new Date().toLocaleTimeString() })
}

function startResize(e: MouseEvent) {
  isSplitResizing.value = true
  const startY = e.clientY
  const startHeight = editorHeight.value
  const doResize = (ev: MouseEvent) => {
    if (isSplitResizing.value) {
      editorHeight.value = Math.max(100, startHeight + (ev.clientY - startY))
      calculateResultHeight()
    }
  }
  const stopResize = () => {
    isSplitResizing.value = false
    document.removeEventListener('mousemove', doResize)
    document.removeEventListener('mouseup', stopResize)
    document.body.style.cursor = ''
  }
  document.body.style.cursor = 'row-resize'
  document.addEventListener('mousemove', doResize)
  document.addEventListener('mouseup', stopResize)
}

function calculateResultHeight() {
  const totalHeight = window.innerHeight - 144
  resultTableHeight.value = totalHeight - editorHeight.value - 100
}

function updateAutocompleteContext() {
  const model = editor?.getModel()
  const connId = props.connectionId || connectionStore.activeConnectionId
  if (model && connId && connectionStore.connections.length > 0) {
    const conn = connectionStore.connections.find(c => c.id === connId)
    autocompleteManager.bindModel(model, {
      connectionId: connId,
      database: selectedDatabase.value || null,
      dbType: conn?.db_type || null
    })
  }
}

async function loadAvailableDatabases() {
  const connId = props.connectionId || connectionStore.activeConnectionId
  if (!connId) return
  loadingDatabases.value = true
  try {
    const dbs = await invoke<any[]>('get_databases', { connectionId: connId })
    availableDatabases.value = dbs
    emit('databasesLoaded', dbs)
  } catch (e) { console.error(e) } finally { loadingDatabases.value = false }
}

function handleDatabaseChange(dbName: string) {
  selectedDatabase.value = dbName
  updateAutocompleteContext()
}

async function executeQuery() { await handlePageChange(1) }

async function handlePageChange(page: number) {
  const connId = props.connectionId || connectionStore.activeConnectionId
  if (!connId || executing.value) return
  const sql = editor?.getValue().trim()
  if (!sql) return message.warning('请输入 SQL')

  executing.value = true
  if (page === 1) { queryResults.value = []; currentResultIndex.value = 0; }
  currentPage.value = page
  resultTabKey.value = 'result'

  try {
    const result = await invoke<QueryResult>('execute_query_paged', {
      connectionId: connId, sql, database: selectedDatabase.value || null, page, pageSize: pageSize.value,
    })
    queryResults.value = [result]
    addMessage('success', `查询成功 (${result.affected_rows} 行)`)
    if (page === 1) saveToHistory(sql)
  } catch (e: any) {
    message.error(`查询失败: ${e}`)
  } finally { executing.value = false }
}

function stopExecution() { executing.value = false; addMessage('info', '已停止') }

async function formatSql() {
  if (!editor) return
  const sql = editor.getValue()
  const connId = props.connectionId || connectionStore.activeConnectionId
  if (!sql.trim() || !connId) return
  try {
    const formatted = await invoke<string>('beautify_sql', { connectionId: connId, sql })
    editor.setValue(formatted)
  } catch (e: any) { message.error(e) }
}

function clearEditor() { editor?.setValue(''); queryResults.value = []; messages.value = []; }
function handleQuerySaved() { message.success('已保存') }

function insertSnippet(sql: string) {
  if (!editor) return
  const selection = editor.getSelection()
  editor.executeEdits('insert-snippet', [{ 
    range: selection || editor.getSelection()!, 
    text: sql 
  }])
  showSnippets.value = false
}

function openHistory() { showHistory.value = true }
function openSnippets() { showSnippets.value = true }

function useHistorySql(sql: string) { editor?.setValue(sql); showHistory.value = false; }

function saveToHistory(sql: string) {
  sqlHistory.value.unshift({ sql, timestamp: Date.now(), database: selectedDatabase.value })
  if (sqlHistory.value.length > 100) sqlHistory.value.pop()
  localStorage.setItem('sql_history', JSON.stringify(sqlHistory.value))
}

async function refreshAutocomplete() {
  const connId = props.connectionId || connectionStore.activeConnectionId
  if (!connId) return
  autocompleteManager.clearCache(connId)
  updateAutocompleteContext()
  message.success('已刷新')
}

async function setSelectedDatabase(database: string) {
  if (availableDatabases.value.length === 0) await loadAvailableDatabases()
  selectedDatabase.value = database
  updateAutocompleteContext()
}

// 7. 生命周期钩子
onMounted(() => {
  if (!editorContainer.value) return
  editor = monaco.editor.create(editorContainer.value, {
    value: props.initialValue || '-- 在此输入 SQL 查询\n',
    language: 'sql',
    theme: appStore.theme === 'dark' ? 'vs-dark' : 'vs',
    automaticLayout: true,
    fontSize: 14,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    lineNumbers: 'on',
    renderLineHighlight: 'all',
    quickSuggestions: { other: true, comments: false, strings: false },
    suggestOnTriggerCharacters: true,
    acceptSuggestionOnCommitCharacter: true,
    acceptSuggestionOnEnter: 'on',
    tabCompletion: 'on',
  })

  updateAutocompleteContext()
  editor.onDidChangeModelContent(() => { emit('contentChange', editor?.getValue() || '') })
  editor.addCommand(monaco.KeyCode.F5, () => executeQuery())
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => showSaveDialog.value = true)

  const history = localStorage.getItem('sql_history')
  if (history) { try { sqlHistory.value = JSON.parse(history) } catch (e) { console.error(e) } }
  
  loadAvailableDatabases()
  calculateResultHeight()
  window.addEventListener('resize', calculateResultHeight)
})

onActivated(() => { nextTick(() => { setTimeout(() => { if (editor) editor.layout() }, 50) }) })

onUnmounted(() => {
  const model = editor?.getModel()
  if (model) autocompleteManager.unbindModel(model)
  editor?.dispose()
  window.removeEventListener('resize', calculateResultHeight)
})

// 8. 监听器
watch(() => appStore.theme, (newTheme) => { if (editor) monaco.editor.setTheme(newTheme === 'dark' ? 'vs-dark' : 'vs') })
watch(() => props.connectionId || connectionStore.activeConnectionId, () => { updateAutocompleteContext(); loadAvailableDatabases(); })

async function handleSave() {
  if (!editor) return
  const content = editor.getValue()
  if (!content.trim()) return message.warning('内容为空')

  let targetPath = props.filePath
  
  if (!targetPath) {
    // 获取当前连接和数据库对应的脚本目录
    const connId = props.connectionId || connectionStore.activeConnectionId
    if (!connId || !selectedDatabase.value) {
      // 如果没有绑定库，弹出普通另存为对话框
      const path = await save({
        filters: [{ name: 'SQL', extensions: ['sql'] }],
        defaultPath: `script-${new Date().getTime()}.sql`
      })
      if (path) targetPath = path
    } else {
      try {
        const dir = await invoke<string>('get_db_scripts_dir', {
          connectionId: connId,
          database: selectedDatabase.value
        })
        const fileName = `script-${new Date().toISOString().replace(/[:.]/g, '-')}.sql`
        targetPath = `${dir}/${fileName}`
      } catch (e) {
        console.error('获取目录失败:', e)
      }
    }
  }

  if (targetPath) {
    try {
      await writeTextFile(targetPath, content)
      const fileName = targetPath.split(/[/\\]/).pop() || 'script.sql'
      emit('fileSaved', targetPath, fileName)
      addMessage('success', `文件已保存: ${targetPath}`)
      message.success('已保存')
    } catch (err: any) {
      message.error(`保存失败: ${err}`)
    }
  }
}

defineExpose({ setSelectedDatabase, executing, executeQuery, handleDatabaseChange, formatSql, clearEditor, openHistory, openSnippets, refreshAutocomplete, handleSave })
</script>

<style scoped>
.sql-editor-container { display: flex; flex-direction: column; height: 100%; overflow: hidden; background: #fff; }
.dark-mode .sql-editor-container { background: #1f1f1f; }
.editor-section { flex-shrink: 0; min-height: 100px; overflow: hidden; }
.monaco-container { height: 100%; width: 100%; }
.split-resizer { height: 4px; background: #f0f0f0; cursor: row-resize; display: flex; align-items: center; justify-content: center; transition: background 0.2s; flex-shrink: 0; }
.split-resizer:hover { background: #1890ff; }
.dark-mode .split-resizer { background: #303030; }
.resizer-handle { width: 30px; height: 2px; background: #d9d9d9; border-radius: 1px; }
.result-section { flex: 1; min-height: 100px; display: flex; flex-direction: column; overflow: hidden; }
.result-tabs { height: 100%; display: flex; flex-direction: column; }
.result-tabs :deep(.ant-tabs-content) { flex: 1; overflow: hidden; }
.result-tabs :deep(.ant-tabs-tabpane) { height: 100%; display: flex; flex-direction: column; }
.result-content { flex: 1; display: flex; flex-direction: column; padding: 12px; overflow: hidden; position: relative; }
.executing-overlay { position: absolute; top: 0; left: 0; right: 0; bottom: 0; background: rgba(255, 255, 255, 0.7); display: flex; flex-direction: column; align-items: center; justify-content: center; z-index: 10; }
.dark-mode .executing-overlay { background: rgba(0, 0, 0, 0.6); }
.result-info { margin-bottom: 12px; flex-shrink: 0; display: flex; align-items: center; }
.page-indicator { font-size: 13px; color: #595959; font-weight: 500; margin: 0 8px; }
.dark-mode .page-indicator { color: #d9d9d9; }
.table-wrapper { flex: 1; overflow: hidden; }
.messages-content { flex: 1; padding: 12px; overflow-y: auto; font-family: monospace; }
.message-item { margin-bottom: 8px; padding: 4px 8px; border-left: 3px solid #d9d9d9; background: #f5f5f5; white-space: pre-wrap; word-break: break-all; }
.dark-mode .message-item { background: #262626; border-left-color: #434343; }
.message-item.success { border-left-color: #52c41a; }
.message-item.error { border-left-color: #ff4d4f; color: #ff4d4f; }
.message-time { color: #8c8c8c; margin-right: 8px; }
.history-item { cursor: pointer; transition: background 0.2s; }
.history-item:hover { background: #f5f5f5; }
.dark-mode .history-item:hover { background: #262626; }
.dark-mode .result-content :deep(.ant-table) { background: #1f1f1f !important; color: rgba(255, 255, 255, 0.65); }
.dark-mode .result-content :deep(.ant-table-thead), .dark-mode .result-content :deep(.ant-table-header) { background-color: #1d1d1d !important; }
.dark-mode .result-content :deep(.ant-table-thead > tr > th) { background: #1d1d1d !important; color: rgba(255, 255, 255, 0.85) !important; border-bottom: 1px solid #303030 !important; }
.dark-mode .result-content :deep(.ant-table-cell) { border-bottom-color: #303030 !important; border-right-color: #303030 !important; }
.dark-mode .result-content :deep(.ant-table-tbody > tr:hover > td) { background: #262626 !important; }
.dark-mode .result-info :deep(.ant-btn) { background-color: #262626; border-color: #434343; color: rgba(255, 255, 255, 0.65); }
.dark-mode .result-info :deep(.ant-select-selector) { background-color: #262626 !important; border-color: #434343 !important; color: rgba(255, 255, 255, 0.65) !important; }
</style>
