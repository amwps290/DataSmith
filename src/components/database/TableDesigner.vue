<template>
  <div class="table-designer">
    <div class="designer-toolbar">
      <a-space>
        <template v-if="!readOnly">
          <a-button :icon="h(SaveOutlined)" @click="saveChanges" type="primary" :loading="saving">
            保存更改
          </a-button>
          <a-button :icon="h(PlusOutlined)" @click="addColumn">
            添加列
          </a-button>
        </template>
        <a-button :icon="h(ReloadOutlined)" @click="loadStructure" :loading="loading">
          刷新
        </a-button>
        <a-divider type="vertical" />
        <a-tag color="blue">{{ database }}{{ schema ? '.' + schema : '' }}.{{ table }}</a-tag>
      </a-space>
    </div>

    <div class="designer-content">
      <a-tabs v-model:activeKey="activeTab">
        <!-- 列定义 -->
        <a-tab-pane key="columns" tab="列">
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
                    placeholder="列名"
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
                    placeholder="列注释"
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
        <a-tab-pane key="indexes" tab="索引">
          <div style="padding: 16px;">
            <a-space v-if="!readOnly" style="margin-bottom: 16px;">
              <a-button :icon="h(PlusOutlined)" @click="addIndex" type="primary">
                添加索引
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
                    删除
                  </a-button>
                </template>
              </template>
            </a-table>
          </div>
        </a-tab-pane>

        <!-- DDL -->
        <a-tab-pane key="ddl" tab="DDL">
          <div class="ddl-container" ref="ddlEditorContainer">
            <a-spin :spinning="loadingDDL" />
          </div>
          <div class="ddl-actions">
            <a-button :icon="h(CopyOutlined)" @click="copyDDL" size="small">
              复制 DDL
            </a-button>
          </div>
        </a-tab-pane>

        <!-- 外键 -->
        <a-tab-pane key="foreign_keys" tab="外键" v-if="tableForeignKeys.length > 0 || !readOnly">
          <div style="padding: 16px;">
            <a-space v-if="!readOnly" style="margin-bottom: 16px;">
              <a-button :icon="h(PlusOutlined)" @click="addForeignKey" type="primary">
                添加外键
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
                    删除
                  </a-button>
                </template>
              </template>
            </a-table>
          </div>
        </a-tab-pane>

        <!-- 表选项 -->
        <a-tab-pane key="options" tab="表选项" v-if="!readOnly">
          <div style="padding: 16px;">
            <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
              <a-form-item label="表名">
                <a-input v-model:value="tableOptions.tableName" />
              </a-form-item>
              <a-form-item label="存储引擎">
                <a-select v-model:value="tableOptions.engine">
                  <a-select-option value="InnoDB">InnoDB</a-select-option>
                  <a-select-option value="MyISAM">MyISAM</a-select-option>
                  <a-select-option value="MEMORY">MEMORY</a-select-option>
                </a-select>
              </a-form-item>
              <a-form-item label="字符集">
                <a-select v-model:value="tableOptions.charset">
                  <a-select-option value="utf8mb4">utf8mb4</a-select-option>
                  <a-select-option value="utf8">utf8</a-select-option>
                  <a-select-option value="latin1">latin1</a-select-option>
                </a-select>
              </a-form-item>
              <a-form-item label="排序规则">
                <a-select v-model:value="tableOptions.collation">
                  <a-select-option value="utf8mb4_general_ci">utf8mb4_general_ci</a-select-option>
                  <a-select-option value="utf8mb4_unicode_ci">utf8mb4_unicode_ci</a-select-option>
                </a-select>
              </a-form-item>
              <a-form-item label="表注释">
                <a-textarea v-model:value="tableOptions.comment" :rows="3" />
              </a-form-item>
            </a-form>
          </div>
        </a-tab-pane>
      </a-tabs>
    </div>
    
    <!-- 添加索引对话框 -->
    <a-modal
      v-model:open="showAddIndexDialog"
      title="添加索引"
      @ok="handleAddIndex"
    >
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item label="索引名称">
          <a-input v-model:value="newIndex.name" placeholder="idx_column_name" />
        </a-form-item>
        <a-form-item label="索引类型">
          <a-select v-model:value="newIndex.type">
            <a-select-option value="INDEX">普通索引</a-select-option>
            <a-select-option value="UNIQUE">唯一索引</a-select-option>
            <a-select-option value="FULLTEXT">全文索引</a-select-option>
          </a-select>
        </a-form-item>
        <a-form-item label="索引列">
          <a-select
            v-model:value="newIndex.columns"
            mode="multiple"
            placeholder="选择列"
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
      title="添加外键"
      @ok="handleAddForeignKey"
    >
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item label="外键名称">
          <a-input v-model:value="newForeignKey.name" placeholder="fk_column_name" />
        </a-form-item>
        <a-form-item label="列">
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
        <a-form-item label="引用表">
          <a-input v-model:value="newForeignKey.refTable" placeholder="referenced_table" />
        </a-form-item>
        <a-form-item label="引用列">
          <a-input v-model:value="newForeignKey.refColumn" placeholder="referenced_column" />
        </a-form-item>
        <a-form-item label="删除时">
          <a-select v-model:value="newForeignKey.onDelete">
            <a-select-option value="CASCADE">CASCADE</a-select-option>
            <a-select-option value="SET NULL">SET NULL</a-select-option>
            <a-select-option value="RESTRICT">RESTRICT</a-select-option>
            <a-select-option value="NO ACTION">NO ACTION</a-select-option>
          </a-select>
        </a-form-item>
        <a-form-item label="更新时">
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
import { h, reactive, ref, onMounted, onUnmounted, watch, nextTick } from 'vue'
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
import { invoke } from '@tauri-apps/api/core'
import { useAppStore } from '@/stores/app'

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
const tableOptions = reactive({
  tableName: props.table,
  engine: 'InnoDB',
  charset: 'utf8mb4',
  collation: 'utf8mb4_general_ci',
  comment: '',
})

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
const columnEditorColumns = [
  { title: '列名', dataIndex: 'name', width: 150 },
  { title: '数据类型', dataIndex: 'data_type', width: 120 },
  { title: '长度', dataIndex: 'length', width: 80 },
  { title: '可空', dataIndex: 'nullable', width: 60 },
  { title: '主键', dataIndex: 'is_primary_key', width: 60 },
  { title: '自增', dataIndex: 'is_auto_increment', width: 60 },
  { title: '默认值', dataIndex: 'default_value', width: 120 },
  { title: '注释', dataIndex: 'comment', width: 200 },
  { title: '操作', dataIndex: 'operation', width: 120, fixed: 'right' as const },
]

