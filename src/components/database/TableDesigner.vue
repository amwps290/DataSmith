<template>
  <div class="table-designer">
    <div class="designer-toolbar">
      <a-space>
        <template v-if="!readOnly">
          <a-button :icon="h(SaveOutlined)" @click="saveChanges" type="primary" :loading="saving">
            {{ $t('common.save') }}
          </a-button>
          <a-button :icon="h(PlusOutlined)" @click="addColumn">
            {{ $t('designer.add_column') }}
          </a-button>
        </template>
        <a-button :icon="h(ReloadOutlined)" @click="loadStructure" :loading="loading">
          {{ $t('common.refresh') }}
        </a-button>
        <a-divider type="vertical" />
        <a-tag color="blue">{{ database }}{{ schema ? '.' + schema : '' }}.{{ table }}</a-tag>
      </a-space>
    </div>

    <div class="designer-content">
      <a-tabs v-model:activeKey="activeTab">
        <!-- 列定义 -->
        <a-tab-pane key="columns" :tab="$t('designer.columns')">
          <TableDesignerColumns
            :columns="tableColumns"
            :loading="loading"
            :read-only="readOnly"
            @remove="removeColumn"
            @move="moveColumn"
          />
        </a-tab-pane>

        <!-- 索引 -->
        <a-tab-pane key="indexes" :tab="$t('designer.indexes')">
          <TableDesignerIndexes
            :indexes="tableIndexes"
            :loading="loading"
            :read-only="readOnly"
            @add="addIndex"
            @remove="removeIndex"
          />
        </a-tab-pane>

        <!-- DDL -->
        <a-tab-pane key="ddl" :tab="$t('designer.ddl')">
          <div class="ddl-container" ref="ddlEditorContainer">
            <a-spin :spinning="loadingDDL" />
          </div>
          <div class="ddl-actions">
            <a-button :icon="h(CopyOutlined)" @click="copyDDL" size="small">
              {{ $t('data.copy_content') }}
            </a-button>
          </div>
        </a-tab-pane>

        <!-- 外键 -->
        <a-tab-pane key="foreign_keys" :tab="$t('designer.foreign_keys')" v-if="tableForeignKeys.length > 0 || !readOnly">
          <TableDesignerForeignKeys
            :foreign-keys="tableForeignKeys"
            :loading="loading"
            :read-only="readOnly"
            @add="addForeignKey"
            @remove="removeForeignKey"
          />
        </a-tab-pane>
      </a-tabs>
    </div>

    <!-- 添加索引对话框 -->
    <a-modal
      v-model:open="showAddIndexDialog"
      :title="$t('designer.add_index')"
      @ok="handleAddIndex"
    >
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item :label="$t('designer.index_name')">
          <a-input v-model:value="newIndex.name" placeholder="idx_column_name" />
        </a-form-item>
        <a-form-item :label="$t('designer.index_type')">
          <a-select v-model:value="newIndex.type">
            <a-select-option value="INDEX">NORMAL</a-select-option>
            <a-select-option value="UNIQUE">UNIQUE</a-select-option>
            <a-select-option value="FULLTEXT">FULLTEXT</a-select-option>
          </a-select>
        </a-form-item>
        <a-form-item :label="$t('designer.index_columns')">
          <a-select
            v-model:value="newIndex.columns"
            mode="multiple"
            :placeholder="$t('common.search')"
          >
            <a-select-option
              v-for="col in tableColumns"
              :key="col.name"
              :value="col.name"
            >
              {{ col.name }}
            </a-select-option>
          </a-select>
        </a-form-item>
      </a-form>
    </a-modal>

    <!-- 添加外键对话框 -->
    <a-modal
      v-model:open="showAddForeignKeyDialog"
      :title="$t('designer.add_fk')"
      @ok="handleAddForeignKey"
    >
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item :label="$t('designer.fk_name')">
          <a-input v-model:value="newForeignKey.name" placeholder="fk_column_name" />
        </a-form-item>
        <a-form-item :label="$t('designer.fk_column')">
          <a-select v-model:value="newForeignKey.column">
            <a-select-option
              v-for="col in tableColumns"
              :key="col.name"
              :value="col.name"
            >
              {{ col.name }}
            </a-select-option>
          </a-select>
        </a-form-item>
        <a-form-item :label="$t('designer.ref_table')">
          <a-input v-model:value="newForeignKey.refTable" placeholder="referenced_table" />
        </a-form-item>
        <a-form-item :label="$t('designer.ref_column')">
          <a-input v-model:value="newForeignKey.refColumn" placeholder="referenced_column" />
        </a-form-item>
        <a-form-item :label="$t('designer.on_delete')">
          <a-select v-model:value="newForeignKey.onDelete">
            <a-select-option value="CASCADE">CASCADE</a-select-option>
            <a-select-option value="SET NULL">SET NULL</a-select-option>
            <a-select-option value="RESTRICT">RESTRICT</a-select-option>
            <a-select-option value="NO ACTION">NO ACTION</a-select-option>
          </a-select>
        </a-form-item>
        <a-form-item :label="$t('designer.on_update')">
          <a-select v-model:value="newForeignKey.onUpdate">
            <a-select-option value="CASCADE">CASCADE</a-select-option>
            <a-select-option value="SET NULL">SET NULL</a-select-option>
            <a-select-option value="RESTRICT">RESTRICT</a-select-option>
            <a-select-option value="NO ACTION">NO ACTION</a-select-option>
          </a-select>
        </a-form-item>
      </a-form>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { h, reactive, ref, onMounted, watch, nextTick } from 'vue'
