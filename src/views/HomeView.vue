<template>
  <a-layout class="main-layout">
    <!-- 顶部导航栏 -->
    <a-layout-header class="header">
      <div class="header-content">
        <div class="logo" @click="handleNewQuery({})" style="cursor: pointer">
          <DatabaseOutlined style="font-size: 18px; margin-right: 6px" />
          <span class="title">DataSmith</span>
        </div>
        <div class="header-menu">
          <a-menu mode="horizontal" :selected-keys="[]" class="top-menu">
            <a-sub-menu key="file">
              <template #title>文件</template>
              <a-menu-item key="new-connection" @click="showConnectionDialog = true">
                <PlusOutlined />
                新建连接
              </a-menu-item>
              <a-menu-divider />
              <a-menu-item key="settings" @click="showSettings = true">
                <SettingOutlined />
                设置
              </a-menu-item>
            </a-sub-menu>
            <a-sub-menu key="view">
              <template #title>视图</template>
              <a-menu-item key="toggle-sidebar" @click="appStore.toggleSidebar()">
                <MenuOutlined />
                {{ appStore.sidebarCollapsed ? '显示' : '隐藏' }}侧边栏
              </a-menu-item>
              <a-menu-divider />
              <a-menu-item key="theme" @click="appStore.toggleTheme()">
                <BulbOutlined />
                {{ appStore.theme === 'light' ? '暗色' : '明亮' }}主题
              </a-menu-item>
            </a-sub-menu>
          </a-menu>
        </div>
        <div class="header-actions">
          <a-space>
            <a-button type="text" @click="showGlobalSearch = true">
              <template #icon><SearchOutlined /></template>
            </a-button>
            <a-button type="primary" @click="showConnectionDialog = true">连接</a-button>
          </a-space>
        </div>
      </div>
    </a-layout-header>

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
            <a-space :size="2">
              <a-tooltip title="执行 (F5)">
                <a-button type="text" size="small" :icon="h(CaretRightOutlined)" @click="callActiveEditor('executeQuery')" :loading="activeEditorExecuting" class="execute-btn" />
              </a-tooltip>
              <a-tooltip title="停止">
                <a-button type="text" size="small" :icon="h(StopOutlined)" @click="callActiveEditor('stopExecution')" :disabled="!activeEditorExecuting" />
              </a-tooltip>
              <a-divider type="vertical" />
              <a-tooltip title="保存 (Ctrl+S)">
                <a-button type="text" size="small" :icon="h(SaveOutlined)" @click="callActiveEditor('handleSave')" />
              </a-tooltip>
              <a-divider type="vertical" />
              <a-tooltip title="格式化 SQL">
                <a-button type="text" size="small" :icon="h(FormatPainterOutlined)" @click="callActiveEditor('formatSql')" />
              </a-tooltip>
              <a-tooltip title="清空编辑器">
                <a-button type="text" size="small" :icon="h(ClearOutlined)" @click="callActiveEditor('clearEditor')" />
              </a-tooltip>
              <a-tooltip title="历史记录">
                <a-button type="text" size="small" :icon="h(HistoryOutlined)" @click="callActiveEditor('openHistory')" />
              </a-tooltip>
              <a-tooltip title="代码片段">
                <a-button type="text" size="small" :icon="h(CodeOutlined)" @click="callActiveEditor('openSnippets')" />
              </a-tooltip>
              <a-tooltip title="刷新补全数据">
                <a-button type="text" size="small" :icon="h(ReloadOutlined)" @click="callActiveEditor('refreshAutocomplete')" />
              </a-tooltip>
            </a-space>
          </div>
          <div class="toolbar-right">
            <a-space>
              <span class="db-label">DB:</span>
              <a-select v-model:value="activeTabDatabase" placeholder="选择数据库" size="small" style="width: 140px" @change="handleToolbarDbChange">
                <a-select-option value="">默认</a-select-option>
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
          <a-empty description="从左侧选择表开始探索">
            <template #extra><a-button type="primary" @click="handleNewQuery({})">新建查询 (Ctrl+N)</a-button></template>
          </a-empty>
        </div>
      </div>
    </a-layout-content>

    <a-dropdown :open="contextMenuVisible" :trigger="['contextmenu']">
      <div v-if="contextMenuVisible" class="context-menu-overlay" @click="contextMenuVisible = false" :style="{ left: contextMenuPosition.x + 'px', top: contextMenuPosition.y + 'px' }"></div>
      <template #overlay>
        <a-menu @click="handleContextMenuClick" class="context-menu">
          <a-menu-item key="close"><CloseOutlined />关闭当前标签</a-menu-item>
          <a-menu-item key="closeOthers" :disabled="dataTabs.length <= 1"><CloseCircleOutlined />关闭其他标签</a-menu-item>
          <a-menu-item key="closeAll" :disabled="dataTabs.length === 0"><CloseSquareOutlined />关闭所有标签</a-menu-item>
          <a-menu-divider />
          <a-menu-item key="closeLeft" :disabled="!hasTabsOnLeft"><VerticalRightOutlined />关闭左侧标签</a-menu-item>
          <a-menu-item key="closeRight" :disabled="!hasTabsOnRight"><VerticalLeftOutlined />关闭右侧标签</a-menu-item>
        </a-menu>
      </template>
    </a-dropdown>

    <ConnectionDialog v-model:visible="showConnectionDialog" :editing-connection="editingConnection" @close="editingConnection = null" />
    <a-modal v-model:open="showSettings" title="设置" @ok="handleSaveSettings">
      <a-form :label-col="{ span: 6 }" :wrapper-col="{ span: 18 }">
        <a-form-item label="主题">
          <a-radio-group v-model:value="settingsForm.theme"><a-radio value="light">明亮</a-radio><a-radio value="dark">暗色</a-radio></a-radio-group>
        </a-form-item>
      </a-form>
    </a-modal>
    <GlobalSearch v-model:visible="showGlobalSearch" :connection-id="connectionStore.activeConnectionId" @view-data="handleTableSelected" />
  </a-layout>