// 索引列定义
const indexColumns = [
  { title: '索引名', dataIndex: 'index_name', key: 'index_name' },
  { title: '列名', dataIndex: 'column_name', key: 'column_name' },
  { title: '索引类型', dataIndex: 'index_type', key: 'index_type' },
  { title: '唯一', dataIndex: 'non_unique', key: 'non_unique',
    customRender: ({ text }: any) => text === 0 ? '是' : '否' },
  { title: '操作', dataIndex: 'operation', width: 100 },
]

// 外键列定义
const foreignKeyColumns = [
  { title: '外键名', dataIndex: 'constraint_name', key: 'constraint_name' },
  { title: '列名', dataIndex: 'column_name', key: 'column_name' },
  { title: '引用表', dataIndex: 'referenced_table_name', key: 'referenced_table_name' },
  { title: '引用列', dataIndex: 'referenced_column_name', key: 'referenced_column_name' },
  { title: '操作', dataIndex: 'operation', width: 100 },
]

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
    value: ddlSql.value || '-- 正在加载 DDL...\n',
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
  loading.value = true
  try {
    const columnsPromise = invoke<any[]>('get_table_structure', {
      connectionId: props.connectionId,
      table: props.table,
      schema: props.schema || props.database,
      database: props.database,
    })
    
    const indexesPromise = invoke<any[]>('get_table_indexes', {
      connectionId: props.connectionId,
      database: props.database,
      table: props.table,
      schema: props.schema,
    })

    const foreignKeysPromise = invoke<any[]>('get_table_foreign_keys', {
      connectionId: props.connectionId,
      database: props.database,
      table: props.table,
      schema: props.schema,
    })

    const [columns, indexes, foreignKeys] = await Promise.all([columnsPromise, indexesPromise, foreignKeysPromise])
    
    tableColumns.value = columns.map(col => ({
      ...col,
      length: extractLength(col.data_type),
      data_type: extractBaseType(col.data_type),
      _modified: false,
      _isNew: false,
    }))
    
    tableIndexes.value = indexes
    tableForeignKeys.value = foreignKeys
    
    if (activeTab.value === 'ddl') {
      loadDDL()
    }
    
    message.success('表结构加载成功')
  } catch (error: any) {
    message.error(`加载表结构失败: ${error}`)
  } finally {
    loading.value = false
  }
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
    title: '确认删除',
    content: `确定要删除列 "${tableColumns.value[index].name}" 吗？`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    onOk() {
      tableColumns.value.splice(index, 1)
    },
  })
}