import {
  SaveOutlined,
  ReloadOutlined,
  PlusOutlined,
  CopyOutlined,
} from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { metadataApi, queryApi } from '@/api'
import { useI18n } from 'vue-i18n'
import { withErrorHandler } from '@/utils/errorHandler'
import { useMonacoEditor } from '@/composables/useMonacoEditor'
import TableDesignerColumns from './TableDesignerColumns.vue'
import TableDesignerIndexes from './TableDesignerIndexes.vue'
import TableDesignerForeignKeys from './TableDesignerForeignKeys.vue'
import type { ColumnInfo, IndexInfo, ForeignKeyInfo } from '@/types/database'

/** Extended column with designer metadata */
interface DesignerColumn extends ColumnInfo {
  length?: number
  _modified: boolean
  _isNew: boolean
  _originalName?: string
}

/** Extended index with designer metadata */
interface DesignerIndex extends IndexInfo {
  _isNew: boolean
}

/** Extended foreign key with designer metadata */
interface DesignerForeignKey extends ForeignKeyInfo {
  _isNew: boolean
}

const { t } = useI18n()
const props = defineProps<{
  connectionId: string
  database: string
  table: string
  schema?: string
  readOnly?: boolean
}>()

const loading = ref(false)
const saving = ref(false)
const activeTab = ref('columns')
const loadingDDL = ref(false)
const ddlSql = ref('')
const showAddIndexDialog = ref(false)
const showAddForeignKeyDialog = ref(false)

const ddlEditorContainer = ref<HTMLElement>()
const { editor: ddlEditor, setValue: setDdlValue, createEditor: createDdlEditor } = useMonacoEditor(ddlEditorContainer, {
  value: '-- Loading DDL...\n',
  language: 'sql',
  readOnly: true,
})

// 表列定义
const tableColumns = ref<DesignerColumn[]>([])
const tableIndexes = ref<DesignerIndex[]>([])
const tableForeignKeys = ref<DesignerForeignKey[]>([])

// 待处理的删除队列
const pendingDeletions = reactive({
  columns: [] as string[],
  indexes: [] as string[],
  foreignKeys: [] as string[],
})

// 新索引
const newIndex = reactive({
  name: '',
  type: 'INDEX',
  columns: [] as string[],
})

// 新外键
const newForeignKey = reactive({
  name: '',
  column: '',
  refTable: '',
  refColumn: '',
  onDelete: 'CASCADE',
  onUpdate: 'CASCADE',
})

