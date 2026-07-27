# Codex 会话索引

- **模块定位**：只读发现 Codex 本地状态数据库，并把可续接会话转换为 CCSM 的统一元数据。
- **对应代码**：`src-tauri/src/providers/codex.rs`、`src-tauri/src/catalog.rs`、`src-tauri/src/models.rs`
- **所属里程碑**：[M1](../roadmap.md#m1)
- **当前状态**：进行中
- **最近更新时间**：2026-07-27

## 职责与边界

本模块读取 `%CODEX_HOME%`；该变量未设置时读取 `%USERPROFILE%\.codex`。它从数字版本最大的 `state_*.sqlite` 获取会话标题、项目目录、活动时间、Token 数、分支、模型、CLI 版本、归档状态和 rollout 路径。

SQLite 连接使用只读标志、两秒忙等待和 `query_only`。模块不会读取 `logs_*.sqlite`，不会修改 Codex 数据库，也不会把首条消息或预览正文返回给前端。

## 结构与数据流

```text
CODEX_HOME / ~/.codex
        │
        ├─ 选择数字版本最大的 state_*.sqlite
        ├─ 只读检查 threads 表字段
        ├─ 兼容秒和毫秒时间字段
        └─ 转换为 SessionSummary(provider = codex)
                              │
Claude SessionSummary ────────┤
                              ▼
                    catalog 按项目路径合并
```

## 关键决策

1. 以 `threads` 表作为 Codex 会话清单，避免逐个扫描大体积 rollout JSONL。
2. `has_user_event` 在当前 Codex 版本中不能作为可见性条件，因此保留有标题的全部 thread，并展示 `source`、`thread_source` 作为来源细节。
3. 标题按 `name`、`title`、`preview`、`first_user_message` 的顺序回退，只把最终标题返回前端。
4. 可选列通过 `PRAGMA table_info` 检测，时间字段兼容 `*_at_ms` 与旧版秒时间戳。
5. Claude 与 Codex 项目路径统一斜杠、大小写和末尾分隔符后再合并。

## 当前实现

- 自动发现 `CODEX_HOME` 和最高版本状态数据库。
- 读取 Session ID、标题、项目目录、时间、Token、分支、模型、推理强度、CLI 版本和归档状态。
- 标记项目目录是否仍存在，目录丢失时保留历史记录并禁用续接。
- 来源不可用或表结构缺失时保留另一 provider 的结果，并在来源状态中显示原因。
- 相同项目目录下的 Claude 与 Codex 会话合并到同一个项目节点。

## 验证方式

Rust 单测使用临时 SQLite 数据库覆盖最高版本选择、动态字段查询、秒时间戳转换、Token 与分支读取，以及只读扫描结果。统一 catalog 单测覆盖大小写和斜杠不同的双来源项目合并。

本机 `state_5.sqlite` 的 `threads` 表结构已核对，当前 58 条记录覆盖 47 个项目目录，并且都具备标题、项目目录、rollout 路径和预览字段。完整门禁与 Windows release 打包已通过。最终人工验收需要打开 release 客户端核对真实 Codex 标题和时间。

## 待扩展项

- 针对未来 Codex schema 版本增加兼容夹具。
- 增量刷新和数据库变更通知。
- 对自动任务、子代理和普通用户会话提供独立筛选。
- 提供可选的本地标题与标签覆盖表。

## 改动历史

- 2026-07-27：建立 Codex SQLite 只读扫描、动态列兼容、来源详情和双来源项目合并。
