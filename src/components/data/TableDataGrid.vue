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
      
      <div class="toolbar-info">
        <div class="row-count">
          第 {{ pagination.current }} 页 (每页 {{ pagination.pageSize }} 条)
        </div>
      </div>
    </div>

    <a-table
      :columns="columns"
      :data-source="dataSource"
      :loading="loading"
      :pagination="pagination"
      :scroll="{ x: 'max-content', y: tableHeight }"
      :row-selection="rowSelection"
      :row-key="(record: any) => record.__rowIndex"
      size="small"
      bordered
      class="data-table"
      @change="handleTableChange"
    >
      <template #bodyCell="{ column, text, record, index }">
        <div
          class="editable-cell"
          @dblclick="startEdit(record, column.dataIndex, index)"
        >
          <div v-if="editingKey === `${record.__rowIndex}-${column.dataIndex}`" class="editing-wrapper">
            <a-textarea
              v-model:value="editingValue"
              @keyup.esc="cancelEdit"
              :auto-size="{ minRows: 2, maxRows: 8 }"
              ref="editInput"
              class="edit-input"
            />
            <div class="edit-buttons">
              <a-button 
                type="primary" 
                size="small" 
                :loading="saving"
                @click.stop="saveEdit(record, column.dataIndex)"
              >
                <template #icon><CheckOutlined /></template>
                保存
              </a-button>
              <a-button 
                size="small" 
                @click.stop="cancelEdit"
                :disabled="saving"
              >
                <template #icon><CloseOutlined /></template>
                取消
              </a-button>
            </div>
          </div>
          <div 
            v-else 
            class="cell-content"
            :title="text !== null && text !== undefined && String(text).length > 30 ? String(text) : undefined"
          >
            <span :class="{ null: text === null }">
              {{ text === null ? 'NULL' : text }}
            </span>
          </div>
        </div>
      </template>
    </a-table>

    <!-- 筛选对话框 -->
    <a-modal
      v-model:open="showFilterDialog"
      title="数据筛选"
      @ok="applyFilter"
    >
      <a-form layout="vertical">
        <a-form-item label="WHERE 条件">
          <a-textarea
            v-model:value="filterCondition"
            :rows="4"
            placeholder="例如: id > 100 AND status = 'active'"
          />
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { h, nextTick, ref, onMounted, onUnmounted, watch, computed, onActivated, onDeactivated } from 'vue'

onActivated(() => {
  const start = performance.now()
  console.log('[TableDataGrid] 组件已激活')
  updateTableHeight()
  console.log(`[TableDataGrid] 激活更新耗时: ${(performance.now() - start).toFixed(2)}ms`)
})

onDeactivated(() => {
  console.log('[TableDataGrid] 组件已进入缓存')
})
import {
  ReloadOutlined,
  PlusOutlined,
  DeleteOutlined,
  FilterOutlined,
  ExportOutlined,
  CheckOutlined,
  CloseOutlined,
} from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import type { QueryResult } from '@/types/database'
import { useConnectionStore } from '@/stores/connection'

const props = defineProps<{
  connectionId: string
  database: string
  table: string
  schema?: string
}>()

const connectionStore = useConnectionStore()

const dbType = computed(() => {
  const connection = connectionStore.connections.find(c => c.id === props.connectionId)
  return connection?.db_type || 'mysql'
})

const quoteIdentifier = (name: string) => {
  return dbType.value === 'sqlite' || dbType.value === 'postgresql' ? `"${name}"` : `\`${name}\``
}

const formatTableRef = () => {
  if (dbType.value === 'sqlite') return quoteIdentifier(props.table)
  if (dbType.value === 'postgresql') return `${quoteIdentifier(props.schema || 'public')}.${quoteIdentifier(props.table)}`
  return `${quoteIdentifier(props.database)}.${quoteIdentifier(props.table)}`
}

const loading = ref(false)
const dataSource = ref<any[]>([])
const columns = ref<any[]>([])
const selectedRowKeys = ref<string[]>([])
const showFilterDialog = ref(false)
const filterCondition = ref('')
const tableStructure = ref<any[]>([])
const primaryKeys = ref<string[]>([])
const tableHeight = ref(400)

function updateTableHeight() {
  const height = window.innerHeight - (64 + 40 + 56 + 48 + 40)
  tableHeight.value = Math.max(height, 200)
}

onMounted(() => {
  updateTableHeight()
  window.addEventListener('resize', updateTableHeight)
})

onUnmounted(() => {
  window.removeEventListener('resize', updateTableHeight)
})

// 高性能分页配置：不显示总数，只提供翻页
const pagination = ref({
  current: 1,
  pageSize: 100,
  total: 0, // 动态调整以实现无限翻页
  showSizeChanger: true,
  showQuickJumper: false,
  pageSizeOptions: ['50', '100', '200', '500'],
  showTotal: () => '', // 不显示总数指标
})

