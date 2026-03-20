<template>
  <div class="table-data-grid">
    <div class="grid-toolbar">
      <a-space>
        <a-button :icon="h(ReloadOutlined)" @click="loadData" :loading="loading">
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
        <div class="pagination-info">
          第 {{ pagination.current }} 页 (每页 {{ pagination.pageSize }} 条)
          <a-divider type="vertical" />
          <a-button-group size="small">
            <a-button :disabled="pagination.current <= 1" @click="changePage(-1)"><LeftOutlined /></a-button>
            <a-button :disabled="!hasMore" @click="changePage(1)"><RightOutlined /></a-button>
          </a-button-group>
        </div>
      </div>
    </div>

    <!-- 高性能虚拟滚动表格 -->
    <div class="grid-wrapper">
      <vxe-grid ref="gridRef" v-bind="gridOptions" @cell-dblclick="handleCellDblClick" @checkbox-change="handleCheckboxChange" @checkbox-all="handleCheckboxChange">
        <template #cell_default="{ row, column }">
          <span :class="{ 'null-text': row[column.field] === null }">
            {{ row[column.field] === null ? 'NULL' : row[column.field] }}
          </span>
        </template>
      </vxe-grid>
    </div>

    <!-- 单元格编辑器 (弹窗式以处理大数据内容) -->
    <a-modal
      v-model:open="showEditor"
      :title="`编辑单元格: ${editingField}`"
      @ok="saveEdit"
      width="600px"
      :confirm-loading="saving"
    >
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
import { h, ref, onMounted, onUnmounted, watch, computed, reactive } from 'vue'
import {
  ReloadOutlined, PlusOutlined, DeleteOutlined, FilterOutlined,
  ExportOutlined, LeftOutlined, RightOutlined
} from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import type { QueryResult } from '@/types/database'
import { useConnectionStore } from '@/stores/connection'
import type { VxeGridProps, VxeGridInstance } from 'vxe-table'

const props = defineProps<{ connectionId: string; database: string; table: string; schema?: string }>()
const connectionStore = useConnectionStore()
const gridRef = ref<VxeGridInstance>()

const loading = ref(false)
const hasMore = ref(false)
const selectedRowKeys = ref<any[]>([])
const showFilterDialog = ref(false)
const filterCondition = ref('')
const primaryKeys = ref<string[]>([])
const tableHeight = ref(400)

const pagination = reactive({ current: 1, pageSize: 100 })

const dbType = computed(() => connectionStore.connections.find(c => c.id === props.connectionId)?.db_type || 'mysql')
const quote = (n: string) => dbType.value === 'sqlite' || dbType.value === 'postgresql' ? `"${n}"` : `\`${n}\``
const tableRef = () => dbType.value === 'postgresql' ? `${quote(props.schema || 'public')}.${quote(props.table)}` : quote(props.table)

const gridOptions = reactive<VxeGridProps>({
  border: true,
  height: 'auto',
  loading: false,
  columnConfig: { resizable: true, drag: true },
  rowConfig: { isCurrent: true, isHover: true, keyField: '__rowIndex' },
  checkboxConfig: { reserve: true, trigger: 'cell' },
  scrollX: { enabled: true, gt: 20 },
  scrollY: { enabled: true, gt: 50 }, // 开启纵向虚拟滚动
  columns: [],
  data: []
})

function updateHeight() { tableHeight.value = window.innerHeight - 200 }
onMounted(() => { updateHeight(); window.addEventListener('resize', updateHeight); })
onUnmounted(() => { window.removeEventListener('resize', updateHeight) })

async function loadData() {
  if (!props.table) return
  loading.value = true
  gridOptions.loading = true
  try {
    // 获取主键
    const struct = await invoke<any[]>('get_table_structure', { connectionId: props.connectionId, table: props.table, schema: props.schema || props.database, database: props.database })
    primaryKeys.value = struct.filter(c => c.is_primary_key).map(c => c.name)

    const offset = (pagination.current - 1) * pagination.pageSize
    let sql = `SELECT * FROM ${tableRef()}`
    if (filterCondition.value) sql += ` WHERE ${filterCondition.value}`
    sql += ` LIMIT ${pagination.pageSize} OFFSET ${offset}`
    
    const result = await invoke<QueryResult>('execute_query', { connectionId: props.connectionId, sql, database: props.database })
    hasMore.value = result.rows.length === pagination.pageSize

    gridOptions.columns = [
      { type: 'checkbox', width: 50, fixed: 'left' },
      { type: 'seq', title: '#', width: 60, fixed: 'left' },
      ...result.columns.map(col => ({ field: col, title: col, minWidth: 120, slots: { default: 'cell_default' } }))
    ]
    gridOptions.data = result.rows.map((row, i) => ({ __rowIndex: offset + i, ...row }))
  } catch (e: any) { message.error(e) } finally { loading.value = false; gridOptions.loading = false }
}

function handleCheckboxChange() {
  const records = gridRef.value?.getCheckboxRecords() || []
  selectedRowKeys.value = records.map(r => r.__rowIndex)
}

function changePage(delta: number) { pagination.current += delta; loadData() }
function applyFilter() { showFilterDialog.value = false; pagination.current = 1; loadData() }

// 编辑逻辑
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
        message.success('删除成功'); loadData()
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

watch(() => props.table, () => { pagination.current = 1; loadData() }, { immediate: true })
</script>

<style scoped>
.table-data-grid { height: 100%; display: flex; flex-direction: column; overflow: hidden; }
.grid-toolbar { display: flex; justify-content: space-between; align-items: center; padding: 8px 12px; border-bottom: 1px solid #d9d9d9; background: #fafafa; flex-shrink: 0; }
.dark-mode .grid-toolbar { background: #1f1f1f; border-bottom-color: #303030; }
.grid-wrapper { flex: 1; min-height: 0; padding: 4px; background: #fff; }
.dark-mode .grid-wrapper { background: #1f1f1f; }
.toolbar-right { display: flex; align-items: center; }
.pagination-info { font-size: 12px; color: #8c8c8c; }
.null-text { color: #bfbfbf; font-style: italic; }
.editor-textarea { font-family: monospace; }
</style>
