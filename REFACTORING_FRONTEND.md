# DataSmith 前端重构建议

> 生成日期：2026-03-21
> 基于代码库版本：0.1.0

本文档列出了前端代码中需要重构的部分，按优先级排序。

---

## 🔴 高优先级（类型安全/架构问题）

### 1. Store 中的对象转换重复

**位置：** `src/stores/connection.ts:26-40, 64-78`

**问题描述：**
```typescript
// saveConnection 和 updateConnection 中重复的对象转换逻辑
const storedConnection = {
  id: config.id,
  name: config.name,
  db_type: config.db_type,
  host: config.host,
  port: config.port,
  username: config.username,
  database: config.database,
  ssl: config.ssl,
  connection_timeout: config.connection_timeout,
  pool_size: config.pool_size,
  tags: config.tags || [],
  created_at: config.created_at || Date.now(),
  updated_at: Date.now(),
}
```

**影响：** 违反 DRY 原则，维护困难

**重构方案：**

```typescript
// src/stores/connection.ts

/**
 * 将 ConnectionConfig 转换为存储格式
 */
function toStoredConnection(config: ConnectionConfig, isNew: boolean = false): any {
  return {
    id: config.id,
    name: config.name,
    db_type: config.db_type,
    host: config.host,
    port: config.port,
    username: config.username,
    database: config.database,
    ssl: config.ssl,
    connection_timeout: config.connection_timeout,
    pool_size: config.pool_size,
    tags: config.tags || [],
    created_at: isNew ? Date.now() : config.created_at,
    updated_at: Date.now(),
  }
}

// 使用
async function saveConnection(config: ConnectionConfig, password?: string) {
  try {
    const storedConnection = toStoredConnection(config, true)
    const saved = await invoke<any>('save_connection', {
      connection: storedConnection,
      password: password || null
    })
    // ...
  } catch (error) {
    console.error('保存连接失败:', error)
    throw error
  }
}

async function updateConnection(config: ConnectionConfig, password?: string) {
  try {
    const storedConnection = toStoredConnection(config, false)
    const updated = await invoke<any>('update_connection', {
      connection: storedConnection,
      password: password || null
    })
    // ...
  } catch (error) {
    console.error('更新连接失败:', error)
    throw error
  }
}
```

---

### 2. 缺少统一的错误处理

**位置：** 所有 Store 文件

**问题描述：**
```typescript
// 每个函数都有相同的 try-catch 模式
try {
  // ...
} catch (error) {
  console.error('xxx失败:', error)
  throw error
}
```

**影响：**
- 错误处理不一致
- 无法统一添加错误上报
- 用户体验差（没有友好的错误提示）

**重构方案：**

创建统一的错误处理工具：

```typescript
// src/utils/errorHandler.ts
import { message } from 'ant-design-vue'

export interface ErrorHandlerOptions {
  showMessage?: boolean
  messagePrefix?: string
  rethrow?: boolean
  onError?: (error: Error) => void
}

/**
 * 统一错误处理装饰器
 */
export function handleError(options: ErrorHandlerOptions = {}) {
  const {
    showMessage = true,
    messagePrefix = '操作失败',
    rethrow = true,
    onError
  } = options

  return function (
    target: any,
    propertyKey: string,
    descriptor: PropertyDescriptor
  ) {
    const originalMethod = descriptor.value

    descriptor.value = async function (...args: any[]) {
      try {
        return await originalMethod.apply(this, args)
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error)

        // 日志记录
        console.error(`[${propertyKey}] ${messagePrefix}:`, error)

        // 显示用户友好的错误消息
        if (showMessage) {
          message.error(`${messagePrefix}: ${errorMessage}`)
        }

        // 自定义错误处理
        if (onError) {
          onError(error as Error)
        }

        // 重新抛出错误（可选）
        if (rethrow) {
          throw error
        }
      }
    }

    return descriptor
  }
}

/**
 * 包装异步函数的错误处理
 */
export async function withErrorHandler<T>(
  fn: () => Promise<T>,
  options: ErrorHandlerOptions = {}
): Promise<T | undefined> {
  const {
    showMessage = true,
    messagePrefix = '操作失败',
    rethrow = false,
    onError
  } = options

  try {
    return await fn()
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error)

    console.error(`${messagePrefix}:`, error)

    if (showMessage) {
      message.error(`${messagePrefix}: ${errorMessage}`)
    }

    if (onError) {
      onError(error as Error)
    }

    if (rethrow) {
      throw error
    }

    return undefined
  }
}
```

