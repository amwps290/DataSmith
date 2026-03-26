<template>
  <div class="sql-editor-container">
    <div class="editor-workbench" :style="{ height: editorHeight + 'px' }">
      <SqlToolbar
        vertical
        :executing="executing"
        :selected-database="selectedDatabase"
        :databases="availableDatabases"
        @action="handleToolbarAction"
        @database-change="handleToolbarDbChange"
      />

      <!-- 编辑器区域 -->
      <div class="editor-section">
        <div ref="editorContainer" class="monaco-container"></div>
      </div>
    </div>

    <!-- 拖拽调整器 -->
    <div class="split-resizer" @mousedown="startResize">
      <div class="resizer-handle"></div>
    </div>

    <!-- 结果区域 -->
    <div class="result-section">
      <div v-if="showExecutionSummary" class="execution-summary-card" :class="`status-${executionState.status}`">
        <a-tag :color="executionStatusColor" class="execution-summary-tag">{{ executionStatusLabel }}</a-tag>
        <span class="execution-summary-text">{{ executionState.summary }}</span>
        <span v-if="executionSummaryMeta" class="execution-summary-meta">{{ executionSummaryMeta }}</span>
      </div>

      <a-tabs v-model:activeKey="resultTabKey" size="small" class="result-tabs">
        <!-- 动态渲染多个结果页签 -->
        <a-tab-pane v-for="(result, index) in queryResults" :key="'result-' + index">
          <template #tab>
            <span class="result-tab-label" @contextmenu.prevent="handleResultTabContextMenu($event, index)">
              <span class="result-tab-title">
                {{ queryResults.length > 1 ? $t('editor.result_n', { n: index + 1 }) : $t('editor.result') }}
              </span>
              <button
                class="result-tab-close"
                type="button"
                :title="$t('common.close')"
                @click.stop="closeResultAt(index)"
              >
                ×
              </button>
            </span>
          </template>
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
              <a-dropdown>
                <template #overlay>
                  <a-menu @click="handleExportMenuClick(index, $event)">
                    <a-menu-item key="csv">{{ $t('data.export_csv') }}</a-menu-item>
                    <a-menu-item key="json">{{ $t('data.export_json') }}</a-menu-item>
                    <a-menu-item key="sql">{{ $t('data.export_sql') }}</a-menu-item>
                  </a-menu>
                </template>
                <a-button size="small" :icon="h(ExportOutlined)" :disabled="result.columns.length === 0">
                  {{ $t('editor.export_result') }}
                </a-button>
              </a-dropdown>
            </div>
            
            <div class="table-wrapper">
              <vxe-grid
                :ref="(el: any) => setGridRef(el, index)"
                v-bind="getGridOptions(result, index)"
                @scroll="(params: any) => handleScroll({ ...params, index })"
              >
                <template #cell_default="{ row, column }">
                  <span class="result-cell-text" :class="{ 'null-text': row[column.field] === null }">
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

    <div
      v-if="resultContextMenuVisible"
      class="result-context-menu-overlay"
      @click="hideResultContextMenu()"
    >
      <div
        class="result-context-menu"
        :style="{ left: resultContextMenuX + 'px', top: resultContextMenuY + 'px' }"
        @click.stop
      >
        <a-menu @click="handleResultMenuClick" size="small">
          <a-menu-item key="close-current" :disabled="resultContextIndex < 0">
            {{ $t('common.close') }}
          </a-menu-item>
          <a-menu-item key="close-left" :disabled="!hasResultTabsOnLeft">
            {{ $t('common.close_left') }}
          </a-menu-item>
          <a-menu-item key="close-right" :disabled="!hasResultTabsOnRight">
            {{ $t('common.close_right') }}
          </a-menu-item>
        </a-menu>
      </div>
    </div>

    <SaveQueryDialog v-model="showSaveDialog" :sql="editor?.getValue() || ''" @saved="handleQuerySaved" />
    <SqlSnippetsManager v-model:visible="showSnippets" @insert="insertSnippet" />
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted, watch, ref, computed, onActivated, nextTick, reactive, h } from 'vue'
import { useI18n } from 'vue-i18n'
import * as monaco from 'monaco-editor'
import { readText } from '@tauri-apps/plugin-clipboard-manager'
import { getSqlAutocompleteManager } from '@/services/sqlAutocomplete'
import { message, Modal } from 'ant-design-vue'
import { ExportOutlined } from '@ant-design/icons-vue'
import { save } from '@tauri-apps/plugin-dialog'
import { exportApi, queryApi, metadataApi, utilsApi } from '@/api'
import type { QueryResult } from '@/types/database'
import type { PreparedSqlStatement, QueryBatchExecutionResult } from '@/api/query'
import { createIdleExecutionState, type SqlExecutionState, type SqlExecutionStatus } from '@/types/sqlExecution'
import { useConnectionStore } from '@/stores/connection'
import { useAppStore } from '@/stores/app'
import { getStorageItem, setStorageItem, STORAGE_KEYS } from '@/utils/storageService'
import { analyzeSqlSafety, analyzeSqlWrites, type SqlDangerIssue } from '@/utils/sqlSafety'
import SaveQueryDialog from './SaveQueryDialog.vue'
import SqlSnippetsManager from './SqlSnippetsManager.vue'
import type { VxeGridProps } from 'vxe-table'
import SqlToolbar from '@/components/layout/SqlToolbar.vue'

