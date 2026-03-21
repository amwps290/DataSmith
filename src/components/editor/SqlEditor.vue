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
        <!-- 动态渲染多个结果页签 -->
        <a-tab-pane v-for="(result, index) in queryResults" :key="'result-' + index" :tab="queryResults.length > 1 ? `结果 ${index + 1}` : '查询结果'">
          <div class="result-content">
            <div v-if="executing" class="executing-overlay">
              <a-spin tip="正在执行查询..." />
              <a-button danger size="small" @click="stopExecution" style="margin-top: 16px">停止执行</a-button>
            </div>
            
            <div class="result-info">
              <a-space>
                <a-tag color="success">已加载 {{ result.rows.length }} 行</a-tag>
                <a-tag color="processing">{{ result.execution_time_ms }} ms</a-tag>
                <a-divider type="vertical" />
                <span class="affected-text" v-if="result.affected_rows > 0">受影响行数: {{ result.affected_rows }}</span>
              </a-space>
            </div>
            
            <div class="table-wrapper">
              <vxe-grid
                :ref="(el: any) => setGridRef(el, index)"
                v-bind="getGridOptions(result)"
              >
                <template #cell_default="{ row, column }">
                  <span :class="{ 'null-text': row[column.field] === null }">
                    {{ row[column.field] === null ? 'NULL' : row[column.field] }}
                  </span>
                </template>
              </vxe-grid>
            </div>
          </div>
        </a-tab-pane>

        <!-- 默认空状态页签 (如果没有结果) -->
        <a-tab-pane v-if="queryResults.length === 0" key="empty" tab="查询结果">
          <div class="result-content">
            <div v-if="executing" class="executing-overlay">
              <a-spin tip="正在执行查询..." />
            </div>
            <a-empty description="暂无查询结果" />
          </div>
        </a-tab-pane>

        <!-- 消息页签 -->
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

    <!-- 历史记录、保存对话框等保持不变 -->
    <a-drawer title="查询历史" placement="right" v-model:open="showHistory" width="400">
      <a-list :data-source="sqlHistory" size="small">
        <template #renderItem="{ item }">
          <a-list-item class="history-item" @click="useHistorySql(item.sql)">
            <a-list-item-meta>
              <template #title>
                <code class="history-sql">{{ item.sql.substring(0, 100) }}{{ item.sql.length > 100 ? '...' : '' }}</code>
              </template>
              <template #description>
                {{ new Date(item.timestamp).toLocaleString() }} • {{ item.database || '默认' }}
              </template>
            </a-list-item-meta>
          </a-list-item>
        </template>
      </a-list>
    </a-drawer>
    <SaveQueryDialog v-model="showSaveDialog" :sql="editor?.getValue() || ''" @saved="handleQuerySaved" />
    <SqlSnippetsManager v-model:visible="showSnippets" @insert="insertSnippet" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, watch, ref, computed, onActivated, nextTick, reactive } from 'vue'
import * as monaco from 'monaco-editor'
import { getSqlAutocompleteManager } from '@/services/sqlAutocomplete'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import type { QueryResult } from '@/types/database'
import { useConnectionStore } from '@/stores/connection'
import { useAppStore } from '@/stores/app'
import SaveQueryDialog from './SaveQueryDialog.vue'
import SqlSnippetsManager from './SqlSnippetsManager.vue'
import type { VxeGridProps } from 'vxe-table'

const props = defineProps<{ connectionId?: string; initialDatabase?: string; initialValue?: string; filePath?: string; tabId?: string; }>()
const emit = defineEmits(['contentChange', 'fileSaved', 'databasesLoaded'])
const connectionStore = useConnectionStore()
const appStore = useAppStore()

const internalSessionId = ref(props.tabId || props.filePath || `editor-${Math.random().toString(36).substring(2, 9)}`)
const sessionConnectionId = computed(() => {
  const baseId = props.connectionId || connectionStore.activeConnectionId
  if (!baseId) return ''
  const sid = internalSessionId.value.replace(/[^a-zA-Z0-9]/g, '_')
  return `${baseId}:tab_${sid}`
})

const editorContainer = ref<HTMLElement>()
let editor: monaco.editor.IStandaloneCodeEditor | null = null
const autocompleteManager = getSqlAutocompleteManager()

const editorHeight = ref(350) 
const isSplitResizing = ref(false)
const availableDatabases = ref<any[]>([])
const selectedDatabase = ref(props.initialDatabase || '')
const executing = ref(false)
const queryResults = ref<QueryResult[]>([])
const resultTabKey = ref('result-0')
const messages = ref<any[]>([])
const showHistory = ref(false)
const sqlHistory = ref<any[]>([])
const showSaveDialog = ref(false)
const showSnippets = ref(false)

