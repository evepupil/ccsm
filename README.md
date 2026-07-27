# Claude Session Manager

一个只在本机运行的 Claude Code 会话浏览器。它把散落在 `.claude/projects` 下的会话按项目归档，并能从原项目目录一键打开终端继续工作。

## 当前能力

- 扫描 `%USERPROFILE%\.claude\projects` 下的 Claude Code JSONL 会话。
- 合并 Microsoft Store 版 Claude Desktop 的本地索引，优先使用已有标题、归档状态和活动时间。
- 按项目树查看会话，支持标题、路径、分支和 Session ID 搜索。
- 展示最后活动时间、消息记录数、文件大小、Git 分支、模型和 Claude Code 版本。
- 在 Windows Terminal 中执行 `claude --resume <session-id>`。
- 支持 `--fork-session` 分叉续接，保留原 Session ID。

## 隐私边界

应用只读取本机 Claude 会话和 Desktop 索引，不修改原文件，不上传会话，也不保存 API Key。界面仅接收标题及统计元数据；完整对话正文留在 Rust 后端的本地解析过程里。

“续接”使用终端当前的 Claude CLI 配置。Claude Desktop 内部的 Gateway/API 配置不会自动传给全局 CLI，需要提前在终端完成登录或 Gateway 环境配置。

## 环境要求

- Windows 10/11
- Node.js 24 与 pnpm 10
- Rust stable MSVC 工具链
- Microsoft Edge WebView2 Runtime
- Claude Code CLI
- Windows Terminal，缺失时自动退回独立 PowerShell 窗口

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
src/                       React 界面、搜索排序与显示格式
src-tauri/src/sessions.rs  会话与 Desktop 索引解析
src-tauri/src/launcher.rs  CLI 状态检测与受控终端启动
docs/roadmap.md            里程碑状态
docs/模块设计/             设计、验证和改动归档
```

## 状态

当前里程碑见 [docs/roadmap.md](docs/roadmap.md)。

无安装包的日常运行文件由 `pnpm tauri build --no-bundle` 生成在：

```text
src-tauri\target\release\claude-session-manager.exe
```
