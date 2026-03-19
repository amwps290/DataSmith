<template>
  <div class="sql-editor-container">
    <!-- 上部：编辑器区域 -->
    <div 
      class="editor-section" 
      :style="{ height: editorHeight + 'px' }"
    >
      <div ref="editorContainer" class="editor-wrapper"></div>
    </div>

    <!-- 分割条 -->
    <div class="split-resizer" @mousedown="startSplitResize">
      <div class="resizer-handle"></div>
    </div>

    <!-- 下部：结果展示区域 -->
    <div class="result-section">
      <div class="result-tabs">
        <a-tabs v-model:activeKey="resultTabKey">
          <a-tab-pane key="result" tab="结果">
            <div class="result-content">
              <div v-if="queryResults.length > 0" class="result-info">
                <a-space>
                  <a-tag color="success">
                    {{ queryResults[currentResultIndex]?.affected_rows || 0 }} 行
                  </a-tag>
                  <a-tag color="processing">
                    {{ queryResults[currentResultIndex]?.execution_time_ms || 0 }} ms
                  </a-tag>
                  
                  <!-- 分页控制 -->
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
                    <a-select-option :value="100">100 条/页</a-select-option>
                    <a-select-option :value="200">200 条/页</a-select-option>
                    <a-select-option :value="500">500 条/页</a-select-option>
                    <a-select-option :value="1000">1000 条/页</a-select-option>
                  </a-select>

                  <a-dropdown v-if="queryResults.length > 1">
                    <a-button size="small">
                      结果集 {{ currentResultIndex + 1 }}/{{ queryResults.length }}
                      <DownOutlined />
                    </a-button>
                    <template #overlay>
                      <a-menu @click="switchResult">
                        <a-menu-item
                          v-for="(result, index) in queryResults"
                          :key="index"
                        >
                          结果集 {{ index + 1 }} ({{ result.affected_rows }} 行)
                        </a-menu-item>
                      </a-menu>
                    </template>
                  </a-dropdown>
                </a-space>
              </div>
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
          </a-tab-pane>
          <a-tab-pane key="messages" tab="消息">
            <div class="messages-content">
              <a-timeline>
                <a-timeline-item
                  v-for="(msg, index) in messages"
                  :key="index"
                  :color="msg.type === 'success' ? 'green' : msg.type === 'error' ? 'red' : 'blue'"
                >
                  <span class="message-time">{{ msg.time }}</span>
                  <span class="message-text">{{ msg.text }}</span>
                </a-timeline-item>
              </a-timeline>
            </div>
          </a-tab-pane>
        </a-tabs>
      </div>
    </div>

    <!-- 对话框部分保持在内部，由外部触发显示 -->
    <a-modal
      v-model:open="showHistory"
      title="SQL 执行历史"
      :width="800"
      :footer="null"
    >
      <a-list :data-source="sqlHistory" size="small">
        <template #renderItem="{ item }">
          <a-list-item>
            <template #actions>
              <a @click="loadFromHistory(item)">加载</a>
              <a @click="removeFromHistory(item)">删除</a>
            </template>
            <a-list-item-meta>
              <template #title>
                <code>{{ item.sql.substring(0, 100) }}...</code>
              </template>
              <template #description>
                {{ new Date(item.timestamp).toLocaleString() }} • 
                {{ item.database || '默认' }}
              </template>
            </a-list-item-meta>
          </a-list-item>
        </template>
      </a-list>
    </a-modal>

    <SaveQueryDialog
      v-model="showSaveDialog"
      :sql="editor?.getValue() || ''"
      @saved="handleQuerySaved"
    />

    <SqlSnippetsManager
      v-model:visible="showSnippets"
      @insert-snippet="insertSnippet"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, watch, ref, computed } from 'vue'
import * as monaco from 'monaco-editor'
import { getSqlAutocompleteManager } from '@/services/sqlAutocomplete'
import { save, open } from '@tauri-apps/plugin-dialog'
import { writeTextFile, readTextFile } from '@tauri-apps/plugin-fs'

// Monaco 环境配置
(window as any).MonacoEnvironment = {
  getWorker: () => {
    return new Worker(
      URL.createObjectURL(new Blob([''], { type: 'application/javascript' }))
    )
  }
}

