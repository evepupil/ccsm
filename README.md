# Claude Code Session Manager（CCSM）

一个只在本机运行的 Claude Code 与 Codex 会话浏览器。它把两个工具的历史会话按真实项目目录合并归档，并能从原项目目录一键打开终端继续工作。

## 当前能力

- 扫描 `%USERPROFILE%\.claude\projects` 下的 Claude Code JSONL 会话。
- 只读扫描 `%CODEX_HOME%\state_*.sqlite` 或 `%USERPROFILE%\.codex\state_*.sqlite` 中的 Codex 会话索引。
- 合并 Microsoft Store 版 Claude Desktop 的本地索引，优先使用已有标题、归档状态和活动时间。
- 按项目树统一查看两种来源，支持跨项目搜索、来源筛选、归档筛选和排序。
- 展示来源、标题、最后活动时间、消息数或 Token 数、文件大小、Git 分支、模型和 CLI 版本。
- 支持 Claude Code 与 Codex 的原会话续接和分叉续接。
- 可临时开启最高权限；Claude Code 使用 `--dangerously-skip-permissions`，Codex 使用 `--yolo`。

## 隐私边界

应用只读取本机会话元数据，不修改原文件，不上传会话，也不保存 API Key。完整对话正文只在本地解析标题和统计信息时短暂读取，不会发送给前端。

续接终端继承当前 Windows 用户环境中的 CLI 配置。最高权限默认关闭，而且不会写入本地存储；打开后界面会持续显示风险提示。

## 环境要求

- Windows 10/11
- Microsoft Edge WebView2 Runtime
- Claude Code CLI 或 Codex CLI，按需要安装
- Windows Terminal，缺失时自动退回独立 PowerShell 窗口

参与开发还需要 Node.js 24、pnpm 10 和 Rust stable MSVC 工具链。

## 下载

Windows 安装包和便携版发布在 [GitHub Releases](https://github.com/evepupil/ccsm/releases)。当前 `v0.2.0` 没有代码签名，首次运行时 Windows 可能显示 SmartScreen 提示。

## 开发

```powershell
pnpm install
pnpm tauri dev
```

项目门禁：

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

## 目录

```text
src/                              React 界面、筛选排序与显示格式
src-tauri/src/catalog.rs          双来源项目合并
src-tauri/src/providers/claude.rs Claude 会话与 Desktop 索引解析
src-tauri/src/providers/codex.rs  Codex SQLite 只读索引
src-tauri/src/launcher.rs         双 CLI 检测与受控终端启动
docs/roadmap.md                   里程碑状态
docs/模块设计/                    设计、验证和改动归档
```

## 状态

当前里程碑见 [docs/roadmap.md](docs/roadmap.md)。

无安装包的日常运行文件由 `pnpm tauri build --no-bundle` 生成在：

```text
src-tauri\target\release\ccsm.exe
```
