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
        <a-tab-pane v-for="(result, index) in queryResults" :key="'result-' + index" :tab="queryResults.length > 1 ? $t('editor.result_n', { n: index + 1 }) : $t('editor.result')">
          <div class="result-content">
            <div v-if="executing" class="executing-overlay">
              <a-spin :tip="$t('editor.executing')" />
              <a-button danger size="small" @click="stopExecution" style="margin-top: 16px">{{ $t('editor.stop_exec') }}</a-button>
            </div>
            
            <div class="result-info">
              <a-space>
                <a-tag color="success">{{ $t('editor.loaded_rows', { n: result.rows.length }) }}</a-tag>
                <a-tag color="processing">{{ result.execution_time_ms }} ms</a-tag>
                <a-divider type="vertical" />
                <span class="affected-text" v-if="result.affected_rows > 0">{{ $t('editor.affected_rows', { n: result.affected_rows }) }}</span>
              </a-space>
            </div>
            
            <div class="table-wrapper">
              <vxe-grid
                :ref="(el: any) => setGridRef(el, index)"
                v-bind="getGridOptions(result, index)"
                @scroll="(params: any) => handleScroll({ ...params, index })"
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
        <a-tab-pane v-if="queryResults.length === 0" key="empty" :tab="$t('editor.result')">
          <div class="result-content">
            <div v-if="executing" class="executing-overlay">
              <a-spin :tip="$t('editor.executing')" />
            </div>
            <a-empty :description="$t('editor.no_result')" />
          </div>
        </a-tab-pane>

        <!-- 消息页签 -->
        <a-tab-pane key="messages" :tab="$t('editor.messages')">
          <div class="messages-content">
            <div v-for="(msg, index) in messages" :key="index" :class="['message-item', msg.type]">
              <span class="message-time">{{ msg.time }}</span>
              <span class="message-text">{{ msg.text }}</span>
            </div>
            <a-empty v-if="messages.length === 0" :description="$t('editor.no_result')" />
          </div>
        </a-tab-pane>
      </a-tabs>
    </div>

    <!-- 历史记录 -->
    <a-drawer :title="$t('editor.history_title')" placement="right" v-model:open="showHistory" width="400">
      <a-list :data-source="sqlHistory" size="small">
        <template #renderItem="{ item }">
          <a-list-item class="history-item" @click="useHistorySql(item.sql)">
            <a-list-item-meta>
              <template #title>
                <code class="history-sql">{{ item.sql.substring(0, 100) }}{{ item.sql.length > 100 ? '...' : '' }}</code>
              </template>
              <template #description>
                {{ new Date(item.timestamp).toLocaleString() }} • {{ item.database || ($t('common.ok') === 'OK' ? 'Default' : '默认') }}
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
import { useI18n } from 'vue-i18n'
import * as monaco from 'monaco-editor'
import { getSqlAutocompleteManager } from '@/services/sqlAutocomplete'
import { message } from 'ant-design-vue'
import { queryApi, metadataApi, utilsApi } from '@/api'
import type { QueryResult } from '@/types/database'
import { useConnectionStore } from '@/stores/connection'
import { useAppStore } from '@/stores/app'
import { getStorageItem, setStorageItem, STORAGE_KEYS } from '@/utils/storageService'
import SaveQueryDialog from './SaveQueryDialog.vue'
import SqlSnippetsManager from './SqlSnippetsManager.vue'
import type { VxeGridProps } from 'vxe-table'

const { t } = useI18n()
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

// 结果集状态追踪
const queryResultStates = reactive<Record<number, {
  pagination: { current: number; pageSize: number };
  loading: boolean;
  hasMore: boolean;
  sql: string;
}>>({})

function getGridOptions(result: QueryResult, index: number): VxeGridProps {
  const state = queryResultStates[index]
  return {
    border: true,
    height: 'auto',
    loading: state?.loading || false,
    columnConfig: { resizable: true },
    rowConfig: { isHover: true, isCurrent: true, height: 36 },
    scrollX: { enabled: true, gt: 20 },
    scrollY: { enabled: true, gt: 0 },
    columns: result.columns.map(col => ({ field: col, title: col, minWidth: 150, showOverflow: true, slots: { default: 'cell_default' } })),
    data: result.rows
  }
}

