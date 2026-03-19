<template>
  <div class="connection-panel">
    <div class="panel-header">
      <span class="panel-title">连接管理器</span>
      <a-button
        type="text"
        size="small"
        :icon="h(PlusOutlined)"
        @click="$emit('add-connection')"
      >
        新建
      </a-button>
    </div>

    <div class="panel-content">
      <div class="search-wrapper">
        <a-input
          v-model:value="searchText"
          placeholder="搜索连接..."
          size="small"
          :bordered="false"
          class="compact-search"
        >
          <template #prefix><SearchOutlined style="color: #bfbfbf" /></template>
        </a-input>
      </div>

      <div class="connection-list">
        <div
          v-for="conn in filteredConnections"
          :key="conn.id"
          class="connection-group"
        >
          <!-- 连接项：表现得像一个标准的顶级树节点 -->
          <div
            class="connection-item"
            :class="{ 
              active: activeConnectionId === conn.id,
              expanded: expandedConnections.has(conn.id)
            }"
            :title="`${conn.db_type} • ${conn.host}:${conn.port}`"
            @click="handleSelectConnection(conn)"
            @dblclick="handleToggleConnection(conn)"
            @contextmenu.prevent="handleContextMenu($event, conn)"
          >
            <div class="connection-expand-icon" @click.stop="handleToggleExpand(conn)">
              <DownOutlined 
                v-if="getConnectionStatus(conn.id) === 'connected' && expandedConnections.has(conn.id)" 
                class="expand-icon"
              />
              <RightOutlined 
                v-else-if="getConnectionStatus(conn.id) === 'connected'" 
                class="expand-icon"
              />
              <span v-else style="width: 12px; display: inline-block;"></span>
            </div>
            <div class="connection-icon">
              <ClusterOutlined :style="{ color: '#1890ff' }" />
            </div>
            <div class="connection-name">{{ conn.name }}</div>
            <div class="connection-actions">
              <a-badge :status="getStatusBadge(conn.id)" size="small" />
              <DisconnectOutlined 
                v-if="getConnectionStatus(conn.id) === 'connected'"
                class="disconnect-btn"
                @click.stop="handleDisconnect(conn)"
              />
            </div>
          </div>
          
          <!-- 数据库对象树 -->
          <div 
            v-if="getConnectionStatus(conn.id) === 'connected' && expandedConnections.has(conn.id)" 
            class="database-tree-wrapper"
          >
            <!-- 贯穿引导线 -->
            <div class="root-tree-line"></div>
            
            <DatabaseTree
              :ref="el => { if (el) databaseTreeRefs.set(conn.id, el) }"
              :connection-id="conn.id"
              :db-type="conn.db_type"
              @table-selected="(data) => emit('table-selected', { ...data, connectionId: conn.id })"
              @database-selected="(data) => emit('database-selected', { ...data, connectionId: conn.id })"
              @new-query="(data) => emit('new-query', data)"
              @design-table="(data) => emit('design-table', { ...data, connectionId: conn.id })"
            />
          </div>
        </div>
      </div>

      <a-empty
        v-if="filteredConnections.length === 0"
        description="暂无连接"
        :image="Empty.PRESENTED_IMAGE_SIMPLE"
        style="margin-top: 40px"
      />
    </div>

    <!-- 右键菜单 -->
    <div
      v-if="contextMenuVisible"
      class="context-menu-overlay"
      @click="contextMenuVisible = false"
    >
      <div
        class="context-menu"
        :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"
        @click.stop
      >
        <a-menu @click="handleMenuClick" size="small">
          <a-menu-item key="connect" v-if="getConnectionStatus(selectedConnection?.id || '') !== 'connected'">
            <LinkOutlined /> 连接
          </a-menu-item>
          <a-menu-item key="disconnect" v-if="getConnectionStatus(selectedConnection?.id || '') === 'connected'">
            <DisconnectOutlined /> 断开连接
          </a-menu-item>
          <a-menu-divider v-if="getConnectionStatus(selectedConnection?.id || '') === 'connected' && canCreateDatabase" />
          <a-menu-item key="create-database" v-if="getConnectionStatus(selectedConnection?.id || '') === 'connected' && canCreateDatabase">
            <DatabaseOutlined /> 新建数据库
          </a-menu-item>
          <a-menu-divider />
          <a-menu-item key="edit"><EditOutlined /> 编辑</a-menu-item>
          <a-menu-item key="delete" danger><DeleteOutlined /> 删除</a-menu-item>
        </a-menu>
      </div>
    </div>
    
    <CreateDatabaseDialog
      v-model:visible="showCreateDatabaseDialog"
      :connection-id="selectedConnection?.id || ''"
      :db-type="selectedConnection?.db_type"
      @created="handleDatabaseCreated"
    />
  </div>
