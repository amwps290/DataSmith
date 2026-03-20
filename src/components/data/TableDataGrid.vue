<template>
  <div class="table-data-grid">
    <div class="grid-toolbar">
      <a-space>
        <a-button :icon="h(ReloadOutlined)" @click="refresh" :loading="loading">
          刷新
        </a-button>
        <a-button :icon="h(PlusOutlined)" @click="addRow">
          新增
        </a-button>
        <a-button 
          :icon="h(DeleteOutlined)" 
          danger 
          :disabled="selectedRowKeys.length === 0"
          @click="deleteSelected"
        >
          删除
        </a-button>
        <a-divider type="vertical" />
        <a-button :icon="h(FilterOutlined)" @click="showFilterDialog = true">
          筛选
        </a-button>
        <a-dropdown>
          <template #overlay>
            <a-menu @click="handleExport">
              <a-menu-item key="csv">导出为 CSV</a-menu-item>
              <a-menu-item key="json">导出为 JSON</a-menu-item>
              <a-menu-item key="sql">导出为 SQL INSERT</a-menu-item>
            </a-menu>
          </template>
          <a-button :icon="h(ExportOutlined)">
            导出
          </a-button>
        </a-dropdown>
      </a-space>
      
      <div class="toolbar-right">
        <div class="data-info">
          已加载 {{ gridOptions.data?.length || 0 }} 行
          <span v-if="loading" class="loading-text">
            <a-spin size="small" style="margin-left: 8px" /> 加载中...
          </span>
          <span v-else-if="!hasMore" class="end-text"> (已加载全部)</span>
        </div>
      </div>
    </div>

    <!-- 高性能虚拟滚动表格 + 滚动加载 -->
    <div class="grid-wrapper">
      <vxe-grid 
        ref="gridRef" 
        v-bind="gridOptions" 
        @cell-dblclick="handleCellDblClick" 
        @checkbox-change="handleCheckboxChange" 
        @checkbox-all="handleCheckboxChange"
        @scroll="handleScroll"
      >
        <template #cell_default="{ row, column }">
          <span :class="{ 'null-text': row[column.field] === null }">
            {{ row[column.field] === null ? 'NULL' : row[column.field] }}
          </span>
        </template>
      </vxe-grid>
    </div>

    <!-- 单元格编辑器 -->
    <a-modal v-model:open="showEditor" :title="`编辑单元格: ${editingField}`" @ok="saveEdit" width="600px" :confirm-loading="saving">
      <a-form layout="vertical">
        <a-form-item label="当前值">
          <a-textarea v-model:value="editingValue" :rows="8" class="editor-textarea" />
        </a-form-item>
        <a-checkbox v-model:checked="isSetNull">设为 NULL</a-checkbox>
      </a-form>
    </a-modal>

    <!-- 筛选对话框 -->
    <a-modal v-model:open="showFilterDialog" title="数据筛选" @ok="applyFilter">
      <a-form layout="vertical">
        <a-form-item label="WHERE 条件">
          <a-textarea v-model:value="filterCondition" :rows="4" placeholder="例如: id > 100 AND status = 'active'" />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { h, ref, watch, computed, reactive } from 'vue'
import {
  ReloadOutlined, PlusOutlined, DeleteOutlined, FilterOutlined,
  ExportOutlined
} from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import type { QueryResult } from '@/types/database'
import { useConnectionStore } from '@/stores/connection'
import type { VxeGridProps, VxeGridInstance, VxeGridEvents } from 'vxe-table'

const props = defineProps<{ connectionId: string; database: string; table: string; schema?: string }>()
const connectionStore = useConnectionStore()
const gridRef = ref<VxeGridInstance>()

const loading = ref(false)
const hasMore = ref(true)
const selectedRowKeys = ref<any[]>([])
const showFilterDialog = ref(false)
const filterCondition = ref('')
const primaryKeys = ref<string[]>([])

const pagination = reactive({ current: 1, pageSize: 100 })

const dbType = computed(() => connectionStore.connections.find(c => c.id === props.connectionId)?.db_type || 'mysql')
const quote = (n: string) => dbType.value === 'sqlite' || dbType.value === 'postgresql' ? `"${n}"` : `\`${n}\``
const tableRef = () => dbType.value === 'postgresql' ? `${quote(props.schema || 'public')}.${quote(props.table)}` : quote(props.table)

const gridOptions = reactive<VxeGridProps>({
  border: true,
  height: 'auto',
  loading: false,
  columnConfig: { resizable: true, drag: true },
  rowConfig: { isCurrent: true, isHover: true, keyField: '__rowIndex', height: 36 },
  checkboxConfig: { reserve: true, trigger: 'cell' },
  scrollX: { enabled: true, gt: 20 },
  scrollY: { enabled: true, gt: 0 }, // 必须开启虚拟滚动以支持大数据追加
  columns: [],
  data: []
})