**使用示例：**

```typescript
// src/stores/connection.ts
import { withErrorHandler } from '@/utils/errorHandler'

export const useConnectionStore = defineStore('connection', () => {
  // ...

  async function fetchConnections() {
    return withErrorHandler(
      async () => {
        connections.value = await invoke<ConnectionConfig[]>('get_connections')
      },
      { messagePrefix: '获取连接列表失败' }
    )
  }

  async function saveConnection(config: ConnectionConfig, password?: string) {
    return withErrorHandler(
      async () => {
        const storedConnection = toStoredConnection(config, true)
        const saved = await invoke<any>('save_connection', {
          connection: storedConnection,
          password: password || null
        })

        const index = connections.value.findIndex(c => c.id === saved.id)
        if (index >= 0) {
          connections.value[index] = { ...config, ...saved }
        } else {
          connections.value.push({ ...config, ...saved })
        }
        return saved
      },
      {
        messagePrefix: '保存连接失败',
        showMessage: true
      }
    )
  }

  // ...
})
```

---

### 3. 类型定义不完整

**位置：** `src/stores/workspace.ts:18`

**问题描述：**
```typescript
export interface SessionState {
  open_tabs: any[]  // ❌ 使用 any
  active_tab_key: string
}
```

**影响：** 失去类型安全，容易出错

**重构方案：**

```typescript
// src/stores/workspace.ts
export interface SessionState {
  open_tabs: TabState[]  // ✅ 使用具体类型
  active_tab_key: string
}

// 或者定义后端返回的格式
export interface StoredTabState {
  key: string
  title: string
  type: 'data' | 'design' | 'query' | 'redis'
  connection_id?: string
  database?: string
  schema?: string
  content?: string
  file_path?: string
  read_only?: boolean
}

export interface SessionState {
  open_tabs: StoredTabState[]
  active_tab_key: string
}
```

---

### 4. localStorage 直接使用

**位置：** `src/stores/app.ts:10, 13, 21, 26`

**问题描述：**
```typescript
const theme = ref<Theme>(localStorage.getItem('theme') as Theme || 'light')

watch(theme, (newTheme) => {
  localStorage.setItem('theme', newTheme)
})
```

**影响：**
- 类型不安全（强制类型转换）
- 无法统一管理存储键
- 难以迁移到其他存储方案

**重构方案：**

创建类型安全的存储工具：

```typescript
// src/utils/storage.ts
type StorageKey = 'theme' | 'language' | 'sidebar_collapsed'

interface StorageSchema {
  theme: 'light' | 'dark'
  language: 'zh-CN' | 'en-US'
  sidebar_collapsed: boolean
}

class TypedStorage {
  get<K extends StorageKey>(key: K): StorageSchema[K] | null {
    const value = localStorage.getItem(key)
    if (value === null) return null

    try {
      return JSON.parse(value) as StorageSchema[K]
    } catch {
      return value as StorageSchema[K]
    }
  }

  set<K extends StorageKey>(key: K, value: StorageSchema[K]): void {
    const serialized = typeof value === 'string' ? value : JSON.stringify(value)
    localStorage.setItem(key, serialized)
  }

  remove(key: StorageKey): void {
    localStorage.removeItem(key)
  }

  clear(): void {
    localStorage.clear()
  }
}

export const storage = new TypedStorage()
```

**使用示例：**

```typescript
// src/stores/app.ts
import { storage } from '@/utils/storage'

export const useAppStore = defineStore('app', () => {
  const theme = ref<Theme>(storage.get('theme') || 'light')

  watch(theme, (newTheme) => {
    storage.set('theme', newTheme)
  })

  // ...
})
```

---

## 🟡 中优先级（代码质量/可维护性）

### 5. Tauri invoke 调用缺少类型

**位置：** 所有使用 `invoke` 的地方

**问题描述：**
```typescript
const result = await invoke<any>('test_connection', { config })
const saved = await invoke<any>('save_connection', { ... })
```

**影响：** 失去类型检查，容易出错

**重构方案：**

创建类型化的 API 层：