</template>

<script setup lang="ts">
import { h, computed, ref, onMounted, onUnmounted } from 'vue'
import { 
  DatabaseOutlined, PlusOutlined, LinkOutlined, EditOutlined, DeleteOutlined, 
  DisconnectOutlined, DownOutlined, RightOutlined, ClusterOutlined, SearchOutlined
} from '@ant-design/icons-vue'
import { message, Modal, Empty } from 'ant-design-vue'
import { useConnectionStore } from '@/stores/connection'
import type { ConnectionConfig } from '@/types/database'
import DatabaseTree from '@/components/database/DatabaseTree.vue'
import CreateDatabaseDialog from '@/components/database/CreateDatabaseDialog.vue'

const emit = defineEmits(['add-connection', 'edit-connection', 'table-selected', 'database-selected', 'new-query', 'design-table'])

const connectionStore = useConnectionStore()
const searchText = ref('')
const activeConnectionId = computed(() => connectionStore.activeConnectionId)
const showCreateDatabaseDialog = ref(false)
const expandedConnections = ref<Set<string>>(new Set())
const contextMenuVisible = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const selectedConnection = ref<ConnectionConfig | null>(null)
const databaseTreeRefs = new Map<string, any>()

const canCreateDatabase = computed(() => {
  if (!selectedConnection.value) return false
  return selectedConnection.value.db_type?.toLowerCase() !== 'sqlite'
})

const filteredConnections = computed(() => {
  const list = connectionStore.connections
  if (!searchText.value) return list
  const text = searchText.value.toLowerCase()
  return list.filter(c => c.name.toLowerCase().includes(text) || c.host.toLowerCase().includes(text))
})

function handleSelectConnection(conn: ConnectionConfig) {
  connectionStore.setActiveConnection(conn.id)
}

function handleToggleExpand(conn: ConnectionConfig) {
  if (getConnectionStatus(conn.id) !== 'connected') {
    handleConnectToDatabase(conn)
    return
  }
  const newExpanded = new Set(expandedConnections.value)
  if (newExpanded.has(conn.id)) newExpanded.delete(conn.id)
  else newExpanded.add(conn.id)
  expandedConnections.value = newExpanded
}

async function handleToggleConnection(conn: ConnectionConfig) {
  if (getConnectionStatus(conn.id) === 'connected') {
    const newExpanded = new Set(expandedConnections.value)
    if (newExpanded.has(conn.id)) newExpanded.delete(conn.id)
    else newExpanded.add(conn.id)
    expandedConnections.value = newExpanded
  } else {
    await handleConnectToDatabase(conn)
  }
}

async function handleConnectToDatabase(conn: ConnectionConfig) {
  try {
    connectionStore.updateConnectionStatus(conn.id, 'connecting')
    await connectionStore.connectToDatabase(conn.id)
    connectionStore.setActiveConnection(conn.id)
    connectionStore.updateConnectionStatus(conn.id, 'connected')
    const newExpanded = new Set(expandedConnections.value)
    newExpanded.add(conn.id)
    expandedConnections.value = newExpanded
    message.success(`已连接到 ${conn.name}`)
  } catch (error: any) {
    connectionStore.updateConnectionStatus(conn.id, 'error')
    message.error(`连接失败: ${error}`)
  }
}

async function handleDisconnect(conn: ConnectionConfig) {
  try {
    await connectionStore.disconnectFromDatabase(conn.id)
    connectionStore.updateConnectionStatus(conn.id, 'disconnected')
    const newExpanded = new Set(expandedConnections.value)
    newExpanded.delete(conn.id)
    expandedConnections.value = newExpanded
    message.success(`已断开连接 ${conn.name}`)
  } catch (error: any) { message.error(`断开连接失败: ${error}`) }
}

