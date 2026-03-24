<template>
  <div class="table-data-grid">
    <div class="grid-toolbar">
      <a-space>
        <a-button-group>
          <a-button :icon="h(ReloadOutlined)" @click="refresh" :loading="loading">
            {{ $t('common.refresh') }}
          </a-button>
          <a-button :icon="h(PlusOutlined)" @click="addRow">
            {{ $t('common.add') }}
          </a-button>
          <a-button 
            :icon="h(DeleteOutlined)" 
            danger 
            :disabled="selectedRowKeys.length === 0"
            @click="deleteSelected"
          >
            {{ $t('common.delete') }}
          </a-button>
        </a-button-group>

        <a-divider type="vertical" />

        <!-- 提交变更按钮 -->
        <a-button-group v-if="hasChanges">
          <a-button type="primary" @click="submitChanges" :loading="saving">
            {{ $t('data.save_changes', { n: changeCount }) }}
          </a-button>
          <a-button @click="discardChanges">
            {{ $t('data.discard_changes') }}
          </a-button>
        </a-button-group>

        <a-divider type="vertical" v-if="hasChanges" />

        <a-button :icon="h(FilterOutlined)" @click="showFilterDialog = true">
          {{ $t('data.filter') }}
        </a-button>
        <a-dropdown>
          <template #overlay>
            <a-menu @click="handleExport">
              <a-menu-item key="csv">{{ $t('data.export_csv') }}</a-menu-item>
              <a-menu-item key="json">{{ $t('data.export_json') }}</a-menu-item>
              <a-menu-item key="sql">{{ $t('data.export_sql') }}</a-menu-item>
            </a-menu>
          </template>
          <a-button :icon="h(ExportOutlined)">
            {{ $t('data.export') }}
          </a-button>
        </a-dropdown>
        <a-divider type="vertical" />
        <a-button :type="showViewer ? 'primary' : 'default'" @click="showViewer = !showViewer">
          {{ $t('data.cell_viewer') }}
        </a-button>
      </a-space>
      
      <div class="toolbar-right">
        <div class="data-info">
          {{ $t('editor.loaded_rows', { n: gridOptions.data?.length || 0 }) }}
          <span v-if="loading" class="loading-text">
            <a-spin size="small" style="margin-left: 8px" /> {{ $t('common.loading') }}
          </span>
          <span v-else-if="!hasMore" class="end-text"> {{ $t('data.loaded_all') }}</span>
        </div>
      </div>
    </div>

    <!-- 高性能虚拟滚动表格 + 滚动加载 -->
    <div class="grid-wrapper">
      <vxe-grid 
        ref="gridRef" 
        v-bind="gridOptions" 
        @checkbox-change="handleCheckboxChange" 
        @checkbox-all="handleCheckboxChange"
        @scroll="handleScroll"
        @edit-closed="handleEditClosed"
        @cell-click="handleCellClick"
        :cell-class-name="getCellClassName"
      >
        <template #cell_default="{ row, column }">
          <span :class="{ 'null-text': row[column.field] === null }">
            {{ row[column.field] === null ? 'NULL' : row[column.field] }}
          </span>
        </template>
      </vxe-grid>
    </div>

    <!-- 单元格查看器 Drawer -->
    <a-drawer
      v-model:open="showViewer"
      :title="$t('data.cell_viewer')"
      placement="right"
      :width="500"
      :mask="false"
      class="cell-viewer-drawer"
    >
      <template #extra>
        <a-space>
          <a-button size="small" @click="formatJsonInViewer">{{ $t('data.format_json') }}</a-button>
          <a-button size="small" @click="copyViewerContent">{{ $t('data.copy_content') }}</a-button>
        </a-space>
      </template>
      <div v-if="selectedCell" class="viewer-container">
        <div class="viewer-header">
          <a-tag color="blue">{{ selectedCell.column.title }}</a-tag>
          <a-checkbox v-model:checked="isViewerSetNull" @change="handleViewerNullChange">{{ $t('data.set_null') }}</a-checkbox>
        </div>
        <a-textarea
          v-model:value="viewerValue"
          :rows="20"
          :disabled="isViewerSetNull"
          class="viewer-textarea"
          @change="handleViewerValueChange"
        />
      </div>
      <a-empty v-else :description="'请选择一个单元格'" />
    </a-drawer>

    <!-- 筛选对话框 -->
    <a-modal v-model:open="showFilterDialog" :title="$t('data.data_filter')" @ok="applyFilter">
      <a-form layout="vertical">
        <a-form-item :label="$t('data.where_condition')">
          <a-textarea v-model:value="filterCondition" :rows="4" :placeholder="$t('data.filter_placeholder')" />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { h, ref, watch, computed, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  ReloadOutlined, PlusOutlined, DeleteOutlined, FilterOutlined,
  ExportOutlined
} from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { queryApi, metadataApi, dataApi } from '@/api'
import { invoke } from '@tauri-apps/api/core'
import { useConnectionStore } from '@/stores/connection'
import type { VxeGridProps, VxeGridInstance, VxeGridEvents } from 'vxe-table'
import type { ColumnInfo } from '@/types/database'

