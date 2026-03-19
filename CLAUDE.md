# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

DataSmith 是一个基于 Tauri 2.x 的轻量级跨平台数据库管理工具，支持 MySQL、PostgreSQL、SQLite、MongoDB、Redis 等多种数据库。

**技术栈：**
- 前端：Vue 3 + TypeScript + Ant Design Vue + Monaco Editor + Pinia
- 后端：Rust + Tauri 2.x + SQLx + Tokio
- 构建工具：Vite 5.x + Cargo

## 开发命令

```bash
# 安装依赖
npm install

# 前端开发（仅 Web 界面）
npm run dev

# Tauri 开发模式（完整应用，自动打开 DevTools）
npm run tauri:dev

# 类型检查
npm run build  # 运行 vue-tsc --noEmit && vite build

# 生产构建
npm run tauri:build

# Rust 后端编译（在 src-tauri 目录）
cd src-tauri && cargo build
cargo build --release  # 发布版本

# Rust 测试
cd src-tauri && cargo test

# 检查特定数据库 feature
cargo check --features mysql
cargo check --features postgresql
```

## 架构设计

### Rust 后端架构（src-tauri/src/）

**核心模块：**
- `database/` - 数据库抽象层和实现
  - `traits.rs` - DatabaseOperations trait 定义统一接口
  - `manager.rs` - ConnectionManager 管理所有数据库连接
  - `mysql.rs`, `postgresql.rs`, `sqlite.rs` - SQL 数据库实现
  - `mongodb.rs`, `redis.rs` - NoSQL 数据库实现
- `commands/` - Tauri 命令处理器（前端调用入口）
  - `connection.rs` - 连接管理命令
  - `query.rs` - SQL 查询执行
  - `metadata.rs` - 元数据获取（数据库、表、视图等）
  - `export.rs` - 数据导出功能
  - `redis.rs` - Redis 专用命令
- `models/` - 数据模型定义
- `utils/` - 工具函数（加密、文件操作等）

**关键设计模式：**
- **Trait 抽象**：所有数据库实现 `DatabaseOperations` trait
- **连接池管理**：使用 `Arc<RwLock<HashMap<String, Box<dyn DatabaseOperations>>>>` 存储连接
- **Feature Flags**：通过 Cargo features 控制数据库支持（default = mysql, postgresql, sqlite, mongodb-support, redis-support）
- **异步架构**：全部使用 `async_trait` 和 tokio runtime

### 前端架构（src/）

**目录结构：**
- `components/` - Vue 组件
  - `database/` - 数据库相关组件（树形结构、表设计器、对话框）
  - `tools/` - 工具组件（查询构建器、数据对比）
  - `connection/` - 连接管理组件
- `stores/` - Pinia 状态管理
  - `connection.ts` - 连接状态
  - `app.ts` - 应用全局状态
- `services/` - 前端服务层
  - `sqlAutocomplete.ts` - SQL 自动补全
  - `redisAutocomplete.ts` - Redis 命令补全
  - `export.ts` - 导出功能
- `views/` - 页面视图
- `router/` - Vue Router 配置
- `types/` - TypeScript 类型定义

**关键特性：**
- **自动导入**：使用 unplugin-auto-import 自动导入 Vue/Pinia API
- **组件自动注册**：使用 unplugin-vue-components 自动注册 Ant Design Vue 组件
- **Monaco Editor**：集成代码编辑器用于 SQL 编写
- **路径别名**：`@` 映射到 `src/`

## 数据库连接流程

1. 前端调用 `test_connection` 命令测试连接
2. 后端 ConnectionManager 根据 `DatabaseType` 创建对应的数据库实例
3. 调用 `connect()` 建立连接池
4. 连接存储在 HashMap 中，使用 UUID 作为 connection_id
5. 后续操作通过 connection_id 获取对应连接

## 添加新数据库支持

1. 在 `src-tauri/src/database/` 创建新文件（如 `newdb.rs`）
2. 实现 `DatabaseOperations` trait
3. 在 `manager.rs` 的 `create_connection` 和 `test_connection` 中添加分支
4. 在 `Cargo.toml` 添加依赖和 feature flag
5. 在 `traits.rs` 的 `DatabaseType` enum 添加新类型
6. 前端在连接配置中添加对应选项

## 重要配置

**Vite 配置（vite.config.ts）：**
- 开发服务器端口：1420（strictPort: true）
- 路径别名：`@` -> `src`
- 代码分割：vue-vendor, ant-design, monaco-editor

**Cargo 配置（src-tauri/Cargo.toml）：**
- Release 优化：`opt-level = "z"`, `lto = true`, `strip = true`
- 默认 features：mysql, postgresql, sqlite, mongodb-support, redis-support

**Tauri 配置：**
- Debug 模式自动打开 DevTools（main.rs:82）
- 使用 tauri-plugin-store 存储连接配置
- 使用 tauri-plugin-dialog 文件对话框
- 使用 tauri-plugin-fs 文件系统访问

## 代码规范

**Rust：**
- 使用中文注释
- 错误处理使用 `thiserror` 定义 `DbError`
- 异步函数使用 `async_trait`
- 连接管理使用 `Arc<Mutex<T>>` 或 `Arc<RwLock<T>>`

**TypeScript/Vue：**
- 使用 Composition API
- 使用 `<script setup>` 语法
- 类型定义放在 `types/` 目录
- 使用 Pinia 管理状态

## 常见问题

**编译错误：**
- 如果缺少数据库驱动，检查 Cargo.toml 的 features 配置
- Windows 编译需要安装 Visual Studio Build Tools
- Linux 需要安装 `libssl-dev`, `pkg-config`

**PostgreSQL 特殊处理：**
- PostgreSQL 需要重新连接才能切换数据库
- 使用 `switch_database` 方法处理数据库切换

**密码加密：**
- 使用 `utils::crypto` 模块加密存储密码
- 使用系统 keyring 存储主密钥

## 测试

目前项目主要通过手动测试，建议添加：
- Rust 单元测试：`cargo test`
- 前端单元测试：使用 Vitest
- 集成测试：测试 Tauri 命令调用
