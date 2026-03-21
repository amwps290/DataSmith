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
        :cell-class-name="getCellClassName"
      >
        <template #cell_default="{ row, column }">
          <span :class="{ 'null-text': row[column.field] === null }">
            {{ row[column.field] === null ? 'NULL' : row[column.field] }}
          </span>
        </template>
      </vxe-grid>
    </div>

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
import { invoke } from '@tauri-apps/api/core'
import type { QueryResult } from '@/types/database'
import { useConnectionStore } from '@/stores/connection'
import type { VxeGridProps, VxeGridInstance, VxeGridEvents } from 'vxe-table'

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
const saving = ref(false)

// 变更追踪状态
// 结构: { [rowIndex]: { [field]: { oldVal, newVal } } }
const pendingEdits = reactive<Record<number, Record<string, { old: any, new: any }>>>({})
const hasChanges = computed(() => Object.keys(pendingEdits).length > 0)
const changeCount = computed(() => {
  return Object.values(pendingEdits).reduce((acc, row) => acc + Object.keys(row).length, 0)
})

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

// 滚动触发加载
const handleScroll: VxeGridEvents.Scroll = ({ isY, scrollTop, bodyHeight, scrollHeight }) => {
  if (isY && !loading.value && hasMore.value) {
    if (scrollTop + bodyHeight + 50 >= scrollHeight) {
      loadNextPage()
    }
  }
}

function getCellClassName({ row, column }: any) {
  const rowIndex = row.__rowIndex
  if (pendingEdits[rowIndex] && pendingEdits[rowIndex][column.field]) {
    return 'cell-modified'
  }
  return ''
}

function handleEditClosed({ row, column }: any) {
  const field = column.field
  const rowIndex = row.__rowIndex
  const newVal = row[field]
  
  // 查找原始值。如果之前已经改过，从 pendingEdits 中拿最初的 oldVal
  const existingEdit = pendingEdits[rowIndex]?.[field]
  const oldVal = existingEdit ? existingEdit.old : row._originalData?.[field]

  if (newVal === oldVal) {
    // 如果改回去了，移除记录
    if (pendingEdits[rowIndex]) {
      delete pendingEdits[rowIndex][field]
      if (Object.keys(pendingEdits[rowIndex]).length === 0) delete pendingEdits[rowIndex]
    }
  } else {
    // 记录变更
    if (!pendingEdits[rowIndex]) pendingEdits[rowIndex] = {}
    pendingEdits[rowIndex][field] = { old: oldVal, new: newVal }
  }
}

async function submitChanges() {
  if (primaryKeys.value.length === 0) return message.error(t('data.no_pk_error'))
  
  saving.value = true
  try {
    for (const [rowIndexStr, fields] of Object.entries(pendingEdits)) {
      const rowIndex = Number(rowIndexStr)
      const row = gridOptions.data?.find((r: any) => r.__rowIndex === rowIndex)
      if (!row) continue

      const where = primaryKeys.value.map(pk => {
        const v = row._originalData[pk]
        return v === null ? `${quote(pk)} IS NULL` : `${quote(pk)} = '${String(v).replace(/'/g, "''")}'`
      }).join(' AND ')

      for (const [field, change] of Object.entries(fields)) {
        await invoke('update_table_data', {
          connectionId: props.connectionId,
          database: props.database,
          table: props.table,
          schema: props.schema || null,
          column: field,
          value: change.new === null ? null : String(change.new),
          whereClause: where
        })
        // 更新原始数据以防后续基于此行的再次修改
        row._originalData[field] = change.new
      }
    }
    
    Object.keys(pendingEdits).forEach(k => delete pendingEdits[Number(k)])
    message.success(t('data.update_success'))
  } catch (e: any) {
    message.error(`${t('data.update_fail')}: ${e}`)
  } finally {
    saving.value = false
  }
}

function discardChanges() {
  Modal.confirm({
    title: t('data.discard_changes'),
    content: t('data.discard_confirm'),
    onOk() {
      // 还原所有数据
      for (const [rowIndexStr, fields] of Object.entries(pendingEdits)) {
        const rowIndex = Number(rowIndexStr)
        const row = gridOptions.data?.find((r: any) => r.__rowIndex === rowIndex)
        if (row) {
          for (const [field, change] of Object.entries(fields)) {
            row[field] = change.old
          }
        }
      }
      Object.keys(pendingEdits).forEach(k => delete pendingEdits[Number(k)])
    }
  })
}

