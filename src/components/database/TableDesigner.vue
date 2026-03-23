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
          <a-table
            :columns="columnEditorColumns"
            :data-source="tableColumns"
            :loading="loading"
            :pagination="false"
            :scroll="{ x: 'max-content', y: 'calc(100vh - 250px)' }"
            size="small"
            bordered
            row-key="name"
          >
            <template #bodyCell="{ column, record, index }">
              <!-- 只读模式下的单元格渲染 -->
              <template v-if="readOnly && column.dataIndex !== 'operation'">
                <span v-if="column.dataIndex === 'nullable'">
                  <a-checkbox :checked="record.nullable" disabled />
                </span>
                <span v-else-if="column.dataIndex === 'is_primary_key'">
                  <a-checkbox :checked="record.is_primary_key" disabled />
                </span>
                <span v-else-if="column.dataIndex === 'is_auto_increment'">
                  <a-checkbox :checked="record.is_auto_increment" disabled />
                </span>
                <span v-else>{{ typeof column.dataIndex === 'string' ? record[column.dataIndex] : '' }}</span>
              </template>

              <!-- 编辑模式下的单元格渲染 -->
              <template v-else-if="!readOnly">
                <!-- 列名 -->
                <template v-if="column.dataIndex === 'name'">
                  <a-input
                    v-model:value="record.name"
                    size="small"
                    :placeholder="$t('designer.column_name')"
                    @change="record._modified = true"
                  />
                </template>
                
                <!-- 数据类型 -->
                <template v-else-if="column.dataIndex === 'data_type'">
                  <a-select
                    v-model:value="record.data_type"
                    size="small"
                    style="width: 100%"
                    @change="record._modified = true"
                  >
                    <a-select-option v-for="type in dataTypes" :key="type" :value="type">
                      {{ type }}
                    </a-select-option>
                  </a-select>
                </template>
                
                <!-- 长度 -->
                <template v-else-if="column.dataIndex === 'length'">
                  <a-input-number
                    v-model:value="record.length"
                    size="small"
                    :min="1"
                    style="width: 100%"
                    @change="record._modified = true"
                  />
                </template>
                
                <!-- 可空 -->
                <template v-else-if="column.dataIndex === 'nullable'">
                  <a-checkbox
                    v-model:checked="record.nullable"
                    @change="record._modified = true"
                  />
                </template>
                
                <!-- 主键 -->
                <template v-else-if="column.dataIndex === 'is_primary_key'">
                  <a-checkbox
                    v-model:checked="record.is_primary_key"
                    @change="handlePrimaryKeyChange(record)"
                  />
                </template>
                
                <!-- 自增 -->
                <template v-else-if="column.dataIndex === 'is_auto_increment'">
                  <a-checkbox
                    v-model:checked="record.is_auto_increment"
                    @change="record._modified = true"
                  />
                </template>
                
                <!-- 默认值 -->
                <template v-else-if="column.dataIndex === 'default_value'">
                  <a-input
                    v-model:value="record.default_value"
                    size="small"
                    placeholder="NULL"
                    @change="record._modified = true"
                  />
                </template>
                
                <!-- 注释 -->
                <template v-else-if="column.dataIndex === 'comment'">
                  <a-input
                    v-model:value="record.comment"
                    size="small"
                    :placeholder="$t('designer.comment')"
                    @change="record._modified = true"
                  />
                </template>
                
                <!-- 操作 -->
                <template v-else-if="column.dataIndex === 'operation'">
                  <a-space>
                    <a-button
                      type="text"
                      size="small"
                      danger
                      :icon="h(DeleteOutlined)"
                      @click="removeColumn(index)"
                    />
                    <a-button
                      type="text"
                      size="small"
                      :icon="h(ArrowUpOutlined)"
                      @click="moveColumn(index, -1)"
                      :disabled="index === 0"
                    />
                    <a-button
                      type="text"
                      size="small"
                      :icon="h(ArrowDownOutlined)"
                      @click="moveColumn(index, 1)"
                      :disabled="index === tableColumns.length - 1"
                    />
                  </a-space>
                </template>
              </template>
            </template>
          </a-table>
        </a-tab-pane>

        <!-- 索引 -->
        <a-tab-pane key="indexes" :tab="$t('designer.indexes')">
          <div style="padding: 16px;">
            <a-space v-if="!readOnly" style="margin-bottom: 16px;">
              <a-button :icon="h(PlusOutlined)" @click="addIndex" type="primary">
                {{ $t('designer.add_index') }}
              </a-button>
            </a-space>
            
            <a-table
              :columns="readOnly ? indexColumns.filter(c => c.dataIndex !== 'operation') : indexColumns"
              :data-source="tableIndexes"
              :loading="loading"
              :pagination="false"
              size="small"
              bordered
              row-key="index_name"
            >
              <template #bodyCell="{ column, record }">
                <template v-if="column.dataIndex === 'operation'">
                  <a-button
                    type="text"
                    size="small"
                    danger
                    :icon="h(DeleteOutlined)"
                    @click="removeIndex(record)"
                  >
                    {{ $t('common.delete') }}
                  </a-button>
                </template>
              </template>
            </a-table>
          </div>
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
          <div style="padding: 16px;">
            <a-space v-if="!readOnly" style="margin-bottom: 16px;">
              <a-button :icon="h(PlusOutlined)" @click="addForeignKey" type="primary">
                {{ $t('designer.add_fk') }}
              </a-button>
            </a-space>
            
            <a-table
              :columns="readOnly ? foreignKeyColumns.filter(c => c.dataIndex !== 'operation') : foreignKeyColumns"
              :data-source="tableForeignKeys"
              :loading="loading"
              :pagination="false"
              size="small"
              bordered
              row-key="constraint_name"
            >
              <template #bodyCell="{ column, record }">
                <template v-if="column.dataIndex === 'operation'">
                  <a-button
                    type="text"
                    size="small"
                    danger
                    :icon="h(DeleteOutlined)"
                    @click="removeForeignKey(record)"
                  >
                    {{ $t('common.delete') }}
                  </a-button>
                </template>
              </template>
            </a-table>
          </div>
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
import { h, reactive, ref, onMounted, onUnmounted, watch, nextTick, computed } from 'vue'
import * as monaco from 'monaco-editor'
import {
  SaveOutlined,
  ReloadOutlined,
  PlusOutlined,
  DeleteOutlined,
  ArrowUpOutlined,
  ArrowDownOutlined,
  CopyOutlined,
} from '@ant-design/icons-vue'
import { message, Modal } from 'ant-design-vue'
import { metadataApi, queryApi } from '@/api'
import { useAppStore } from '@/stores/app'
import { useI18n } from 'vue-i18n'
import { withErrorHandler } from '@/utils/errorHandler'