async function loadData() {
  if (!props.table) return
  loading.value = true
  
  try {
    // 1. 获取结构 (仅一次)
    if (tableStructure.value.length === 0) {
      const structure = await invoke<any[]>('get_table_structure', {
        connectionId: props.connectionId,
        table: props.table,
        schema: props.schema || props.database,
        database: props.database,
      })
      tableStructure.value = structure
      primaryKeys.value = structure.filter((col: any) => col.is_primary_key).map((col: any) => col.name)
    }

    // 2. 直接拉取分页数据，不执行 COUNT
    const offset = (pagination.value.current - 1) * pagination.value.pageSize
    let sql = `SELECT * FROM ${formatTableRef()}`
    if (filterCondition.value) sql += ` WHERE ${filterCondition.value}`
    sql += ` LIMIT ${pagination.value.pageSize} OFFSET ${offset}`
    
    const result = await invoke<QueryResult>('execute_query', {
      connectionId: props.connectionId,
      sql,
      database: props.database,
    })
    
    // 3. 核心技巧：根据返回行数动态“伪造”总数，让 Ant Design 的下一页按钮可用
    if (result.rows.length === pagination.value.pageSize) {
      // 如果正好填满一页，说明可能还有后续，设置 total 为当前已加载数 + 1
      pagination.value.total = offset + result.rows.length + 1
    } else {
      // 如果不满一页，说明到头了，设置 total 为当前实际总数
      pagination.value.total = offset + result.rows.length
    }

    columns.value = result.columns.map((col) => ({
      title: col, dataIndex: col, key: col, width: 150, resizable: true, sorter: true,
      ellipsis: { showTitle: false },
    }))

    dataSource.value = result.rows.map((row, index) => ({
      __rowIndex: offset + index,
      ...row,
    }))

    if (pagination.value.current === 1 && result.rows.length > 0) {
      message.success(`已加载前 ${result.rows.length} 条数据`)
    }
  } catch (error: any) {
    message.error(`加载失败: ${error}`)
  } finally {
    loading.value = false
  }
}

function handleTableChange(pag: any) {
  pagination.value.current = pag.current
  pagination.value.pageSize = pag.pageSize
  loadData()
}

const editingKey = ref('')
const editingValue = ref('')
const editInput = ref()
const saving = ref(false)

function startEdit(record: any, field: any, _index: number) {
  if (!field) return
  editingKey.value = `${record.__rowIndex}-${field}`
  editingValue.value = record[field] === null ? '' : String(record[field])
  nextTick(() => { if (editInput.value) { const el = editInput.value.$el?.querySelector('textarea') || editInput.value.$el; el?.focus(); el?.select(); } })
}

async function saveEdit(record: any, field: any) {
  if (saving.value) return
  if (primaryKeys.value.length === 0) { message.error('无主键，无法更新'); cancelEdit(); return; }
  const newValue = editingValue.value === '' ? null : editingValue.value
  if (String(record[field]) === String(newValue)) { cancelEdit(); return; }
  saving.value = true
  try {
    const whereClause = primaryKeys.value.map(pk => {
      const v = record[pk]; return v === null ? `${quoteIdentifier(pk)} IS NULL` : `${quoteIdentifier(pk)} = '${String(v).replace(/'/g, "''")}'`
    }).join(' AND ')
    await invoke('update_table_data', { connectionId: props.connectionId, database: props.database, table: props.table, schema: props.schema, column: field, value: newValue === null ? null : String(newValue), whereClause })
    record[field] = newValue; editingKey.value = ''; message.success('已更新')
  } catch (error: any) { message.error(`失败: ${error}`) } finally { saving.value = false }
}

function cancelEdit() { editingKey.value = ''; editingValue.value = '' }

async function addRow() {
  if (tableStructure.value.length === 0) return
  const newRow: any = { __rowIndex: -1, __isNew: true }
  tableStructure.value.forEach(col => { newRow[col.name] = col.default_value !== null ? col.default_value : (col.nullable ? null : '') })
  dataSource.value.unshift(newRow); message.info('已添加新行')
}

async function deleteSelected() {
  if (selectedRowKeys.value.length === 0 || primaryKeys.value.length === 0) return
  Modal.confirm({
    title: '确认删除', content: `确定删除选中行吗？`, okText: '删除', okType: 'danger',
    async onOk() {
      try {
        for (const key of selectedRowKeys.value) {
          const record = dataSource.value.find(item => item.__rowIndex === key)
          if (!record || record.__isNew) continue
          const whereClause = primaryKeys.value.map(pk => {
            const v = record[pk]; return v === null ? `${quoteIdentifier(pk)} IS NULL` : `${quoteIdentifier(pk)} = '${String(v).replace(/'/g, "''")}'`
          }).join(' AND ')
          await invoke('delete_table_data', { connectionId: props.connectionId, database: props.database, table: props.table, schema: props.schema, whereClause })
        }
        await loadData(); selectedRowKeys.value = []; message.success('已删除')
      } catch (error: any) { message.error(`失败: ${error}`) }
    },
  })
}

