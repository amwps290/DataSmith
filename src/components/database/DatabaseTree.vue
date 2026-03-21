<template>
  <div class="database-tree">
    <a-spin :spinning="loading" tip="加载中...">
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
      <a-empty v-if="!loading && filteredTreeData.length === 0" :description="searchValue ? '未找到匹配的对象' : '请先选择一个连接'" :image-style="{ height: '60px' }" />
    </a-spin>

    <!-- 右键菜单 -->
    <div v-if="contextMenuVisible" class="context-menu-overlay" @click="contextMenuVisible = false">
      <div class="context-menu" :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }" @click.stop>
        <a-menu @click="handleMenuClick">
          <!-- 数据库级别 -->
          <template v-if="selectedNode?.type === 'database'">
            <a-menu-item v-if="isSqlSupported" key="new-query"><template #icon><FileTextOutlined /></template>新建查询脚本</a-menu-item>
            <a-menu-item v-if="isSqlSupported" key="open-scripts"><template #icon><FolderOpenOutlined /></template>打开已有脚本...</a-menu-item>
            <a-menu-divider />
            <a-menu-item key="refresh"><template #icon><ReloadOutlined /></template>刷新数据库内容</a-menu-item>
          </template>
          
          <!-- Schema 或 文件夹级别 -->
          <template v-else-if="['schema', 'tables', 'views', 'schemas', 'functions', 'schema-tables', 'schema-views', 'schema-functions', 'schema-indexes'].includes(selectedNode?.type || '')">
            <a-menu-item key="refresh"><template #icon><ReloadOutlined /></template>刷新</a-menu-item>
          </template>
          
          <!-- 表级别 -->
          <template v-if="selectedNode?.type === 'table'">
            <a-menu-item key="view-data"><template #icon><TableOutlined /></template>查看数据</a-menu-item>
            <a-menu-item key="design-table"><template #icon><EditOutlined /></template>设计表</a-menu-item>
            <a-menu-divider />
            <a-menu-item key="refresh"><template #icon><ReloadOutlined /></template>刷新字段</a-menu-item>
            <a-menu-item key="drop-table" danger><template #icon><DeleteOutlined /></template>删除表</a-menu-item>
          </template>

          <a-menu-item key="copy-name"><template #icon><CopyOutlined /></template>复制名称</a-menu-item>
        </a-menu>
      </div>
    </div>

    <!-- 脚本选择对话框 -->
    <a-modal v-model:open="showScriptsModal" title="选择已有脚本" :footer="null" width="600px">
      <a-list v-if="savedScripts.length > 0" :loading="loadingScripts" :data-source="savedScripts" size="small">
        <template #renderItem="{ item }">
          <a-list-item class="script-list-item" @click="openSavedScript(item)">
            <a-list-item-meta>
              <template #title>
                <div class="script-item-title"><FileTextOutlined style="margin-right: 8px; color: #1890ff" />{{ item.name }}</div>
              </template>
              <template #description>{{ new Date(item.last_modified * 1000).toLocaleString() }} • {{ (item.size / 1024).toFixed(2) }} KB</template>
            </a-list-item-meta>
          </a-list-item>
        </template>
      </a-list>
      <a-empty v-else-if="!loadingScripts" description="该数据库暂无保存的脚本" />
    </a-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, watch } from 'vue'
import {
  TableOutlined, ReloadOutlined, CopyOutlined,
  FolderOpenOutlined, DeleteOutlined, EditOutlined,
  FileTextOutlined
} from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { invoke } from '@tauri-apps/api/core'
import type { DatabaseInfo } from '@/types/database'
import { useConnectionStore } from '@/stores/connection'
import TreeNodeItem from './TreeNodeItem.vue'

interface TreeNode {
  key: string; title: string; type: string; children?: TreeNode[];
  isLeaf?: boolean; metadata?: any; isAutoExpanded?: boolean;
}

const props = defineProps<{ connectionId: string | null; dbType?: string; searchValue?: string; }>()
const emit = defineEmits(['table-selected', 'database-selected', 'new-query', 'design-table', 'view-structure', 'open-scripts'])
const connectionStore = useConnectionStore()

const isSqlSupported = computed(() => {
  if (!props.dbType) return true
  return !['redis', 'mongodb', 'elasticsearch'].includes(props.dbType.toLowerCase())
})

const loading = ref(false), treeData = ref<TreeNode[]>([]), expandedKeys = ref<string[]>([]), selectedKeys = ref<string[]>([]), loadingNodes = ref<Set<string>>(new Set())
const contextMenuVisible = ref(false), contextMenuX = ref(0), contextMenuY = ref(0), selectedNode = ref<TreeNode | null>(null)

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
      // 统一 SQLite 结构，增加一个 database 层级的节点
      treeData.value = [
        { 
          key: 'db-main', 
          title: 'main', 
          type: 'database', 
          isLeaf: false, 
          metadata: { name: 'main', database: 'main' } 
        }
      ]
    } else {
      const dbs = await invoke<DatabaseInfo[]>('get_databases', { connectionId: props.connectionId })
      treeData.value = dbs.map(db => ({ key: `db-${db.name}`, title: db.name, type: 'database', isLeaf: false, metadata: db }))
    }
  } catch (e: any) { message.error(e) } finally { loading.value = false }
}