interface GridRow extends Record<string, any> {
  __rowIndex: number
  _originalData: Record<string, any>
  _isNew?: boolean
  _newTouchedFields?: Record<string, boolean>
}

const { t } = useI18n()
const props = defineProps<{ connectionId: string; database: string; table: string; schema?: string }>()
const connectionStore = useConnectionStore()
const gridRef = ref<VxeGridInstance>()

const loading = ref(false)
const hasMore = ref(true)
const selectedRowKeys = ref<any[]>([])
const showFilterDialog = ref(false)
const filterCondition = ref('')
const primaryKeys = ref<string[]>([])
const tableColumns = ref<ColumnInfo[]>([])
const saving = ref(false)
const nextRowIndex = ref(0)

// 变更追踪状态
const pendingEdits = reactive<Record<number, Record<string, { old: any, new: any }>>>({})
const newRows = computed(() => ((gridOptions.data || []) as GridRow[]).filter(row => row._isNew))
const hasChanges = computed(() => Object.keys(pendingEdits).length > 0 || newRows.value.length > 0)
const changeCount = computed(() => {
  const updatedCells = Object.values(pendingEdits).reduce((acc, row) => acc + Object.keys(row).length, 0)
  return updatedCells + newRows.value.length
})

// 查看器状态
const showViewer = ref(false)
const selectedCell = ref<any>(null)
const viewerValue = ref('')
const isViewerSetNull = ref(false)

const pagination = reactive({ current: 1, pageSize: 100 })

const dbType = computed(() => connectionStore.connections.find(c => c.id === props.connectionId)?.db_type || 'mysql')
const quote = (n: string) => dbType.value === 'sqlite' || dbType.value === 'postgresql' ? `"${n}"` : `\`${n}\``
const tableRef = () => {
  if (dbType.value === 'postgresql') return `${quote(props.schema || 'public')}.${quote(props.table)}`
  return quote(props.table)
}

const gridOptions = reactive<VxeGridProps>({
  border: true,
  height: 'auto',
  loading: false,
  columnConfig: { resizable: true, drag: true },
  rowConfig: { isCurrent: true, isHover: true, keyField: '__rowIndex', height: 36 },
  checkboxConfig: { reserve: true, trigger: 'cell' },
  editConfig: { trigger: 'dblclick', mode: 'cell', showStatus: true },
  scrollX: { enabled: true, gt: 20 },
  scrollY: { enabled: true, gt: 0 },
  columns: [],
  data: []
})

const handleScroll: VxeGridEvents.Scroll = ({ isY, scrollTop, bodyHeight, scrollHeight }) => {
  if (isY && !loading.value && hasMore.value) {
    if (scrollTop + bodyHeight + 50 >= scrollHeight) {
      loadNextPage()
    }
  }
}

function getCellClassName({ row, column }: any) {
  if (row._isNew) {
    return 'cell-new-row'
  }
  const rowIndex = row.__rowIndex
  if (pendingEdits[rowIndex] && pendingEdits[rowIndex][column.field]) {
    return 'cell-modified'
  }
  return ''
}

function handleEditClosed({ row, column }: any) {
  recordChange(row, column.field, row[column.field])
  // 如果当前查看器正在显示这个单元格，同步它
  if (selectedCell.value && selectedCell.value.row.__rowIndex === row.__rowIndex && selectedCell.value.column.field === column.field) {
    viewerValue.value = row[column.field] === null ? '' : String(row[column.field])
    isViewerSetNull.value = row[column.field] === null
  }
}

