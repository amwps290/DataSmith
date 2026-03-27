# Kudu

一个面向桌面场景的轻量级数据库客户端，强调把常用能力做实、把高频操作做顺。

当前项目基于一个 GPL 协议的上游仓库继续开发，并在此基础上加入了大量交互优化、结果管理、剪贴板统一、表数据编辑闭环等改进。

## 项目定位

Kudu 不是“大而全”的数据库平台，更偏向一个日常可用、启动直接、界面紧凑、适合频繁查询和轻量管理的客户端。

目标包括：

- 常用数据库连接与浏览足够直接
- SQL 编辑、执行、取消、结果查看足够顺手
- 表数据查看、复制、编辑、提交形成完整闭环
- 设置、工具、结果页尽量减少打断感

## 当前支持

- MySQL
- PostgreSQL
- SQLite
- MongoDB
- Redis

## 已实现的核心能力

### 连接与工作区

- 多连接管理
- 工作区多标签页
- SQL 编辑器标签页与结果页联动
- 数据库对象树浏览
- 基于连接的只读模式

### SQL 编辑体验

- Monaco SQL 编辑器
- SQL 格式化
- 执行中止
- 执行历史搜索
- SQL 片段管理与复用
- 自动补全缓存刷新

### 结果与数据管理

- 多结果集展示
- 结果复制与系统剪贴板统一
- 结果面板收起/展开
- 表数据筛选、导入、导出
- 单元格查看器
- 单元格复制、整行 JSON 复制、整行 INSERT SQL 复制
- 表数据快速新增
- 表单新增记录
- 表数据修改 / 删除预览后提交

### 结构与工具

- 表结构查看与设计器
- 查询构建器
- 数据对比工具
- PostgreSQL `application_name` 标识

## 技术栈

- 前端：Vue 3、TypeScript、Pinia、Ant Design Vue、Monaco Editor、VXE Table
- 桌面端：Tauri 2
- 后端：Rust

## 本地开发

### 环境要求

- Node.js 18+
- Rust stable
- Tauri 2 对应构建环境

### 安装依赖

```bash
npm install
```

### 前端开发

```bash
npm run dev
```

### 桌面联调

```bash
npm run tauri:dev
```

### 生产构建

```bash
npm run build
npm run tauri:build
```

## 目录结构

```text
src/             Vue 前端
src-tauri/       Rust + Tauri 后端
docs/            规划、任务拆分、发布说明文档
```

## 发布前建议

- 补齐最终 `LICENSE` 文件
- 在 `README` 和发布页中明确说明项目基于哪个上游仓库继续开发
- 补一份变更说明，区分“继承能力”和“本项目新增/重构能力”
- 至少完成一次 `npm run build` 和 `cargo check --manifest-path src-tauri/Cargo.toml`

## 许可证说明

当前仓库的发布前法律整理请看：

[GPL 发布注意事项](/data/hn/code/rust/DataSmith/docs/release-and-gpl-notes.md)

在没有确认上游仓库的准确许可证版本前，不建议对外发布二进制或源码包。