function handleContextMenu(event: MouseEvent, conn: ConnectionConfig) {
  event.preventDefault()
  selectedConnection.value = conn
  contextMenuX.value = event.clientX
  contextMenuY.value = event.clientY
  contextMenuVisible.value = true
}

async function handleMenuClick({ key }: any) {
  if (!selectedConnection.value) return
  contextMenuVisible.value = false
  if (key === 'connect') await handleConnectToDatabase(selectedConnection.value)
  else if (key === 'disconnect') await handleDisconnect(selectedConnection.value)
  else if (key === 'create-database') showCreateDatabaseDialog.value = true
  else if (key === 'edit') emit('edit-connection', selectedConnection.value)
  else if (key === 'delete') {
    Modal.confirm({
      title: '确认删除',
      content: `确定要删除连接 "${selectedConnection.value.name}" 吗？`,
      async onOk() {
        try {
          await connectionStore.deleteConnection(selectedConnection.value!.id)
          message.success('连接已删除')
        } catch (error: any) { message.error(`删除失败: ${error}`) }
      }
    })
  }
}

function handleDatabaseCreated() {
  if (selectedConnection.value) databaseTreeRefs.get(selectedConnection.value.id)?.refresh()
}

function getConnectionStatus(id: string) { return connectionStore.getConnectionStatus(id) }
function getStatusBadge(id: string) {
  const s = connectionStore.getConnectionStatus(id)
  return s === 'connected' ? 'success' : s === 'connecting' ? 'processing' : s === 'error' ? 'error' : 'default'
}

onMounted(() => {
  connectionStore.fetchConnections()
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape') contextMenuVisible.value = false })
})
</script>

<style scoped>
.connection-panel { display: flex; flex-direction: column; height: 100%; background: transparent; }

.panel-header {
  display: flex; justify-content: space-between; align-items: center;
  padding: 8px 12px; border-bottom: 1px solid #f0f0f0;
}
.dark-mode .panel-header { border-bottom-color: #303030; }
.panel-title { font-size: 12px; font-weight: 600; color: #8c8c8c; text-transform: uppercase; }

.search-wrapper { padding: 4px 8px; border-bottom: 1px solid #f0f0f0; }
.dark-mode .search-wrapper { border-bottom-color: #303030; }
.compact-search { background: transparent; }

.panel-content { flex: 1; overflow: auto; padding: 4px 0; }

.connection-group { position: relative; }

.connection-item {
  display: flex; align-items: center; padding: 0 8px; height: 24px;
  cursor: pointer; transition: background-color 0.1s; user-select: none;
  position: relative;
}

.connection-item:hover { background-color: rgba(0, 0, 0, 0.04); }
.dark-mode .connection-item:hover { background-color: rgba(255, 255, 255, 0.05); }
.connection-item.active { background-color: #e6f7ff; color: #1890ff; }
.dark-mode .connection-item.active { background-color: #111b26; color: #177ddc; }

.connection-expand-icon {
  display: flex; align-items: center; justify-content: center;
  width: 16px; font-size: 10px; color: #bfbfbf; margin-right: 2px;
}

.connection-icon { font-size: 14px; margin-right: 6px; flex-shrink: 0; }

.connection-name {
  flex: 1; font-size: 12px; font-weight: 500;
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}

.connection-actions { display: flex; align-items: center; gap: 6px; opacity: 0.6; }
.connection-item:hover .connection-actions { opacity: 1; }

.disconnect-btn { font-size: 12px; color: #ff4d4f; cursor: pointer; }
.disconnect-btn:hover { color: #ff7875; }

.database-tree-wrapper { position: relative; padding-left: 0; }

/* 贯穿引导线：从集群图标中心开始向下延伸 */
.root-tree-line {
  position: absolute; left: 16px; top: 0; bottom: 0;
  width: 1px; background-color: #e8e8e8; z-index: 1; pointer-events: none;
}
.dark-mode .root-tree-line { background-color: #303030; }

/* 右键菜单 */
.context-menu-overlay { position: fixed; top: 0; left: 0; right: 0; bottom: 0; z-index: 9999; }
.context-menu {
  position: absolute; background: #fff; border-radius: 4px; border: 1px solid #d9d9d9;
  box-shadow: 0 2px 8px rgba(0,0,0,0.15); z-index: 10000; min-width: 120px;
}
.dark-mode .context-menu { background: #1f1f1f; border-color: #303030; }
</style>
