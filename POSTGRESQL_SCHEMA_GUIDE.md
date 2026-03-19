# PostgreSQL Schema 功能使用指南

## 概述

已为 PostgreSQL 添加完整的 schema 功能支持，树形结构为：

```
数据库
├── Schemas
│   ├── public (Schema)
│   │   ├── 表
│   │   ├── 视图
│   │   ├── 函数
│   │   ├── 聚合函数
│   │   └── 索引
│   ├── myschema (Schema)
│   │   ├── 表
│   │   ├── 视图
│   │   ├── 函数
│   │   ├── 聚合函数
│   │   └── 索引
│   └── ...
└── 扩展 (Extensions)
    ├── pg_trgm
    ├── uuid-ossp
    └── ...
```

**特点：**
- 数据库节点展开后显示 Schemas 容器和扩展容器
- 每个 schema 下包含：表、视图、函数、聚合函数、索引
- 扩展是数据库级别的，与 Schemas 同级
- 自动过滤系统 schema（pg_catalog、information_schema 等）
- 支持懒加载，提升性能

## 后端实现

### 数据结构

#### SchemaInfo
```rust
pub struct SchemaInfo {
    pub name: String,           // Schema 名称
    pub owner: Option<String>,  // 所有者
    pub comment: Option<String>, // 注释
}
```

#### FunctionInfo
```rust
pub struct FunctionInfo {
    pub name: String,              // 函数名称
    pub schema: Option<String>,    // 所属 schema
    pub return_type: Option<String>, // 返回类型
    pub arguments: Option<String>,  // 参数列表
    pub language: Option<String>,   // 语言（如 plpgsql, sql）
    pub function_type: String,      // "function" 或 "aggregate"
    pub comment: Option<String>,    // 注释
}
```

### Tauri 命令

#### 1. 获取 Schema 列表
```rust
#[tauri::command]
pub async fn get_schemas(
    connection_id: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SchemaInfo>, String>
```

**前端调用示例：**
```typescript
import { invoke } from '@tauri-apps/api/core'

const schemas = await invoke<SchemaInfo[]>('get_schemas', {
  connectionId: 'your-connection-id',
  database: 'your-database-name'
})
```

#### 2. 获取指定 Schema 下的表
```rust
#[tauri::command]
pub async fn get_schema_tables(
    connection_id: String,
    database: String,
    schema: String,
    state: State<'_, AppState>,
) -> Result<Vec<TableInfo>, String>
```

**前端调用示例：**
```typescript
const tables = await invoke<TableInfo[]>('get_schema_tables', {
  connectionId: 'your-connection-id',
  database: 'your-database-name',
  schema: 'public'
})
```

#### 3. 获取指定 Schema 下的视图
```rust
#[tauri::command]
pub async fn get_schema_views(
    connection_id: String,
    database: String,
    schema: String,
    state: State<'_, AppState>,
) -> Result<Vec<TableInfo>, String>
```

**前端调用示例：**
```typescript
const views = await invoke<TableInfo[]>('get_schema_views', {
  connectionId: 'your-connection-id',
  database: 'your-database-name',
  schema: 'public'
})
```

#### 4. 获取指定 Schema 下的函数
```rust
#[tauri::command]
pub async fn get_schema_functions(
    connection_id: String,
    database: String,
    schema: String,
    state: State<'_, AppState>,
) -> Result<Vec<FunctionInfo>, String>
```

**前端调用示例：**
```typescript
const functions = await invoke<FunctionInfo[]>('get_schema_functions', {
  connectionId: 'your-connection-id',
  database: 'your-database-name',
  schema: 'public'
})
```

#### 5. 获取指定 Schema 下的聚合函数
```rust
#[tauri::command]
pub async fn get_schema_aggregate_functions(
    connection_id: String,
    database: String,
    schema: String,
    state: State<'_, AppState>,
) -> Result<Vec<FunctionInfo>, String>
```

**前端调用示例：**
```typescript
const aggregateFunctions = await invoke<FunctionInfo[]>('get_schema_aggregate_functions', {
  connectionId: 'your-connection-id',
  database: 'your-database-name',
  schema: 'public'
})
```

#### 6. 获取指定 Schema 下的索引
```rust
#[tauri::command]
pub async fn get_schema_indexes(
    connection_id: String,
    database: String,
    schema: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String>
```

**前端调用示例：**
```typescript
const indexes = await invoke<any[]>('get_schema_indexes', {
  connectionId: 'your-connection-id',
  database: 'your-database-name',
  schema: 'public'
})
```

