<template>
  <a-layout class="main-layout" :class="{ 'dark-mode': appStore.theme === 'dark' }">
    <!-- 顶部标题栏 (原生 header 确保拖拽可靠性) -->
    <header class="header" @mousedown="handleHeaderMouseDown">
      <div class="header-content">
        <!-- Logo -->
        <div class="logo">
          <DatabaseOutlined style="font-size: 18px; margin-right: 6px" />
          <span class="title">DataSmith</span>
        </div>
        
        <!-- 菜单区域 (阻止冒泡，避免触发拖拽) -->
        <div class="header-menu" @mousedown.stop>
          <a-menu mode="horizontal" :selected-keys="[]" class="top-menu">
            <a-sub-menu key="file">
              <template #title>{{ $t('common.file') }}</template>
              <a-menu-item key="new-connection" @click="showConnectionDialog = true">
                <PlusOutlined /> {{ $t('connection.new') }}
              </a-menu-item>
              <a-menu-divider />
              <a-menu-item key="settings" @click="openSettings">
                <SettingOutlined /> {{ $t('common.settings') }}
              </a-menu-item>
            </a-sub-menu>
            <a-sub-menu key="view">
              <template #title>{{ $t('common.view') }}</template>
              <a-menu-item key="toggle-sidebar" @click="appStore.toggleSidebar()">
                <MenuOutlined /> {{ appStore.sidebarCollapsed ? $t('common.show_sidebar') : $t('common.hide_sidebar') }}
              </a-menu-item>
              <a-menu-divider />
              <a-menu-item key="theme" @click="appStore.toggleTheme()">
                <BulbOutlined /> {{ appStore.theme === 'light' ? $t('common.dark_theme') : $t('common.light_theme') }}
              </a-menu-item>
            </a-sub-menu>
          </a-menu>
        </div>

        <!-- 动作区与窗口控制 (阻止冒泡) -->
        <div class="header-actions" @mousedown.stop>
          <a-space :size="0">
            <a-button type="text" size="small" @click="showGlobalSearch = true" class="search-btn">
              <template #icon><SearchOutlined /></template>
            </a-button>
            
            <div class="window-controls">
              <div class="win-btn" title="最小化" @click="minimizeWindow">
                <Icon icon="fluent:subtract-16-filled" />
              </div>
              <div class="win-btn" title="最大化/还原" @click="toggleMaximize">
                <Icon :icon="isMaximized ? 'fluent:square-multiple-16-regular' : 'fluent:square-16-regular'" />
              </div>
              <div class="win-btn close" title="关闭" @click="closeWindow">
                <Icon icon="fluent:dismiss-16-filled" />
              </div>
            </div>
          </a-space>
        </div>
      </div>
    </header>

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
        <div v-if="activeTabType === 'query'" class="global-sql-toolbar">
          <div class="toolbar-left">
            <a-space :size="8">
              <a-button-group>
                <a-tooltip :title="`${$t('common.run')} (F5)`">
                  <a-button type="text" size="small" @click="callActiveEditor('executeQuery')" :loading="activeEditorExecuting" class="btn-run">
                    <template #icon><PlayCircleFilled /></template>
                  </a-button>
                </a-tooltip>
                <a-tooltip :title="$t('common.explain')">
                  <a-button type="text" size="small" @click="callActiveEditor('explainQuery')" :disabled="activeEditorExecuting" class="btn-explain">
                    <template #icon><SearchOutlined /></template>
                  </a-button>
                </a-tooltip>
                <a-tooltip :title="$t('common.stop')">
                  <a-button type="text" size="small" @click="callActiveEditor('stopExecution')" :disabled="!activeEditorExecuting" class="btn-stop">
                    <template #icon><StopOutlined /></template>
                  </a-button>
                </a-tooltip>
              </a-button-group>
              <a-divider type="vertical" />
              <a-button-group>
                <a-tooltip :title="`${$t('common.save')} (Ctrl+S)`">
                  <a-button type="text" size="small" @click="callActiveEditor('handleSave')"><template #icon><SaveOutlined /></template></a-button>
                </a-tooltip>
                <a-tooltip :title="$t('common.format')">
                  <a-button type="text" size="small" @click="callActiveEditor('formatSql')"><template #icon><FormatPainterOutlined /></template></a-button>
                </a-tooltip>
                <a-tooltip :title="$t('common.clear')">
                  <a-button type="text" size="small" @click="callActiveEditor('clearEditor')"><template #icon><ClearOutlined /></template></a-button>
                </a-tooltip>
              </a-button-group>
              <a-divider type="vertical" />
              <a-button-group>
                <a-tooltip :title="$t('common.history')">
                  <a-button type="text" size="small" @click="callActiveEditor('openHistory')"><template #icon><HistoryOutlined /></template></a-button>
                </a-tooltip>
                <a-tooltip :title="$t('common.snippets')">
                  <a-button type="text" size="small" @click="callActiveEditor('openSnippets')"><template #icon><CodeOutlined /></template></a-button>
                </a-tooltip>
                <a-tooltip :title="$t('common.refresh')">
                  <a-button type="text" size="small" @click="callActiveEditor('refreshAutocomplete')"><template #icon><SyncOutlined /></template></a-button>
                </a-tooltip>
              </a-button-group>
            </a-space>
          </div>
          <div class="toolbar-right">
            <a-space>
              <span class="db-label">{{ $t('common.database') }}:</span>
              <a-select v-model:value="activeTabDatabase" :placeholder="$t('common.database')" size="small" style="width: 160px" @change="handleToolbarDbChange">
                <a-select-option value="">{{ appStore.language === 'zh-CN' ? '默认' : 'Default' }}</a-select-option>
                <a-select-option v-for="db in availableDatabases" :key="db.name" :value="db.name">{{ db.name }}</a-select-option>
              </a-select>
            </a-space>
          </div>
        </div>

        <a-tabs v-model:activeKey="mainTabKey" type="editable-card" size="small" @edit="onTabEdit" class="workspace-tabs">
          <a-tab-pane v-for="tab in dataTabs" :key="tab.key" :closable="tab.closable !== false">
            <template #tab>
              <span class="tab-title" @contextmenu.prevent="handleTabContextMenu($event, tab.key, tab.closable !== false)">
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
import { reactive, ref, computed, nextTick, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Icon } from '@iconify/vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { 
  DatabaseOutlined, BulbOutlined, PlusOutlined, SettingOutlined, 
  MenuOutlined, FileTextOutlined, SearchOutlined, 
  TableOutlined, EditOutlined,
  PlayCircleFilled, StopOutlined, SaveOutlined,
  FormatPainterOutlined, ClearOutlined, HistoryOutlined, CodeOutlined, SyncOutlined
} from '@ant-design/icons-vue'
import { useAppStore } from '@/stores/app'
import { useConnectionStore } from '@/stores/connection'
import { useWorkspaceStore } from '@/stores/workspace'
import ConnectionPanel from '@/components/connection/ConnectionPanel.vue'
import ConnectionDialog from '@/components/connection/ConnectionDialog.vue'
import SqlEditor from '@/components/editor/SqlEditor.vue'
import RedisEditor from '@/components/editor/RedisEditor.vue'
import TableDataGrid from '@/components/data/TableDataGrid.vue'
import TableDesigner from '@/components/database/TableDesigner.vue'
import GlobalSearch from '@/components/search/GlobalSearch.vue'
import { invoke } from '@tauri-apps/api/core'
import { message } from 'ant-design-vue'

