<template>
  <a-layout class="main-layout" :class="{ 'dark-mode': appStore.theme === 'dark' }">
    <!-- 顶部标题栏 -->
    <AppHeader
      @new-connection="showConnectionDialog = true"
      @open-settings="openSettings"
      @open-search="showGlobalSearch = true"
    />

    <a-layout-content class="content-container">
      <div class="sidebar-wrapper" :style="{ width: appStore.sidebarCollapsed ? '0' : sidebarWidth + 'px' }">
        <div class="sidebar-inner">
          <ConnectionPanel
            @add-connection="showConnectionDialog = true"
            @edit-connection="handleEditConnection"
            @table-selected="handleTableSelected"
            @database-selected="handleDatabaseSelected"
            @new-query="handleNewQuery"
            @design-table="handleDesignTable"
            @view-structure="handleViewStructure"
            @open-scripts="handleOpenSavedScript"
          />
        </div>
      </div>

      <div v-if="!appStore.sidebarCollapsed" class="sidebar-resizer" @mousedown="startResize"></div>

      <div class="main-workspace">
        <SqlToolbar
          v-if="activeTabType === 'query'"
          :executing="activeEditorExecuting"
          :selected-database="activeTabDatabase"
          :databases="availableDatabases"
          @action="callActiveEditor($event)"
          @database-change="handleToolbarDbChange"
        />

        <a-tabs v-model:activeKey="mainTabKey" type="editable-card" size="small" @edit="onTabEdit" class="workspace-tabs">
          <a-tab-pane v-for="tab in dataTabs" :key="tab.key" :closable="tab.closable !== false">
            <template #tab>
              <span class="tab-title" @contextmenu.prevent="handleTabContextMenu($event, tab.key, tab.closable !== false)">
                <span v-if="getConnectionColor(tab.connectionId)" class="tab-connection-dot" :style="{ backgroundColor: getConnectionColor(tab.connectionId) }"></span>
                <FileTextOutlined v-if="tab.type === 'query'" />
                <TableOutlined v-else-if="tab.type === 'data'" />
                <EditOutlined v-else-if="tab.type === 'design'" />
                <span class="title-text">{{ tab.title }}</span>
              </span>
            </template>
            <div class="tab-content-wrapper">
              <KeepAlive>
                <SqlEditor v-if="tab.type === 'query'" :key="tab.key" :ref="(el) => setSqlEditorRef(el, tab.key)" :connection-id="tab.connectionId" :initial-database="tab.database" :initial-value="tab.content" :file-path="tab.filePath" @content-change="(val) => handleContentChange(tab.key, val)" @file-saved="(path, title) => handleFileSaved(tab.key, path, title)" @databases-loaded="(dbs) => availableDatabases = dbs" />
                <TableDataGrid v-else-if="tab.type === 'data'" :key="tab.key" :connection-id="tab.connectionId!" :database="tab.database!" :table="tab.table!" :schema="tab.schema" />
                <TableDesigner v-else-if="tab.type === 'design'" :key="tab.key" :connection-id="tab.connectionId!" :database="tab.database!" :table="tab.table!" :schema="tab.schema" :read-only="tab.readOnly" />
                <RedisEditor v-else-if="tab.type === 'redis'" :key="tab.key" :ref="redisEditorRef" />
              </KeepAlive>
            </div>
          </a-tab-pane>
        </a-tabs>

        <div v-if="dataTabs.length === 0" class="empty-workspace">
          <a-empty :description="$t('common.loading')">
            <template #extra><a-button type="primary" @click="handleNewQuery({})">{{ $t('tree.new_query') }}</a-button></template>
          </a-empty>
        </div>
      </div>
    </a-layout-content>

    <ConnectionDialog v-model:visible="showConnectionDialog" :editing-connection="editingConnection" @close="editingConnection = null" />
    <GlobalSearch v-model:visible="showGlobalSearch" :connection-id="connectionStore.activeConnectionId" @view-data="handleTableSelected" />
  </a-layout>
</template>