function recordChange(row: any, field: string, newVal: any) {
  if (row._isNew) {
    if (!row._newTouchedFields) row._newTouchedFields = {}
    row._newTouchedFields[field] = true
    return
  }

  const rowIndex = row.__rowIndex
  const existingEdit = pendingEdits[rowIndex]?.[field]
  const oldVal = existingEdit ? existingEdit.old : row._originalData?.[field]

  if (newVal === oldVal) {
    if (pendingEdits[rowIndex]) {
      delete pendingEdits[rowIndex][field]
      if (Object.keys(pendingEdits[rowIndex]).length === 0) delete pendingEdits[rowIndex]
    }
  } else {
    if (!pendingEdits[rowIndex]) pendingEdits[rowIndex] = {}
    pendingEdits[rowIndex][field] = { old: oldVal, new: newVal }
  }
}

function handleCellClick({ row, column }: any) {
  if (column.type === 'checkbox' || column.type === 'seq') return
  selectedCell.value = { row, column }
  viewerValue.value = row[column.field] === null ? '' : String(row[column.field])
  isViewerSetNull.value = row[column.field] === null
}

function handleViewerValueChange() {
  if (!selectedCell.value) return
  const { row, column } = selectedCell.value
  row[column.field] = viewerValue.value
  recordChange(row, column.field, viewerValue.value)
}

function handleViewerNullChange() {
  if (!selectedCell.value) return
  const { row, column } = selectedCell.value
  const newVal = isViewerSetNull.value ? null : ''
  row[column.field] = newVal
  viewerValue.value = newVal === null ? '' : ''
  recordChange(row, column.field, newVal)
}

function formatJsonInViewer() {
  try {
    const obj = JSON.parse(viewerValue.value)
    viewerValue.value = JSON.stringify(obj, null, 2)
    handleViewerValueChange()
  } catch (e) {
    message.error('无效的 JSON 格式')
  }
}

function copyViewerContent() {
  navigator.clipboard.writeText(viewerValue.value)
  message.success(t('common.copy'))
}

function clearPendingEdits() {
  Object.keys(pendingEdits).forEach(k => delete pendingEdits[Number(k)])
}

function clearSelection() {
  selectedRowKeys.value = []
  ;(gridRef.value as any)?.clearCheckboxRow?.()
}

function clearViewerIfNeeded(rowIndexes?: Set<number>) {
  if (!selectedCell.value) return
  if (!rowIndexes || rowIndexes.has(selectedCell.value.row.__rowIndex)) {
    selectedCell.value = null
    viewerValue.value = ''
    isViewerSetNull.value = false
  }
}

function createGridRow(rowData: Record<string, any>, options: { isNew?: boolean } = {}): GridRow {
  const row: GridRow = {
    __rowIndex: nextRowIndex.value++,
    ...rowData,
    _originalData: { ...rowData },
  }

  if (options.isNew) {
    row._isNew = true
    row._newTouchedFields = {}
  }

  return row
}

function buildGridColumns(columnNames: string[]): NonNullable<VxeGridProps['columns']> {
  return [
    { type: 'checkbox', width: 50, fixed: 'left' },
    ...columnNames.map(col => ({
      field: col,
      title: col,
      minWidth: 120,
      showOverflow: true,
      slots: { default: 'cell_default' },
      editRender: { name: 'input' }
    }))
  ] as NonNullable<VxeGridProps['columns']>
}

function buildInsertPayload(row: GridRow) {
  const data: Record<string, any> = {}
  const missingRequired: string[] = []

  for (const column of tableColumns.value) {
    const value = row[column.name]
    const touched = Boolean(row._newTouchedFields?.[column.name])
    const hasDefault = column.default_value !== undefined && column.default_value !== null && column.default_value !== ''

    if (value === null || value === undefined) {
      if (touched) {
        if (column.nullable) {
          data[column.name] = null
        } else if (!column.is_auto_increment) {
          missingRequired.push(column.name)
        }
      } else if (!column.nullable && !column.is_auto_increment && !hasDefault) {
        missingRequired.push(column.name)
      }
      continue
    }

    data[column.name] = value
  }

  return { data, missingRequired }
}

function removeRows(rowIndexes: number[]) {
  if (rowIndexes.length === 0) return

  const rowIndexSet = new Set(rowIndexes)
  gridOptions.data = ((gridOptions.data || []) as GridRow[]).filter(row => !rowIndexSet.has(row.__rowIndex))
  rowIndexes.forEach(rowIndex => delete pendingEdits[rowIndex])
  clearSelection()
  clearViewerIfNeeded(rowIndexSet)
}