const { t } = useI18n()
const appStore = useAppStore()
const connectionStore = useConnectionStore()
const workspaceStore = useWorkspaceStore()
const appWindow = getCurrentWindow()

const showConnectionDialog = ref(false)
const showSettings = ref(false)
const showGlobalSearch = ref(false)
const mainTabKey = ref('')
const editingConnection = ref<any>(null)
const sqlEditorRefs = reactive<Record<string, any>>({})
const redisEditorRef = ref<any>(null)
const availableDatabases = ref<any[]>([])
const sidebarWidth = ref(280)
const isMaximized = ref(false)

const settingsForm = reactive({ theme: appStore.theme, language: appStore.language })

// 编程式拖拽逻辑：解决 Linux 下属性无效的问题
async function handleHeaderMouseDown(e: MouseEvent) {
  // 仅左键点击且当前不是点击在按钮/菜单上时触发拖拽
  if (e.button === 0) {
    try {
      await appWindow.startDragging()
    } catch (err) {
      console.error('拖拽失败:', err)
    }
  }
}

// 窗口控制
async function minimizeWindow() { await appWindow.minimize() }
async function toggleMaximize() { 
  await appWindow.toggleMaximize()
  isMaximized.value = await appWindow.isMaximized()
}
async function closeWindow() { await appWindow.close() }