const { t } = useI18n()
const props = defineProps<{ connectionId?: string; initialDatabase?: string; initialValue?: string; filePath?: string; tabId?: string; }>()
const emit = defineEmits(['contentChange', 'fileSaved', 'databasesLoaded', 'databaseChange', 'executionStateChange'])
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
const executionSeq = ref(0)
const activeExecutionId = ref(0)
const executionState = ref<SqlExecutionState>(createIdleExecutionState())
const queryResults = ref<QueryResult[]>([])
const resultTabKey = ref('empty')
const messages = ref<any[]>([])
const showHistory = ref(false)
const sqlHistory = ref<any[]>([])
const showSaveDialog = ref(false)
const showSnippets = ref(false)
const currentConnection = computed(() => {
  const connectionId = props.connectionId || connectionStore.activeConnectionId
  return connectionStore.connections.find(c => c.id === connectionId) || null
})
const currentDatabaseLabel = computed(() => {
  const conn = currentConnection.value
  if (selectedDatabase.value) return selectedDatabase.value
  if (props.initialDatabase) return props.initialDatabase
  if (conn?.database) return conn.database
  if (conn?.db_type === 'sqlite') return 'main'
  return t('editor.default_database')
})
const executionStatusLabel = computed(() => t(`editor.status.${executionState.value.status}`))
const executionStatusColorMap: Record<SqlExecutionStatus, string> = {
  idle: 'default',
  running: 'processing',
  success: 'success',
  failed: 'error',
  cancelled: 'warning',
  partial_success: 'gold',
}
const executionStatusColor = computed(() => executionStatusColorMap[executionState.value.status])
const showExecutionSummary = computed(() => executionState.value.status !== 'idle' && Boolean(executionState.value.summary))
const executionSummaryMeta = computed(() => {
  const state = executionState.value
  const parts: string[] = []

  if (state.statementCount > 0) {
    parts.push(t('editor.statement_progress', { completed: state.completedStatements, total: state.statementCount }))
  }
  if (state.resultSetCount > 0) {
    parts.push(`${state.resultSetCount} ${t('editor.messages_result_sets')}`)
  }
  if (state.affectedRows > 0) {
    parts.push(t('editor.affected_rows_short', { n: state.affectedRows }))
  }

  return parts.join(' · ')
})

const gridRefs = reactive<Record<number, any>>({})
function setGridRef(el: any, index: number) { if (el) gridRefs[index] = el; else delete gridRefs[index]; }
const resultContextMenuVisible = ref(false)
const resultContextMenuX = ref(0)
const resultContextMenuY = ref(0)
const resultContextIndex = ref(-1)
const activeResultIndex = computed(() => {
  const match = /^result-(\d+)$/.exec(resultTabKey.value)
  return match ? Number(match[1]) : -1
})
const hasResultTabsOnLeft = computed(() => resultContextIndex.value > 0)
const hasResultTabsOnRight = computed(() => resultContextIndex.value >= 0 && resultContextIndex.value < queryResults.value.length - 1)

function hideResultContextMenu() {
  resultContextMenuVisible.value = false
  resultContextIndex.value = -1
}

function handleResultTabContextMenu(event: MouseEvent, index: number) {
  resultContextIndex.value = index
  resultContextMenuX.value = event.clientX
  resultContextMenuY.value = event.clientY
  resultContextMenuVisible.value = true
}

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
    mouseConfig: { selected: true },
    scrollX: { enabled: true, gt: 20 },
    scrollY: { enabled: true, gt: 0 },
    columns: result.columns.map(col => ({ field: col, title: col, minWidth: 150, showOverflow: true, slots: { default: 'cell_default' } })),
    data: result.rows
  }
}

function buildResultState(statement: PreparedSqlStatement | undefined, result: QueryResult) {
  return {
    pagination: { current: 1, pageSize: 100 },
    loading: false,
    hasMore: Boolean(statement?.can_page) && result.rows.length === 100,
    sql: statement?.sql || '',
  }
}

function appendQueryResults(results: QueryResult[], states: Array<{ pagination: { current: number; pageSize: number }; loading: boolean; hasMore: boolean; sql: string }>) {
  const startIndex = queryResults.value.length
  queryResults.value = [...queryResults.value, ...results]
  states.forEach((state, offset) => {
    queryResultStates[startIndex + offset] = {
      pagination: { ...state.pagination },
      loading: state.loading,
      hasMore: state.hasMore,
      sql: state.sql,
    }
  })
  return startIndex
}