// 移动列
function moveColumn(index: number, direction: number) {
  const newIndex = index + direction
  if (newIndex < 0 || newIndex >= tableColumns.value.length) return
  
  const temp = tableColumns.value[index]
  tableColumns.value[index] = tableColumns.value[newIndex]
  tableColumns.value[newIndex] = temp
  
  // 标记为已修改
  tableColumns.value[index]._modified = true
  tableColumns.value[newIndex]._modified = true
}

// 处理主键变更
function handlePrimaryKeyChange(record: any) {
  record._modified = true
  // 如果设置为主键，自动设置为非空
  if (record.is_primary_key) {
    record.nullable = false
  }
}

// 保存更改
async function saveChanges() {
  saving.value = true
  try {
    const alterStatements: string[] = []
    
    for (const col of tableColumns.value) {
      if (!col._modified) continue
      
      const columnDef = buildColumnDefinition(col)
      
      if (col._isNew) {
        alterStatements.push(`ADD COLUMN ${columnDef}`)
      } else {
        alterStatements.push(`MODIFY COLUMN ${columnDef}`)
      }
    }
    
    if (alterStatements.length === 0) {
      message.info('没有需要保存的更改')
      return
    }
    
    const sql = `ALTER TABLE \`${props.database}\`.\`${props.table}\` ${alterStatements.join(', ')}`
    
    await invoke('execute_query', {
      connectionId: props.connectionId,
      sql,
    })
    
    message.success('表结构已保存')
    await loadStructure()
  } catch (error: any) {
    message.error(`保存失败: ${error}`)
  } finally {
    saving.value = false
  }
}

// 构建列定义
function buildColumnDefinition(col: any): string {
  let def = `\`${col.name}\``
  if (col.length && ['VARCHAR', 'CHAR'].includes(col.data_type)) {
    def += ` ${col.data_type}(${col.length})`
  } else {
    def += ` ${col.data_type}`
  }
  def += col.nullable ? ' NULL' : ' NOT NULL'
  if (col.is_auto_increment) def += ' AUTO_INCREMENT'
  if (col.default_value !== null && col.default_value !== undefined && col.default_value !== '') {
    def += ` DEFAULT '${col.default_value}'`
  }
  if (col.comment) {
    def += ` COMMENT '${col.comment.replace(/'/g, "''")}'`
  }
  return def
}

