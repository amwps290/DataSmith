<template>
  <div class="database-tree">
    <a-spin :spinning="loading" :tip="$t('common.loading')">
      <div class="custom-tree">
        <div v-for="node in filteredTreeData" :key="node.key" class="tree-node-wrapper">
          <TreeNodeItem
            :node="node"
            :level="0"
            :expanded-keys="expandedKeys"
            :selected-keys="selectedKeys"
            :loading-nodes="loadingNodes"
            @toggle="handleToggle"
            @select="handleSelect"
            @dblclick="handleDoubleClick"
            @contextmenu="onRightClick"
          />
        </div>
      </div>
      <a-empty v-if="!loading && filteredTreeData.length === 0" :description="searchValue ? $t('tree.no_data') : '请选择连接'" :image-style="{ height: '60px' }" />
    </a-spin>

    <!-- 右键菜单 -->
    <div v-if="contextMenuVisible" class="context-menu-overlay" @click="contextMenuVisible = false">
      <div class="context-menu" :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }" @click.stop>
        <a-menu @click="handleMenuClick" size="small">
          <template v-if="selectedNode?.type === 'database'">
            <a-menu-item key="new-query"><template #icon><FileTextOutlined /></template>{{ $t('tree.new_query') }}</a-menu-item>
            <a-menu-item key="open-scripts"><template #icon><FolderOpenOutlined /></template>{{ $t('tree.open_scripts') }}</a-menu-item>
            <a-menu-divider />
            <a-menu-item key="refresh"><template #icon><ReloadOutlined /></template>{{ $t('tree.refresh_db') }}</a-menu-item>
          </template>
          
          <template v-else-if="['schema', 'tables', 'views', 'schemas', 'functions', 'schema-tables', 'schema-views', 'schema-functions', 'schema-indexes'].includes(selectedNode?.type || '')">
            <a-menu-item key="refresh"><template #icon><ReloadOutlined /></template>{{ $t('common.refresh') }}</a-menu-item>
          </template>
          
          <template v-if="selectedNode?.type === 'table'">
            <a-menu-item key="view-data"><template #icon><TableOutlined /></template>{{ $t('tree.view_data') }}</a-menu-item>
            <a-menu-item key="view-ddl"><template #icon><CodeOutlined /></template>{{ $t('tree.view_ddl') }}</a-menu-item>
            <a-menu-item key="design-table"><template #icon><EditOutlined /></template>{{ $t('tree.design_table') }}</a-menu-item>
            <a-menu-divider />
            <a-menu-item key="refresh"><template #icon><ReloadOutlined /></template>{{ $t('common.refresh') }}</a-menu-item>
            <a-menu-item key="drop-table" danger><template #icon><DeleteOutlined /></template>{{ $t('tree.drop_table') }}</a-menu-item>
          </template>

          <a-menu-item key="copy-name"><template #icon><CopyOutlined /></template>{{ $t('tree.copy_name') }}</a-menu-item>
        </a-menu>
      </div>
    </div>

    <!-- DDL 预览弹窗 -->
    <a-modal v-model:open="showDdlModal" :title="`DDL: ${selectedNode?.title}`" width="800px" :footer="null">
      <div ref="ddlEditorContainer" style="height: 500px; border: 1px solid #d9d9d9"></div>
    </a-modal>

    <!-- 脚本列表弹窗 -->
    <a-modal v-model:open="showScriptsModal" :title="$t('tree.open_scripts')" :footer="null" width="500px">
      <a-list :data-source="savedScripts" :loading="loadingScripts" size="small">
        <template #renderItem="{ item }">
          <a-list-item @click="openSavedScript(item)" class="script-item">
            <a-list-item-meta :title="item.name">
              <template #description>{{ new Date(item.last_modified * 1000).toLocaleString() }}</template>
            </a-list-item-meta>
          </a-list-item>
        </template>
      </a-list>
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import * as monaco from 'monaco-editor'
import {
  TableOutlined, ReloadOutlined, CopyOutlined,
  FolderOpenOutlined, DeleteOutlined, EditOutlined,
  FileTextOutlined, CodeOutlined
} from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import type { DatabaseInfo } from '@/types/database'
import { useConnectionStore } from '@/stores/connection'
import { useAppStore } from '@/stores/app'
import TreeNodeItem from './TreeNodeItem.vue'

interface TreeNode {
  key: string; title: string; type: string; children?: TreeNode[];
  isLeaf?: boolean; metadata?: any; isAutoExpanded?: boolean;
}

