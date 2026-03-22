# DataSmith 重构建议

> 生成日期：2026-03-21
> 基于代码库版本：0.1.0

本文档列出了代码库中需要重构的部分，按优先级排序。

---

## 🔴 高优先级（安全/功能缺陷）

### 1. SQL 注入风险

**位置：** `src-tauri/src/commands/query.rs:140`

**问题描述：**
```rust
let sql = format!("DELETE FROM {} WHERE {}", table, where_clause);
```
直接字符串拼接构造 SQL，存在严重的 SQL 注入风险。

**影响：** 安全漏洞，可能导致数据泄露或破坏

**重构方案：**

**方案 A：在 DatabaseOperations trait 中添加专用方法**
```rust
// src-tauri/src/database/traits.rs
#[async_trait]
pub trait DatabaseOperations: Send + Sync {
    // ... 现有方法

    /// 删除表数据（使用参数化查询）
    async fn delete_data(
        &self,
        table: &str,
        schema: Option<&str>,
        where_conditions: &HashMap<String, serde_json::Value>
    ) -> DbResult<u64>;
}
```

**方案 B：使用标识符转义**
```rust
// src-tauri/src/utils/sql_escape.rs
pub fn escape_identifier(identifier: &str, db_type: &DatabaseType) -> String {
    match db_type {
        DatabaseType::MySQL => format!("`{}`", identifier.replace("`", "``")),
        DatabaseType::PostgreSQL => format!("\"{}\"", identifier.replace("\"", "\"\"")),
        DatabaseType::SQLite => format!("\"{}\"", identifier.replace("\"", "\"\"")),
        _ => identifier.to_string(),
    }
}

// 使用
let table_escaped = escape_identifier(&table, &db_type);
let sql = format!("DELETE FROM {} WHERE {}", table_escaped, where_clause);
```

**推荐：** 方案 A（更安全）

---

### 2. 分页查询未实现

**位置：** `src-tauri/src/commands/query.rs:70-84`

**问题描述：**
```rust
pub async fn execute_query_paged(
    connection_id: String,
    sql: String,
    database: Option<String>,
    _page: u32,        // ❌ 参数被忽略
    _page_size: u32,   // ❌ 参数被忽略
    state: State<'_, AppState>,
) -> Result<Vec<QueryResult>, String> {
    let manager = &state.connection_manager;

    // ❌ 直接返回全部结果，未实现分页
    manager
        .execute_query(&connection_id, &sql, database.as_deref())
        .await
        .map_err(|e| e.to_string())
}
```

**影响：** 大数据集查询可能导致内存溢出

**重构方案：**

**方案 A：实现真正的分页**
```rust
pub async fn execute_query_paged(
    connection_id: String,
    sql: String,
    database: Option<String>,
    page: u32,
    page_size: u32,
    state: State<'_, AppState>,
) -> Result<Vec<QueryResult>, String> {
    let manager = &state.connection_manager;
    let db_type = manager.get_database_type(&connection_id).await
        .map_err(|e| e.to_string())?;

    // 构造分页 SQL
    let offset = (page - 1) * page_size;
    let paged_sql = match db_type {
        DatabaseType::MySQL | DatabaseType::PostgreSQL | DatabaseType::SQLite => {
            format!("{} LIMIT {} OFFSET {}", sql.trim_end_matches(';'), page_size, offset)
        },
        _ => sql, // 其他数据库类型不支持分页
    };

    manager
        .execute_query(&connection_id, &paged_sql, database.as_deref())
        .await
        .map_err(|e| e.to_string())
}
```

**方案 B：移除此函数，使用前端分页**
- 删除 `execute_query_paged` 函数
- 从 `main.rs` 的 `invoke_handler` 中移除注册
- 前端使用虚拟滚动或客户端分页

**推荐：** 方案 A（更好的用户体验）

---

### 3. 大量占位命令

**位置：** `src-tauri/src/commands/metadata.rs:77-96`

**问题描述：**
```rust
// 14 个占位函数，行为不一致
#[tauri::command]
pub async fn view_table_data(...) -> Result<Vec<QueryResult>, String> {
    Err("Not implemented".into())  // ❌ 返回错误
}

#[tauri::command]
pub async fn truncate_table(...) -> Result<(), String> {
    Ok(())  // ❌ 返回成功但什么都不做
}