const { t } = useI18n()
const props = defineProps<{
  connectionId: string
  database: string
  table: string
  schema?: string
  readOnly?: boolean
}>()

const appStore = useAppStore()
const loading = ref(false)
const saving = ref(false)
const activeTab = ref('columns')
const loadingDDL = ref(false)
const ddlSql = ref('')
const showAddIndexDialog = ref(false)
const showAddForeignKeyDialog = ref(false)

const ddlEditorContainer = ref<HTMLElement>()
let ddlEditor: monaco.editor.IStandaloneCodeEditor | null = null

// 表列定义
const tableColumns = ref<any[]>([])
const tableIndexes = ref<any[]>([])
const tableForeignKeys = ref<any[]>([])

// 数据类型列表
const dataTypes = [
  'INT', 'BIGINT', 'SMALLINT', 'TINYINT',
  'VARCHAR', 'CHAR', 'TEXT', 'LONGTEXT', 'MEDIUMTEXT',
  'DECIMAL', 'FLOAT', 'DOUBLE',
  'DATE', 'DATETIME', 'TIMESTAMP', 'TIME',
  'BOOLEAN', 'BOOL',
  'JSON',
  'BLOB', 'LONGBLOB',
]

// 列编辑器列定义
const columnEditorColumns = computed(() => [
  { title: t('designer.column_name'), dataIndex: 'name', width: 150 },
  { title: t('designer.data_type'), dataIndex: 'data_type', width: 120 },
  { title: t('designer.length'), dataIndex: 'length', width: 80 },
  { title: t('designer.nullable'), dataIndex: 'nullable', width: 60 },
  { title: t('designer.pk'), dataIndex: 'is_primary_key', width: 60 },
  { title: t('designer.auto_increment'), dataIndex: 'is_auto_increment', width: 60 },
  { title: t('designer.default_value'), dataIndex: 'default_value', width: 120 },
  { title: t('designer.comment'), dataIndex: 'comment', width: 200 },
  { title: t('common.view'), dataIndex: 'operation', width: 120, fixed: 'right' as const },
])