function replaceResultTabs(keptSourceIndices: number[]) {
  const nextResults = keptSourceIndices.map((sourceIndex) => queryResults.value[sourceIndex])
  const nextStates = keptSourceIndices.map((sourceIndex) => queryResultStates[sourceIndex])
  const previousActiveIndex = activeResultIndex.value

  queryResults.value = nextResults
  Object.keys(queryResultStates).forEach((key) => delete queryResultStates[Number(key)])
  nextStates.forEach((state, index) => {
    if (!state) return
    queryResultStates[index] = {
      pagination: { ...state.pagination },
      loading: state.loading,
      hasMore: state.hasMore,
      sql: state.sql,
    }
  })

  if (previousActiveIndex < 0) {
    if (queryResults.value.length === 0 && resultTabKey.value !== 'messages') {
      resultTabKey.value = 'empty'
    }
    return
  }

  const preservedIndex = keptSourceIndices.indexOf(previousActiveIndex)
  if (preservedIndex >= 0) {
    resultTabKey.value = `result-${preservedIndex}`
    return
  }

  if (queryResults.value.length === 0) {
    resultTabKey.value = 'empty'
    return
  }

  const nearestRightIndex = keptSourceIndices.findIndex((sourceIndex) => sourceIndex > previousActiveIndex)
  resultTabKey.value = nearestRightIndex >= 0
    ? `result-${nearestRightIndex}`
    : `result-${queryResults.value.length - 1}`
}

function closeResultAt(index: number) {
  if (index < 0 || index >= queryResults.value.length) return
  replaceResultTabs(queryResults.value.map((_, itemIndex) => itemIndex).filter((itemIndex) => itemIndex !== index))
}

function closeResultTabsLeftOf(index: number) {
  if (index <= 0) return
  replaceResultTabs(queryResults.value.map((_, itemIndex) => itemIndex).filter((itemIndex) => itemIndex >= index))
}

function closeResultTabsRightOf(index: number) {
  if (index < 0 || index >= queryResults.value.length - 1) return
  replaceResultTabs(queryResults.value.map((_, itemIndex) => itemIndex).filter((itemIndex) => itemIndex <= index))
}

