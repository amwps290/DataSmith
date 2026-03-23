# DataSmith

跨平台数据库管理工具，支持 MySQL、PostgreSQL、SQLite、MongoDB、Redis。

## 技术栈

- **前端：** Vue 3 + TypeScript + Ant Design Vue + Monaco Editor + Pinia + VxeTable
- **后端：** Rust + Tauri 2.x + Tokio + 原生数据库驱动
- **构建：** Vite 5.x + Cargo

## 快速开始

```bash
# 安装依赖
npm install

# 完整应用开发模式（自动打开 DevTools）
npm run tauri:dev

# 仅前端开发（无后端功能）
npm run dev

# 类型检查 + 前端构建
npm run build

# 生产构建
npm run tauri:build
```

## 功能路线图

### 数据库驱动支持

| 数据库 | 完成度 | 状态 |
|--------|--------|------|
| PostgreSQL | 95% | :white_check_mark: 最完整：schemas / views / functions / aggregate / extensions / indexes / foreign keys / DDL / EXPLAIN 全部实现 |
| Redis | 90% | :white_check_mark: 专有 API 完整（5 种数据类型读写、服务器信息、TTL） |
| MySQL | 80% | :warning: 核心完整，缺少 `get_views` / `get_functions` / `get_schemas` 元数据 |
| SQLite | 75% | :warning: 核心完整，`alter_table` 仅支持 AddColumn（SQLite 本身限制），缺少 views 元数据 |
| MongoDB | 25% | :x: 仅有连接和库/集合列表；`execute_query` 仅按集合名做 find；结构 / 索引 / DDL / EXPLAIN / 数据操作全部缺失 |
| Elasticsearch | 0% | :x: `DatabaseType` 枚举中已声明，无任何实现 |

### 后端功能

#### :white_check_mark: 已完成

| 模块 | 命令数 | 说明 |
|------|--------|------|
| 连接管理 | 8 | 创建 / 断开 / 测试 / 保存 / 更新 / 删除 / 列表 / 创建 SQLite 库 |
| 查询执行 | 5 | 执行 / EXPLAIN / 批量执行 / SQL 美化 / ALTER TABLE |
| 数据操作 | 2 | UPDATE / DELETE |
| 元数据查询 | 14 | 库 / 表 / 视图 / Schema / 函数 / 聚合函数 / 扩展 / 索引 / 外键 / DDL |
| 工作区 | 5 | 会话保存 / 加载、SQL 脚本管理 |
| 数据导出 | 3 | CSV / JSON / SQL |
| 文件工具 | 2 | 读 / 写文件 |
| Redis 专用 | 5 | 命令执行 / 服务器信息 / 键值读写删 |

#### :x: 未完成

| 命令 | 问题 |
|------|------|
| `insert_table_data` | 空实现 — 函数体仅 `Ok(())` |
| `get_autocomplete_data` | 空实现 — 返回空 JSON `{}`，SQL 自动补全无后端数据 |
| `export_table_ddl` | 硬编码 MySQL 语法 `SHOW CREATE TABLE`，未使用跨数据库的 `get_table_ddl()` |

#### :x: 前端调用但后端不存在的命令

| 命令 | 调用位置 |
|------|----------|
| `get_view_definition` | BackupDatabaseDialog |
| `truncate_table` | ImportDataDialog |
| `get_procedures` | GlobalSearch |
| `get_functions`（命令级） | GlobalSearch |

### 前端 UI 功能

#### :white_check_mark: 已完成