async function refresh() {
  if (hasChanges.value) {
    return Modal.confirm({
      title: t('common.refresh'),
      content: t('data.discard_confirm'),
      onOk: () => doRefresh()
    })
  }
  doRefresh()
}

async function doRefresh() {
  pagination.current = 1
  hasMore.value = true
  gridOptions.data = []
  Object.keys(pendingEdits).forEach(k => delete pendingEdits[Number(k)])
  await loadData(false)
}

async function loadNextPage() {
  if (loading.value || !hasMore.value) return
  pagination.current++
  await loadData(true)
}

async function loadData(isAppend: boolean) {
  if (!props.table) return
  loading.value = true
  if (!isAppend) gridOptions.loading = true
  
  try {
    if (primaryKeys.value.length === 0) {
      const struct = await invoke<any[]>('get_table_structure', { connectionId: props.connectionId, table: props.table, schema: props.schema || props.database, database: props.database })
      primaryKeys.value = struct.filter(c => c.is_primary_key).map(c => c.name)
    }

    const offset = (pagination.current - 1) * pagination.pageSize
    let sql = `SELECT * FROM ${tableRef()}`
    if (filterCondition.value) sql += ` WHERE ${filterCondition.value}`
    sql += ` LIMIT ${pagination.pageSize} OFFSET ${offset}`
    
    const results = await invoke<QueryResult[]>('execute_query', { connectionId: props.connectionId, sql, database: props.database })
    const result = results[0]
    
    if (!result) {
      hasMore.value = false
      if (!isAppend) { gridOptions.columns = []; gridOptions.data = [] }
      return
    }

    hasMore.value = result.rows.length === pagination.pageSize

    if (!isAppend) {
      gridOptions.columns = [
        { type: 'checkbox', width: 50, fixed: 'left' },
        ...result.columns.map(col => ({ 
          field: col, title: col, minWidth: 120, showOverflow: true, 
          slots: { default: 'cell_default' },
          editRender: { name: 'input' }
        }))
      ]
      gridOptions.data = result.rows.map((row, i) => ({ 
        __rowIndex: i, 
        ...row, 
        _originalData: { ...row } // 备份原始数据用于 WHERE 子句
      }))
    } else {
      const currentCount = gridOptions.data?.length || 0
      const newRows = result.rows.map((row, i) => ({ 
        __rowIndex: currentCount + i, 
        ...row,
        _originalData: { ...row }
      }))
      gridOptions.data = [...(gridOptions.data || []), ...newRows]
    }
  } catch (e: any) { 
    message.error(e) 
    pagination.current = Math.max(1, pagination.current - 1)
  } finally { 
    loading.value = false
    gridOptions.loading = false 
  }
}

function handleCheckboxChange() {
  const records = gridRef.value?.getCheckboxRecords() || []
  selectedRowKeys.value = records.map((r: any) => r.__rowIndex)
}

function applyFilter() { showFilterDialog.value = false; refresh() }

function addRow() { message.info(t('data.new_row_not_implemented')) }
async function deleteSelected() {
  Modal.confirm({
    title: t('common.delete'), content: t('data.delete_confirm_n', { n: selectedRowKeys.value.length }), okType: 'danger',
    async onOk() {
      try {
        const records = gridRef.value?.getCheckboxRecords() || []
        for (const record of records) {
          const where = primaryKeys.value.map(pk => {
            const v = record._originalData[pk]
            return v === null ? `${quote(pk)} IS NULL` : `${quote(pk)} = '${String(v).replace(/'/g, "''")}'`
          }).join(' AND ')
          await invoke('delete_table_data', { connectionId: props.connectionId, database: props.database, table: props.table, schema: props.schema || null, whereClause: where })
        }
        message.success(t('data.delete_success')); refresh()
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

watch(() => props.table, () => { refresh() }, { immediate: true })
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

/* 样式增强：突出显示已修改单元格 */
:deep(.cell-modified) {
  background-color: #fff7e6 !important;
  position: relative;
}
:deep(.cell-modified)::after {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  border: 4px solid transparent;
  border-left-color: #ffa940;
  border-top-color: #ffa940;
}
.dark-mode :deep(.cell-modified) {
  background-color: #3e2b1a !important;
}
</style>