// 索引列定义
const indexColumns = computed(() => [
  { title: t('designer.index_name'), dataIndex: 'name', key: 'name' },
  { title: t('designer.index_columns'), dataIndex: 'columns', key: 'columns', customRender: ({ text }: any) => Array.isArray(text) ? text.join(', ') : text },
  { title: t('designer.index_type'), dataIndex: 'index_type', key: 'index_type' },
  { title: t('designer.unique'), dataIndex: 'is_unique', key: 'is_unique',
    customRender: ({ text }: any) => text ? t('common.ok') : '-' },
  { title: t('common.delete'), dataIndex: 'operation', width: 100 },
])

// 外键列定义
const foreignKeyColumns = computed(() => [
  { title: t('designer.fk_name'), dataIndex: 'name', key: 'name' },
  { title: t('designer.fk_column'), dataIndex: 'column_name', key: 'column_name' },
  { title: t('designer.ref_table'), dataIndex: 'referenced_table_name', key: 'referenced_table_name' },
  { title: t('designer.ref_column'), dataIndex: 'referenced_column_name', key: 'referenced_column_name' },
  { title: t('common.delete'), dataIndex: 'operation', width: 100 },
])

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

// 初始化 DDL 编辑器
async function initDdlEditor() {
  if (ddlEditor) return
  
  await nextTick()
  if (!ddlEditorContainer.value) return
  
  ddlEditor = monaco.editor.create(ddlEditorContainer.value, {
    value: ddlSql.value || '-- Loading DDL...\n',
    language: 'sql',
    theme: appStore.theme === 'dark' ? 'vs-dark' : 'vs',
    readOnly: true,
    automaticLayout: true,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    fontSize: 13,
    lineNumbers: 'on',
  })
}

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
      _modified: false,
      _isNew: false,
      _originalName: col.name, // 记录原始名称
    }))
    
    tableIndexes.value = indexes
    tableForeignKeys.value = foreignKeys
    
    if (activeTab.value === 'ddl') {
      loadDDL()
    }
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
    data_type: 'VARCHAR',
    length: 255,
    nullable: true,
    is_primary_key: false,
    is_auto_increment: false,
    default_value: null,
    comment: '',
    _modified: true,
    _isNew: true,
  })
}