```typescript
// src/api/connection.ts
import { invoke } from '@tauri-apps/api/core'
import type {
  ConnectionConfig,
  ConnectionTestResult
} from '@/types/database'

export const connectionApi = {
  /**
   * 测试连接
   */
  async testConnection(config: ConnectionConfig): Promise<ConnectionTestResult> {
    return invoke<ConnectionTestResult>('test_connection', { config })
  },

  /**
   * 保存连接
   */
  async saveConnection(
    connection: any,
    password: string | null
  ): Promise<any> {
    return invoke('save_connection', { connection, password })
  },

  /**
   * 更新连接
   */
  async updateConnection(
    connection: any,
    password: string | null
  ): Promise<any> {
    return invoke('update_connection', { connection, password })
  },

  /**
   * 获取所有连接
   */
  async getConnections(): Promise<ConnectionConfig[]> {
    return invoke<ConnectionConfig[]>('get_connections')
  },

  /**
   * 删除连接
   */
  async deleteConnection(id: string): Promise<boolean> {
    return invoke<boolean>('delete_connection', { id })
  },

  /**
   * 创建连接
   */
  async createConnection(connectionId: string): Promise<void> {
    return invoke('create_connection', { connectionId })
  },

  /**
   * 断开连接
   */
  async disconnectDatabase(connectionId: string): Promise<void> {
    return invoke('disconnect_database', { connectionId })
  },
}

// src/api/query.ts
import { invoke } from '@tauri-apps/api/core'
import type { QueryResult } from '@/types/database'

export const queryApi = {
  /**
   * 执行查询
   */
  async executeQuery(
    connectionId: string,
    sql: string,
    database?: string
  ): Promise<QueryResult[]> {
    return invoke<QueryResult[]>('execute_query', {
      connectionId,
      sql,
      database: database || null
    })
  },

  /**
   * 格式化 SQL
   */
  async beautifySql(
    connectionId: string,
    sql: string
  ): Promise<string> {
    return invoke<string>('beautify_sql', { connectionId, sql })
  },

  /**
   * 获取执行计划
   */
  async explainQuery(
    connectionId: string,
    sql: string,
    database?: string
  ): Promise<QueryResult[]> {
    return invoke<QueryResult[]>('explain_query', {
      connectionId,
      sql,
      database: database || null
    })
  },
}

// src/api/index.ts
export * from './connection'
export * from './query'
export * from './metadata'
```

**使用示例：**

```typescript
// src/stores/connection.ts
import { connectionApi } from '@/api'

export const useConnectionStore = defineStore('connection', () => {
  async function fetchConnections() {
    return withErrorHandler(
      async () => {
        connections.value = await connectionApi.getConnections()
      },
      { messagePrefix: '获取连接列表失败' }
    )
  }

  async function testConnection(config: ConnectionConfig) {
    return withErrorHandler(
      async () => {
        const result = await connectionApi.testConnection(config)
        if (!result.success) {
          throw new Error(result.message || '连接失败')
        }
        return result
      },
      { messagePrefix: '测试连接失败' }
    )
  }
})
```

---

### 6. 组件过大

**位置：** `src/components/editor/SqlEditor.vue`

**问题描述：**
- 单个组件包含编辑器、结果展示、历史记录等多个功能
- 代码行数过多，难以维护

**影响：** 可维护性差，难以测试

**重构方案：**

拆分为多个子组件：

```
src/components/editor/
├── SqlEditor.vue              # 主组件（协调器）
├── MonacoEditor.vue           # Monaco 编辑器封装
├── QueryResultTabs.vue        # 查询结果标签页
├── QueryResultGrid.vue        # 单个结果表格
├── QueryMessages.vue          # 消息面板
├── QueryHistory.vue           # 历史记录抽屉
└── composables/
    ├── useMonacoEditor.ts     # 编辑器逻辑
    ├── useQueryExecution.ts   # 查询执行逻辑
    └── useQueryHistory.ts     # 历史记录逻辑
```

**示例：**