async function submitChanges() {
  const hasPendingUpdates = Object.keys(pendingEdits).length > 0
  if (hasPendingUpdates && primaryKeys.value.length === 0) return message.error(t('data.no_pk_error'))

  saving.value = true
  let shouldRefresh = false
  try {
    for (const row of newRows.value) {
      const { data, missingRequired } = buildInsertPayload(row)

      if (missingRequired.length > 0) {
        throw new Error(t('data.required_fields_missing', { fields: missingRequired.join(', ') }))
      }
      if (Object.keys(data).length === 0) {
        throw new Error(t('data.insert_empty_error'))
      }

      await dataApi.insertTableData({
        connectionId: props.connectionId,
        database: props.database,
        table: props.table,
        schema: props.schema || undefined,
        data,
      })
      shouldRefresh = true
    }

    for (const [rowIndexStr, fields] of Object.entries(pendingEdits)) {
      const rowIndex = Number(rowIndexStr)
      const row = (gridOptions.data || []).find((item: any) => item.__rowIndex === rowIndex) as GridRow | undefined
      if (!row) continue

      const whereConditions: Record<string, any> = {}
      primaryKeys.value.forEach(pk => {
        whereConditions[pk] = row._originalData[pk]
      })

      for (const [field, change] of Object.entries(fields)) {
        await dataApi.updateTableData({
          connectionId: props.connectionId, database: props.database, table: props.table,
          schema: props.schema || null, column: field, value: change.new === null ? null : String(change.new), whereConditions
        })
        row._originalData[field] = change.new
      }
    }

    clearPendingEdits()

    if (shouldRefresh) {
      await doRefresh()
    }

    message.success(t('data.save_success'))
  } catch (e: any) {
    if (shouldRefresh) {
      await doRefresh()
    }
    message.error(t('data.save_fail', { error: String(e) }))
  } finally {
    saving.value = false
  }
}

function discardChanges() {
  Modal.confirm({
    title: t('data.discard_changes'), content: t('data.discard_confirm'),
    onOk() {
      for (const [rowIndexStr, fields] of Object.entries(pendingEdits)) {
        const rowIndex = Number(rowIndexStr)
        const row = gridOptions.data?.find((r: any) => r.__rowIndex === rowIndex)
        if (row) { for (const [field, change] of Object.entries(fields)) { row[field] = change.old } }
      }
      gridOptions.data = ((gridOptions.data || []) as GridRow[]).filter(row => !row._isNew)
      clearPendingEdits()
      if (selectedCell.value?.row._isNew) {
        clearViewerIfNeeded()
      } else if (selectedCell.value) {
        viewerValue.value = selectedCell.value.row[selectedCell.value.column.field] === null ? '' : String(selectedCell.value.row[selectedCell.value.column.field])
        isViewerSetNull.value = selectedCell.value.row[selectedCell.value.column.field] === null
      }
      clearSelection()
    }
  })
}

async function refresh() {
  if (hasChanges.value) {
    return Modal.confirm({ title: t('common.refresh'), content: t('data.discard_confirm'), onOk: () => doRefresh() })
  }
  doRefresh()
}

async function doRefresh() {
  pagination.current = 1
  hasMore.value = true
  gridOptions.data = []
  nextRowIndex.value = 0
  clearPendingEdits()
  clearSelection()
  clearViewerIfNeeded()
  await loadData(false)
}

async function loadNextPage() {
  if (loading.value || !hasMore.value) return
  pagination.current++; await loadData(true)
}

async function loadData(isAppend: boolean) {
  if (!props.table) return
  loading.value = true; if (!isAppend) gridOptions.loading = true
  try {
    if (tableColumns.value.length === 0) {
      tableColumns.value = await metadataApi.getTableStructure({
        connectionId: props.connectionId,
        table: props.table,
        schema: props.schema || null,
        database: props.database
      })
      primaryKeys.value = tableColumns.value.filter(c => c.is_primary_key).map(c => c.name)
    }
    const offset = (pagination.current - 1) * pagination.pageSize
    let sql = `SELECT * FROM ${tableRef()}`
    if (filterCondition.value) sql += ` WHERE ${filterCondition.value}`
    sql += ` LIMIT ${pagination.pageSize} OFFSET ${offset}`
    const results = await queryApi.executeQuery(props.connectionId, sql, props.database)
    const result = results[0]
    const fallbackColumns = tableColumns.value.map(column => column.name)
    if (!result) {
      hasMore.value = false
      if (!isAppend) {
        gridOptions.columns = buildGridColumns(fallbackColumns)
        gridOptions.data = []
      }
      return
    }

    const visibleColumns = result.columns.length > 0 ? result.columns : fallbackColumns
    hasMore.value = result.rows.length === pagination.pageSize
    if (!isAppend) {
      gridOptions.columns = buildGridColumns(visibleColumns)
      gridOptions.data = result.rows.map(row => createGridRow(row))
    } else {
      const appendedRows = result.rows.map(row => createGridRow(row))
      gridOptions.data = [...(gridOptions.data || []), ...appendedRows]
    }
  } catch (e: any) { 
    message.error(e); pagination.current = Math.max(1, pagination.current - 1)
  } finally { 
    loading.value = false; gridOptions.loading = false 
  }
}