function applyFilter() { showFilterDialog.value = false; pagination.value.current = 1; loadData(); }

async function handleExport({ key }: any) {
  try {
    let sql = `SELECT * FROM ${formatTableRef()}${filterCondition.value ? ' WHERE ' + filterCondition.value : ''}`
    const result = await invoke<string>(`export_to_${key}`, { connectionId: props.connectionId, database: props.database, table: props.table, query: sql })
    message.success(`导出成功: ${result}`)
  } catch (error: any) { message.error(`失败: ${error}`) }
}

const rowSelection = computed(() => ({
  selectedRowKeys: selectedRowKeys.value,
  onChange: (keys: any) => { selectedRowKeys.value = keys },
}))

watch(() => props.table, () => { tableStructure.value = []; pagination.value.current = 1; loadData() }, { immediate: true })
</script>

<style scoped>
.table-data-grid { height: 100%; display: flex; flex-direction: column; overflow: hidden; background: #fff; }
.dark-mode .table-data-grid { background: #1f1f1f; }
.grid-toolbar { display: flex; justify-content: space-between; align-items: center; padding: 12px; border-bottom: 1px solid #e8e8e8; background: #fafafa; flex-shrink: 0; }
.dark-mode .grid-toolbar { background: #1f1f1f; border-bottom-color: #303030; }
.toolbar-info { display: flex; gap: 12px; align-items: center; }
.row-count { font-size: 13px; color: #8c8c8c; }
.editable-cell { min-height: 32px; padding: 4px 8px; cursor: text; position: relative; }
.editable-cell:hover { background: #f0f0f0; }
.dark-mode .editable-cell:hover { background: #262626; }
.editing-wrapper { position: fixed; z-index: 9999; background: #fff; border: 2px solid #1890ff; border-radius: 8px; padding: 16px; box-shadow: 0 6px 24px rgba(0,0,0,0.2); min-width: 400px; }
.dark-mode .editing-wrapper { background: #1f1f1f; border-color: #177ddc; }
.edit-input { width: 100%; margin-bottom: 12px; font-family: monospace; }
.edit-buttons { display: flex; gap: 8px; justify-content: flex-end; }
.cell-content { min-height: 24px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.null { color: #bfbfbf; font-style: italic; }
.data-table { flex: 1; display: flex; flex-direction: column; min-height: 0; }
:deep(.ant-table-wrapper), :deep(.ant-spin-nested-loading), :deep(.ant-spin-container), :deep(.ant-table), :deep(.ant-table-container) { flex: 1; display: flex; flex-direction: column; min-height: 0; }
:deep(.ant-table-content) { flex: 1; }
:deep(.ant-pagination.ant-table-pagination) { margin: 0 !important; padding: 12px 16px !important; background: #fafafa; border-top: 1px solid #f0f0f0; width: 100%; flex-shrink: 0; }
.dark-mode :deep(.ant-pagination.ant-table-pagination) { background: #1f1f1f !important; border-top-color: #303030 !important; }
.dark-mode :deep(.ant-pagination) { color: rgba(255,255,255,0.85) !important; }
.dark-mode :deep(.ant-pagination-item), .dark-mode :deep(.ant-pagination-prev .ant-pagination-item-link), .dark-mode :deep(.ant-pagination-next .ant-pagination-item-link) { background-color: #262626 !important; border-color: #434343 !important; color: rgba(255,255,255,0.65) !important; }
.dark-mode :deep(.ant-pagination-item-active) { background-color: #177ddc !important; border-color: #177ddc !important; }
.dark-mode :deep(.ant-pagination-item-active a) { color: #fff !important; }
:deep(.ant-table-thead > tr > th) { background: #fafafa; font-weight: 600; padding: 10px 12px; }
.dark-mode :deep(.ant-table-thead > tr > th) { background: #1d1d1d !important; color: rgba(255,255,255,0.85) !important; border-bottom: 1px solid #303030 !important; }
.dark-mode :deep(.ant-table) { background-color: #1f1f1f !important; }
.dark-mode :deep(.ant-table-thead), .dark-mode :deep(.ant-table-header) { background-color: #1d1d1d !important; }
.dark-mode :deep(.ant-table-container) { background-color: #1f1f1f !important; border-color: #303030 !important; }
:deep(.ant-table-tbody > tr > td) { padding: 8px 12px; }
.dark-mode :deep(.ant-table-tbody > tr:hover > td) { background: #262626 !important; }
.dark-mode :deep(.ant-table-cell) { border-bottom-color: #303030 !important; border-right-color: #303030 !important; }
</style>