// 查看DDL
async function loadDDL() {
  loadingDDL.value = true
  try {
    const result = await invoke<string>('get_create_table_ddl', {
      connectionId: props.connectionId,
      database: props.database,
      table: props.table,
      schema: props.schema,
    })
    // 处理可能出现的字面量 \n
    const formattedResult = result.replace(/\\n/g, '\n')
    ddlSql.value = formattedResult
    
    if (!ddlEditor) {
      await initDdlEditor()
    }
    
    if (ddlEditor) {
      ddlEditor.setValue(formattedResult)
      nextTick(() => ddlEditor?.layout())
    }
  } catch (error: any) {
    message.error(`获取DDL失败: ${error}`)
    if (ddlEditor) ddlEditor.setValue(`-- 获取 DDL 失败: ${error}`)
  } finally {
    loadingDDL.value = false
  }
}

// 复制DDL
function copyDDL() {
  navigator.clipboard.writeText(ddlSql.value)
  message.success('DDL已复制到剪贴板')
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
  if (!newIndex.name || newIndex.columns.length === 0) {
    message.error('请填写索引名称和选择列')
    return
  }
  
  try {
    const columns = newIndex.columns.map(c => `\`${c}\``).join(', ')
    const sql = `ALTER TABLE \`${props.database}\`.\`${props.table}\` ADD ${newIndex.type} \`${newIndex.name}\` (${columns})`
    
    await invoke('execute_query', {
      connectionId: props.connectionId,
      sql,
    })
    
    message.success('索引已添加')
    showAddIndexDialog.value = false
    await loadStructure()
  } catch (error: any) {
    message.error(`添加索引失败: ${error}`)
  }
}

// 删除索引
async function removeIndex(record: any) {
  Modal.confirm({
    title: '确认删除',
    content: `确定要删除索引 "${record.index_name}" 吗？`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        const sql = `ALTER TABLE \`${props.database}\`.\`${props.table}\` DROP INDEX \`${record.index_name}\``
        await invoke('execute_query', {
          connectionId: props.connectionId,
          sql,
        })
        message.success('索引已删除')
        await loadStructure()
      } catch (error: any) {
        message.error(`删除索引失败: ${error}`)
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
  if (!newForeignKey.name || !newForeignKey.column || !newForeignKey.refTable || !newForeignKey.refColumn) {
    message.error('请填写所有必填字段')
    return
  }
  
  try {
    const sql = `ALTER TABLE \`${props.database}\`.\`${props.table}\` 
      ADD CONSTRAINT \`${newForeignKey.name}\` 
      FOREIGN KEY (\`${newForeignKey.column}\`) 
      REFERENCES \`${newForeignKey.refTable}\`(\`${newForeignKey.refColumn}\`)
      ON DELETE ${newForeignKey.onDelete}
      ON UPDATE ${newForeignKey.onUpdate}`
    
    await invoke('execute_query', {
      connectionId: props.connectionId,
      sql,
    })
    
    message.success('外键已添加')
    showAddForeignKeyDialog.value = false
    await loadStructure()
  } catch (error: any) {
    message.error(`添加外键失败: ${error}`)
  }
}

// 删除外键
async function removeForeignKey(record: any) {
  Modal.confirm({
    title: '确认删除',
    content: `确定要删除外键 "${record.constraint_name}" 吗？`,
    okText: '删除',
    okType: 'danger',
    cancelText: '取消',
    async onOk() {
      try {
        const sql = `ALTER TABLE \`${props.database}\`.\`${props.table}\` DROP FOREIGN KEY \`${record.constraint_name}\``
        await invoke('execute_query', {
          connectionId: props.connectionId,
          sql,
        })
        message.success('外键已删除')
        await loadStructure()
      } catch (error: any) {
        message.error(`删除外键失败: ${error}`)
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
.toolbar-info { display: flex; gap: 12px; align-items: center; user-select: none; }
.designer-content { flex: 1; overflow: hidden; }
.designer-content :deep(.ant-tabs) { height: 100%; }
.designer-content :deep(.ant-tabs-content) { height: 100%; }
.ddl-container { height: calc(100vh - 300px); border: 1px solid #e8e8e8; margin: 16px; position: relative; }
.dark-mode .ddl-container { border-color: #303030; }
.ddl-actions { margin: 0 16px 16px 16px; }
</style>