function handleCheckboxChange() {
  const records = gridRef.value?.getCheckboxRecords() || []
  selectedRowKeys.value = records.map((r: any) => r.__rowIndex)
}

function applyFilter() { showFilterDialog.value = false; refresh() }

function addRow() {
  const rowData = tableColumns.value.reduce<Record<string, any>>((acc, column) => {
    acc[column.name] = null
    return acc
  }, {})

  const newRow = createGridRow(rowData, { isNew: true })
  gridOptions.data = [newRow, ...((gridOptions.data || []) as GridRow[])]
}

async function deleteSelected() {
  Modal.confirm({
    title: t('common.delete'), content: t('data.delete_confirm_n', { n: selectedRowKeys.value.length }), okType: 'danger',
    async onOk() {
      try {
        const records = (gridRef.value?.getCheckboxRecords() || []) as GridRow[]
        const existingRecords = records.filter(record => !record._isNew)

        if (existingRecords.length > 0 && primaryKeys.value.length === 0) {
          message.error(t('data.no_pk_error'))
          return
        }

        for (const record of existingRecords) {
          const whereConditions: Record<string, any> = {}
          primaryKeys.value.forEach(pk => {
            whereConditions[pk] = record._originalData[pk]
          })
          
          await dataApi.deleteTableData({
            connectionId: props.connectionId, database: props.database, table: props.table,
            schema: props.schema || null, whereConditions
          })
        }

        removeRows(records.map(record => record.__rowIndex))
        message.success(t('data.delete_success'))
      } catch (e: any) { message.error(e) }
    }
  })
}

async function handleExport({ key }: any) {
  try {
    const sql = `SELECT * FROM ${tableRef()}${filterCondition.value ? ' WHERE ' + filterCondition.value : ''}`
    const path = await invoke<string>(`export_to_${key}`, { connectionId: props.connectionId, database: props.database, table: props.table, query: sql })
    message.success(t('data.export_success', { path }))
  } catch (e: any) { message.error(e) }
}

watch(
  () => [props.connectionId, props.database, props.schema, props.table],
  () => {
    primaryKeys.value = []
    tableColumns.value = []
    refresh()
  },
  { immediate: true }
)
</script>

<style scoped>
.table-data-grid { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
.grid-toolbar { display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; border-bottom: 1px solid #d9d9d9; background: #fafafa; flex-shrink: 0; }
.dark-mode .grid-toolbar { background: #1f1f1f; border-bottom-color: #303030; }
.grid-wrapper { flex: 1; min-height: 0; padding: 4px; background: #fff; }
.dark-mode .grid-wrapper { background: #1f1f1f; }
.toolbar-right { display: flex; align-items: center; }
.data-info { font-size: 12px; color: #8c8c8c; }
.loading-text { color: #1890ff; font-weight: 500; }
.end-text { color: #bfbfbf; }
.null-text { color: #bfbfbf; font-style: italic; }

:deep(.cell-modified) { background-color: #fff7e6 !important; position: relative; }
:deep(.cell-modified)::after { content: ""; position: absolute; top: 0; left: 0; border: 4px solid transparent; border-left-color: #ffa940; border-top-color: #ffa940; }
.dark-mode :deep(.cell-modified) { background-color: #3e2b1a !important; }
:deep(.cell-new-row) { background-color: #e6f4ff !important; }
.dark-mode :deep(.cell-new-row) { background-color: #11263c !important; }

.viewer-container { padding: 12px; height: 100%; display: flex; flex-direction: column; }
.viewer-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px; }
.viewer-textarea { flex: 1; font-family: monospace; }
</style>