import {
  DownOutlined,
} from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import { useConnectionStore } from '@/stores/connection'
import { useAppStore } from '@/stores/app'
import type { QueryResult } from '@/types/database'
import SaveQueryDialog from './SaveQueryDialog.vue'
import SqlSnippetsManager from './SqlSnippetsManager.vue'

const props = defineProps<{
  initialValue?: string
  initialDatabase?: string
  connectionId?: string
  filePath?: string
}>()

const emit = defineEmits<{
  contentChange: [value: string]
  fileSaved: [path: string, fileName: string]
  databasesLoaded: [databases: any[]]
}>()

const connectionStore = useConnectionStore()
const appStore = useAppStore()

const editorContainer = ref<HTMLElement>()
let editor: monaco.editor.IStandaloneCodeEditor | null = null

// 获取补全管理器单例
const autocompleteManager = getSqlAutocompleteManager()

// 布局相关
const editorHeight = ref(300) 
const isSplitResizing = ref(false)
const resultTableHeight = ref(300) 

const executing = ref(false)
const queryResults = ref<QueryResult[]>([])
const currentResultIndex = ref(0)
const resultTabKey = ref('result')
const showHistory = ref(false)
const showSaveDialog = ref(false)
const showSnippets = ref(false)

const currentPage = ref(1)
const pageSize = ref(200)

const selectedDatabase = ref(props.initialDatabase || '')
const availableDatabases = ref<any[]>([])
const loadingDatabases = ref(false)

interface Message {
  type: 'success' | 'error' | 'info'
  text: string
  time: string
}
const messages = ref<Message[]>([])

interface SqlHistoryItem {
  sql: string
  timestamp: number
  database?: string
}
const sqlHistory = ref<SqlHistoryItem[]>([])

const currentResult = computed(() => queryResults.value[currentResultIndex.value] || null)

const resultColumns = computed(() => {
  if (!currentResult.value) return []
  return currentResult.value.columns.map((col) => ({
    title: col,
    dataIndex: col,
    key: col,
    ellipsis: true,
    width: 150,
  }))
})

// 动态调整表格高度
function calculateResultHeight() {
  const totalContainerHeight = window.innerHeight - 64 - 40 - 56 - editorHeight.value - 10
  resultTableHeight.value = Math.max(totalContainerHeight - 120, 200)
}