const gridRefs = reactive<Record<number, any>>({})
function setGridRef(el: any, index: number) { if (el) gridRefs[index] = el; else delete gridRefs[index]; }

function getGridOptions(result: QueryResult): VxeGridProps {
  return {
    border: true,
    height: 'auto',
    loading: false,
    columnConfig: { resizable: true },
    rowConfig: { isHover: true, isCurrent: true, height: 36 },
    scrollX: { enabled: true, gt: 20 },
    scrollY: { enabled: true, gt: 0 },
    columns: result.columns.map(col => ({ field: col, title: col, minWidth: 150, showOverflow: true, slots: { default: 'cell_default' } })),
    data: result.rows
  }
}

function addMessage(type: string, text: string) { messages.value.unshift({ type, text, time: new Date().toLocaleTimeString() }) }

async function executeQuery() {
  const connId = sessionConnectionId.value
  if (!connId) return
  
  const selection = editor?.getSelection()
  const model = editor?.getModel()
  let sql = editor?.getValue().trim() || ''
  let isSelection = false

  if (selection && model && !selection.isEmpty()) {
    const selectedText = model.getValueInRange(selection).trim()
    if (selectedText) { sql = selectedText; isSelection = true; }
  }

  if (!sql) return message.warning('请输入 SQL')

  executing.value = true
  queryResults.value = []
  if (isSelection) addMessage('info', '执行选中部分 SQL...')

  try {
    const results = await invoke<QueryResult[]>('execute_query', {
      connectionId: connId,
      sql,
      database: selectedDatabase.value || null,
    })
    
    queryResults.value = results
    if (results.length > 0) {
      resultTabKey.value = 'result-0'
      const totalAffected = results.reduce((acc, r) => acc + r.affected_rows, 0)
      addMessage('success', `执行完成，返回 ${results.length} 个结果集，总计影响 ${totalAffected} 行`)
    } else {
      resultTabKey.value = 'messages'
    }
    saveToHistory(sql)
  } catch (e: any) {
    message.error(`查询失败: ${e}`)
    addMessage('error', String(e))
    resultTabKey.value = 'messages'
  } finally { executing.value = false }
}

async function explainQuery() {
  const connId = sessionConnectionId.value
  if (!connId || !editor) return
  const sql = editor.getValue().trim()
  if (!sql) return message.warning('请输入 SQL')

  executing.value = true
  try {
    const results = await invoke<QueryResult[]>('explain_query', { connectionId: connId, sql, database: selectedDatabase.value || null })
    queryResults.value = results
    resultTabKey.value = 'result-0'
    addMessage('success', '执行计划分析完成')
  } catch (e: any) {
    message.error(String(e))
    addMessage('error', String(e))
  } finally { executing.value = false }
}

function stopExecution() { executing.value = false; addMessage('info', '已手动停止') }
async function formatSql() { if (!editor) return; try { const formatted = await invoke<string>('beautify_sql', { connectionId: sessionConnectionId.value, sql: editor.getValue() }); editor.setValue(formatted); message.success('格式化完成') } catch (e: any) { message.error(e) } }
function clearEditor() { editor?.setValue(''); queryResults.value = []; messages.value = []; }
function handleQuerySaved() { message.success('已保存') }
function insertSnippet(sql: string) { if (!editor) return; const selection = editor.getSelection(); editor.executeEdits('insert-snippet', [{ range: selection || editor.getSelection()!, text: sql }]); showSnippets.value = false }
function openHistory() { showHistory.value = true }
function openSnippets() { showSnippets.value = true }
function useHistorySql(sql: string) { editor?.setValue(sql); showHistory.value = false; }
function saveToHistory(sql: string) { sqlHistory.value.unshift({ sql, timestamp: Date.now(), database: selectedDatabase.value }); if (sqlHistory.value.length > 100) sqlHistory.value.pop(); localStorage.setItem('sql_history', JSON.stringify(sqlHistory.value)) }
async function refreshAutocomplete() { const baseId = props.connectionId || connectionStore.activeConnectionId; if (!baseId) return; autocompleteManager.clearCache(baseId); updateAutocompleteContext(); message.success('已刷新提示缓存') }
async function setSelectedDatabase(database: string) { if (availableDatabases.value.length === 0) await loadAvailableDatabases(); selectedDatabase.value = database; updateAutocompleteContext() }
async function handleSave(isAuto = false) { if (!editor || !props.filePath) return; const content = editor.getValue(); if (!content.trim()) return; try { await invoke('write_file', { path: props.filePath, content: content }); if (!isAuto) message.success('已保存') } catch (err: any) { if (!isAuto) message.error(`保存失败: ${err}`) } }
function startResize(e: MouseEvent) { isSplitResizing.value = true; const startY = e.clientY, startHeight = editorHeight.value; const doResize = (ev: MouseEvent) => { if (isSplitResizing.value) { editorHeight.value = Math.max(100, startHeight + (ev.clientY - startY)); } }; const stopResize = () => { isSplitResizing.value = false; document.removeEventListener('mousemove', doResize); document.removeEventListener('mouseup', stopResize); document.body.style.cursor = '' }; document.body.style.cursor = 'row-resize'; document.addEventListener('mousemove', doResize); document.addEventListener('mouseup', stopResize) }
function updateAutocompleteContext() { const model = editor?.getModel(), baseId = props.connectionId || connectionStore.activeConnectionId; if (model && baseId && connectionStore.connections.length > 0) { const conn = connectionStore.connections.find(c => c.id === baseId); autocompleteManager.bindModel(model, { connectionId: baseId, database: selectedDatabase.value || null, dbType: conn?.db_type || null }) } }
async function loadAvailableDatabases() { const baseId = props.connectionId || connectionStore.activeConnectionId; if (!baseId) return; try { const dbs = await invoke<any[]>('get_databases', { connectionId: baseId }); availableDatabases.value = dbs; emit('databasesLoaded', dbs) } catch (e) { console.error(e) } }
function handleDatabaseChange(dbName: string) { selectedDatabase.value = dbName; updateAutocompleteContext() }