</template>

<script setup lang="ts">
import { h, reactive, ref, computed, nextTick, onMounted, watch } from 'vue'
import { 
  DatabaseOutlined, BulbOutlined, PlusOutlined, SettingOutlined, 
  MenuOutlined, FileTextOutlined, SearchOutlined, 
  CloseOutlined, CloseCircleOutlined, CloseSquareOutlined, 
  VerticalRightOutlined, VerticalLeftOutlined, TableOutlined, EditOutlined,
  CaretRightOutlined, StopOutlined, SaveOutlined,
  FormatPainterOutlined, ClearOutlined, HistoryOutlined, CodeOutlined, ReloadOutlined
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

const appStore = useAppStore()
const connectionStore = useConnectionStore()
const workspaceStore = useWorkspaceStore()
const showConnectionDialog = ref(false)
const showSettings = ref(false)
const showGlobalSearch = ref(false)
const mainTabKey = ref('')
const editingConnection = ref<any>(null)
const sqlEditorRefs = reactive<Record<string, any>>({})
const redisEditorRef = ref<any>(null)
const availableDatabases = ref<any[]>([])
const sidebarWidth = ref(280)

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
  console.log('[Restore] 开始恢复会话...')
  workspaceStore.isRestoring = true
  try {
    const session = await workspaceStore.loadSession()
    if (session && session.open_tabs.length > 0) {
      dataTabs.value = session.open_tabs.map(t => ({ ...t, type: (t as any).tab_type || t.type })) as any
      mainTabKey.value = session.active_tab_key
      console.log(`[Restore] 加载了 ${dataTabs.value.length} 个标签页`)
      
      if (connectionStore.connections.length === 0) await connectionStore.fetchConnections()
      
      const connectionIds = [...new Set(dataTabs.value.map(t => t.connectionId).filter(Boolean))] as string[]
      for (const id of connectionIds) {
        const conn = connectionStore.connections.find(c => c.id === id)
        if (conn) {
          console.log(`[Restore] 自动重连数据库: ${conn.name}`)
          connectionStore.connectToDatabase(conn.id).catch(e => console.warn(`自动重连失败 (${conn.name}):`, e))
        }
      }
    } else if (isSqlSupported.value) { handleNewQuery({}) }
  } catch (e) { console.error('恢复失败:', e); if (isSqlSupported.value) handleNewQuery({}) } finally { workspaceStore.isRestoring = false }
}