// 滚动触发加载
const handleScroll: VxeGridEvents.Scroll = ({ isY, scrollTop, bodyHeight, scrollHeight }) => {
  if (isY && !loading.value && hasMore.value) {
    // 距离底部 50px 时加载
    if (scrollTop + bodyHeight + 50 >= scrollHeight) {
      loadNextPage()
    }
  }
}

async function refresh() {
  pagination.current = 1
  hasMore.value = true
  gridOptions.data = []
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
    // 获取主键
    if (primaryKeys.value.length === 0) {
      const struct = await invoke<any[]>('get_table_structure', { connectionId: props.connectionId, table: props.table, schema: props.schema || props.database, database: props.database })
      primaryKeys.value = struct.filter(c => c.is_primary_key).map(c => c.name)
    }

    const offset = (pagination.current - 1) * pagination.pageSize
    let sql = `SELECT * FROM ${tableRef()}`
    if (filterCondition.value) sql += ` WHERE ${filterCondition.value}`
    sql += ` LIMIT ${pagination.pageSize} OFFSET ${offset}`
    
    const result = await invoke<QueryResult>('execute_query', { connectionId: props.connectionId, sql, database: props.database })
    
    // 如果返回的数据少于页大小，说明没有更多了
    hasMore.value = result.rows.length === pagination.pageSize

    if (!isAppend) {
      gridOptions.columns = [
        { type: 'checkbox', width: 50, fixed: 'left' },
        ...result.columns.map(col => ({ 
          field: col, title: col, minWidth: 120, showOverflow: true, slots: { default: 'cell_default' } 
        }))
      ]
      gridOptions.data = result.rows.map((row, i) => ({ __rowIndex: i, ...row }))
    } else {
      // 追加模式
      const currentCount = gridOptions.data?.length || 0
      const newRows = result.rows.map((row, i) => ({ __rowIndex: currentCount + i, ...row }))
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

// 编辑与删除逻辑保持不变 (已适配)
const showEditor = ref(false), editingField = ref(''), editingRow = ref<any>(null), editingValue = ref(''), isSetNull = ref(false), saving = ref(false)
function handleCellDblClick({ row, column }: any) {
  if (column.type === 'checkbox' || column.type === 'seq') return
  editingRow.value = row; editingField.value = column.field
  editingValue.value = row[column.field] === null ? '' : String(row[column.field])
  isSetNull.value = row[column.field] === null
  showEditor.value = true
}

async function saveEdit() {
  if (primaryKeys.value.length === 0) return message.error('无主键，无法执行精准更新')
  saving.value = true
  const newValue = isSetNull.value ? null : editingValue.value
  try {
    const where = primaryKeys.value.map(pk => {
      const v = editingRow.value[pk]; return v === null ? `${quote(pk)} IS NULL` : `${quote(pk)} = '${String(v).replace(/'/g, "''")}'`
    }).join(' AND ')
    await invoke('update_table_data', { connectionId: props.connectionId, database: props.database, table: props.table, schema: props.schema, column: editingField.value, value: newValue === null ? null : String(newValue), whereClause: where })
    editingRow.value[editingField.value] = newValue
    showEditor.value = false; message.success('更新成功')
  } catch (e: any) { message.error(e) } finally { saving.value = false }
}

function addRow() { message.info('新增行功能待完善') }
async function deleteSelected() {
  Modal.confirm({
    title: '确认删除', content: `确定删除选中的 ${selectedRowKeys.value.length} 行数据吗？`, okType: 'danger',
    async onOk() {
      try {
        const records = gridRef.value?.getCheckboxRecords() || []
        for (const record of records) {
          const where = primaryKeys.value.map(pk => {
            const v = record[pk]; return v === null ? `${quote(pk)} IS NULL` : `${quote(pk)} = '${String(v).replace(/'/g, "''")}'`
          }).join(' AND ')
          await invoke('delete_table_data', { connectionId: props.connectionId, database: props.database, table: props.table, schema: props.schema, whereClause: where })
        }
        message.success('删除成功'); refresh()
      } catch (e: any) { message.error(e) }
    }
  })
}

async function handleExport({ key }: any) {
  try {
    const sql = `SELECT * FROM ${tableRef()}${filterCondition.value ? ' WHERE ' + filterCondition.value : ''}`
    const path = await invoke<string>(`export_to_${key}`, { connectionId: props.connectionId, database: props.database, table: props.table, query: sql })
    message.success(`导出完成: ${path}`)
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
.editor-textarea { font-family: monospace; }
</style>