// 滚动触发加载
const handleScroll = ({ isY, scrollTop, bodyHeight, scrollHeight, index }: any) => {
  if (isY && !executing.value && queryResultStates[index]?.hasMore && !queryResultStates[index]?.loading) {
    if (scrollTop + bodyHeight + 50 >= scrollHeight) {
      loadNextPage(index)
    }
  }
}

async function loadNextPage(index: number) {
  const state = queryResultStates[index]
  if (!state || state.loading || !state.hasMore) return
  
  state.loading = true
  state.pagination.current++
  const offset = (state.pagination.current - 1) * state.pagination.pageSize
  
  // 剥离末尾分号以追加 LIMIT
  let baseSql = state.sql.trim()
  if (baseSql.endsWith(';')) baseSql = baseSql.slice(0, -1).trim()

  // 判定逻辑：剥离注释后检查特征
  const cleanSql = baseSql
    .replace(/\/\*[\s\S]*?\*\//g, '')
    .replace(/(--|#).*$/gm, '')
    .trim()
  
  const normalizedClean = cleanSql.toUpperCase()
  const isSelect = normalizedClean.startsWith('SELECT')
  const hasLimit = /\bLIMIT\b/i.test(normalizedClean)

  // 仅针对单条 SELECT 语句且不含 LIMIT 的情况尝试追加
  if (isSelect && !hasLimit && !cleanSql.includes(';')) {
    const pagedSql = `${baseSql} LIMIT ${state.pagination.pageSize} OFFSET ${offset};`
    try {
      const results = await queryApi.executeQuery(
        sessionConnectionId.value,
        pagedSql,
        selectedDatabase.value || null,
      )
      if (results.length > 0) {
        const result = results[0]
        state.hasMore = result.rows.length === state.pagination.pageSize
        const newRows = [...queryResults.value[index].rows, ...result.rows]
        queryResults.value[index] = { ...queryResults.value[index], rows: newRows }
      } else {
        state.hasMore = false
      }
    } catch (e: any) {
      message.error(e)
      state.hasMore = false
    } finally {
      state.loading = false
    }
  } else {
    state.hasMore = false
    state.loading = false
  }
}

function addMessage(type: string, text: string) { messages.value.unshift({ type, text, time: new Date().toLocaleTimeString() }) }

async function executeQuery() {
  const connId = sessionConnectionId.value
  if (!connId) return
  
  const selection = editor?.getSelection()
  const model = editor?.getModel()
  let fullSql = editor?.getValue().trim() || ''
  let isSelection = false

  if (selection && model && !selection.isEmpty()) {
    const selectedText = model.getValueInRange(selection).trim()
    if (selectedText) { fullSql = selectedText; isSelection = true; }
  }

  if (!fullSql) return message.warning(t('editor.input_sql_warn'))

  executing.value = true
  queryResults.value = []
  Object.keys(queryResultStates).forEach(k => delete queryResultStates[Number(k)])
  
  if (isSelection) addMessage('info', t('editor.executing_selection'))

  try {
    // 1. 拆分 SQL 语句 (简单拆分，实际可更复杂)
    const statements = fullSql.split(';').map(s => s.trim()).filter(s => s.length > 0)
    if (statements.length === 0) {
      executing.value = false
      return
    }

    // 2. 为每一条符合条件的语句注入分页限制，并记录原始语句
    const processedStatements: string[] = []
    const statementConfigs: { sql: string; canPage: boolean }[] = []

    for (const stmt of statements) {
      // 判定逻辑：剥离注释后检查是否以 SELECT 开头
      const cleanStmt = stmt
        .replace(/\/\*[\s\S]*?\*\//g, '') // 移除块注释
        .replace(/(--|#).*$/gm, '')       // 移除行注释
        .trim()
      
      const normalizedClean = cleanStmt.toUpperCase()
      // 检查是否为 SELECT 且不含 LIMIT 关键字（使用正则单词边界匹配）
      const isSelect = normalizedClean.startsWith('SELECT')
      const hasLimit = /\bLIMIT\b/i.test(normalizedClean)
      const canPage = isSelect && !hasLimit
      
      statementConfigs.push({ sql: stmt, canPage })
      if (canPage) {
        // 在原始语句（保留注释）末尾追加 LIMIT
        processedStatements.push(`${stmt} LIMIT 100`)
      } else {
        processedStatements.push(stmt)
      }
    }

    // 3. 合并回一个脚本发送给后端
    const finalSql = processedStatements.join('; ') + ';'

    const results = await queryApi.executeQuery(
      connId,
      finalSql,
      selectedDatabase.value || null,
    )
    
    queryResults.value = results
    
    // 4. 为每个结果集绑定其对应的原始语句和分页状态
    results.forEach((r, i) => {
      // 注意：如果结果集数量多于语句数量（某些数据库特性），安全起见做个兜底
      const config = statementConfigs[i] || { sql: statements[statements.length - 1], canPage: false }
      
      queryResultStates[i] = {
        pagination: { current: 1, pageSize: 100 },
        loading: false,
        hasMore: config.canPage && r.rows.length === 100,
        sql: config.sql // 存储该结果集对应的原始单条 SQL
      }
    })

    if (results.length > 0) {
      resultTabKey.value = 'result-0'
      const totalAffected = results.reduce((acc, r) => acc + r.affected_rows, 0)
      addMessage('success', t('editor.exec_success', { sets: results.length, rows: totalAffected }))
    } else {
      resultTabKey.value = 'messages'
    }
    saveToHistory(fullSql)
  } catch (e: any) {
    message.error(`${t('connection.fail')}: ${e}`)
    addMessage('error', String(e))
    resultTabKey.value = 'messages'
  } finally { executing.value = false }
}

async function explainQuery() {
  const connId = sessionConnectionId.value
  if (!connId || !editor) return
  const sql = editor.getValue().trim()
  if (!sql) return message.warning(t('editor.input_sql_warn'))

  executing.value = true
  try {
    const results = await queryApi.explainQuery(connId, sql, selectedDatabase.value || null)
    queryResults.value = results
    resultTabKey.value = 'result-0'
    addMessage('success', t('editor.explain_success'))
  } catch (e: any) {
    message.error(String(e))
    addMessage('error', String(e))
  } finally { executing.value = false }
}

function stopExecution() { executing.value = false; addMessage('info', t('editor.manual_stop')) }
async function formatSql() { if (!editor) return; try { const formatted = await queryApi.beautifySql(sessionConnectionId.value, editor.getValue()); editor.setValue(formatted); message.success(t('editor.format_success')) } catch (e: any) { message.error(e) } }
function clearEditor() { editor?.setValue(''); queryResults.value = []; messages.value = []; }
function handleQuerySaved() { message.success(t('common.save')) }
function insertSnippet(sql: string) { if (!editor) return; const selection = editor.getSelection(); editor.executeEdits('insert-snippet', [{ range: selection || editor.getSelection()!, text: sql }]); showSnippets.value = false }
function openHistory() { showHistory.value = true }
function openSnippets() { showSnippets.value = true }
function useHistorySql(sql: string) { editor?.setValue(sql); showHistory.value = false; }
function saveToHistory(sql: string) { sqlHistory.value.unshift({ sql, timestamp: Date.now(), database: selectedDatabase.value }); if (sqlHistory.value.length > 100) sqlHistory.value.pop(); setStorageItem(STORAGE_KEYS.SQL_HISTORY, sqlHistory.value) }
async function refreshAutocomplete() { const baseId = props.connectionId || connectionStore.activeConnectionId; if (!baseId) return; autocompleteManager.clearCache(baseId); updateAutocompleteContext(); message.success(t('editor.refresh_cache_success')) }
async function setSelectedDatabase(database: string) { if (availableDatabases.value.length === 0) await loadAvailableDatabases(); selectedDatabase.value = database; updateAutocompleteContext() }
async function handleSave(isAuto = false) { if (!editor || !props.filePath) return; const content = editor.getValue(); if (!content.trim()) return; try { await utilsApi.writeFile(props.filePath, content); if (!isAuto) message.success(t('common.save')) } catch (err: any) { if (!isAuto) message.error(`${t('common.fail')}: ${err}`) } }
function startResize(e: MouseEvent) { isSplitResizing.value = true; const startY = e.clientY, startHeight = editorHeight.value; const doResize = (ev: MouseEvent) => { if (isSplitResizing.value) { editorHeight.value = Math.max(100, startHeight + (ev.clientY - startY)); } }; const stopResize = () => { isSplitResizing.value = false; document.removeEventListener('mousemove', doResize); document.removeEventListener('mouseup', stopResize); document.body.style.cursor = '' }; document.body.style.cursor = 'row-resize'; document.addEventListener('mousemove', doResize); document.addEventListener('mouseup', stopResize) }
function updateAutocompleteContext() {
  const model = editor?.getModel(), baseId = props.connectionId || connectionStore.activeConnectionId
  if (model && baseId && connectionStore.connections.length > 0) {
    const conn = connectionStore.connections.find(c => c.id === baseId)
    const fallbackDatabase = selectedDatabase.value || props.initialDatabase || conn?.database || (conn?.db_type === 'sqlite' ? 'main' : null)
    autocompleteManager.bindModel(model, { connectionId: baseId, database: fallbackDatabase || null, dbType: conn?.db_type || null })
  }
}
async function loadAvailableDatabases() { const baseId = props.connectionId || connectionStore.activeConnectionId; if (!baseId) return; try { const dbs = await metadataApi.getDatabases(baseId); availableDatabases.value = dbs; emit('databasesLoaded', dbs) } catch (e) { console.error(e) } }
function handleDatabaseChange(dbName: string) { selectedDatabase.value = dbName; updateAutocompleteContext() }

onMounted(() => {
  if (!editorContainer.value) return
  editor = monaco.editor.create(editorContainer.value, { value: props.initialValue || t('editor.placeholder'), language: 'sql', theme: appStore.theme === 'dark' ? 'vs-dark' : 'vs', automaticLayout: true, readOnly: false, domReadOnly: false, fontSize: appStore.editorSettings.fontSize, fontFamily: appStore.editorSettings.fontFamily, minimap: { enabled: appStore.editorSettings.minimap }, scrollBeyondLastLine: false, lineNumbers: appStore.editorSettings.lineNumbers, renderLineHighlight: 'all', quickSuggestions: { other: true, comments: false, strings: false }, suggestOnTriggerCharacters: true, acceptSuggestionOnCommitCharacter: true, acceptSuggestionOnEnter: 'on', tabCompletion: 'on' })
  updateAutocompleteContext(); editor.onDidChangeModelContent(() => { emit('contentChange', editor?.getValue() || ''); triggerAutoSave() }); editor.onKeyUp((event) => { if (event.keyCode === monaco.KeyCode.Space || event.keyCode === monaco.KeyCode.Period) editor?.trigger('keyboard', 'editor.action.triggerSuggest', {}) }); editor.addCommand(monaco.KeyCode.F5, () => executeQuery()); editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => handleSave());
  sqlHistory.value = getStorageItem(STORAGE_KEYS.SQL_HISTORY, [])
  loadAvailableDatabases();
})
let autoSaveTimer: any = null; function triggerAutoSave() { if (autoSaveTimer) clearTimeout(autoSaveTimer); autoSaveTimer = setTimeout(() => { handleSave(true) }, 2000) }
onActivated(() => { nextTick(() => { if (editor) editor.layout() }) })
onUnmounted(() => { const model = editor?.getModel(); if (model) autocompleteManager.unbindModel(model); editor?.dispose(); })
watch(() => [appStore.theme, appStore.editorSettings.fontSize, appStore.editorSettings.minimap, appStore.editorSettings.lineNumbers, appStore.editorSettings.fontFamily], ([theme]) => {
  if (!editor) return
  monaco.editor.setTheme(theme === 'dark' ? 'vs-dark' : 'vs')
  editor.updateOptions({
    readOnly: false,
    domReadOnly: false,
    fontSize: appStore.editorSettings.fontSize,
    fontFamily: appStore.editorSettings.fontFamily,
    minimap: { enabled: appStore.editorSettings.minimap },
    lineNumbers: appStore.editorSettings.lineNumbers,
  })
}, { immediate: true })
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