onMounted(() => {
  if (!editorContainer.value) return
  editor = monaco.editor.create(editorContainer.value, { value: props.initialValue || '-- 在此输入 SQL 查询\n', language: 'sql', theme: appStore.theme === 'dark' ? 'vs-dark' : 'vs', automaticLayout: true, fontSize: 14, minimap: { enabled: false }, scrollBeyondLastLine: false, lineNumbers: 'on', renderLineHighlight: 'all', quickSuggestions: { other: true, comments: false, strings: false }, suggestOnTriggerCharacters: true, acceptSuggestionOnCommitCharacter: true, acceptSuggestionOnEnter: 'on', tabCompletion: 'on' })
  updateAutocompleteContext(); editor.onDidChangeModelContent(() => { emit('contentChange', editor?.getValue() || ''); triggerAutoSave() }); editor.addCommand(monaco.KeyCode.F5, () => executeQuery()); editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => handleSave());
  const history = localStorage.getItem('sql_history'); if (history) { try { sqlHistory.value = JSON.parse(history) } catch (e) { console.error(e) } }
  loadAvailableDatabases();
})
let autoSaveTimer: any = null; function triggerAutoSave() { if (autoSaveTimer) clearTimeout(autoSaveTimer); autoSaveTimer = setTimeout(() => { handleSave(true) }, 2000) }
onActivated(() => { nextTick(() => { if (editor) editor.layout() }) })
onUnmounted(() => { const model = editor?.getModel(); if (model) autocompleteManager.unbindModel(model); editor?.dispose(); })
watch(() => appStore.theme, (newTheme) => { if (editor) monaco.editor.setTheme(newTheme === 'dark' ? 'vs-dark' : 'vs') }, { immediate: true })
watch(() => props.connectionId || connectionStore.activeConnectionId, () => { updateAutocompleteContext(); loadAvailableDatabases(); })
defineExpose({ setSelectedDatabase, executing, executeQuery, explainQuery, handleDatabaseChange, formatSql, clearEditor, openHistory, openSnippets, refreshAutocomplete, handleSave })
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
.executing-overlay { position: absolute; top: 0; left: 0; right: 0; bottom: 0; background: rgba(255, 255, 255, 0.7); display: flex; align-items: center; justify-content: center; z-index: 10; }
.dark-mode .executing-overlay { background: rgba(0, 0, 0, 0.6); }
.result-info { margin-bottom: 8px; flex-shrink: 0; }
.affected-text { font-size: 12px; color: #8c8c8c; }
.table-wrapper { flex: 1; min-height: 0; overflow: hidden; }
.messages-content { flex: 1; padding: 12px; overflow-y: auto; font-family: monospace; }
.message-item { margin-bottom: 8px; padding: 4px 8px; border-left: 3px solid #d9d9d9; background: #f5f5f5; white-space: pre-wrap; word-break: break-all; }
.dark-mode .message-item { background: #262626; border-left-color: #434343; }
.message-item.success { border-left-color: #52c41a; }
.message-item.error { border-left-color: #ff4d4f; color: #ff4d4f; }
.message-time { color: #8c8c8c; margin-right: 8px; }
.history-item { cursor: pointer; transition: background 0.2s; }
.history-item:hover { background: #f5f5f5; }
.dark-mode .history-item:hover { background: #262626; }
.history-sql { font-family: monospace; background: transparent; padding: 0; }
.null-text { color: #bfbfbf; font-style: italic; }
</style>