const activeTabType = computed(() => dataTabs.value.find(t => t.key === mainTabKey.value)?.type)
const activeTabDatabase = computed({
  get: () => dataTabs.value.find(t => t.key === mainTabKey.value)?.database || '',
  set: (val) => { const t = dataTabs.value.find(t => t.key === mainTabKey.value); if (t) t.database = val; }
})
const activeEditorExecuting = computed(() => sqlEditorRefs[mainTabKey.value]?.executing || false)

function callActiveEditor(method: string, ...args: any[]) { const editor = sqlEditorRefs[mainTabKey.value]; if (editor && editor[method]) editor[method](...args) }
function handleToolbarDbChange(val: any) { callActiveEditor('handleDatabaseChange', String(val || '')) }
onMounted(() => { restoreSession() })
function setSqlEditorRef(el: any, key: string) { if (el) sqlEditorRefs[key] = el; else delete sqlEditorRefs[key]; }
function handleContentChange(key: string, val: string) { const t = dataTabs.value.find(t => t.key === key); if (t) t.content = val; }
function handleFileSaved(key: string, path: string, title: string) {
  const t = dataTabs.value.find(t => t.key === key); if (t) { t.filePath = path; t.title = title; }
}
function handleOpenSavedScript(data: any) { handleNewQuery(data) }
function handleViewStructure(data: any) {
  const key = `structure-${data.connectionId}-${data.database}-${data.table}`
  if (dataTabs.value.some(t => t.key === key)) { mainTabKey.value = key; return; }
  dataTabs.value.push({ key, title: `结构: ${data.table}`, type: 'design', ...data, readOnly: true })
  mainTabKey.value = key
}
function handleDesignTable(data: any) {
  const key = `design-${data.connectionId}-${data.database}-${data.table}`
  if (dataTabs.value.some(t => t.key === key)) { mainTabKey.value = key; return; }
  dataTabs.value.push({ key, title: `设计: ${data.table}`, type: 'design', ...data, readOnly: false })
  mainTabKey.value = key
}

const contextMenuVisible = ref(false), contextMenuPosition = reactive({ x: 0, y: 0 }), currentContextTab = reactive({ key: '', closable: false })
const hasTabsOnLeft = computed(() => dataTabs.value.findIndex(t => t.key === currentContextTab.key) > 0)
const hasTabsOnRight = computed(() => { const i = dataTabs.value.findIndex(t => t.key === currentContextTab.key); return i >= 0 && i < dataTabs.value.length - 1 })
function handleTabContextMenu(e: MouseEvent, key: string, closable: boolean) { e.preventDefault(); currentContextTab.key = key; currentContextTab.closable = closable; contextMenuPosition.x = e.clientX; contextMenuPosition.y = e.clientY; contextMenuVisible.value = true; }
function handleContextMenuClick({ key }: any) {
  contextMenuVisible.value = false; const idx = dataTabs.value.findIndex(t => t.key === currentContextTab.key)
  if (key === 'close') closeTab(currentContextTab.key)
  else if (key === 'closeOthers') dataTabs.value = dataTabs.value.filter((t, i) => i === idx || t.closable === false)
  else if (key === 'closeAll') dataTabs.value = dataTabs.value.filter(t => t.closable === false)
  if (!dataTabs.value.some(t => t.key === mainTabKey.value) && dataTabs.value.length > 0) mainTabKey.value = dataTabs.value[0].key
}
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
  const dbName = d.database
  
  // 核心逻辑：如果绑定了库，强制先在后端创建物理文件
  let filePath = d.filePath
  let title = d.title
  let initialContent = d.content || '-- 在此输入 SQL 查询\n'

  if (connId && dbName && !filePath) {
    try {
      const script = await invoke<any>('create_db_script', {
        connectionId: connId,
        database: dbName,
        content: initialContent
      })
      filePath = script.path
      title = script.name
    } catch (e) {
      console.error('物理脚本创建失败:', e)
      message.error('无法在磁盘上创建脚本文件')
      return
    }
  }

  const key = `query-${Date.now()}`
  dataTabs.value.push({ 
    key, 
    title: title || `script-${new Date().getTime()}.sql`, 
    type: 'query', 
    connectionId: connId || undefined, 
    database: dbName, 
    content: initialContent, 
    filePath 
  })
  mainTabKey.value = key
}