function openSettings() {
  settingsForm.theme = appStore.theme
  settingsForm.language = appStore.language
  showSettings.value = true
}

function handleSaveSettings() {
  appStore.setTheme(settingsForm.theme as any)
  appStore.setLanguage(settingsForm.language as any)
  showSettings.value = false
  message.success(t('common.save'))
}

const isSqlSupported = computed(() => {
  const activeConnection = connectionStore.getActiveConnection()
  return activeConnection ? !['redis', 'mongodb', 'elasticsearch'].includes(activeConnection.db_type) : true
})

interface DataTab {
  key: string; title: string; type: 'data' | 'design' | 'query' | 'redis';
  connectionId?: string; database?: string; table?: string; schema?: string;
  content?: string; filePath?: string; closable?: boolean; readOnly?: boolean;
}
const dataTabs = ref<DataTab[]>([])

watch([dataTabs, mainTabKey], () => { workspaceStore.saveSession(dataTabs.value as any, mainTabKey.value) }, { deep: true })

async function restoreSession() {
  workspaceStore.isRestoring = true
  try {
    const session = await workspaceStore.loadSession()
    if (session && session.open_tabs.length > 0) {
      dataTabs.value = session.open_tabs.map(t => ({ ...t, type: (t as any).tab_type || t.type })) as any
      mainTabKey.value = session.active_tab_key
      if (connectionStore.connections.length === 0) await connectionStore.fetchConnections()
      const connectionIds = [...new Set(dataTabs.value.map(t => t.connectionId).filter(Boolean))] as string[]
      for (const id of connectionIds) {
        const conn = connectionStore.connections.find(c => c.id === id)
        if (conn) connectionStore.connectToDatabase(conn.id).catch(() => {})
      }
    } else if (isSqlSupported.value) { handleNewQuery({}) }
  } catch (e) { if (isSqlSupported.value) handleNewQuery({}) } finally { workspaceStore.isRestoring = false }
}

const activeTabType = computed(() => dataTabs.value.find(t => t.key === mainTabKey.value)?.type)
const activeTabDatabase = computed({
  get: () => dataTabs.value.find(t => t.key === mainTabKey.value)?.database || '',
  set: (val) => { const t = dataTabs.value.find(t => t.key === mainTabKey.value); if (t) t.database = val; }
})
const activeEditorExecuting = computed(() => sqlEditorRefs[mainTabKey.value]?.executing || false)

function callActiveEditor(method: string, ...args: any[]) { const editor = sqlEditorRefs[mainTabKey.value]; if (editor && editor[method]) editor[method](...args) }
function handleToolbarDbChange(val: any) { callActiveEditor('handleDatabaseChange', String(val || '')) }

onMounted(async () => { 
  restoreSession()
  try {
    isMaximized.value = await appWindow.isMaximized()
    appWindow.onResized(async () => { isMaximized.value = await appWindow.isMaximized() })
  } catch (e) { console.error(e) }
})