```typescript
// src/components/editor/composables/useQueryExecution.ts
import { ref } from 'vue'
import { queryApi } from '@/api'
import type { QueryResult } from '@/types/database'

export function useQueryExecution() {
  const executing = ref(false)
  const queryResults = ref<QueryResult[]>([])
  const messages = ref<Array<{ type: string; text: string; time: string }>>([])

  async function executeQuery(
    connectionId: string,
    sql: string,
    database?: string
  ) {
    executing.value = true
    queryResults.value = []

    try {
      const results = await queryApi.executeQuery(connectionId, sql, database)
      queryResults.value = results

      messages.value.push({
        type: 'success',
        text: `查询成功，返回 ${results.length} 个结果集`,
        time: new Date().toLocaleTimeString()
      })
    } catch (error) {
      messages.value.push({
        type: 'error',
        text: error instanceof Error ? error.message : String(error),
        time: new Date().toLocaleTimeString()
      })
      throw error
    } finally {
      executing.value = false
    }
  }

  function stopExecution() {
    // 实现停止逻辑
    executing.value = false
  }

  return {
    executing,
    queryResults,
    messages,
    executeQuery,
    stopExecution
  }
}
```

---

### 7. 命名不一致

**位置：** 多个文件

**问题描述：**
```typescript
// 有些使用 camelCase
connectionId
activeConnectionId

// 有些使用 snake_case（来自后端）
connection_id
db_type
```

**影响：** 代码风格不统一

**重构方案：**

创建统一的命名转换工具：

```typescript
// src/utils/caseConverter.ts

/**
 * 将对象的键从 snake_case 转换为 camelCase
 */
export function toCamelCase<T = any>(obj: any): T {
  if (obj === null || typeof obj !== 'object') {
    return obj
  }

  if (Array.isArray(obj)) {
    return obj.map(item => toCamelCase(item)) as any
  }

  const result: any = {}
  for (const [key, value] of Object.entries(obj)) {
    const camelKey = key.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase())
    result[camelKey] = toCamelCase(value)
  }
  return result
}

/**
 * 将对象的键从 camelCase 转换为 snake_case
 */
export function toSnakeCase<T = any>(obj: any): T {
  if (obj === null || typeof obj !== 'object') {
    return obj
  }

  if (Array.isArray(obj)) {
    return obj.map(item => toSnakeCase(item)) as any
  }

  const result: any = {}
  for (const [key, value] of Object.entries(obj)) {
    const snakeKey = key.replace(/[A-Z]/g, letter => `_${letter.toLowerCase()}`)
    result[snakeKey] = toSnakeCase(value)
  }
  return result
}
```

---

### 8. 缺少 Composables 复用

**位置：** 多个组件

**问题描述：**
- 连接状态管理逻辑在多个组件中重复
- 数据库树操作逻辑分散

**重构方案：**

创建可复用的 Composables：

```typescript
// src/composables/useConnection.ts
import { computed } from 'vue'
import { useConnectionStore } from '@/stores/connection'

export function useConnection() {
  const connectionStore = useConnectionStore()

  const activeConnection = computed(() =>
    connectionStore.getActiveConnection()
  )

  const isConnected = computed(() =>
    activeConnection.value
      ? connectionStore.getConnectionStatus(activeConnection.value.id) === 'connected'
      : false
  )

  async function connect(id: string) {
    await connectionStore.connectToDatabase(id)
    connectionStore.setActiveConnection(id)
  }

  async function disconnect(id: string) {
    await connectionStore.disconnectFromDatabase(id)
    if (connectionStore.activeConnectionId === id) {
      connectionStore.setActiveConnection(null)
    }
  }

  return {
    activeConnection,
    isConnected,
    connect,
    disconnect,
    connectionStore
  }
}

// src/composables/useDatabaseTree.ts
import { ref } from 'vue'
import type { DatabaseTreeNode } from '@/types/database'

export function useDatabaseTree() {
  const treeData = ref<DatabaseTreeNode[]>([])
  const expandedKeys = ref<string[]>([])
  const selectedKeys = ref<string[]>([])

  async function loadDatabases(connectionId: string) {
    // 实现加载逻辑
  }

  async function loadTables(connectionId: string, database: string) {
    // 实现加载逻辑
  }

  function expandNode(key: string) {
    if (!expandedKeys.value.includes(key)) {
      expandedKeys.value.push(key)
    }
  }

  function collapseNode(key: string) {
    expandedKeys.value = expandedKeys.value.filter(k => k !== key)
  }

  return {
    treeData,
    expandedKeys,
    selectedKeys,
    loadDatabases,
    loadTables,
    expandNode,
    collapseNode
  }
}
```

---

## 🟢 低优先级（优化/清理）

### 9. @ts-ignore 使用

**位置：** `src/stores/app.ts:27`

**问题描述：**
```typescript
// @ts-ignore
i18n.global.locale.value = newLang
```

**重构方案：**

正确定义 i18n 类型：

