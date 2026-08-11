# Claude Code Session Manager（CCSM）

[English](README.en.md) | 中文

CCSM 是一个面向 Windows 的本地桌面应用，用于统一浏览 Claude Code 和 Codex 的历史会话。它会按真实项目目录合并两类会话，支持搜索、筛选、查看元数据，并从原项目目录一键继续或分叉工作。

> 当前源码版本：`0.3.0`

## 功能

- 扫描 `%USERPROFILE%\.claude\projects` 下的 Claude Code JSONL 会话。
- 只读扫描 `%CODEX_HOME%\state_*.sqlite`；未设置 `CODEX_HOME` 时回退到 `%USERPROFILE%\.codex\state_*.sqlite`。
- 自动合并 Microsoft Store 版 Claude Desktop 的本地索引，补充标题、归档状态和活动时间。
- 按真实项目目录合并 Claude Code 与 Codex 会话，支持来源切换和跨项目搜索。
- 支持归档筛选，以及按最近活动时间或标题排序；最近活动视图按今天、昨天、本周和更早分组。
- 展示来源、标题、活动时间、消息数或 Token 数、文件大小、Git 分支、模型和 CLI 版本。
- 支持原会话续接和分叉续接：

  | 来源        | 原会话                 | 分叉会话                              |
  | ----------- | ---------------------- | ------------------------------------- |
  | Claude Code | `claude --resume <id>` | `claude --resume <id> --fork-session` |
  | Codex       | `codex resume <id>`    | `codex fork <id>`                     |

- 续接时优先打开 Windows Terminal；未安装时回退到独立 PowerShell 窗口。
- 可临时开启最高权限。Claude Code 使用 `--dangerously-skip-permissions`，Codex 使用 `--yolo`；该选项默认关闭并持续显示风险提示。

## 数据来源

| 来源           | 默认位置                                                                | 读取内容                           |
| -------------- | ----------------------------------------------------------------------- | ---------------------------------- |
| Claude Code    | `%USERPROFILE%\.claude\projects`                                        | JSONL 会话元数据                   |
| Codex          | `%CODEX_HOME%\state_*.sqlite`，或 `%USERPROFILE%\.codex\state_*.sqlite` | `threads` 表中的会话索引           |
| Claude Desktop | Microsoft Store 版的本地索引                                            | 标题、归档状态和活动时间等补充信息 |

## 隐私与安全

- CCSM 只读取本机会话数据，不修改、迁移或删除原始文件。
- 完整对话正文只在本地解析标题和统计信息时短暂读取，不上传会话，也不保存 API Key。
- 续接前会重新扫描并校验 Session ID 和项目目录；项目目录不存在时不会启动会话。
- 续接终端继承当前 Windows 用户的 CLI 配置。最高权限默认关闭，不写入本地存储；开启后界面会持续显示风险提示。

## 下载

Windows 安装包和便携版请前往 [GitHub Releases](https://github.com/evepupil/ccsm/releases)。未配置代码签名的构建在首次运行时可能触发 Windows SmartScreen 提示。

## 环境要求

运行 CCSM 需要：

- Windows 10 或 Windows 11
- Microsoft Edge WebView2 Runtime
- Claude Code CLI 或 Codex CLI（按需安装；续接对应来源的会话时必须可用）
- Windows Terminal（推荐；缺失时自动回退到 PowerShell）

从源码开发还需要：

- Node.js 24
- pnpm 10（仓库锁定版本为 `pnpm@10.21.0`）
- Rust stable MSVC 工具链

## 从源码运行

```powershell
git clone https://github.com/evepupil/ccsm.git
cd ccsm
pnpm install
pnpm tauri dev
```

`pnpm tauri dev` 会启动 Vite 前端和 Tauri 桌面应用。CCSM 当前仅支持 Windows。

## 构建

构建前端：

```powershell
pnpm build
```

构建 Tauri 安装包：

```powershell
pnpm tauri build
```

只生成不打包的 Windows 可执行文件：

```powershell
pnpm tauri build --no-bundle
```

无安装包的可执行文件位于：

```text
src-tauri\target\release\ccsm.exe
```

完整安装包位于 `src-tauri\target\release\bundle\`。

## 项目门禁

提交前运行：

```powershell
pnpm format:check
pnpm check
pnpm test
cargo fmt --manifest-path src-tauri\Cargo.toml --check
cargo clippy --manifest-path src-tauri\Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri\Cargo.toml
pnpm build
pnpm tauri build --no-bundle
```

## 项目结构

```text
src/                              React 界面、筛选排序和显示格式
src/components/                   侧栏、标题栏和会话列表组件
src/hooks/                        会话扫描、筛选和终端启动状态
src/lib/                          格式化、筛选和启动命令等纯函数
src-tauri/src/catalog.rs          Claude Code 与 Codex 项目合并
src-tauri/src/providers/claude.rs Claude 会话和 Desktop 索引解析
src-tauri/src/providers/codex.rs  Codex SQLite 只读索引
src-tauri/src/launcher.rs         双 CLI 检测和受控终端启动
docs/roadmap.md                   里程碑状态
docs/模块设计/                    设计、验证和改动归档
```

## 项目状态

当前里程碑和退出标准见 [docs/roadmap.md](docs/roadmap.md)。M0、M1 和 M2 的实现持续完善中，真实会话标题、时间和终端续接仍以 Windows release 应用中的人工验收为准。