#[tauri::command]
pub async fn get_procedures(...) -> Result<Vec<Value>, String> {
    Ok(vec![])  // ❌ 返回空数组
}
```

**影响：**
- 误导前端开发者
- 增加二进制大小
- 维护负担

**重构方案：**

**方案 A：实现这些功能**
```rust
#[tauri::command]
pub async fn truncate_table(
    connection_id: String,
    table: String,
    database: Option<String>,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let manager = &state.connection_manager;
    let db_type = manager.get_database_type(&connection_id).await
        .map_err(|e| e.to_string())?;

    let sql = match db_type {
        DatabaseType::MySQL => format!("TRUNCATE TABLE {}", table),
        DatabaseType::PostgreSQL => {
            let full_name = if let Some(s) = schema {
                format!("{}.{}", s, table)
            } else {
                table
            };
            format!("TRUNCATE TABLE {} RESTART IDENTITY CASCADE", full_name)
        },
        DatabaseType::SQLite => format!("DELETE FROM {}", table),
        _ => return Err("不支持的数据库类型".into()),
    };

    manager.execute_query(&connection_id, &sql, database.as_deref())
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
```

**方案 B：移除未使用的命令**
1. 从 `metadata.rs` 删除占位函数
2. 从 `main.rs:35-96` 的 `invoke_handler!` 中移除对应注册
3. 前端移除相关调用

**方案 C：使用 feature flag 标记**
```rust
#[cfg(feature = "unstable")]
#[tauri::command]
pub async fn truncate_table(...) -> Result<(), String> {
    Ok(())
}
```

**推荐：** 方案 B（清理代码）或方案 A（完善功能）

---

## 🟡 中优先级（代码质量/性能）

### 4. 错误处理重复模式

**位置：** 所有 Tauri 命令文件

**问题描述：**
```rust
// 在每个命令中重复
.map_err(|e| e.to_string())?
```

**影响：** 违反 DRY 原则，代码冗余

**重构方案：**

创建辅助 trait：

```rust
// src-tauri/src/utils/command_result.rs
pub trait ToCommandResult<T> {
    fn to_cmd_result(self) -> Result<T, String>;
}

impl<T, E: std::fmt::Display> ToCommandResult<T> for Result<T, E> {
    fn to_cmd_result(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}

// src-tauri/src/utils/mod.rs
pub mod command_result;
pub use command_result::ToCommandResult;
```

**使用示例：**
```rust
// 之前
state.connection_manager
    .get_databases(&connection_id)
    .await
    .map_err(|e| e.to_string())

// 之后
use crate::utils::ToCommandResult;

state.connection_manager
    .get_databases(&connection_id)
    .await
    .to_cmd_result()
```

---

### 5. DatabaseType 缺少 trait 实现

**位置：** `src-tauri/src/commands/connection.rs:21-28`

**问题描述：**
```rust
let db_type = match stored.db_type.as_str() {
    "mysql" => DatabaseType::MySQL,
    "postgresql" => DatabaseType::PostgreSQL,
    "sqlite" => DatabaseType::SQLite,
    "mongodb" => DatabaseType::MongoDB,
    "redis" => DatabaseType::Redis,
    _ => DatabaseType::MySQL,  // ❌ 不安全的默认值
};
```

**影响：** 类型不安全，可能导致运行时错误

**重构方案：**

```rust
// src-tauri/src/database/traits.rs
use std::str::FromStr;

impl FromStr for DatabaseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mysql" => Ok(DatabaseType::MySQL),
            "postgresql" => Ok(DatabaseType::PostgreSQL),
            "sqlite" => Ok(DatabaseType::SQLite),
            "mongodb" => Ok(DatabaseType::MongoDB),
            "redis" => Ok(DatabaseType::Redis),
            "elasticsearch" => Ok(DatabaseType::Elasticsearch),
            _ => Err(format!("不支持的数据库类型: {}", s))
        }
    }
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DatabaseType::MySQL => "mysql",
            DatabaseType::PostgreSQL => "postgresql",
            DatabaseType::SQLite => "sqlite",
            DatabaseType::MongoDB => "mongodb",
            DatabaseType::Redis => "redis",
            DatabaseType::Elasticsearch => "elasticsearch",
        };
        write!(f, "{}", s)
    }
}
```

**使用示例：**
```rust
// src-tauri/src/commands/connection.rs
fn stored_to_config_with_password(stored: &StoredConnection, password: &str) -> Result<ConnectionConfig, String> {
    let db_type = stored.db_type.parse::<DatabaseType>()?;

    Ok(ConnectionConfig {
        id: stored.id.clone(),
        name: stored.name.clone(),
        db_type,
        // ...
    })
}
```

---

### 6. Store 操作重复

**位置：** `src-tauri/src/commands/connection.rs` 多处

**问题描述：**
```rust
// 在 5 个函数中重复
let store = app.store("connections.json")
    .map_err(|e| format!("Failed to get store: {}", e))?;
