<template>
  <div class="redis-editor-container">
    <!-- 工具栏 -->
    <div class="editor-toolbar">
      <a-space>
        <a-button
          type="primary"
          :icon="h(CaretRightOutlined)"
          @click="executeCommand"
          :loading="executing"
          :disabled="!hasActiveConnection"
        >
          {{ $t('redis.execute') }}
        </a-button>
        <a-button
          :icon="h(ClearOutlined)"
          @click="clearEditor"
        >
          {{ $t('common.clear') }}
        </a-button>
        <a-divider type="vertical" />
        <a-button :icon="h(HistoryOutlined)" @click="showHistory = true">
          {{ $t('common.history') }}
        </a-button>
        <a-button :icon="h(InfoCircleOutlined)" @click="showInfo = true">
          {{ $t('redis.server_info') }}
        </a-button>
        <a-divider type="vertical" />
        <a-select
          v-model:value="selectedDatabase"
          :placeholder="$t('redis.select_database')"
          style="width: 150px"
          :disabled="!hasActiveConnection"
          @change="handleDatabaseChange"
        >
          <a-select-option
            v-for="i in 16"
            :key="i - 1"
            :value="`db${i - 1}`"
          >
            db{{ i - 1 }}
          </a-select-option>
        </a-select>
      </a-space>
      <div class="editor-info">
        <a-tag v-if="connectionInfo" color="red">
          <DatabaseOutlined /> {{ connectionInfo.name }}
        </a-tag>
        <a-tag v-if="selectedDatabase" color="orange">
          {{ selectedDatabase }}
        </a-tag>
        <span class="cursor-position">{{ $t('redis.line_col', { line: cursorLine, col: cursorColumn }) }}</span>
      </div>
    </div>

    <!-- 命令输入编辑器 -->
    <RedisCommandInput
      ref="commandInputRef"
      @execute="executeCommand"
      @cursor-change="handleCursorChange"
    />

    <!-- 结果面板 -->
    <RedisResultPanel
      ref="resultPanelRef"
      :results="commandResults"
      :messages="messages"
    />

    <!-- 历史记录对话框 -->
    <a-modal
      v-model:open="showHistory"
      :title="$t('redis.command_history')"
      :width="800"
      :footer="null"
    >
      <a-list :data-source="commandHistory" size="small">
        <template #renderItem="{ item }">
          <a-list-item>
            <template #actions>
              <a @click="loadFromHistory(item)">{{ $t('redis.load') }}</a>
              <a @click="removeFromHistory(item)">{{ $t('common.delete') }}</a>
            </template>
            <a-list-item-meta>
              <template #title>
                <code>{{ item.command }}</code>
              </template>
              <template #description>
                {{ new Date(item.timestamp).toLocaleString() }} •
                {{ item.database || $t('redis.default_db') }}
              </template>
            </a-list-item-meta>
          </a-list-item>
        </template>
      </a-list>
    </a-modal>

    <!-- 服务器信息对话框 -->
    <RedisServerInfo
      v-model:open="showInfo"
      :connection-id="connectionStore.activeConnectionId"
    />
  </div>
</template>

<script setup lang="ts">
import { h, onMounted, onUnmounted, watch, ref, computed } from 'vue'
import {
  CaretRightOutlined,
  ClearOutlined,
  HistoryOutlined,
  InfoCircleOutlined,
  DatabaseOutlined,
} from '@ant-design/icons-vue'
import { message } from 'ant-design-vue'
import { useI18n } from 'vue-i18n'
import { redisApi } from '@/api'
import { useConnectionStore } from '@/stores/connection'
import { getStorageItem, setStorageItem, STORAGE_KEYS } from '@/utils/storageService'
import RedisCommandInput from './RedisCommandInput.vue'
import RedisResultPanel from './RedisResultPanel.vue'
import type { RedisMessage } from './RedisResultPanel.vue'
import RedisServerInfo from './RedisServerInfo.vue'

const { t } = useI18n()
const connectionStore = useConnectionStore()

const commandInputRef = ref<InstanceType<typeof RedisCommandInput>>()
const resultPanelRef = ref<InstanceType<typeof RedisResultPanel>>()

const executing = ref(false)
const commandResults = ref<Record<string, unknown>[]>([])
const showHistory = ref(false)
const showInfo = ref(false)
const cursorLine = ref(1)
const cursorColumn = ref(1)
const selectedDatabase = ref('db0')
let keepAliveTimer: number | null = null

const messages = ref<RedisMessage[]>([])

interface CommandHistoryItem {
  command: string
  timestamp: number
  database?: string
}

const commandHistory = ref<CommandHistoryItem[]>([])

// 连接信息
const connectionInfo = computed(() => {
  const activeId = connectionStore.activeConnectionId
  if (!activeId) return null
  return connectionStore.connections.find((c) => c.id === activeId)
})

const hasActiveConnection = computed(() => !!connectionStore.activeConnectionId)

function handleCursorChange(line: number, column: number) {
  cursorLine.value = line
  cursorColumn.value = column
}

// 初始化
onMounted(() => {
  loadHistory()
  if (connectionStore.activeConnectionId) {
    startKeepAlive()
  }
})