```typescript
// src/i18n.ts
import { createI18n } from 'vue-i18n'
import type { Language } from '@/stores/app'

const i18n = createI18n<false>({
  legacy: false,
  locale: 'zh-CN',
  fallbackLocale: 'en-US',
  messages: {
    // ...
  }
})

export function setLocale(locale: Language) {
  i18n.global.locale.value = locale
}

export default i18n

// src/stores/app.ts
import { setLocale } from '@/i18n'

watch(language, (newLang) => {
  localStorage.setItem('language', newLang)
  setLocale(newLang)
}, { immediate: true })
```

---

### 10. 魔法字符串

**位置：** 多个文件

**问题描述：**
```typescript
if (t.type === 'query' || t.type === 'redis')
```

**重构方案：**

使用常量或枚举：

```typescript
// src/types/workspace.ts
export enum TabType {
  Data = 'data',
  Design = 'design',
  Query = 'query',
  Redis = 'redis'
}

export interface TabState {
  key: string
  title: string
  type: TabType
  // ...
}

// 使用
if (t.type === TabType.Query || t.type === TabType.Redis)
```

---

### 11. 内联样式

**位置：** `src/components/editor/SqlEditor.vue:21`

**问题描述：**
```vue
<a-button danger size="small" @click="stopExecution" style="margin-top: 16px">
```

**重构方案：**

使用 CSS 类：

```vue
<a-button danger size="small" @click="stopExecution" class="stop-button">

<style scoped>
.stop-button {
  margin-top: 16px;
}
</style>
```

---

### 12. 缺少加载状态

**位置：** 多个组件

**问题描述：**
- 数据加载时没有 loading 状态
- 用户体验差

**重构方案：**

创建统一的加载状态管理：

```typescript
// src/composables/useLoading.ts
import { ref } from 'vue'

export function useLoading(initialState = false) {
  const loading = ref(initialState)

  async function withLoading<T>(fn: () => Promise<T>): Promise<T> {
    loading.value = true
    try {
      return await fn()
    } finally {
      loading.value = false
    }
  }

  return {
    loading,
    withLoading
  }
}

// 使用
const { loading, withLoading } = useLoading()

async function fetchData() {
  await withLoading(async () => {
    const data = await api.getData()
    // 处理数据
  })
}
```

---

## 📊 重构优先级总结

| 优先级 | 项目 | 预计工作量 | 风险 |
|--------|------|-----------|------|
| 🔴 高 | #1 对象转换重复 | 1 小时 | 低 |
| 🔴 高 | #2 错误处理 | 3-4 小时 | 低 |
| 🔴 高 | #3 类型定义 | 1-2 小时 | 低 |
| 🔴 高 | #4 localStorage | 2 小时 | 低 |
| 🟡 中 | #5 API 类型化 | 4-6 小时 | 中 |
| 🟡 中 | #6 组件拆分 | 6-8 小时 | 中 |
| 🟡 中 | #7 命名统一 | 2-3 小时 | 低 |
| 🟡 中 | #8 Composables | 4-5 小时 | 低 |
| 🟢 低 | #9-12 清理 | 2-4 小时 | 低 |

**总计：** 约 25-37 小时

---

## 🎯 建议的重构顺序

### 第一阶段（基础设施）
1. 创建错误处理工具（#2）
2. 创建类型安全的存储工具（#4）
3. 创建 API 类型化层（#5）

### 第二阶段（代码质量）
4. 消除对象转换重复（#1）
5. 完善类型定义（#3）
6. 统一命名规范（#7）

### 第三阶段（架构优化）
7. 拆分大组件（#6）
8. 创建可复用 Composables（#8）

### 第四阶段（清理优化）
9. 清理技术债务（#9-12）

---

## 📝 注意事项

1. **渐进式重构：** 不要一次性重构所有代码
2. **测试覆盖：** 重构前后需要充分测试
3. **向后兼容：** 确保不破坏现有功能
4. **团队协作：** 重构需要团队成员共识

---

## 🔗 相关资源

- [Vue 3 Composition API](https://vuejs.org/guide/extras/composition-api-faq.html)
- [TypeScript Best Practices](https://www.typescriptlang.org/docs/handbook/declaration-files/do-s-and-don-ts.html)
- [Pinia Best Practices](https://pinia.vuejs.org/core-concepts/)
- [Ant Design Vue](https://antdv.com/components/overview)