// 移除列
function removeColumn(index: number) {
  Modal.confirm({
    title: t('common.delete'),
    content: `${t('common.delete')} "${tableColumns.value[index].name}"?`,
    okText: t('common.delete'),
    okType: 'danger',
    cancelText: t('common.cancel'),
    onOk() {
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
  
  // 标记为已修改
  tableColumns.value[index]._modified = true
  tableColumns.value[newIdx]._modified = true
}

// 处理主键变更
function handlePrimaryKeyChange(record: any) {
  record._modified = true
  if (record.is_primary_key) {
    record.nullable = false
  }
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
        for (const col of tableColumns.value) {
          if (!col._modified) continue
          
          // 构造标准列信息
          const columnInfo = {
            name: col.name,
            data_type: col.data_type,
            nullable: col.nullable,
            default_value: col.default_value || null,
            is_primary_key: col.is_primary_key,
            is_auto_increment: col.is_auto_increment,
            comment: col.comment || null,
            character_maximum_length: col.length ? Number(col.length) : null
          }

          if (col._isNew) {
            changes.push({ type: 'add_column', data: columnInfo })
          } else {
            // 注意：这里需要传入旧名，以防发生了重命名
            changes.push({ 
              type: 'modify_column', 
              data: { 
                old_name: col._originalName || col.name, 
                new_column: columnInfo 
              } 
            })
          }
        }
        
        if (changes.length === 0) {
          message.info(t('common.no_data'))
          return
        }
        
        await queryApi.alterTableStructure({
          connectionId: props.connectionId,
          database: props.database,
          table: props.table,
          schema: props.schema || null,
          changes
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
      connectionId: props.connectionId,
      database: props.database,
      table: props.table,
      schema: props.schema,
    })
    const formattedResult = result.replace(/\\n/g, '\n')
    ddlSql.value = formattedResult
    
    if (!ddlEditor) await initDdlEditor()
    if (ddlEditor) {
      ddlEditor.setValue(formattedResult)
      nextTick(() => ddlEditor?.layout())
    }
  } catch (error: any) {
    message.error(`${t('designer.ddl')} ${t('common.fail')}: ${error}`)
    if (ddlEditor) ddlEditor.setValue(`-- Error: ${error}`)
  } finally {
    loadingDDL.value = false
  }
}

// 复制DDL
function copyDDL() {
  navigator.clipboard.writeText(ddlSql.value)
  message.success(t('common.copy') + ' ' + t('common.ok'))
}

// 添加索引
function addIndex() {
  newIndex.name = ''
  newIndex.type = 'INDEX'
  newIndex.columns = []
  showAddIndexDialog.value = true
}

// 处理添加索引
async function handleAddIndex() {
  if (!newIndex.name || newIndex.columns.length === 0) return
  
  try {
    const columns = newIndex.columns.map(c => `\`${c}\``).join(', ')
    const sql = `ALTER TABLE \`${props.database}\`.\`${props.table}\` ADD ${newIndex.type} \`${newIndex.name}\` (${columns})`
    await queryApi.executeQuery(props.connectionId, sql, props.database)
    message.success(t('designer.save_success'))
    showAddIndexDialog.value = false
    await loadStructure()
  } catch (error: any) {
    message.error(error)
  }
}

// 删除索引
async function removeIndex(record: any) {
  Modal.confirm({
    title: t('common.delete'),
    content: `${t('common.delete')} "${record.name}"?`,
    async onOk() {
      try {
        const sql = `ALTER TABLE \`${props.database}\`.\`${props.table}\` DROP INDEX \`${record.name}\``
        await queryApi.executeQuery(props.connectionId, sql, props.database)
        message.success(t('common.delete') + ' ' + t('common.ok'))
        await loadStructure()
      } catch (error: any) {
        message.error(error)
      }
    },
  })
}

// 添加外键
function addForeignKey() {
  newForeignKey.name = ''
  newForeignKey.column = ''
  newForeignKey.refTable = ''
  newForeignKey.refColumn = ''
  newForeignKey.onDelete = 'CASCADE'
  newForeignKey.onUpdate = 'CASCADE'
  showAddForeignKeyDialog.value = true
}

// 处理添加外键
async function handleAddForeignKey() {
  if (!newForeignKey.name || !newForeignKey.column || !newForeignKey.refTable || !newForeignKey.refColumn) return
  
  try {
    const sql = `ALTER TABLE \`${props.database}\`.\`${props.table}\` 
      ADD CONSTRAINT \`${newForeignKey.name}\` 
      FOREIGN KEY (\`${newForeignKey.column}\`) 
      REFERENCES \`${newForeignKey.refTable}\`(\`${newForeignKey.refColumn}\`)
      ON DELETE ${newForeignKey.onDelete}
      ON UPDATE ${newForeignKey.onUpdate}`
    
    await queryApi.executeQuery(props.connectionId, sql, props.database)
    message.success(t('designer.save_success'))
    showAddForeignKeyDialog.value = false
    await loadStructure()
  } catch (error: any) {
    message.error(error)
  }
}

// 删除外键
async function removeForeignKey(record: any) {
  Modal.confirm({
    title: t('common.delete'),
    content: `${t('common.delete')} "${record.name}"?`,
    async onOk() {
      try {
        const sql = `ALTER TABLE \`${props.database}\`.\`${props.table}\` DROP FOREIGN KEY \`${record.name}\``
        await queryApi.executeQuery(props.connectionId, sql, props.database)
        message.success(t('common.delete') + ' ' + t('common.ok'))
        await loadStructure()
      } catch (error: any) {
        message.error(error)
      }
    },
  })
}

// 初始加载
onMounted(() => {
  loadStructure()
})

onUnmounted(() => {
  if (ddlEditor) ddlEditor.dispose()
})

// 监听 Tab 切换
watch(activeTab, (tab) => {
  if (tab === 'ddl') {
    loadDDL()
  }
})

// 监听主题变化
watch(() => appStore.theme, (newTheme) => {
  if (ddlEditor) monaco.editor.setTheme(newTheme === 'dark' ? 'vs-dark' : 'vs')
})

// 监听表变化
watch(() => props.table, () => {
  loadStructure()
})
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