function setSqlEditorRef(el: any, key: string) { if (el) sqlEditorRefs[key] = el; else delete sqlEditorRefs[key]; }
function handleContentChange(key: string, val: string) { const t = dataTabs.value.find(t => t.key === key); if (t) t.content = val; }
function handleFileSaved(key: string, path: string, title: string) {
  const t = dataTabs.value.find(t => t.key === key); if (t) { t.filePath = path; t.title = title; }
}
function handleOpenSavedScript(data: any) { handleNewQuery(data) }
function handleViewStructure(data: any) {
  const key = `structure-${data.connectionId}-${data.database}-${data.table}`
  if (dataTabs.value.some(t => t.key === key)) { mainTabKey.value = key; return; }
  dataTabs.value.push({ key, title: `${t('common.file')}: ${data.table}`, type: 'design', ...data, readOnly: true })
  mainTabKey.value = key
}
function handleDesignTable(data: any) {
  const key = `design-${data.connectionId}-${data.database}-${data.table}`
  if (dataTabs.value.some(t => t.key === key)) { mainTabKey.value = key; return; }
  dataTabs.value.push({ key, title: `${t('tree.design_table')}: ${data.table}`, type: 'design', ...data, readOnly: false })
  mainTabKey.value = key
}

const contextMenuVisible = ref(false), contextMenuPosition = reactive({ x: 0, y: 0 }), currentContextTab = reactive({ key: '', closable: false })
function handleTabContextMenu(e: MouseEvent, key: string, closable: boolean) { e.preventDefault(); currentContextTab.key = key; currentContextTab.closable = closable; contextMenuPosition.x = e.clientX; contextMenuPosition.y = e.clientY; contextMenuVisible.value = true; }
function closeTab(key: string) { const i = dataTabs.value.findIndex(t => t.key === key); if (i >= 0) { dataTabs.value.splice(i, 1); if (mainTabKey.value === key && dataTabs.value.length > 0) mainTabKey.value = dataTabs.value[Math.min(i, dataTabs.value.length - 1)].key } }
function handleTableSelected(d: any) {
  const id = d.connectionId || connectionStore.activeConnectionId, key = `table-${id}-${d.database}-${d.table}`
  if (dataTabs.value.some(t => t.key === key)) { mainTabKey.value = key; return; }
  dataTabs.value.push({ key, title: d.table, type: 'data', connectionId: id!, database: d.database, table: d.table, schema: d.schema || d.metadata?.schema }); mainTabKey.value = key
}
async function handleDatabaseSelected(d: any) {
  if (d.connectionId) connectionStore.setActiveConnection(d.connectionId)
  if (!isSqlSupported.value) {
    if (connectionStore.getActiveConnection()?.db_type === 'redis') {
      if (!dataTabs.value.some(t => t.key === 'redis')) dataTabs.value.push({ key: 'redis', title: 'Redis 命令行', type: 'redis', closable: false })
      mainTabKey.value = 'redis'; await nextTick(); setTimeout(() => redisEditorRef.value?.switchDatabase(d.name), 100)
    }
    return
  }
  const cur = dataTabs.value.find(t => t.key === mainTabKey.value); if (cur?.type === 'query') sqlEditorRefs[mainTabKey.value]?.setSelectedDatabase(d.name)
}

async function handleNewQuery(d: any) {
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
      const script = await invoke<any>('create_db_script', { connectionId: connId, database: dbName, content: initialContent })
      filePath = script.path; title = script.name
    } catch (e) { message.error(t('common.fail')); return }
  }
  const key = `query-${Date.now()}`
  dataTabs.value.push({ key, title: title || `script-${new Date().getTime()}.sql`, type: 'query', connectionId: connId || undefined, database: dbName, content: initialContent, filePath })
  mainTabKey.value = key
}

function onTabEdit(key: any, action: string) { if (action === 'add') handleNewQuery({}); else closeTab(String(key)); }
function startResize(e: MouseEvent) {
  const sx = e.clientX, sw = sidebarWidth.value; const dr = (ev: MouseEvent) => { const nw = sw + (ev.clientX - sx); if (nw >= 200 && nw <= 600) sidebarWidth.value = nw }; const sr = () => { document.removeEventListener('mousemove', dr); document.removeEventListener('mouseup', sr) }
  document.addEventListener('mousemove', dr); document.addEventListener('mouseup', sr)
}
function handleEditConnection(c: any) { editingConnection.value = c; showConnectionDialog.value = true; }
</script>