#### 7. 获取数据库的扩展列表
```rust
#[tauri::command]
pub async fn get_database_extensions(
    connection_id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String>
```

**前端调用示例：**
```typescript
const extensions = await invoke<any[]>('get_database_extensions', {
  connectionId: 'your-connection-id',
  database: 'your-database-name'
})
```

## 前端 TypeScript 类型定义

建议在前端添加以下类型定义：

```typescript
// src/types/database.ts

export interface SchemaInfo {
  name: string
  owner?: string
  comment?: string
}

export interface FunctionInfo {
  name: string
  schema?: string
  return_type?: string
  arguments?: string
  language?: string
  function_type: 'function' | 'aggregate'
  comment?: string
}

export interface TableInfo {
  name: string
  schema?: string
  table_type: string
  engine?: string
  rows?: number
  size_mb?: number
  comment?: string
}
```

## 使用示例

### 完整的数据库树形结构加载

```typescript
async function loadDatabaseTree(connectionId: string, database: string) {
  // 1. 获取所有 schema
  const schemas = await invoke<SchemaInfo[]>('get_schemas', {
    connectionId,
    database
  })

  // 2. 获取数据库级别的扩展
  const extensions = await invoke<any[]>('get_database_extensions', {
    connectionId,
    database
  })

  // 3. 为每个 schema 加载子项
  const schemaTree = await Promise.all(
    schemas.map(async (schema) => {
      const [tables, views, functions, aggregateFunctions, indexes] = await Promise.all([
        invoke<TableInfo[]>('get_schema_tables', { connectionId, database, schema: schema.name }),
        invoke<TableInfo[]>('get_schema_views', { connectionId, database, schema: schema.name }),
        invoke<FunctionInfo[]>('get_schema_functions', { connectionId, database, schema: schema.name }),
        invoke<FunctionInfo[]>('get_schema_aggregate_functions', { connectionId, database, schema: schema.name }),
        invoke<any[]>('get_schema_indexes', { connectionId, database, schema: schema.name })
      ])

      return {
        schema,
        tables,
        views,
        functions,
        aggregateFunctions,
        indexes
      }
    })
  )

  return {
    schemas: schemaTree,
    extensions
  }
}
```

## SQL 查询说明

### Schema 列表查询
```sql
SELECT
    n.nspname as schema_name,
    pg_catalog.pg_get_userbyid(n.nspowner) as owner,
    obj_description(n.oid, 'pg_namespace') as comment
FROM pg_catalog.pg_namespace n
WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
  AND n.nspname NOT LIKE 'pg_toast%'
  AND n.nspname NOT LIKE 'pg_temp%'
ORDER BY n.nspname
```

### 函数列表查询
```sql
SELECT
    p.proname as function_name,
    n.nspname as schema_name,
    pg_catalog.pg_get_function_result(p.oid) as return_type,
    pg_catalog.pg_get_function_arguments(p.oid) as arguments,
    l.lanname as language,
    obj_description(p.oid, 'pg_proc') as comment
FROM pg_catalog.pg_proc p
JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
JOIN pg_catalog.pg_language l ON l.oid = p.prolang
WHERE p.prokind = 'f'  -- 'f' 表示普通函数，'a' 表示聚合函数
  AND n.nspname = 'your_schema'
ORDER BY n.nspname, p.proname
```

### 索引列表查询
```sql
SELECT
    i.relname as index_name,
    t.relname as table_name,
    a.attname as column_name,
    ix.indisunique as is_unique,
    ix.indisprimary as is_primary,
    am.amname as index_type
FROM pg_class t
JOIN pg_index ix ON t.oid = ix.indrelid
JOIN pg_class i ON i.oid = ix.indexrelid
JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
JOIN pg_am am ON i.relam = am.oid
JOIN pg_namespace n ON n.oid = t.relnamespace
WHERE n.nspname = 'your_schema'
ORDER BY i.relname, a.attnum
```

## 注意事项

1. **权限要求**：查询 schema 信息需要相应的数据库权限
2. **性能考虑**：对于包含大量对象的 schema，建议按需加载而非一次性加载所有内容
3. **系统 Schema**：默认过滤了 `pg_catalog`、`information_schema` 等系统 schema
4. **函数类型**：PostgreSQL 11+ 使用 `prokind` 字段区分函数类型（'f' = function, 'a' = aggregate）

## 扩展建议

如需添加更多功能，可以考虑：
- 添加 Trigger（触发器）支持
- 添加 Sequence（序列）支持
- 添加 Type（自定义类型）支持
- 添加 Domain（域）支持
- 添加权限信息查询
