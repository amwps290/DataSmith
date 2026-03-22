# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

DataSmith 是基于 Tauri 2.x 的跨平台数据库管理工具，支持 MySQL、PostgreSQL、SQLite、MongoDB、Redis。

**技术栈：**
- 前端：Vue 3 + TypeScript + Ant Design Vue + Monaco Editor + Pinia
- 后端：Rust + Tauri 2.x + Tokio + 原生数据库驱动
- 构建：Vite 5.x + Cargo

## 开发命令

```bash
# 安装依赖
npm install

# Tauri 开发模式（完整应用，自动打开 DevTools）
npm run tauri:dev

# 前端开发（仅 Web 界面，无后端功能）
npm run dev

# 类型检查 + 构建
npm run build

# 生产构建
npm run tauri:build

# Rust 后端
cd src-tauri
cargo build              # 开发构建
cargo build --release    # 发布构建
cargo test               # 运行测试
cargo check --features mysql  # 检查特定 feature
```

## 核心架构

### 会话管理机制（关键）

**Composite ID 格式：** `config_id:session_id`

- 每个连接配置（config）可以有多个会话（session）
- 默认会话 ID 为 `metadata`（用于元数据查询）
- 查询会话使用自定义 ID（如 `query-1`, `query-2`）
- 示例：`uuid-123:metadata` 或 `uuid-123:query-1`

**ConnectionManager 设计：**
```rust
// 三个核心 HashMap，使用 Arc<RwLock> 实现细粒度锁
connections: Arc<RwLock<HashMap<String, Arc<dyn DatabaseOperations>>>>
connection_types: Arc<RwLock<HashMap<String, DatabaseType>>>
configs: Arc<RwLock<HashMap<String, ConnectionConfig>>>
```

**关键方法：**
- `get_db_ref()` - 获取驱动实例后立即释放锁，避免长时间持锁
- `ensure_session()` - 双重检查锁定模式，自动创建不存在的会话
- `ensure_db_context()` - 确保驱动连接到正确的数据库

### Trait 抽象层

所有数据库实现 `DatabaseOperations` trait（src-tauri/src/database/traits.rs）：

**核心方法：**
- `connect()` / `disconnect()` - 连接管理
- `execute_query()` - 执行 SQL，返回 `Vec<QueryResult>` 支持多结果集
- `get_databases()` / `get_tables()` / `get_table_structure()` - 元数据查询
- `switch_database()` - 切换数据库（PostgreSQL 需重新连接）
- `get_table_ddl()` - 生成 CREATE 语句
- `explain_query()` - 执行计划分析

**数据库实现：**
- MySQL: `mysql_async` + 连接池
- PostgreSQL: `tokio-postgres` + `deadpool-postgres`
- SQLite: `rusqlite` (bundled)
- MongoDB: `mongodb` crate (optional feature)
- Redis: `redis` crate (optional feature)

### Feature Flags

在 `Cargo.toml` 中通过 features 控制数据库支持：

```toml
[features]
default = ["mysql", "postgresql", "sqlite", "mongodb-support", "redis-support"]
mysql = []
postgresql = ["deadpool-postgres"]
sqlite = []
mongodb-support = ["mongodb"]
redis-support = ["redis"]
```

编译时使用 `#[cfg(feature = "mysql")]` 条件编译。

### 前端架构

**状态管理（Pinia stores）：**
- `connection.ts` - 连接配置和活动连接
- `workspace.ts` - 工作区和查询标签页
- `app.ts` - 全局应用状态

**自动导入配置：**
- `unplugin-auto-import` - 自动导入 Vue/Pinia API
- `unplugin-vue-components` - 自动注册 Ant Design Vue 组件
- 生成的类型文件：`src/auto-imports.d.ts`, `src/components.d.ts`

**Monaco Editor 集成：**
- SQL 语法高亮和自动补全
- 使用 `services/sqlAutocomplete.ts` 提供智能补全
- Worker 配置在 `vite.config.ts` 中

## 关键开发注意事项

### PostgreSQL 特殊处理

PostgreSQL 不支持 `USE database` 语句，切换数据库需要：
1. 断开当前连接
2. 使用新的数据库名重新连接
3. `switch_database()` 方法内部处理此逻辑

### 密码加密存储

- 使用 `keyring` crate 存储主密钥到系统密钥链
- 使用 AES-GCM 加密连接密码
- 初始化：`utils::crypto::initialize_master_key()`

### 日志系统

使用 `tracing` 框架：
- 日志文件位置：应用数据目录 + `/logs/`
- Debug 模式：控制台输出
- Release 模式：文件输出（按日期滚动）
- 使用 `#[instrument]` 宏追踪函数调用

### 无边框窗口

Tauri 配置使用 `decorations: false`，需要：
- 前端实现自定义标题栏
- 处理窗口拖拽、最小化、最大化、关闭
- 使用 Tauri API：`appWindow.startDragging()`

### 构建优化

**Release 配置（Cargo.toml）：**
```toml
[profile.release]
opt-level = "z"      # 优化二进制大小
lto = true           # 链接时优化
strip = true         # 移除调试符号
codegen-units = 1    # 单个代码生成单元
```

**Vite 代码分割：**
- `vue-vendor` - Vue 核心库
- `ant-design` - UI 组件库
- `monaco-editor` - 代码编辑器

## 添加新数据库支持

1. 在 `src-tauri/src/database/` 创建新文件（如 `newdb.rs`）
2. 实现 `DatabaseOperations` trait 的所有方法
3. 在 `manager.rs` 的 `create_instance()` 添加分支
4. 在 `Cargo.toml` 添加依赖和 feature flag
5. 在 `traits.rs` 的 `DatabaseType` enum 添加新类型
6. 前端在连接配置中添加对应选项

## 常见问题

**编译错误：**
- 缺少数据库驱动：检查 `Cargo.toml` features 配置
- Windows：需要 Visual Studio Build Tools
- Linux：需要 `libssl-dev`, `pkg-config`

**运行时错误：**
- 密钥初始化失败：检查系统 keyring 服务是否可用
- 连接超时：检查 `connection_timeout` 配置（默认 10 秒）
- 会话不存在：确保调用 `create_connection` 或 `ensure_session`

**性能优化：**
- 使用 `Arc::clone()` 而非 `Box` 共享数据库连接
- 查询大数据集时使用分页（`execute_query_paged` 命令）
- PostgreSQL 使用连接池（`deadpool-postgres`）