// 加载表结构
async function loadStructure() {
  return withErrorHandler(async () => {
    loading.value = true
    const params = {
      connectionId: props.connectionId,
      table: props.table,
      schema: props.schema || null,
      database: props.database,
    }

    const [columns, indexes, foreignKeys] = await Promise.all([
      metadataApi.getTableStructure(params),
      metadataApi.getTableIndexes(params),
      metadataApi.getTableForeignKeys(params)
    ])

    tableColumns.value = columns.map(col => ({
      ...col,
      length: extractLength(col.data_type),
      data_type: extractBaseType(col.data_type),
      _modified: false, _isNew: false, _originalName: col.name,
    }))

    tableIndexes.value = indexes.map(idx => ({ ...idx, _isNew: false }))
    tableForeignKeys.value = foreignKeys.map(fk => ({ ...fk, _isNew: false }))

    // 重置删除队列
    pendingDeletions.columns = []
    pendingDeletions.indexes = []
    pendingDeletions.foreignKeys = []

    if (activeTab.value === 'ddl') loadDDL()
  }, {
    messagePrefix: t('designer.load_fail'),
    onError: () => { loading.value = false },
    showMessage: true
  }).finally(() => {
    loading.value = false
  })
}

// 提取数据类型的长度
function extractLength(dataType: string): number | undefined {
  const match = dataType.match(/\((\d+)\)/)
  return match ? parseInt(match[1]) : undefined
}

// 提取基础数据类型
function extractBaseType(dataType: string): string {
  return dataType.replace(/\(.*\)/, '').toUpperCase()
}

// 添加列
function addColumn() {
  tableColumns.value.push({
    name: `column_${tableColumns.value.length + 1}`,
    data_type: 'VARCHAR', length: 255, nullable: true,
    is_primary_key: false, is_auto_increment: false,
    default_value: undefined, comment: '', _modified: true, _isNew: true,
  })
}

// 移除列
function removeColumn(index: number) {
  const col = tableColumns.value[index]
  Modal.confirm({
    title: t('common.delete'),
    content: `${t('common.delete')} "${col.name}"?`,
    okText: t('common.delete'), okType: 'danger',
    onOk() {
      if (!col._isNew) {
        pendingDeletions.columns.push(col._originalName || col.name)
      }
      tableColumns.value.splice(index, 1)
    },
  })
}

// 移动列
function moveColumn(index: number, direction: number) {
  const newIdx = index + direction
  if (newIdx < 0 || newIdx >= tableColumns.value.length) return
  const temp = tableColumns.value[index]
  tableColumns.value[index] = tableColumns.value[newIdx]
  tableColumns.value[newIdx] = temp
  tableColumns.value[index]._modified = true
  tableColumns.value[newIdx]._modified = true
}

// 保存更改
async function saveChanges() {
  Modal.confirm({
    title: t('common.save'),
    content: t('designer.confirm_save'),
    async onOk() {
      saving.value = true
      try {
        const changes: any[] = []

        // 1. 处理列变更 (Add/Modify)
        for (const col of tableColumns.value) {
          if (!col._modified && !col._isNew) continue
          const columnInfo = {
            name: col.name, data_type: col.data_type, nullable: col.nullable,
            default_value: col.default_value || null, is_primary_key: col.is_primary_key,
            is_auto_increment: col.is_auto_increment, comment: col.comment || null,
            character_maximum_length: col.length ? Number(col.length) : null
          }
          if (col._isNew) {
            changes.push({ type: 'add_column', data: columnInfo })
          } else {
            changes.push({ type: 'modify_column', data: { old_name: col._originalName || col.name, new_column: columnInfo } })
          }
        }

        // 2. 处理删除变更
        pendingDeletions.columns.forEach(name => changes.push({ type: 'drop_column', data: name }))
        pendingDeletions.indexes.forEach(name => changes.push({ type: 'drop_index', data: name }))
        pendingDeletions.foreignKeys.forEach(name => changes.push({ type: 'drop_foreign_key', data: name }))

        // 3. 处理新增索引
        for (const idx of tableIndexes.value) {
          if (idx._isNew) {
            changes.push({ type: 'add_index', data: { ...idx, _isNew: undefined } })
          }
        }

        // 4. 处理新增外键
        for (const fk of tableForeignKeys.value) {
          if (fk._isNew) {
            changes.push({ type: 'add_foreign_key', data: { ...fk, _isNew: undefined } })
          }
        }

        if (changes.length === 0) {
          message.info(t('common.no_data')); saving.value = false; return
        }

        await queryApi.alterTableStructure({
          connectionId: props.connectionId, database: props.database, table: props.table,
          schema: props.schema || null, changes
        })

        message.success(t('designer.save_success'))
        await loadStructure()
      } catch (error: any) {
        message.error(`${t('common.fail')}: ${error}`)
      } finally {
        saving.value = false
      }
    }
  })
}