function onTabEdit(key: any, action: string) { if (action === 'add') handleNewQuery({}); else closeTab(String(key)); }
const settingsForm = reactive({ theme: appStore.theme }); function handleSaveSettings() { appStore.setTheme(settingsForm.theme); showSettings.value = false; }
function startResize(e: MouseEvent) {
  const sx = e.clientX, sw = sidebarWidth.value; const dr = (ev: MouseEvent) => { const nw = sw + (ev.clientX - sx); if (nw >= 200 && nw <= 600) sidebarWidth.value = nw }; const sr = () => { document.removeEventListener('mousemove', dr); document.removeEventListener('mouseup', sr) }
  document.addEventListener('mousemove', dr); document.addEventListener('mouseup', sr)
}
function handleEditConnection(c: any) { editingConnection.value = c; showConnectionDialog.value = true; }
</script>

<style scoped>
.main-layout { height: 100vh; width: 100vw; display: flex; flex-direction: column; overflow: hidden; }
.header { height: 64px; padding: 0 16px; background: #fff; border-bottom: 1px solid #f0f0f0; flex-shrink: 0; z-index: 100; }
.dark-mode .header { background: #1f1f1f; border-bottom-color: #303030; }
.header-content { display: flex; justify-content: space-between; align-items: center; height: 100%; }
.logo { display: flex; align-items: center; font-size: 20px; font-weight: bold; color: #1890ff; }
.header-menu { flex: 1; margin-left: 24px; }
.top-menu { border-bottom: none; background: transparent; }
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
.toolbar-left :deep(.ant-btn-text) { width: 32px; height: 32px; display: flex; align-items: center; justify-content: center; border-radius: 4px; color: #595959; font-size: 16px; }
.dark-mode .toolbar-left :deep(.ant-btn-text) { color: #aaa; }
.toolbar-left :deep(.ant-btn-text:hover) { background: rgba(0,0,0,0.06); color: #1890ff; }
.execute-btn { color: #52c41a !important; }
.db-label { font-size: 12px; color: #8c8c8c; margin-right: 8px; }
.workspace-tabs { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.workspace-tabs :deep(.ant-tabs-nav) { margin-bottom: 0; padding: 0 4px; background: #f0f0f0; flex-shrink: 0; }
.dark-mode .workspace-tabs :deep(.ant-tabs-nav) { background: #141414; }
.workspace-tabs :deep(.ant-tabs-content) { flex: 1; height: 100%; overflow: hidden; }
.workspace-tabs :deep(.ant-tabs-tabpane) { height: 100%; display: flex; flex-direction: column; }
.tab-content-wrapper { flex: 1; height: 100%; overflow: hidden; position: relative; }
.empty-workspace { flex: 1; display: flex; align-items: center; justify-content: center; }
.context-menu-overlay { position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 9999; }
.context-menu { position: absolute; background: #fff; border-radius: 4px; border: 1px solid #d9d9d9; box-shadow: 0 2px 8px rgba(0,0,0,0.15); z-index: 10000; min-width: 120px; }
.dark-mode .context-menu { background: #1f1f1f; border-color: #303030; }
</style>