// 分割条拖拽逻辑
function startSplitResize(e: MouseEvent) {
  isSplitResizing.value = true
  const startY = e.clientY
  const startHeight = editorHeight.value

  const doResize = (e: MouseEvent) => {
    if (!isSplitResizing.value) return
    const delta = e.clientY - startY
    const newHeight = startHeight + delta
    if (newHeight >= 100 && newHeight <= window.innerHeight - 300) {
      editorHeight.value = newHeight
      calculateResultHeight()
      editor?.layout() 
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

// 更新补全上下文
function updateAutocompleteContext() {
  const model = editor?.getModel()
  const connId = props.connectionId || connectionStore.activeConnectionId
  if (model && connId) {
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
  if (!connId) return (availableDatabases.value = [])
  loadingDatabases.value = true
  try {
    const databases = await invoke<any[]>('get_databases', { connectionId: connId })
    availableDatabases.value = databases
    emit('databasesLoaded', databases)
  } catch (error: any) {
    console.error('加载数据库列表失败:', error)
    availableDatabases.value = []
  } finally {
    loadingDatabases.value = false
  }
}

function handleDatabaseChange(dbName: string) {
  selectedDatabase.value = dbName
  updateAutocompleteContext()
}

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

  // 绑定模型到补全管理器
  updateAutocompleteContext()

  editor.onDidChangeModelContent(() => {
    emit('contentChange', editor?.getValue() || '')
  })

  editor.addCommand(monaco.KeyCode.F5, () => executeQuery())
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => handleSaveMenuClick('save'))

  loadHistory()
  loadAvailableDatabases()
  calculateResultHeight()
  window.addEventListener('resize', calculateResultHeight)
})

onUnmounted(() => {
  const model = editor?.getModel()
  if (model) {
    autocompleteManager.unbindModel(model)
  }
  editor?.dispose()
  window.removeEventListener('resize', calculateResultHeight)
})

watch(() => appStore.theme, (newTheme) => {
  if (editor) monaco.editor.setTheme(newTheme === 'dark' ? 'vs-dark' : 'vs')
})

watch(() => props.connectionId || connectionStore.activeConnectionId, (newId) => {
  if (newId) {
    updateAutocompleteContext()
    loadAvailableDatabases()
  }
})

async function handleSaveMenuClick(key: string) {
  const content = editor?.getValue() || ''
  if (!content.trim()) return message.warning('内容为空')

  if (key === 'save') {
    if (props.filePath) {
      try {
        await writeTextFile(props.filePath, content)
        message.success('已保存到文件')
      } catch (err: any) { message.error(`保存失败: ${err}`) }
    } else {
      await handleSaveAs()
    }
  } else if (key === 'saveAs') {
    await handleSaveAs()
  } else if (key === 'saveToDb') {
    showSaveDialog.value = true
  }
}

async function handleSaveAs() {
  try {
    const path = await save({
      filters: [{ name: 'SQL Script', extensions: ['sql'] }],
      defaultPath: 'query.sql'
    })
    if (path) {
      const content = editor?.getValue() || ''
      await writeTextFile(path, content)
      const parts = path.split(/[\\/]/)
      const name = parts[parts.length - 1]
      emit('fileSaved', path, name)
      message.success('文件另存成功')
    }
  } catch (err: any) { message.error(`另存失败: ${err}`) }
}

async function handleOpenFile() {
  try {
    const path = await open({
      filters: [{ name: 'SQL Script', extensions: ['sql'] }],
      multiple: false
    })
    if (path && typeof path === 'string') {
      const content = await readTextFile(path)
      editor?.setValue(content)
      const parts = path.split(/[\\/]/)
      const name = parts[parts.length - 1]
      emit('fileSaved', path, name)
      message.success('文件已打开')
    }
  } catch (err: any) { message.error(`读取失败: ${err}`) }
}

async function executeQuery() {
  await handlePageChange(1)
}

async function handlePageChange(page: number) {
  const connId = props.connectionId || connectionStore.activeConnectionId
  if (!connId) return message.warning('请先选择一个数据库连接')
  const sql = editor?.getValue().trim()
  if (!sql) return message.warning('请输入 SQL 语句')

  executing.value = true
  // 如果是第一页，清空之前的结果
  if (page === 1) {
    queryResults.value = []
    currentResultIndex.value = 0
  }
  currentPage.value = page
  resultTabKey.value = 'result'

  addMessage('info', `执行查询 (第 ${page} 页)...${selectedDatabase.value ? ' (数据库: ' + selectedDatabase.value + ')' : ''}`)

  try {
    const result = await invoke<QueryResult>('execute_query_paged', {
      connectionId: connId,
      sql,
      database: selectedDatabase.value || null,
      page,
      pageSize: pageSize.value,
    })
    
    // 目前只处理单结果集的分页
    queryResults.value = [result]
    addMessage('success', `第 ${page} 页查询成功！影响 ${result.affected_rows} 行，耗时 ${result.execution_time_ms} ms`)
    if (page === 1) saveToHistory(sql)
  } catch (error: any) {
    if (page === 1) queryResults.value = []
    addMessage('error', `查询失败: ${error}`)
    message.error(`查询失败: ${error}`)
  } finally {
    executing.value = false
  }
}

function stopExecution() {
  executing.value = false
  addMessage('info', '已停止执行')
}

async function formatSql() {
  if (!editor) return
  const sql = editor.getValue()
  if (!sql.trim()) return

  const connId = props.connectionId || connectionStore.activeConnectionId
  if (!connId) return message.warning('请先选择一个数据库连接')

  try {
    const formatted = await invoke<string>('beautify_sql', {
      connectionId: connId,
      sql,
    })
    editor.setValue(formatted)
    message.success('SQL 已格式化')
  } catch (error: any) {
    message.error(`格式化失败: ${error}`)
  }
}

function clearEditor() {
  editor?.setValue('')
  queryResults.value = []
  messages.value = []
}

function handleQuerySaved() { message.success('查询已保存') }

function insertSnippet(sql: string) {
  if (!editor) return
  const selection = editor.getSelection()
  if (selection) {
    editor.executeEdits('insert-snippet', [{ range: selection, text: sql }])
  } else {
    const position = editor.getPosition()
    if (position) {
      editor.executeEdits('insert-snippet', [{
        range: new monaco.Range(position.lineNumber, position.column, position.lineNumber, position.column),
        text: sql,
      }])
    }
  }
  editor.focus()
}

function switchResult({ key }: { key: string | number }) {
  currentResultIndex.value = typeof key === 'number' ? key : parseInt(String(key))
}

function addMessage(type: Message['type'], text: string) {
  messages.value.unshift({ type, text, time: new Date().toLocaleTimeString() })
}

function saveToHistory(sql: string) {
  sqlHistory.value.unshift({ sql, timestamp: Date.now(), database: selectedDatabase.value })
  if (sqlHistory.value.length > 100) sqlHistory.value = sqlHistory.value.slice(0, 100)
  localStorage.setItem('sql_history', JSON.stringify(sqlHistory.value))
}

function loadHistory() {
  const stored = localStorage.getItem('sql_history')
  if (stored) {
    try { sqlHistory.value = JSON.parse(stored) } catch (e) { console.error('加载历史记录失败', e) }
  }
}

function loadFromHistory(item: SqlHistoryItem) {
  editor?.setValue(item.sql)
  showHistory.value = false
  message.success('已加载历史记录')
}

function removeFromHistory(item: SqlHistoryItem) {
  sqlHistory.value = sqlHistory.value.filter((h) => h.timestamp !== item.timestamp)
  localStorage.setItem('sql_history', JSON.stringify(sqlHistory.value))
}

async function refreshAutocomplete() {
  const connId = props.connectionId || connectionStore.activeConnectionId
  if (!completionProvider || !connId) return message.warning('请先连接到数据库')
  try {
    await completionProvider.refresh()
    message.success('自动补全数据已刷新')
  } catch (error: any) { message.error(`刷新失败: ${error}`) }
}

async function setSelectedDatabase(database: string) {
  if (availableDatabases.value.length === 0) await loadAvailableDatabases()
  selectedDatabase.value = database
  if (completionProvider) completionProvider.setCurrentDatabase(database || null)
}

// 暴露所有控制方法给父组件
defineExpose({ 
  setSelectedDatabase,
  handleDatabaseChange,
  executeQuery,
  stopExecution,
  formatSql,
  clearEditor,
  openHistory: () => { showHistory.value = true },
  handleSave: (key: string) => handleSaveMenuClick(key),
  handleOpen: handleOpenFile,
  openSnippets: () => { showSnippets.value = true },
  refreshAutocomplete,
  executing
})
</script>

<style scoped>
.sql-editor-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.editor-section {
  flex-shrink: 0;
  min-height: 100px;
  overflow: hidden;
}

.editor-wrapper {
  height: 100%;
}

.split-resizer {
  height: 6px;
  cursor: row-resize;
  background: #f0f0f0;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.2s;
  z-index: 10;
}

.split-resizer:hover {
  background: #1890ff;
}

.resizer-handle {
  width: 30px;
  height: 3px;
  background: #ccc;
  border-radius: 2px;
}

.dark-mode .split-resizer {
  background: #262626;
}

.dark-mode .resizer-handle {
  background: #444;
}

.result-section {
  flex: 1;
  min-height: 150px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.result-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.result-tabs :deep(.ant-tabs) {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.result-tabs :deep(.ant-tabs-nav) {
  margin-bottom: 0;
  padding-left: 12px;
  flex-shrink: 0;
}

.result-tabs :deep(.ant-tabs-content) {
  flex: 1;
  overflow: hidden;
}

.result-tabs :deep(.ant-tabs-tabpane) {
  height: 100%;
  overflow: hidden;
}

.result-content,
.messages-content {
  height: 100%;
  padding: 12px;
  display: flex;
  flex-direction: column;
}

.result-content :deep(.ant-table-wrapper) {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.result-content :deep(.ant-spin-nested-loading) {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.result-content :deep(.ant-spin-container) {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.result-content :deep(.ant-table) {
  flex: 1;
  overflow: hidden;
}

.result-content :deep(.ant-table-pagination.ant-pagination) {
  margin: 16px 0 0 0;
  padding-top: 12px;
  border-top: 1px solid #f0f0f0;
  flex-shrink: 0;
}

.dark-mode .result-content :deep(.ant-table-pagination.ant-pagination) {
  border-top-color: #303030;
}

.result-info {
  margin-bottom: 12px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
}

.page-indicator {
  font-size: 13px;
  color: #595959;
  font-weight: 500;
  margin: 0 8px;
}

.dark-mode .page-indicator {
  color: #d9d9d9;
}

.message-time {
  color: #8c8c8c;
  margin-right: 8px;
}

.message-text {
  font-family: monospace;
}
</style>