const { t } = useI18n()
const appStore = useAppStore()
const props = defineProps<{ connectionId: string | null; dbType?: string; searchValue?: string; }>()
const emit = defineEmits(['table-selected', 'database-selected', 'new-query', 'design-table', 'view-structure', 'open-scripts'])
const connectionStore = useConnectionStore()

const loading = ref(false), treeData = ref<TreeNode[]>([]), expandedKeys = ref<string[]>([]), selectedKeys = ref<string[]>([]), loadingNodes = ref<Set<string>>(new Set())
const contextMenuVisible = ref(false), contextMenuX = ref(0), contextMenuY = ref(0), selectedNode = ref<TreeNode | null>(null)

const showDdlModal = ref(false), ddlEditorContainer = ref<HTMLElement>()
let ddlEditor: monaco.editor.IStandaloneCodeEditor | null = null

const filteredTreeData = computed(() => {
  if (!props.searchValue) return treeData.value
  const search = props.searchValue.toLowerCase()
  const filter = (nodes: TreeNode[]): TreeNode[] => {
    return nodes.reduce((acc, node) => {
      const children = node.children ? filter(node.children) : []
      if (node.title.toLowerCase().includes(search) || children.length > 0) {
        acc.push({ ...node, children, isAutoExpanded: children.length > 0 })
      }
      return acc
    }, [] as TreeNode[])
  }
  return filter(treeData.value)
})

async function loadDatabases() {
  if (!props.connectionId) return
  loading.value = true
  try {
    if (props.dbType === 'sqlite') {
      treeData.value = [{ key: 'db-main', title: 'main', type: 'database', isLeaf: false, metadata: { name: 'main', database: 'main' } }]
    } else {
      const dbs = await invoke<DatabaseInfo[]>('get_databases', { connectionId: props.connectionId })
      treeData.value = dbs.map(db => ({ key: `db-${db.name}`, title: db.name, type: 'database', isLeaf: false, metadata: db }))
    }
  } catch (e: any) { message.error(e) } finally { loading.value = false }
}

async function handleRefreshNode(node: TreeNode) {
  expandedKeys.value = expandedKeys.value.filter(k => k !== node.key)
  updateNodeInTree(treeData.value, node.key, (target) => { target.children = undefined })
  treeData.value = [...treeData.value]
  message.success(t('common.refresh'))
}

function updateNodeInTree(nodes: TreeNode[], targetKey: string, updater: (node: TreeNode) => void): boolean {
  for (const node of nodes) {
    if (node.key === targetKey) { updater(node); return true }
    if (node.children && updateNodeInTree(node.children, targetKey, updater)) return true
  }
  return false
}