function handleResultMenuClick({ key }: { key: string | number }) {
  const action = String(key)
  const targetIndex = resultContextIndex.value
  if (action === 'close-current') {
    closeResultAt(targetIndex)
  } else if (action === 'close-left') {
    closeResultTabsLeftOf(targetIndex)
  } else if (action === 'close-right') {
    closeResultTabsRightOf(targetIndex)
  }
  hideResultContextMenu()
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
      const errorMessage = getErrorMessage(e)
      message.error(errorMessage)
      addMessage('error', errorMessage)
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

function handleToolbarAction(method: string) {
  const actionMap: Record<string, () => void | Promise<void>> = {
    executeQuery,
    explainQuery,
    stopExecution,
    handleSave,
    formatSql,
    clearEditor,
    openHistory,
    openSnippets,
    refreshAutocomplete,
  }

  const action = actionMap[method]
  if (!action) return
  void action()
}

function handleToolbarDbChange(database: string) {
  void handleDatabaseChange(database)
}

function getMessageTypeForStatus(status: SqlExecutionStatus) {
  switch (status) {
    case 'success':
      return 'success'
    case 'failed':
      return 'error'
    case 'cancelled':
    case 'partial_success':
      return 'warning'
    default:
      return 'info'
  }
}

function updateExecutionState(patch: Partial<SqlExecutionState> & { status?: SqlExecutionStatus }) {
  executionState.value = {
    ...executionState.value,
    ...patch,
    updatedAt: Date.now(),
  }
  emit('executionStateChange', { ...executionState.value })
}

function resetExecutionState() {
  executionState.value = createIdleExecutionState()
  emit('executionStateChange', { ...executionState.value })
}

function finalizeExecutionState(
  status: SqlExecutionStatus,
  summary: string,
  options: Partial<Omit<SqlExecutionState, 'status' | 'summary' | 'updatedAt'>> = {}
) {
  updateExecutionState({
    status,
    summary,
    detail: options.detail || '',
    mode: options.mode ?? executionState.value.mode,
    statementCount: options.statementCount ?? executionState.value.statementCount,
    completedStatements: options.completedStatements ?? executionState.value.completedStatements,
    resultSetCount: options.resultSetCount ?? executionState.value.resultSetCount,
    affectedRows: options.affectedRows ?? executionState.value.affectedRows,
  })

  addMessage(getMessageTypeForStatus(status), summary)
  if (options.detail) {
    addMessage('error', options.detail)
  }
}

function getAffectedRows(results: QueryResult[]) {
  return results.reduce((acc, result) => acc + result.affected_rows, 0)
}

function isCancelledMessage(messageText: string) {
  const normalized = messageText.toLowerCase()
  return normalized.includes('查询已取消')
    || normalized.includes('cancelled')
    || normalized.includes('canceled')
    || normalized.includes('canceling statement due to user request')
    || normalized.includes('interrupted')
}

function applyBatchExecutionState(response: QueryBatchExecutionResult) {
  const resultSetCount = response.results.length
  const affectedRows = getAffectedRows(response.results)
  const completedStatements = response.statements_succeeded
  const statementCount = response.statements_total

  if (response.was_cancelled) {
    const status: SqlExecutionStatus = completedStatements > 0 ? 'partial_success' : 'cancelled'
    const summary = completedStatements > 0
      ? t('editor.summary.query_cancelled_partial', { success: completedStatements, total: statementCount })
      : t('editor.summary.query_cancelled')

    finalizeExecutionState(status, summary, {
      mode: 'query',
      statementCount,
      completedStatements,
      resultSetCount,
      affectedRows,
    })
    return
  }

  if (response.error_message) {
    const status: SqlExecutionStatus = completedStatements > 0 ? 'partial_success' : 'failed'
    const summary = completedStatements > 0
      ? t('editor.summary.query_partial', {
          success: completedStatements,
          total: statementCount,
          failed: response.failed_statement_index || completedStatements + 1,
        })
      : t('editor.summary.query_failed', {
          failed: response.failed_statement_index || 1,
        })

    finalizeExecutionState(status, summary, {
      mode: 'query',
      detail: response.error_message,
      statementCount,
      completedStatements,
      resultSetCount,
      affectedRows,
    })
    return
  }

  const summary = resultSetCount > 0
    ? t('editor.summary.query_success', { count: completedStatements, sets: resultSetCount })
    : t('editor.summary.query_success_empty', { count: completedStatements })

  finalizeExecutionState('success', summary, {
    mode: 'query',
    statementCount,
    completedStatements,
    resultSetCount,
    affectedRows,
  })
}

function sanitizeFileName(name: string) {
  return name.replace(/[\\/:*?"<>|]+/g, '_').trim() || t('editor.export_default_name')
}

function unquoteIdentifier(name: string) {
  return name.replace(/^["'`[]+|["'`\]]+$/g, '')
}

function inferInsertTargetTable(index: number) {
  const sql = queryResultStates[index]?.sql?.trim() || ''
  const normalized = sql.replace(/\s+/g, ' ')
  const match = normalized.match(/\bFROM\s+((?:["`\[]?[\w$]+["`\]]?\.)?["`\[]?[\w$]+["`\]]?)/i)
  const target = match?.[1]?.split('.').pop()
  return target ? unquoteIdentifier(target) : `query_result_${index + 1}`
}

function inferExportBaseName(index: number, format: string) {
  const tableName = inferInsertTargetTable(index)
  const databaseName = selectedDatabase.value || currentDatabaseLabel.value || t('editor.export_default_name')
  return sanitizeFileName(`${databaseName}_${tableName}.${format}`)
}

async function handleExportResult(index: number, format: string) {
  const result = queryResults.value[index]
  if (!result || result.columns.length === 0) return

  try {
    const defaultPath = inferExportBaseName(index, format)
    const path = await save({
      defaultPath,
      filters: [{ name: format.toUpperCase(), extensions: [format] }],
    })
    if (!path) return

    if (format === 'csv') {
      await exportApi.toCsv(result, path)
    } else if (format === 'json') {
      await exportApi.toJson(result, path)
    } else if (format === 'sql') {
      await exportApi.toSql(result, inferInsertTargetTable(index), path)
    } else {
      throw new Error(`Unsupported export format: ${format}`)
    }

    message.success(t('data.export_success', { path }))
  } catch (e: any) {
    const errorMessage = getErrorMessage(e)
    message.error(errorMessage)
    addMessage('error', errorMessage)
  }
}

function handleExportMenuClick(index: number, { key }: { key: string | number }) {
  return handleExportResult(index, String(key))
}

function getErrorMessage(error: unknown): string {
  if (typeof error === 'string') return error
  if (error instanceof Error) return error.message
  if (error && typeof error === 'object') {
    const messageValue = Reflect.get(error, 'message')
    if (typeof messageValue === 'string' && messageValue.trim()) return messageValue
    const errorValue = Reflect.get(error, 'error')
    if (typeof errorValue === 'string' && errorValue.trim()) return errorValue
    const causeValue = Reflect.get(error, 'cause')
    if (causeValue) {
      const causeMessage: string = getErrorMessage(causeValue)
      if (causeMessage && causeMessage !== '[object Object]') return causeMessage
    }
    const propertyMap = Object.fromEntries(
      Object.getOwnPropertyNames(error).map((key) => [key, Reflect.get(error, key)])
    )
    if (Object.keys(propertyMap).length > 0) {
      try {
        return JSON.stringify(propertyMap)
      } catch {
        // ignore
      }
    }
    try {
      return JSON.stringify(error)
    } catch {
      return String(error)
    }
  }
  return String(error)
}

function beginExecution(mode: 'query' | 'explain', statementCount = 1) {
  const executionId = executionSeq.value + 1
  executionSeq.value = executionId
  activeExecutionId.value = executionId
  executing.value = true
  updateExecutionState({
    status: 'running',
    mode,
    summary: mode === 'query'
      ? t('editor.summary.running_query', { count: statementCount })
      : t('editor.summary.running_explain'),
    detail: '',
    statementCount,
    completedStatements: 0,
    resultSetCount: 0,
    affectedRows: 0,
  })
  return executionId
}

function isExecutionStale(executionId: number) {
  return executionId !== activeExecutionId.value
}

function getDangerIssueLabel(issue: SqlDangerIssue) {
  return t(`editor.danger.${issue.type}`)
}

function formatDangerSqlPreview(sql: string) {
  return sql.replace(/\s+/g, ' ').trim().slice(0, 160)
}

async function confirmDangerousExecution(issues: SqlDangerIssue[]) {
  if (issues.length === 0) return true

  return new Promise<boolean>((resolve) => {
    const content = h('div', { class: 'danger-confirm-content' }, [
      h('p', { class: 'danger-confirm-intro' }, t('editor.danger.confirm_intro')),
      h('ul', { class: 'danger-confirm-list' }, issues.map((issue, index) =>
        h('li', { key: `${issue.type}-${index}` }, `${getDangerIssueLabel(issue)}: ${formatDangerSqlPreview(issue.statement)}`)
      )),
    ])

    Modal.confirm({
      title: t('editor.danger.confirm_title'),
      content,
      okText: t('editor.danger.confirm_ok'),
      cancelText: t('common.cancel'),
      okType: 'danger',
      width: 720,
      onOk: () => {
        addMessage('warning', t('editor.danger.confirmed'))
        resolve(true)
      },
      onCancel: () => {
        addMessage('info', t('editor.danger.cancelled'))
        resultTabKey.value = 'messages'
        resolve(false)
      },
    })
  })
}

async function executeQuery() {
  const connId = sessionConnectionId.value
  if (!connId) return
  
  const selection = editor?.getSelection()
  const model = editor?.getModel()
  let fullSql = editor?.getValue().trim() || ''
  let isSelection = false
  let executionId: number | null = null

  if (selection && model && !selection.isEmpty()) {
    const selectedText = model.getValueInRange(selection).trim()
    if (selectedText) { fullSql = selectedText; isSelection = true; }
  }

  if (!fullSql) return message.warning(t('editor.input_sql_warn'))

  try {
    const preparedStatements = await queryApi.prepareSqlScript(connId, fullSql)
    if (preparedStatements.length === 0) {
      return
    }

    const writeAnalysis = analyzeSqlWrites(preparedStatements.map(statement => statement.sql))
    if (currentConnection.value?.read_only && writeAnalysis.hasWrites) {
      const warningText = t('editor.read_only_blocked', { count: writeAnalysis.writeStatements.length })
      message.warning(warningText)
      addMessage('warning', warningText)
      resultTabKey.value = 'messages'
      return
    }

    const safetyAnalysis = analyzeSqlSafety(preparedStatements.map(statement => statement.sql))
    const confirmed = await confirmDangerousExecution(safetyAnalysis.issues)
    if (!confirmed) {
      return
    }

    executionId = beginExecution('query', preparedStatements.length)

    if (isSelection) addMessage('info', t('editor.executing_selection'))
    addMessage('info', t('editor.exec_context', { database: currentDatabaseLabel.value }))
    console.info('[SQL] execute', {
      connectionId: connId,
      database: selectedDatabase.value || null,
      displayDatabase: currentDatabaseLabel.value,
    })

    const processedStatements = preparedStatements.map((statement: PreparedSqlStatement) =>
      statement.can_page ? `${statement.sql} LIMIT 100` : statement.sql
    )

    const response = await queryApi.executeQueryBatch(
      connId,
      processedStatements,
      selectedDatabase.value || null,
      executionId,
    )

    if (isExecutionStale(executionId)) {
      return
    }

    const appendedIndex = appendQueryResults(
      response.results,
      response.results.map((r, i) => {
        const config = preparedStatements[i] || preparedStatements[preparedStatements.length - 1]
        return buildResultState(config, r)
      })
    )

    applyBatchExecutionState(response)

    if (response.results.length > 0) {
      resultTabKey.value = `result-${appendedIndex}`
    } else {
      resultTabKey.value = 'messages'
    }
    if (!response.error_message && !response.was_cancelled) {
      saveToHistory(fullSql)
    }
  } catch (e: any) {
    if (executionId !== null && isExecutionStale(executionId)) {
      return
    }
    const errorMessage = getErrorMessage(e)

    if (isCancelledMessage(errorMessage)) {
      finalizeExecutionState('cancelled', t('editor.summary.query_cancelled'), {
        mode: 'query',
        statementCount: executionState.value.statementCount,
        completedStatements: executionState.value.completedStatements,
      })
    } else {
      message.error(errorMessage)
      finalizeExecutionState('failed', t('editor.summary.query_failed', { failed: 1 }), {
        mode: 'query',
        detail: errorMessage,
        statementCount: executionState.value.statementCount || 1,
        completedStatements: 0,
      })
    }
    resultTabKey.value = 'messages'
  } finally {
    if (executionId !== null && !isExecutionStale(executionId)) {
      executing.value = false
    }
  }
}

async function explainQuery() {
  const connId = sessionConnectionId.value
  if (!connId || !editor) return
  const sql = editor.getValue().trim()
  if (!sql) return message.warning(t('editor.input_sql_warn'))

  const executionId = beginExecution('explain')
  try {
    addMessage('info', t('editor.exec_context', { database: currentDatabaseLabel.value }))
    console.info('[SQL] explain', {
      connectionId: connId,
      database: selectedDatabase.value || null,
      displayDatabase: currentDatabaseLabel.value,
    })
    const results = await queryApi.explainQuery(connId, sql, selectedDatabase.value || null, executionId)
    if (isExecutionStale(executionId)) {
      return
    }
    const appendedIndex = appendQueryResults(
      results,
      results.map((result) => buildResultState(undefined, result))
    )
    resultTabKey.value = `result-${appendedIndex}`
    finalizeExecutionState('success', t('editor.summary.explain_success', { sets: results.length }), {
      mode: 'explain',
      statementCount: 1,
      completedStatements: 1,
      resultSetCount: results.length,
      affectedRows: getAffectedRows(results),
    })
  } catch (e: any) {
    if (isExecutionStale(executionId)) {
      return
    }
    const errorMessage = getErrorMessage(e)
    if (isCancelledMessage(errorMessage)) {
      finalizeExecutionState('cancelled', t('editor.summary.explain_cancelled'), {
        mode: 'explain',
        statementCount: 1,
        completedStatements: 0,
      })
    } else {
      message.error(errorMessage)
      finalizeExecutionState('failed', t('editor.summary.explain_failed'), {
        mode: 'explain',
        detail: errorMessage,
        statementCount: 1,
        completedStatements: 0,
      })
    }
    resultTabKey.value = 'messages'
  } finally {
    if (!isExecutionStale(executionId)) {
      executing.value = false
    }
  }
}

async function stopExecution() {
  console.info('[SQL] stop requested', {
    connectionId: sessionConnectionId.value,
    executionId: activeExecutionId.value,
    executing: executing.value,
    status: executionState.value.status,
  })

  if (!executing.value) {
    console.warn('[SQL] stop ignored because editor is not executing', {
      connectionId: sessionConnectionId.value,
      executionId: activeExecutionId.value,
      status: executionState.value.status,
    })
    return
  }

  const connId = sessionConnectionId.value
  const executionId = activeExecutionId.value
  const previousState = executionState.value
  activeExecutionId.value = executionSeq.value + 1
  executing.value = false
  resultTabKey.value = 'messages'

  if (connId && executionId > 0) {
    try {
      console.info('[SQL] sending cancel request', {
        connectionId: connId,
        executionId,
      })
      const cancelled = await queryApi.cancelQuery(connId, executionId)
      console.info('[SQL] cancel request completed', {
        connectionId: connId,
        executionId,
        cancelled,
      })
      if (!cancelled) {
        console.warn('[SQL] backend reported no active query to cancel', {
          connectionId: connId,
          executionId,
        })
      }
    } catch (e: any) {
      console.error('[SQL] cancel failed', {
        connectionId: connId,
        executionId,
        error: e,
      })
    }
  } else {
    console.warn('[SQL] cancel request skipped because execution context is missing', {
      connectionId: connId,
      executionId,
    })
  }

  finalizeExecutionState('cancelled', t('editor.manual_stop'), {
    mode: previousState.mode,
    statementCount: previousState.statementCount,
    completedStatements: previousState.completedStatements,
    resultSetCount: previousState.resultSetCount,
    affectedRows: previousState.affectedRows,
  })
}
async function formatSql() { if (!editor) return; try { const formatted = await queryApi.beautifySql(sessionConnectionId.value, editor.getValue()); editor.setValue(formatted); message.success(t('editor.format_success')) } catch (e: any) { message.error(getErrorMessage(e)) } }
function clearEditor() { editor?.setValue(''); queryResults.value = []; resultTabKey.value = 'empty'; messages.value = []; Object.keys(queryResultStates).forEach(k => delete queryResultStates[Number(k)]); hideResultContextMenu(); resetExecutionState(); }
function handleQuerySaved() { message.success(t('common.save')) }
function insertSnippet(sql: string) { if (!editor) return; const selection = editor.getSelection(); editor.executeEdits('insert-snippet', [{ range: selection || editor.getSelection()!, text: sql }]); showSnippets.value = false }
async function pasteFromSystemClipboard() {
  if (!editor) return

  try {
    const text = await readText()
    const selections = editor.getSelections() || (editor.getSelection() ? [editor.getSelection()!] : [])
    if (selections.length === 0) return

    editor.executeEdits('system-clipboard-paste', selections.map((range) => ({ range, text })))
    editor.focus()
  } catch (e: any) {
    message.error(getErrorMessage(e))
  }
}
function openHistory() { showHistory.value = true }
function openSnippets() { showSnippets.value = true }
function useHistorySql(sql: string) { editor?.setValue(sql); showHistory.value = false; }
function saveToHistory(sql: string) { sqlHistory.value.unshift({ sql, timestamp: Date.now(), database: selectedDatabase.value }); if (sqlHistory.value.length > 100) sqlHistory.value.pop(); setStorageItem(STORAGE_KEYS.SQL_HISTORY, sqlHistory.value) }
async function refreshAutocomplete() { const baseId = props.connectionId || connectionStore.activeConnectionId; if (!baseId) return; autocompleteManager.clearCache(baseId); updateAutocompleteContext(); message.success(t('editor.refresh_cache_success')) }
async function setSelectedDatabase(database: string) {
  if (availableDatabases.value.length === 0) await loadAvailableDatabases()
  selectedDatabase.value = database
  updateAutocompleteContext()
  emit('databaseChange', database)
}
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
function handleDatabaseChange(dbName: string) {
  selectedDatabase.value = dbName
  updateAutocompleteContext()
  emit('databaseChange', dbName)
  const notice = t('editor.database_switched', { database: currentDatabaseLabel.value })
  addMessage('info', notice)
  message.info(notice)
  console.info('[SQL] database switched', {
    connectionId: sessionConnectionId.value,
    database: dbName || null,
    displayDatabase: currentDatabaseLabel.value,
  })
}

onMounted(() => {
  if (!editorContainer.value) return
  editor = monaco.editor.create(editorContainer.value, { value: props.initialValue || t('editor.placeholder'), language: 'sql', theme: appStore.theme === 'dark' ? 'vs-dark' : 'vs', automaticLayout: true, readOnly: false, domReadOnly: false, fontSize: appStore.editorSettings.fontSize, fontFamily: appStore.editorSettings.fontFamily, minimap: { enabled: appStore.editorSettings.minimap }, scrollBeyondLastLine: false, lineNumbers: appStore.editorSettings.lineNumbers, renderLineHighlight: 'all', quickSuggestions: { other: true, comments: false, strings: false }, suggestOnTriggerCharacters: true, acceptSuggestionOnCommitCharacter: true, acceptSuggestionOnEnter: 'on', tabCompletion: 'on' })
  updateAutocompleteContext(); editor.onDidChangeModelContent(() => { emit('contentChange', editor?.getValue() || ''); triggerAutoSave() }); editor.onKeyUp((event) => { if (event.keyCode === monaco.KeyCode.Space || event.keyCode === monaco.KeyCode.Period) editor?.trigger('keyboard', 'editor.action.triggerSuggest', {}) }); editor.addCommand(monaco.KeyCode.F5, () => executeQuery()); editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => handleSave()); editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyV, () => { void pasteFromSystemClipboard() });
  sqlHistory.value = getStorageItem(STORAGE_KEYS.SQL_HISTORY, [])
  loadAvailableDatabases();
})
let autoSaveTimer: any = null; function triggerAutoSave() { if (autoSaveTimer) clearTimeout(autoSaveTimer); autoSaveTimer = setTimeout(() => { handleSave(true) }, 2000) }
onActivated(() => { nextTick(() => { if (editor) editor.layout() }) })
onUnmounted(() => { hideResultContextMenu(); const model = editor?.getModel(); if (model) autocompleteManager.unbindModel(model); editor?.dispose(); })
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
defineExpose({ setSelectedDatabase, executing, executionState, executeQuery, explainQuery, stopExecution, handleDatabaseChange, formatSql, clearEditor, openHistory, openSnippets, refreshAutocomplete, handleSave })
</script>

<style scoped>
.sql-editor-container { display: flex; flex-direction: column; height: 100%; overflow: hidden; background: #fff; }
.dark-mode .sql-editor-container { background: #1f1f1f; }
.editor-workbench { display: flex; flex-shrink: 0; min-height: 100px; overflow: hidden; }
.editor-section { flex: 1; min-width: 0; min-height: 100px; overflow: hidden; }
.monaco-container { height: 100%; width: 100%; }
.split-resizer { height: 4px; background: #f0f0f0; cursor: row-resize; display: flex; align-items: center; justify-content: center; transition: background 0.2s; flex-shrink: 0; }
.split-resizer:hover { background: #1890ff; }
.dark-mode .split-resizer { background: #303030; }
.resizer-handle { width: 30px; height: 2px; background: #d9d9d9; border-radius: 1px; }
.result-section { flex: 1; min-height: 100px; display: flex; flex-direction: column; overflow: hidden; }
.execution-summary-card { display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-bottom: 1px solid #f0f0f0; background: #fafafa; flex-shrink: 0; }
.dark-mode .execution-summary-card { background: #1a1a1a; border-bottom-color: #303030; }
.execution-summary-card.status-success { background: #f6ffed; border-bottom-color: #b7eb8f; }
.execution-summary-card.status-failed { background: #fff2f0; border-bottom-color: #ffccc7; }
.execution-summary-card.status-cancelled { background: #fffbe6; border-bottom-color: #ffe58f; }
.execution-summary-card.status-partial_success { background: #fff7e6; border-bottom-color: #ffd591; }
.execution-summary-card.status-running { background: #e6f4ff; border-bottom-color: #91caff; }
.dark-mode .execution-summary-card.status-success { background: #162312; border-bottom-color: #274916; }
.dark-mode .execution-summary-card.status-failed { background: #2a1215; border-bottom-color: #58181c; }
.dark-mode .execution-summary-card.status-cancelled { background: #2b2111; border-bottom-color: #594214; }
.dark-mode .execution-summary-card.status-partial_success { background: #2b1d11; border-bottom-color: #593815; }
.dark-mode .execution-summary-card.status-running { background: #111b26; border-bottom-color: #15417e; }
.execution-summary-tag { margin-inline-end: 0; }
.execution-summary-text { color: #262626; font-size: 13px; font-weight: 500; }
.dark-mode .execution-summary-text { color: #f5f5f5; }
.execution-summary-meta { color: #8c8c8c; font-size: 12px; }
.dark-mode .execution-summary-meta { color: #a6a6a6; }
.result-tabs { height: 100%; display: flex; flex-direction: column; }
.result-tabs :deep(.ant-tabs-content) { flex: 1; overflow: hidden; }
.result-tabs :deep(.ant-tabs-tabpane) { height: 100%; display: flex; flex-direction: column; }
.result-content { flex: 1; display: flex; flex-direction: column; padding: 12px; overflow: hidden; position: relative; }
.executing-overlay { position: absolute; top: 0; left: 0; right: 0; bottom: 0; background: rgba(255, 255, 255, 0.7); display: flex; align-items: center; justify-content: center; z-index: 10; }
.dark-mode .executing-overlay { background: rgba(0, 0, 0, 0.6); }
.result-info { margin-bottom: 8px; flex-shrink: 0; display: flex; align-items: center; justify-content: space-between; gap: 12px; }
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
.result-tab-label { display: inline-flex; align-items: center; gap: 6px; max-width: 180px; }
.result-tab-title { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.result-tab-close { display: inline-flex; align-items: center; justify-content: center; width: 16px; height: 16px; padding: 0; border: 0; border-radius: 999px; background: transparent; color: #8c8c8c; font-size: 12px; line-height: 1; cursor: pointer; flex-shrink: 0; transition: background-color 0.2s, color 0.2s; }
.result-tab-close:hover { background: rgba(0, 0, 0, 0.08); color: #262626; }
.dark-mode .result-tab-close { color: #a6a6a6; }
.dark-mode .result-tab-close:hover { background: rgba(255, 255, 255, 0.12); color: #f5f5f5; }
.result-context-menu-overlay { position: fixed; inset: 0; z-index: 9999; }
.result-context-menu { position: absolute; min-width: 140px; background: #fff; border: 1px solid #d9d9d9; border-radius: 8px; box-shadow: 0 8px 24px rgba(15, 23, 42, 0.16); overflow: hidden; }
.dark-mode .result-context-menu { background: #1f1f1f; border-color: #303030; box-shadow: 0 8px 24px rgba(0, 0, 0, 0.36); }
.result-cell-text,
.messages-content,
:deep(.table-wrapper .vxe-cell),
:deep(.table-wrapper .vxe-cell--label),
:deep(.table-wrapper .vxe-body--row .vxe-cell) {
  user-select: text !important;
  -webkit-user-select: text !important;
}
.null-text { color: #bfbfbf; font-style: italic; }
:deep(.danger-confirm-content) { display: flex; flex-direction: column; gap: 12px; }
:deep(.danger-confirm-intro) { margin: 0; color: #595959; }
:deep(.danger-confirm-list) { margin: 0; padding-left: 18px; color: #262626; }
:deep(.danger-confirm-list li) { margin-bottom: 8px; line-height: 1.5; word-break: break-word; }

@media (max-width: 768px) {
  .result-info {
    align-items: flex-start;
    flex-direction: column;
  }
}
</style>