| 功能 | 说明 |
|------|------|
| 连接管理 | 5 种数据库创建 / 编辑 / 删除 / 测试连接 |
| SQL 查询编辑器 | Monaco 集成、语法高亮、自动补全、多结果集、选中执行、EXPLAIN、格式化 |
| 查询结果展示 | vxe-grid 虚拟滚动、自动分页、多结果集 Tab |
| 数据库浏览器 | 递归树、懒加载、PostgreSQL Schema 层级、右键菜单 |
| 表数据查看 / 编辑 | 双击编辑、变更追踪（单元格高亮）、批量提交、删除行、WHERE 筛选 |
| 表设计器 | 列 / 索引 / 外键增删改、DDL 预览、批量变更提交 |
| DDL 查看 | 右键菜单 Monaco 弹窗展示 |
| 数据导出 | CSV / JSON / SQL 格式，支持行数限制、文件选择 |
| 数据导入 | CSV / JSON / SQL 格式，插入 / 替换 / 清空后插入模式 |
| 暗色 / 亮色主题 | Ant Design + Monaco + vxe-table 全链路适配 |
| Redis 命令编辑器 | 141 命令自动补全、db 切换、服务器信息、保活 PING |
| SQL 历史记录 | 抽屉面板，LocalStorage 持久化（最多 100 条） |
| SQL 代码片段 | 7 个预置模板，CRUD 管理 |
| 全局搜索 | 搜索表 / 列 / 视图 / 过程 / 函数，高亮和分类 |
| 数据库备份 / 还原 | 备份弹窗（结构 / 数据选择、压缩）、还原弹窗（追加 / 替换模式） |
| 建库 / 建表 / 建视图 | 各有独立对话框 |
| 自定义标题栏 | 无边框窗口、拖拽、最小化 / 最大化 / 关闭 |
| 侧边栏 | 折叠 / 展开、拖拽调整宽度 |
| 会话恢复 | 标签页状态持久化，启动时自动恢复 |
| 单元格查看器 | JSON 格式化、NULL 设置、复制 |
| 键盘快捷键 | F5 执行 SQL、Ctrl+S 保存、Ctrl+Enter 执行 Redis |

#### :warning: 已实现但未集成到主界面

| 功能 | 组件 | 说明 |
|------|------|------|
| 可视化查询构建器 | `QueryBuilder.vue` | 完整但无入口 |
| 数据比较工具 | `DataCompare.vue` | 基础比较完成，同步脚本生成未实现，无入口 |
| Redis 键值查看器 | `RedisKeyViewer.vue` | 支持 5 种数据类型查看 / 编辑，未挂载 |
| ExportService | `services/export.ts` | 服务类已定义但组件直接 invoke，未使用 |

#### :x: 未实现

| 功能 | 说明 |
|------|------|
| 新增数据行 | `TableDataGrid.addRow()` 仅显示占位提示 |
| MongoDB 专属 UI | 无文档查看 / 编辑界面 |
| Redis 键浏览树 | 无法从侧边栏浏览 Redis 键列表 |
| 存储过程 / 触发器管理 | 无展示节点，无编辑 UI |
| 表关系 / ER 图 | 无 |
| 数据库用户 / 权限管理 | 无 |
| 多 Tab 拖拽排序 | 标签页不可拖拽 |
| 快捷键自定义 | 无设置界面 |

### 国际化（i18n）

| 状态 | 组件 |
|------|------|
| :white_check_mark: 已国际化 | HomeView, ConnectionPanel, ConnectionDialog, DatabaseTree, SqlEditor, TableDataGrid, TableDesigner |
| :x: 硬编码中文 (~15 个) | CreateDatabaseDialog, CreateTableDialog, CreateViewDialog, ExportTableDialog, ImportDataDialog, BackupDatabaseDialog, RestoreDatabaseDialog, InsertRecordDialog, RedisEditor (部分), RedisKeyViewer, SaveQueryDialog, SqlSnippetsManager, GlobalSearch, QueryBuilder, DataCompare |

### 已知问题

| 问题 | 位置 |
|------|------|
| `export_table_ddl` 硬编码 MySQL 语法 | `commands/export.rs` |
| MySQL `get_foreign_keys` 的 update_rule / delete_rule 硬编码 CASCADE | `database/mysql.rs` |
| Redis 连接不支持密码认证 | `database/redis.rs` — URL 格式无 username / password |
| 建表 / 建视图对话框仅生成 MySQL 语法 | `CreateTableDialog.vue`, `CreateViewDialog.vue` |

### 工程质量

| 维度 | 状态 |
|------|------|
| 单元测试 | :x: 前后端均无测试 |
| Lint / Format | :x: 无配置 |
| CI/CD | :warning: 有 release workflow，仅用于发布构建 |

## 许可证

MIT