```

**影响：** 代码重复，错误消息不一致

**重构方案：**

```rust
// src-tauri/src/commands/connection.rs
use tauri_plugin_store::Store;

fn get_connection_store(app: &AppHandle) -> Result<Store, String> {
    app.store("connections.json")
        .map_err(|e| format!("无法访问连接存储: {}", e))
}

// 使用
#[tauri::command]
pub async fn save_connection(
    app: AppHandle,
    mut connection: StoredConnection,
    password: Option<String>,
) -> Result<StoredConnection, String> {
    let store = get_connection_store(&app)?;

    if let Some(pwd) = password {
        if !pwd.is_empty() {
            connection.encrypted_password = Some(crypto::encrypt_password(&pwd)?);
        }
    }

    store.set(connection.id.clone(), json!(connection));
    store.save().map_err(|e| e.to_string())?;
    Ok(connection)
}
```

---

### 7. PostgreSQL 未使用连接池

**位置：** `src-tauri/src/database/postgresql.rs`

**问题描述：**
- `Cargo.toml` 中声明了 `deadpool-postgres` 依赖
- 实际代码使用单个 `Client`，未使用连接池

**影响：** 并发性能差，无法复用连接

**重构方案：**

```rust
// src-tauri/src/database/postgresql.rs
use deadpool_postgres::{Config as PoolConfig, Pool, Runtime, ManagerConfig, RecyclingMethod};

struct PgState {
    pool: Option<Pool>,
    config: Option<ConnectionConfig>,
}

impl PostgreSqlDatabase {
    async fn create_pool(config: &ConnectionConfig) -> DbResult<Pool> {
        let mut pg_config = tokio_postgres::Config::new();
        pg_config
            .host(&config.host)
            .port(config.port)
            .user(&config.username)
            .password(&config.password)
            .dbname(config.database.as_deref().unwrap_or("postgres"));

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };

        let pool_config = PoolConfig {
            manager: Some(mgr_config),
            ..Default::default()
        };

        let pool = if config.ssl {
            let connector = TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
            let connector = MakeTlsConnector::new(connector);
            pool_config.create_pool(Some(Runtime::Tokio1), connector)
                .map_err(|e| DbError::ConnectionFailed(e.to_string()))?
        } else {
            pool_config.create_pool(Some(Runtime::Tokio1), NoTls)
                .map_err(|e| DbError::ConnectionFailed(e.to_string()))?
        };

        Ok(pool)
    }
}

#[async_trait]
impl DatabaseOperations for PostgreSqlDatabase {
    async fn connect(&self, config: ConnectionConfig) -> DbResult<()> {
        let pool = Self::create_pool(&config).await?;

        // 测试连接
        let client = pool.get().await
            .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
        client.query("SELECT 1", &[]).await
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;

        let mut state = self.state.lock().await;
        state.pool = Some(pool);
        state.config = Some(config);
        Ok(())
    }

    async fn execute_query(&self, sql: &str, _database: Option<&str>) -> DbResult<Vec<QueryResult>> {
        let start = Instant::now();
        let state = self.state.lock().await;
        let pool = state.pool.as_ref()
            .ok_or(DbError::ConnectionFailed("未连接数据库".into()))?;

        let client = pool.get().await
            .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;

        // ... 执行查询逻辑
    }
}
```

---

### 8. 连接创建逻辑重复

**位置：** `mysql.rs` 和 `postgresql.rs` 的 `switch_database` 方法

**问题描述：**
两个数据库驱动的 `switch_database` 方法有相同的模式：
1. 检查是否需要切换
2. 克隆配置
3. 创建新连接
4. 替换旧连接

**影响：** 代码重复，维护困难

**重构方案：**

**方案 A：在 trait 中提供默认实现**
```rust
// src-tauri/src/database/traits.rs
#[async_trait]
pub trait DatabaseOperations: Send + Sync {
    // ... 现有方法

    /// 重新连接到新数据库（默认实现）
    async fn reconnect_with_database(&self, database: &str) -> DbResult<()> {
        // 子类需要实现 get_config 和 reconnect
        Err(DbError::Other("需要子类实现".into()))
    }