async function onLoadData(treeNode: TreeNode) {
  if (treeNode.children && treeNode.children.length > 0) return
  const connId = props.connectionId
  if (!connId) return

  if (treeNode.type === 'database') {
    const dbName = treeNode.metadata.name
    let children: TreeNode[] = []
    if (props.dbType === 'postgresql') {
      children = [
        { key: `${treeNode.key}-schemas`, title: 'Schemas', type: 'schemas', isLeaf: false, metadata: { database: dbName } },
        { key: `${treeNode.key}-extensions`, title: t('tree.extensions'), type: 'database-extensions', isLeaf: false, metadata: { database: dbName } }
      ]
    } else {
      children = [
        { key: `${treeNode.key}-tables`, title: t('tree.tables'), type: 'tables', isLeaf: false, metadata: { database: dbName } },
        { key: `${treeNode.key}-views`, title: t('tree.views'), type: 'views', isLeaf: false, metadata: { database: dbName } }
      ]
    }
    updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children)
    treeData.value = [...treeData.value]
  }
  else if (treeNode.type === 'schemas') {
    try {
      const res = await invoke<any[]>('get_schemas', { connectionId: connId, database: treeNode.metadata.database })
      const children = res.map(s => ({ key: `${treeNode.key}-${s.name}`, title: s.name, type: 'schema', isLeaf: false, metadata: { database: treeNode.metadata.database, name: s.name } }))
      updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children)
      treeData.value = [...treeData.value]
    } catch (e: any) { message.error(e) }
  }
  else if (treeNode.type === 'schema') {
    const db = treeNode.metadata.database, schema = treeNode.metadata.name
    const children = [
      { key: `${treeNode.key}-tables`, title: t('tree.tables'), type: 'schema-tables', isLeaf: false, metadata: { database: db, schema } },
      { key: `${treeNode.key}-views`, title: t('tree.views'), type: 'schema-views', isLeaf: false, metadata: { database: db, schema } },
      { key: `${treeNode.key}-indexes`, title: t('tree.indexes'), type: 'schema-indexes', isLeaf: false, metadata: { database: db, schema } },
      { key: `${treeNode.key}-functions`, title: t('tree.functions'), type: 'schema-functions', isLeaf: false, metadata: { database: db, schema } },
      { key: `${treeNode.key}-aggregates`, title: t('tree.aggregates'), type: 'schema-aggregates', isLeaf: false, metadata: { database: db, schema } }
    ]
    updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children)
    treeData.value = [...treeData.value]
  }
  else if (['schema-tables', 'schema-views', 'tables', 'views'].includes(treeNode.type)) {
    const isSchema = treeNode.type.startsWith('schema-'), method = treeNode.type.includes('views') ? 'get_views' : (isSchema ? 'get_schema_tables' : 'get_tables')
    try {
      const res = await invoke<any[]>(method, { connectionId: connId, database: treeNode.metadata.database, schema: treeNode.metadata.schema || null })
      const children = res.map(t => ({ key: `${treeNode.key}-${t.name}`, title: t.name, type: treeNode.type.includes('views') ? 'view' : 'table', isLeaf: false, metadata: { ...t, database: treeNode.metadata.database, schema: treeNode.metadata.schema } }))
      updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children.length ? children : [{ key: `${treeNode.key}-empty`, title: t('tree.empty'), type: 'empty', isLeaf: true }])
      treeData.value = [...treeData.value]
    } catch (e: any) { message.error(e) }
  }
  else if (['schema-indexes', 'schema-functions', 'schema-aggregates', 'database-extensions'].includes(treeNode.type)) {
    const isExtension = treeNode.type === 'database-extensions'
    const isFunction = treeNode.type === 'schema-functions'
    const isAggregate = treeNode.type === 'schema-aggregates'
    const isIndex = treeNode.type === 'schema-indexes'
    
    let method = 'get_database_extensions'
    if (isIndex) method = 'get_schema_indexes'
    else if (isFunction) method = 'get_schema_functions'
    else if (isAggregate) method = 'get_schema_aggregate_functions'
    
    try {
      const params: any = { connectionId: connId, database: treeNode.metadata.database }
      if (!isExtension) params.schema = treeNode.metadata.schema
      
      const res = await invoke<any[]>(method, params)
      const children = res.map(item => {
        let title = item.name || item.index_name
        
        // 针对函数和聚合函数，拼接参数列表
        if ((isFunction || isAggregate) && item.arguments) {
          title = `${item.name}(${item.arguments})`
        }
        // 针对索引类型，拼接关联列名
        else if (isIndex && item.columns && item.columns.length > 0) {
          title = `${item.name} (${item.columns.join(', ')})`
        }
        
        return { 
          key: `${treeNode.key}-${item.name || item.index_name}`, 
          title, 
          type: 'leaf', 
          isLeaf: true, 
          metadata: { ...item, database: treeNode.metadata.database, schema: treeNode.metadata.schema } 
        }
      })
      updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children.length ? children : [{ key: `${treeNode.key}-empty`, title: t('tree.empty'), type: 'empty', isLeaf: true }])
      treeData.value = [...treeData.value]
    } catch (e: any) { message.error(e) }
  }
  else if (['table', 'view'].includes(treeNode.type)) {
    try {
      const res = await invoke<any[]>('get_table_structure', { connectionId: connId, table: treeNode.metadata.name || treeNode.title, database: treeNode.metadata.database, schema: treeNode.metadata.schema })
      const children = res.map(c => ({ key: `${treeNode.key}-col-${c.name}`, title: `${c.name}${c.data_type ? ' : ' + c.data_type : ''}${c.is_primary_key ? ' [PK]' : ''}`, type: 'column', isLeaf: true, metadata: { ...c, database: treeNode.metadata.database, table: treeNode.metadata.name, schema: treeNode.metadata.schema } }))
      updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children.length ? children : [{ key: `${treeNode.key}-empty`, title: t('tree.empty'), type: 'empty', isLeaf: true }])
      treeData.value = [...treeData.value]
    } catch (e: any) { message.error(e) }
  }
}

async function handleToggle(node: TreeNode) {
  if (!expandedKeys.value.includes(node.key)) {
    expandedKeys.value = [...expandedKeys.value, node.key]
    if (!node.children || node.children.length === 0) {
      loadingNodes.value.add(node.key); loadingNodes.value = new Set(loadingNodes.value)
      try { await onLoadData(node) } finally { loadingNodes.value.delete(node.key); loadingNodes.value = new Set(loadingNodes.value) }
    }
  } else { expandedKeys.value = expandedKeys.value.filter(k => k !== node.key) }
}