/**
 * 局部刷新逻辑：关闭当前层级并清空缓存
 */
async function handleRefreshNode(node: TreeNode) {
  // 1. 同步 UI 状态：从展开列表中移除，箭头会立即变回“折叠”状态
  expandedKeys.value = expandedKeys.value.filter(k => k !== node.key)
  
  // 2. 清空缓存：将该节点的子节点数组设为 undefined
  updateNodeInTree(treeData.value, node.key, (target) => {
    target.children = undefined
  })
  
  // 3. 强制更新树引用以触发 Vue 响应式
  treeData.value = [...treeData.value]
  
  message.success(`已重置 "${node.title}" 的本地缓存`)
  
  // 提示：下一次用户点击该节点的箭头展开时，handleToggle 会自动调用 onLoadData 重新加载最新的第一层子节点
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
        { key: `${treeNode.key}-extensions`, title: '扩展', type: 'database-extensions', isLeaf: false, metadata: { database: dbName } }
      ]
    } else {
      children = [
        { key: `${treeNode.key}-tables`, title: '表', type: 'tables', isLeaf: false, metadata: { database: dbName } },
        { key: `${treeNode.key}-views`, title: '视图', type: 'views', isLeaf: false, metadata: { database: dbName } }
      ]
    }
    updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children)
    treeData.value = [...treeData.value]
  }
  else if (treeNode.type === 'schemas') {
    try {
      const res = await invoke<any[]>('get_schemas', { connectionId: connId, database: treeNode.metadata.database })
      const children = res.map(s => ({ 
        key: `${treeNode.key}-${s.name}`, 
        title: s.name, 
        type: 'schema', 
        isLeaf: false, 
        metadata: { database: treeNode.metadata.database, name: s.name } 
      }))
      updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children)
      treeData.value = [...treeData.value]
    } catch (e: any) { message.error(e) }
  }
  else if (treeNode.type === 'schema') {
    const db = treeNode.metadata.database
    const schema = treeNode.metadata.name
    const children = [
      { key: `${treeNode.key}-tables`, title: '表', type: 'schema-tables', isLeaf: false, metadata: { database: db, schema } },
      { key: `${treeNode.key}-views`, title: '视图', type: 'schema-views', isLeaf: false, metadata: { database: db, schema } },
      { key: `${treeNode.key}-indexes`, title: '索引', type: 'schema-indexes', isLeaf: false, metadata: { database: db, schema } },
      { key: `${treeNode.key}-functions`, title: '函数', type: 'schema-functions', isLeaf: false, metadata: { database: db, schema } }
    ]
    updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children)
    treeData.value = [...treeData.value]
  }
  else if (['schema-tables', 'schema-views', 'tables', 'views'].includes(treeNode.type)) {
    const isSchema = treeNode.type.startsWith('schema-')
    const method = treeNode.type.includes('views') ? 'get_views' : (isSchema ? 'get_schema_tables' : 'get_tables')
    try {
      const res = await invoke<any[]>(method, { 
        connectionId: connId, database: treeNode.metadata.database, schema: treeNode.metadata.schema || null 
      })
      const children = res.map(t => ({
        key: `${treeNode.key}-${t.name}`,
        title: t.name,
        type: treeNode.type.includes('views') ? 'view' : 'table',
        isLeaf: false,
        metadata: { ...t, database: treeNode.metadata.database, schema: treeNode.metadata.schema }
      }))
      updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children.length ? children : [{ key: `${treeNode.key}-empty`, title: '(无数据)', type: 'empty', isLeaf: true }])
      treeData.value = [...treeData.value]
    } catch (e: any) { message.error(e) }
  }
  else if (treeNode.type === 'schema-indexes') {
    try {
      const res = await invoke<any[]>('get_schema_indexes', { 
        connectionId: connId, database: treeNode.metadata.database, schema: treeNode.metadata.schema 
      })
      const children = res.map(idx => ({
        key: `${treeNode.key}-${idx.index_name}`, title: idx.index_name, type: 'index', isLeaf: true,
        metadata: { ...idx, database: treeNode.metadata.database, schema: treeNode.metadata.schema }
      }))
      updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children.length ? children : [{ key: `${treeNode.key}-empty`, title: '(无索引)', type: 'empty', isLeaf: true }])
      treeData.value = [...treeData.value]
    } catch (e: any) { message.error(e) }
  }
  else if (treeNode.type === 'schema-functions') {
    try {
      const res = await invoke<any[]>('get_schema_functions', { 
        connectionId: connId, database: treeNode.metadata.database, schema: treeNode.metadata.schema 
      })
      const children = res.map(f => ({
        key: `${treeNode.key}-${f.name}`,
        title: `${f.name}(${f.arguments || ''})`,
        type: 'function', isLeaf: true,
        metadata: { ...f, database: treeNode.metadata.database, schema: treeNode.metadata.schema }
      }))
      updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children.length ? children : [{ key: `${treeNode.key}-empty`, title: '(无函数)', type: 'empty', isLeaf: true }])
      treeData.value = [...treeData.value]
    } catch (e: any) { message.error(e) }
  }
  else if (treeNode.type === 'database-extensions') {
    try {
      const res = await invoke<any[]>('get_database_extensions', { 
        connectionId: connId, database: treeNode.metadata.database 
      })
      const children = res.map(ext => ({
        key: `${treeNode.key}-${ext.name}`,
        title: `${ext.name} (${ext.version})`,
        type: 'extension', isLeaf: true,
        metadata: { ...ext, database: treeNode.metadata.database }
      }))
      updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children.length ? children : [{ key: `${treeNode.key}-empty`, title: '(无扩展)', type: 'empty', isLeaf: true }])
      treeData.value = [...treeData.value]
    } catch (e: any) { message.error(e) }
  }
  else if (['table', 'view'].includes(treeNode.type)) {
    try {
      const res = await invoke<any[]>('get_table_structure', { 
        connectionId: connId, table: treeNode.metadata.name || treeNode.title, database: treeNode.metadata.database, schema: treeNode.metadata.schema 
      })
      const children = res.map(c => ({
        key: `${treeNode.key}-col-${c.name}`,
        title: `${c.name}${c.data_type ? ' : ' + c.data_type : ''}${c.is_primary_key ? ' [PK]' : ''}`,
        type: 'column', isLeaf: true,
        metadata: { ...c, database: treeNode.metadata.database, table: treeNode.metadata.name, schema: treeNode.metadata.schema }
      }))
      updateNodeInTree(treeData.value, treeNode.key, (n) => n.children = children.length ? children : [{ key: `${treeNode.key}-empty`, title: '(无字段)', type: 'empty', isLeaf: true }])
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
  else if (['table', 'view'].includes(node.type)) {
    emit('table-selected', { database: node.metadata.database, table: node.metadata.name || node.title, schema: node.metadata.schema, metadata: node.metadata })
  }
}

function onRightClick({ event, node }: any) {
  event.preventDefault(); selectedNode.value = node; contextMenuX.value = event.clientX; contextMenuY.value = event.clientY; contextMenuVisible.value = true;
  nextTick(() => { const el = document.querySelector('.context-menu') as HTMLElement; if (!el) return; const r = el.getBoundingClientRect(); if (contextMenuX.value + r.width > window.innerWidth) contextMenuX.value -= r.width; if (contextMenuY.value + r.height > window.innerHeight) contextMenuY.value -= r.height; })
}

const showScriptsModal = ref(false), savedScripts = ref<any[]>([]), loadingScripts = ref(false)
async function handleMenuClick({ key }: any) {
  contextMenuVisible.value = false; if (!selectedNode.value) return
  if (key === 'new-query') emit('new-query', { database: selectedNode.value.metadata.name || selectedNode.value.metadata.database, connectionId: props.connectionId })
  else if (key === 'open-scripts') handleOpenScripts()
  else if (key === 'refresh') handleRefreshNode(selectedNode.value) // 使用局部刷新逻辑
  else if (key === 'copy-name') { navigator.clipboard.writeText(selectedNode.value.title); message.success('已复制') }
}

async function handleOpenScripts() {
  if (!selectedNode.value || !props.connectionId) return
  loadingScripts.value = true; showScriptsModal.value = true
  try { savedScripts.value = await invoke<any[]>('list_db_scripts', { connectionId: props.connectionId, database: selectedNode.value.metadata.name || selectedNode.value.metadata.database }) }
  catch (e: any) { message.error(e) } finally { loadingScripts.value = false }
}

async function openSavedScript(s: any) {
  try {
    const content = await invoke<string>('read_file', { path: s.path });
    emit('new-query', { database: selectedNode.value?.metadata.database || selectedNode.value?.title, connectionId: props.connectionId, content, filePath: s.path, title: s.name });
    showScriptsModal.value = false
  }
  catch (e: any) { message.error(e) }
}

watch(() => props.connectionId, (id) => { if (id) loadDatabases(); else treeData.value = []; }, { immediate: true })
watch(() => connectionStore.getConnectionStatus(props.connectionId || ''), (s) => { if (s === 'connected' && treeData.value.length === 0 && !loading.value) loadDatabases(); })
defineExpose({ refresh: loadDatabases, isSqlSupported })
</script>

<style scoped>
.database-tree { height: 100%; overflow: auto; padding: 8px 0; user-select: none; }
.custom-tree { width: 100%; }
.context-menu-overlay { position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 9999; }
.context-menu { position: absolute; background: #fff; border-radius: 6px; box-shadow: 0 6px 16px rgba(0,0,0,0.08); z-index: 10000; min-width: 180px; }
.dark-mode .context-menu { background: #1f1f1f; border: 1px solid #303030; }
.script-list-item { cursor: pointer; padding: 8px 12px; }
.script-list-item:hover { background-color: #f5f5f5; }
.dark-mode .script-list-item:hover { background-color: #262626; }
.script-item-title { font-weight: 500; display: flex; align-items: center; }
</style>