<style scoped>
.main-layout { height: 100vh; width: 100vw; display: flex; flex-direction: column; overflow: hidden; }
.header { 
  height: 40px; 
  background: #fff; 
  border-bottom: 1px solid #f0f0f0; 
  flex-shrink: 0; 
  z-index: 100; 
  padding: 0;
  overflow: hidden;
}
.dark-mode .header { background: #1f1f1f; border-bottom-color: #303030; }

.header-content { display: flex; justify-content: space-between; align-items: center; height: 100%; width: 100%; }

.logo { display: flex; align-items: center; font-size: 16px; font-weight: bold; color: #1890ff; padding: 0 16px; height: 100%; }
.header-menu { flex: 1; height: 100%; display: flex; align-items: center; }
.top-menu { border-bottom: none; background: transparent; height: 100%; line-height: 40px; width: 100%; }
.top-menu :deep(.ant-menu-submenu-title) { height: 40px !important; line-height: 40px !important; padding: 0 12px; }

.header-actions { display: flex; align-items: center; height: 100%; padding-right: 0; }
.search-btn { margin-right: 8px; }

.window-controls { display: flex; height: 100%; }
.win-btn { display: inline-flex; justify-content: center; align-items: center; width: 46px; height: 100%; cursor: pointer; transition: background-color 0.2s; font-size: 14px; color: #595959; }
.dark-mode .win-btn { color: #aaa; }
.win-btn:hover { background-color: rgba(0, 0, 0, 0.05); }
.dark-mode .win-btn:hover { background-color: rgba(255, 255, 255, 0.1); }
.win-btn.close:hover { background-color: #e81123 !important; color: #fff !important; }

.content-container { flex: 1; display: flex; flex-direction: row; overflow: hidden; position: relative; }
.sidebar-wrapper { background: #fafafa; border-right: 1px solid #e8e8e8; height: 100%; overflow: hidden; flex-shrink: 0; }
.dark-mode .sidebar-wrapper { background: #141414; border-right-color: #303030; }
.sidebar-inner { height: 100%; overflow: auto; padding: 0 8px; }
.sidebar-resizer { width: 4px; cursor: col-resize; background: transparent; transition: background 0.2s; z-index: 10; }
.sidebar-resizer:hover { background: #1890ff; }
.main-workspace { flex: 1; display: flex; flex-direction: column; overflow: hidden; background: #fff; min-width: 0; }
.dark-mode .main-workspace { background: #1f1f1f; }
.global-sql-toolbar { display: flex; justify-content: space-between; align-items: center; padding: 0 12px; height: 40px; background: #f5f5f5; border-bottom: 1px solid #d9d9d9; flex-shrink: 0; }
.dark-mode .global-sql-toolbar { background: #1a1a1a; border-bottom-color: #303030; }
.toolbar-left :deep(.ant-btn-text) { width: auto; min-width: 32px; padding: 0 8px; height: 32px; display: flex; align-items: center; justify-content: center; border-radius: 4px; color: #595959; font-size: 14px; }
.dark-mode .toolbar-left :deep(.ant-btn-text) { color: #aaa; }
.toolbar-left :deep(.ant-btn-text:hover) { background: rgba(0,0,0,0.06); color: #1890ff; }
.btn-run { color: #52c41a !important; font-weight: bold; }
.btn-run:hover { background: #f6ffed !important; }
.btn-stop { color: #ff4d4f !important; }
.btn-stop:hover { background: #fff1f0 !important; }
.db-label { font-size: 12px; color: #8c8c8c; margin-right: 8px; }
.workspace-tabs { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.workspace-tabs :deep(.ant-tabs-nav) { margin-bottom: 0; padding: 0 4px; background: #f0f0f0; flex-shrink: 0; }
.dark-mode .workspace-tabs :deep(.ant-tabs-nav) { background: #141414; }
.workspace-tabs :deep(.ant-tabs-content) { flex: 1; height: 100%; overflow: hidden; }
.workspace-tabs :deep(.ant-tabs-tabpane) { height: 100%; display: flex; flex-direction: column; }
.tab-content-wrapper { flex: 1; height: 100%; overflow: hidden; position: relative; }
.empty-workspace { flex: 1; display: flex; align-items: center; justify-content: center; }
</style>