// 查看DDL
async function loadDDL() {
  loadingDDL.value = true
  try {
    const result = await metadataApi.getCreateTableDdl({
      connectionId: props.connectionId, database: props.database,
      table: props.table, schema: props.schema,
    })
    const formattedResult = result.replace(/\\n/g, '\n')
    ddlSql.value = formattedResult
    if (!ddlEditor.value) await createDdlEditor()
    if (ddlEditor.value) {
      setDdlValue(formattedResult)
      nextTick(() => ddlEditor.value?.layout())
    }
  } catch (error: any) {
    message.error(`${t('designer.ddl')} ${t('common.fail')}: ${error}`)
    if (ddlEditor.value) setDdlValue(`-- Error: ${error}`)
  } finally {
    loadingDDL.value = false
  }
}

// 复制DDL
function copyDDL() { navigator.clipboard.writeText(ddlSql.value); message.success(t('common.copy') + ' ' + t('common.ok')) }

// 添加索引
function addIndex() { newIndex.name = ''; newIndex.type = 'INDEX'; newIndex.columns = []; showAddIndexDialog.value = true }

// 处理添加索引
async function handleAddIndex() {
  if (!newIndex.name || newIndex.columns.length === 0) return
  tableIndexes.value.push({
    name: newIndex.name, columns: [...newIndex.columns],
    is_unique: newIndex.type === 'UNIQUE', is_primary: false,
    index_type: newIndex.type, _isNew: true
  })
  showAddIndexDialog.value = false
}

// 删除索引
async function removeIndex(record: any) {
  if (!record._isNew) pendingDeletions.indexes.push(record.name)
  tableIndexes.value = tableIndexes.value.filter(i => i.name !== record.name)
}

// 添加外键
function addForeignKey() {
  newForeignKey.name = ''; newForeignKey.column = ''; newForeignKey.refTable = '';
  newForeignKey.refColumn = ''; newForeignKey.onDelete = 'CASCADE'; newForeignKey.onUpdate = 'CASCADE';
  showAddForeignKeyDialog.value = true
}

// 处理添加外键
async function handleAddForeignKey() {
  if (!newForeignKey.name || !newForeignKey.column || !newForeignKey.refTable || !newForeignKey.refColumn) return
  tableForeignKeys.value.push({
    name: newForeignKey.name, column_name: newForeignKey.column,
    referenced_table_name: newForeignKey.refTable, referenced_column_name: newForeignKey.refColumn,
    update_rule: newForeignKey.onUpdate, delete_rule: newForeignKey.onDelete, _isNew: true
  })
  showAddForeignKeyDialog.value = false
}

// 删除外键
async function removeForeignKey(record: any) {
  if (!record._isNew) pendingDeletions.foreignKeys.push(record.name)
  tableForeignKeys.value = tableForeignKeys.value.filter(f => f.name !== record.name)
}

// 初始加载
onMounted(() => { loadStructure() })
watch(activeTab, (tab) => { if (tab === 'ddl') loadDDL() })
watch(() => props.table, () => { loadStructure() })
</script>

<style scoped>
.table-designer { height: 100%; display: flex; flex-direction: column; }
.designer-toolbar { display: flex; justify-content: space-between; align-items: center; padding: 12px; border-bottom: 1px solid #e8e8e8; background: #fafafa; }
.dark-mode .designer-toolbar { background: #1f1f1f; border-bottom-color: #303030; }
.designer-content { flex: 1; overflow: hidden; }
.designer-content :deep(.ant-tabs) { height: 100%; }
.designer-content :deep(.ant-tabs-content) { height: 100%; }
.ddl-container { height: calc(100vh - 300px); border: 1px solid #e8e8e8; margin: 16px; position: relative; }
.dark-mode .ddl-container { border-color: #303030; }
.ddl-actions { margin: 0 16px 16px 16px; }
</style>