    /// 切换数据库（提供默认实现）
    async fn switch_database(&self, database: &str) -> DbResult<()> {
        self.reconnect_with_database(database).await
    }
}
```

**方案 B：创建辅助宏**
```rust
// src-tauri/src/database/macros.rs
#[macro_export]
macro_rules! impl_switch_database {
    ($state_type:ty, $create_fn:ident) => {
        async fn switch_database(&self, database: &str) -> DbResult<()> {
            let mut state = self.state.lock().await;
            let mut config = state.config.clone()
                .ok_or(DbError::Other("未找到初始配置".into()))?;

            if config.database.as_deref() == Some(database) {
                return Ok(());
            }

            info!(new_db = %database, "正在切换数据库...");
            config.database = Some(database.to_string());

            let new_connection = Self::$create_fn(&config).await?;
            state.pool = Some(new_connection);
            state.config = Some(config);
            Ok(())
        }
    };
}
```

**推荐：** 方案 A（更清晰）

---

## 🟢 低优先级（清理/优化）

### 9. 未使用的函数

**位置：** `src-tauri/src/commands/query.rs:120-128`

**问题描述：**
```rust
#[tauri::command]
pub async fn insert_table_data(
    _connection_id: String,
    _database: String,
    _table: String,
    _data: std::collections::HashMap<String, serde_json::Value>,
    _state: State<'_, AppState>,
) -> Result<(), String> {
    Ok(())  // 空实现
}
```

**重构方案：** 移除或实现

---

### 10. 日志导入不一致

**位置：** 多个文件

**问题描述：**
```rust
// 有些文件
use tracing::{info, instrument};
info!("...");

// 有些地方
tracing::info!("...");
```

**重构方案：** 统一使用导入的简短形式

---

### 11. 配置克隆过多

**位置：** `manager.rs` 和各数据库实现

**问题描述：**
```rust
let config = state.config.clone().ok_or(...)?;  // 频繁克隆
```

**重构方案：** 使用 `Arc<ConnectionConfig>` 共享配置

```rust
struct MySqlState {
    pool: Option<Pool>,
    config: Option<Arc<ConnectionConfig>>,
}
```

---

### 12. 后台任务未追踪

**位置：** `mysql.rs:97` 和 `postgresql.rs:43,47`

**问题描述：**
```rust
tokio::spawn(async move {
    old_pool.disconnect().await.ok();
});
```

无法追踪或取消这些任务。

**重构方案：**

```rust
use tokio::task::JoinSet;

struct MySqlState {
    pool: Option<Pool>,
    config: Option<ConnectionConfig>,
    background_tasks: JoinSet<()>,
}

// 使用
if let Some(old_pool) = state.pool.replace(pool) {
    state.background_tasks.spawn(async move {
        if let Err(e) = old_pool.disconnect().await {
            error!("断开旧连接失败: {}", e);
        }
    });
}
```

---

## 📊 重构优先级总结

| 优先级 | 项目 | 预计工作量 | 风险 |
|--------|------|-----------|------|
| 🔴 高 | #1 SQL 注入 | 2-4 小时 | 低 |
| 🔴 高 | #2 分页查询 | 1-2 小时 | 低 |
| 🔴 高 | #3 占位命令 | 4-8 小时 | 中 |
| 🟡 中 | #4 错误处理 | 2-3 小时 | 低 |
| 🟡 中 | #5 DatabaseType | 1-2 小时 | 低 |
| 🟡 中 | #6 Store 操作 | 1 小时 | 低 |
| 🟡 中 | #7 PG 连接池 | 3-4 小时 | 中 |
| 🟡 中 | #8 连接逻辑 | 2-3 小时 | 低 |
| 🟢 低 | #9-12 清理 | 2-4 小时 | 低 |

**总计：** 约 18-33 小时

---

## 🎯 建议的重构顺序

### 第一阶段（安全修复）
1. 修复 SQL 注入风险（#1）
2. 实现或移除占位命令（#3）

### 第二阶段（功能完善）
3. 实现分页查询（#2）
4. 实现 PostgreSQL 连接池（#7）

### 第三阶段（代码质量）
5. 错误处理抽象（#4）
6. DatabaseType traits（#5）
7. Store 操作重构（#6）

### 第四阶段（优化清理）
8. 连接逻辑抽象（#8）
9. 代码清理（#9-12）

---

## 📝 注意事项

1. **测试覆盖：** 每次重构后需要进行充分测试
2. **向后兼容：** 前端 API 变更需要同步更新
3. **性能监控：** 连接池等性能优化需要基准测试
4. **文档更新：** 重构后更新 CLAUDE.md

---

## 🔗 相关资源

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Tauri Security Best Practices](https://tauri.app/v1/guides/security/)
- [tokio-postgres Documentation](https://docs.rs/tokio-postgres/)
- [deadpool Documentation](https://docs.rs/deadpool/)