function handleSelect(node: TreeNode) { selectedKeys.value = [node.key]; if (node.type === 'database') emit('database-selected', node.metadata); }
async function handleDoubleClick(node: TreeNode) {
  if (['database', 'schema', 'schemas', 'tables', 'views', 'schema-tables', 'schema-views', 'schema-indexes'].includes(node.type)) handleToggle(node)
  else if (['table', 'view'].includes(node.type)) { emit('table-selected', { database: node.metadata.database, table: node.metadata.name || node.title, schema: node.metadata.schema, metadata: node.metadata }) }
}

function onRightClick({ event, node }: any) {
  event.preventDefault(); selectedNode.value = node; contextMenuX.value = event.clientX; contextMenuY.value = event.clientY; contextMenuVisible.value = true;
}

const showScriptsModal = ref(false), savedScripts = ref<any[]>([]), loadingScripts = ref(false)
async function handleMenuClick({ key }: any) {
  contextMenuVisible.value = false; if (!selectedNode.value) return
  if (key === 'new-query') emit('new-query', { database: selectedNode.value.metadata.name || selectedNode.value.metadata.database, connectionId: props.connectionId })
  else if (key === 'open-scripts') { showScriptsModal.value = true; loadingScripts.value = true; try { savedScripts.value = await invoke<any[]>('list_db_scripts', { connectionId: props.connectionId, database: selectedNode.value.metadata.name || selectedNode.value.metadata.database }) } finally { loadingScripts.value = false } }
  else if (key === 'refresh') handleRefreshNode(selectedNode.value)
  else if (key === 'copy-name') { navigator.clipboard.writeText(selectedNode.value.title); message.success(t('common.copy')) }
  else if (key === 'view-data') {
    emit('table-selected', { 
      database: selectedNode.value.metadata.database, 
      table: selectedNode.value.metadata.name || selectedNode.value.title, 
      schema: selectedNode.value.metadata.schema, 
      metadata: selectedNode.value.metadata 
    })
  }
  else if (key === 'design-table') {
    emit('design-table', { 
      database: selectedNode.value.metadata.database, 
      table: selectedNode.value.metadata.name || selectedNode.value.title, 
      schema: selectedNode.value.metadata.schema 
    })
  }
  else if (key === 'view-ddl') {
    try {
      const ddl = await invoke<string>('get_create_table_ddl', { 
        connectionId: props.connectionId, 
        table: selectedNode.value.metadata.name || selectedNode.value.title,
        database: selectedNode.value.metadata.database,
        schema: selectedNode.value.metadata.schema
      })
      showDdlModal.value = true
      await nextTick()
      if (ddlEditorContainer.value) {
        if (ddlEditor) ddlEditor.dispose()
        ddlEditor = monaco.editor.create(ddlEditorContainer.value, {
          value: ddl,
          language: 'sql',
          theme: appStore.theme === 'dark' ? 'vs-dark' : 'vs',
          readOnly: true,
          automaticLayout: true,
          minimap: { enabled: false }
        })
      }
    } catch (e: any) { message.error(e) }
  }
}

async function openSavedScript(s: any) { try { const content = await invoke<string>('read_file', { path: s.path }); emit('new-query', { database: selectedNode.value?.metadata.database || selectedNode.value?.title, connectionId: props.connectionId, content, filePath: s.path, title: s.name }); showScriptsModal.value = false } catch (e: any) { message.error(e) } }

watch(() => props.connectionId, (id) => { if (id) loadDatabases(); else treeData.value = []; }, { immediate: true })
watch(() => connectionStore.getConnectionStatus(props.connectionId || ''), (s) => { if (s === 'connected' && treeData.value.length === 0 && !loading.value) loadDatabases(); })
defineExpose({ refresh: loadDatabases })
</script>

<style scoped>
.database-tree { height: 100%; overflow: visible; padding: 0; user-select: none; }
.custom-tree { width: 100%; }
.context-menu-overlay { position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 9999; }
.context-menu { position: absolute; background: #fff; border-radius: 4px; border: 1px solid #d9d9d9; box-shadow: 0 2px 8px rgba(0,0,0,0.15); z-index: 10000; min-width: 120px; }
.dark-mode .context-menu { background: #1f1f1f; border-color: #303030; }
.script-item { cursor: pointer; transition: background 0.2s; }
.script-item:hover { background: #f5f5f5; }
</style>