onUnmounted(() => {
  stopKeepAlive()
})

// 监听连接变化
watch(
  () => connectionStore.activeConnectionId,
  (newId) => {
    commandResults.value = []
    messages.value = []
    if (newId) {
      startKeepAlive()
    } else {
      stopKeepAlive()
    }
  }
)

// 执行 Redis 命令
async function executeCommand() {
  if (!hasActiveConnection.value) {
    message.warning(t('redis.no_connection'))
    return
  }

  const command = commandInputRef.value?.getValue() || ''
  if (!command) {
    message.warning(t('redis.input_command'))
    return
  }

  executing.value = true
  resultPanelRef.value?.setActiveKey('result')

  const dbInfo = selectedDatabase.value ? ` (${selectedDatabase.value})` : ''
  addMessage('info', `${t('redis.executing')}${dbInfo}`)

  try {
    const result = await redisApi.executeCommand(
      connectionStore.activeConnectionId!,
      command,
      [],
    )

    commandResults.value.push(result)

    if (result.success) {
      addMessage('success', `${t('redis.exec_success', { time: result.execution_time_ms })}${dbInfo}`)
      saveToHistory(command)
    } else {
      addMessage('error', `${t('redis.exec_fail')}${dbInfo}: ${result.error}`)
      message.error(`${t('redis.exec_fail')}: ${result.error}`)
    }
  } catch (error: unknown) {
    addMessage('error', `${t('redis.exec_fail')}${dbInfo}: ${error}`)
    message.error(`${t('redis.exec_fail')}: ${error}`)
  } finally {
    executing.value = false
  }
}

// 清空编辑器
function clearEditor() {
  commandInputRef.value?.setValue('')
  commandResults.value = []
  messages.value = []
}

// 切换数据库
async function handleDatabaseChange(database: unknown) {
  const dbStr = String(database || 'db0')
  selectedDatabase.value = dbStr
  const dbNum = dbStr.replace('db', '')

  try {
    await redisApi.executeCommand(
      connectionStore.activeConnectionId!,
      `SELECT ${dbNum}`,
      [],
    )
    message.success(t('redis.switched_db', { db: dbStr }))
  } catch (error: unknown) {
    message.error(t('redis.switch_fail', { error: String(error) }))
  }
}

// 添加消息
function addMessage(type: RedisMessage['type'], text: string) {
  messages.value.unshift({
    type,
    text,
    time: new Date().toLocaleTimeString(),
  })
}

// 保存到历史
function saveToHistory(command: string) {
  commandHistory.value.unshift({
    command,
    timestamp: Date.now(),
    database: selectedDatabase.value,
  })
  if (commandHistory.value.length > 100) {
    commandHistory.value = commandHistory.value.slice(0, 100)
  }
  setStorageItem(STORAGE_KEYS.REDIS_HISTORY, commandHistory.value)
}

// 加载历史
function loadHistory() {
  commandHistory.value = getStorageItem<CommandHistoryItem[]>(STORAGE_KEYS.REDIS_HISTORY, [])
}

// 从历史加载
function loadFromHistory(item: CommandHistoryItem) {
  commandInputRef.value?.setValue(item.command)
  showHistory.value = false
  message.success(t('redis.history_loaded'))
}

// 从历史删除
function removeFromHistory(item: CommandHistoryItem) {
  commandHistory.value = commandHistory.value.filter((h) => h.timestamp !== item.timestamp)
  setStorageItem(STORAGE_KEYS.REDIS_HISTORY, commandHistory.value)
}

// 切换数据库（供外部调用）
async function switchDatabase(dbName: string) {
  selectedDatabase.value = dbName
  const dbNum = dbName.replace('db', '')

  try {
    await redisApi.executeCommand(
      connectionStore.activeConnectionId!,
      `SELECT ${dbNum}`,
      [],
    )
  } catch (error: unknown) {
    console.error('Failed to switch database:', error)
    throw error
  }
}

// 启动保活定时器（每30秒发送一次PING）
function startKeepAlive() {
  stopKeepAlive()

  keepAliveTimer = window.setInterval(async () => {
    if (!connectionStore.activeConnectionId) {
      stopKeepAlive()
      return
    }

    try {
      await redisApi.executeCommand(
        connectionStore.activeConnectionId!,
        'PING',
        [],
      )
      console.log('Redis keepalive: PING OK')
    } catch (error) {
      console.error('Redis keepalive failed:', error)
      message.warning(t('redis.keepalive_fail'))
      stopKeepAlive()
    }
  }, 30000)
}

// 停止保活定时器
function stopKeepAlive() {
  if (keepAliveTimer !== null) {
    clearInterval(keepAliveTimer)
    keepAliveTimer = null
  }
}

// 暴露方法供父组件调用
defineExpose({
  switchDatabase,
})
</script>

<style scoped>
.redis-editor-container {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.editor-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  border-bottom: 1px solid #e8e8e8;
  background: #fafafa;
}

.dark-mode .editor-toolbar {
  background: #1f1f1f;
  border-bottom-color: #303030;
}

.editor-info {
  display: flex;
  gap: 12px;
  align-items: center;
}

.cursor-position {
  font-size: 12px;
  color: #8c8c8c;
}
</style>