<script setup lang="ts">
import { defineAsyncComponent, reactive, ref, computed, nextTick, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import {
  FileTextOutlined,
  TableOutlined, EditOutlined,
} from '@ant-design/icons-vue'
import { useAppStore } from '@/stores/app'
import { useConnectionStore } from '@/stores/connection'
import { useWorkspaceStore } from '@/stores/workspace'
import ConnectionPanel from '@/components/connection/ConnectionPanel.vue'
import ConnectionDialog from '@/components/connection/ConnectionDialog.vue'
import AppHeader from '@/components/layout/AppHeader.vue'
import SqlToolbar from '@/components/layout/SqlToolbar.vue'
import { invoke } from '@tauri-apps/api/core'
import { message } from 'ant-design-vue'
import type { ConnectionConfig, DatabaseInfo, ScriptInfo } from '@/types/database'
import { TabType } from '@/types/workspace'
import { useSidebarResize } from '@/composables/useSidebarResize'
import { useTabManager, type DataTab } from '@/composables/useTabManager'
import { useContextMenu } from '@/composables/useContextMenu'

// Step 34: 重量级组件懒加载
const SqlEditor = defineAsyncComponent(() => import('@/components/editor/SqlEditor.vue'))
const RedisEditor = defineAsyncComponent(() => import('@/components/editor/RedisEditor.vue'))
const TableDataGrid = defineAsyncComponent(() => import('@/components/data/TableDataGrid.vue'))
const TableDesigner = defineAsyncComponent(() => import('@/components/database/TableDesigner.vue'))
const GlobalSearch = defineAsyncComponent(() => import('@/components/search/GlobalSearch.vue'))

const { t } = useI18n()
const router = useRouter()
const appStore = useAppStore()
const connectionStore = useConnectionStore()
const workspaceStore = useWorkspaceStore()

// Composables
const { sidebarWidth, startResize } = useSidebarResize()
const {
  dataTabs, mainTabKey, sqlEditorRefs,
  activeTabType, activeTabDatabase, activeEditorExecuting,
  setSqlEditorRef, callActiveEditor, closeTab,
  tabExists, addTab, handleContentChange, handleFileSaved,
} = useTabManager()

const showConnectionDialog = ref(false)
const showGlobalSearch = ref(false)
const editingConnection = ref<ConnectionConfig | null>(null)
const redisEditorRef = ref()
const availableDatabases = ref<DatabaseInfo[]>([])

function openSettings() {
  router.push({ name: 'Settings' })
}

const isSqlSupported = computed(() => {
  const activeConnection = connectionStore.getActiveConnection()
  return activeConnection ? !['redis', 'mongodb', 'elasticsearch'].includes(activeConnection.db_type) : true
})

watch([dataTabs, mainTabKey], () => { workspaceStore.saveSession(dataTabs.value, mainTabKey.value) }, { deep: true })

async function restoreSession() {
  workspaceStore.isRestoring = true
  try {
    const session = await workspaceStore.loadSession()
    if (session && session.open_tabs.length > 0) {
      dataTabs.value = session.open_tabs.map(tab => ({
        ...tab,
        type: tab.type,
      })) as DataTab[]
      mainTabKey.value = session.active_tab_key
      if (connectionStore.connections.length === 0) await connectionStore.fetchConnections()
      const connectionIds = [...new Set(dataTabs.value.map(tab => tab.connectionId).filter(Boolean))] as string[]
      for (const id of connectionIds) {
        const conn = connectionStore.connections.find(c => c.id === id)
        if (conn) connectionStore.connectToDatabase(conn.id).catch(() => {})
      }
    } else if (isSqlSupported.value) { handleNewQuery({}) }
  } catch (_e) { if (isSqlSupported.value) handleNewQuery({}) } finally { workspaceStore.isRestoring = false }
}

function handleToolbarDbChange(val: unknown) { callActiveEditor('handleDatabaseChange', String(val || '')) }

onMounted(async () => {
  restoreSession()
})

interface TableEventData { connectionId?: string; database?: string; table?: string; schema?: string; metadata?: { schema?: string } }
interface QueryEventData { connectionId?: string; database?: string; filePath?: string; title?: string; content?: string }
interface DatabaseEventData { connectionId?: string; name?: string }

function handleOpenSavedScript(data: QueryEventData) { handleNewQuery(data) }
function handleViewStructure(data: TableEventData) {
  const key = `structure-${data.connectionId}-${data.database}-${data.table}`
  if (tabExists(key)) { mainTabKey.value = key; return }
  addTab({ key, title: `${t('common.file')}: ${data.table}`, type: TabType.Design, connectionId: data.connectionId, database: data.database, table: data.table, schema: data.schema, readOnly: true })
}
function handleDesignTable(data: TableEventData) {
  const key = `design-${data.connectionId}-${data.database}-${data.table}`
  if (tabExists(key)) { mainTabKey.value = key; return }
  addTab({ key, title: `${t('tree.design_table')}: ${data.table}`, type: TabType.Design, connectionId: data.connectionId, database: data.database, table: data.table, schema: data.schema, readOnly: false })
}

const { showContextMenu } = useContextMenu()
const currentContextTab = reactive({ key: '', closable: false })
function handleTabContextMenu(e: MouseEvent, key: string, closable: boolean) { currentContextTab.key = key; currentContextTab.closable = closable; showContextMenu(e); }

function handleTableSelected(d: TableEventData) {
  const id = d.connectionId || connectionStore.activeConnectionId, key = `table-${id}-${d.database}-${d.table}`
  if (tabExists(key)) { mainTabKey.value = key; return }
  addTab({ key, title: d.table || '', type: TabType.Data, connectionId: id!, database: d.database, table: d.table, schema: d.schema || d.metadata?.schema })
}
async function handleDatabaseSelected(d: DatabaseEventData) {
  if (d.connectionId) connectionStore.setActiveConnection(d.connectionId)
  if (!isSqlSupported.value) {
    if (connectionStore.getActiveConnection()?.db_type === 'redis') {
      if (!tabExists('redis')) addTab({ key: 'redis', title: 'Redis 命令行', type: TabType.Redis, closable: false })
      mainTabKey.value = 'redis'; await nextTick(); setTimeout(() => redisEditorRef.value?.switchDatabase(d.name || ''), 100)
    }
    return
  }
  const cur = dataTabs.value.find(tab => tab.key === mainTabKey.value); if (cur?.type === 'query') sqlEditorRefs[mainTabKey.value]?.setSelectedDatabase(d.name || '')
}

async function handleNewQuery(d: QueryEventData) {
  if (!isSqlSupported.value) return
  const connId = d.connectionId || connectionStore.activeConnectionId
  let dbName = d.database
  if (connId && !dbName) {
    const conn = connectionStore.connections.find(c => c.id === connId)
    if (conn?.db_type === 'sqlite') dbName = 'main'
  }
  let filePath = d.filePath, title = d.title, initialContent = d.content || t('editor.placeholder')
  if (connId && dbName && !filePath) {
    try {
      const script = await invoke<ScriptInfo>('create_db_script', { connectionId: connId, database: dbName, content: initialContent })
      filePath = script.path; title = script.name
    } catch (_e) { message.error(t('common.fail')); return }
  }
  const key = `query-${Date.now()}`
  addTab({ key, title: title || `script-${new Date().getTime()}.sql`, type: TabType.Query, connectionId: connId || undefined, database: dbName, content: initialContent, filePath })
}

function onTabEdit(key: string | number | MouseEvent | KeyboardEvent, action: string) { if (action === 'add') handleNewQuery({}); else closeTab(String(key)); }
function handleEditConnection(c: ConnectionConfig) { editingConnection.value = c; showConnectionDialog.value = true; }
function getConnectionColor(connectionId?: string) {
  if (!connectionId) return ''
  return connectionStore.connections.find(connection => connection.id === connectionId)?.color || ''
}
</script>

<style scoped>
.main-layout { height: 100vh; width: 100vw; display: flex; flex-direction: column; overflow: hidden; }

.content-container { flex: 1; display: flex; flex-direction: row; overflow: hidden; position: relative; }
.sidebar-wrapper { background: #fafafa; border-right: 1px solid #e8e8e8; height: 100%; overflow: hidden; flex-shrink: 0; }
.dark-mode .sidebar-wrapper { background: #141414; border-right-color: #303030; }
.sidebar-inner { height: 100%; overflow: auto; padding: 0 8px; }
.sidebar-resizer { width: 4px; cursor: col-resize; background: transparent; transition: background-color 0.2s; z-index: 10; }
.sidebar-resizer:hover { background: #1890ff; }
.main-workspace { flex: 1; display: flex; flex-direction: column; overflow: hidden; background: #fff; min-width: 0; }
.dark-mode .main-workspace { background: #1f1f1f; }
.workspace-tabs { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.workspace-tabs :deep(.ant-tabs-nav) { margin-bottom: 0; padding: 0 4px; background: #f0f0f0; flex-shrink: 0; }
.dark-mode .workspace-tabs :deep(.ant-tabs-nav) { background: #141414; }
.workspace-tabs :deep(.ant-tabs-content) { flex: 1; height: 100%; overflow: hidden; }
.workspace-tabs :deep(.ant-tabs-tabpane) { height: 100%; display: flex; flex-direction: column; }
.tab-title { display: inline-flex; align-items: center; gap: 6px; min-width: 0; }
.tab-connection-dot { width: 8px; height: 8px; border-radius: 999px; flex-shrink: 0; box-shadow: 0 0 0 1px rgba(15, 23, 42, 0.08); }
.title-text { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.tab-content-wrapper { flex: 1; height: 100%; overflow: hidden; position: relative; }
.empty-workspace { flex: 1; display: flex; align-items: center; justify-content: center; }
</style>
