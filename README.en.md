# Claude Code Session Manager (CCSM)

[中文](README.md) | English

CCSM is a local Windows desktop application for browsing Claude Code and Codex session history in one place. It groups both sources by their real project directories, provides search and filtering, and opens a terminal in the original project directory to resume or fork a session.

> Current source version: `0.4.0`

## Features

- Scans Claude Code JSONL sessions under `%USERPROFILE%\.claude\projects`.
- Read-only scans Codex session indexes in `%CODEX_HOME%\state_*.sqlite`, falling back to `%USERPROFILE%\.codex\state_*.sqlite` when `CODEX_HOME` is not set.
- Automatically merges the local index created by the Microsoft Store edition of Claude Desktop, including titles, archive state, and activity times.
- Merges Claude Code and Codex sessions by their normalized project directory, with provider switching and cross-project search.
- Supports archived-session filtering, sorting by recent activity or title, and grouping recent sessions into Today, Yesterday, This week, and Earlier.
- Displays provider, title, activity time, message or token count, file size, Git branch, model, and CLI version.
- Prioritizes user-defined session titles from Claude Code and Codex, then falls back through each provider's available title sources.
- Supports CCSM-local archiving, multi-select, bulk archive and restore, and starting a new Claude Code or Codex session for the current project.
- Provides a temporary-session project that groups both providers from temporary directories and refreshes the local catalog automatically.
- Hovering or focusing the Claude/Codex logo at the left of a session row shows the five most recent user-message previews in order.
- Supports resuming or forking sessions:

  | Provider    | Resume                 | Fork                                  |
  | ----------- | ---------------------- | ------------------------------------- |
  | Claude Code | `claude --resume <id>` | `claude --resume <id> --fork-session` |
  | Codex       | `codex resume <id>`    | `codex fork <id>`                     |

- Opens Windows Terminal when available and falls back to a standalone PowerShell window.
- Can temporarily enable the highest-permission mode. Claude Code uses `--dangerously-skip-permissions` and Codex uses `--yolo`; the option is disabled by default and keeps a visible warning when enabled.

## Data Sources

| Source         | Default location                                                        | Read data                                              |
| -------------- | ----------------------------------------------------------------------- | ------------------------------------------------------ |
| Claude Code    | `%USERPROFILE%\.claude\projects`                                        | JSONL session metadata                                 |
| Codex          | `%CODEX_HOME%\state_*.sqlite`, or `%USERPROFILE%\.codex\state_*.sqlite` | Session indexes from the `threads` table               |
| Claude Desktop | Local index of the Microsoft Store edition                              | Supplementary titles, archive state, and activity data |

## Privacy and Security

- CCSM reads local session data only. It does not modify, migrate, or delete the original files.
- Full conversation bodies are read briefly on the local machine only when needed for titles, statistics, or an on-demand preview of the five most recent user messages. Preview text stays in process memory; sessions are never uploaded and API keys are not stored.
- Before launching a session, CCSM rescans and validates the Session ID and project directory. It will not launch a session when the directory no longer exists.
- The terminal inherits the current Windows user's CLI configuration. Highest-permission mode is disabled by default, is not persisted locally, and keeps a visible warning while enabled.

## Download

Windows installers and portable builds are published on [GitHub Releases](https://github.com/evepupil/ccsm/releases). Builds without code signing may trigger a Windows SmartScreen warning on first launch.

## Requirements

To run CCSM:

- Windows 10 or Windows 11
- Microsoft Edge WebView2 Runtime
- Claude Code CLI or Codex CLI, depending on the sessions you want to resume
- Windows Terminal (recommended; PowerShell is used automatically when it is unavailable)

For development from source:

- Node.js 24
- pnpm 10 (the repository pins `pnpm@10.21.0`)
- The Rust stable MSVC toolchain

## Run From Source

```powershell
git clone https://github.com/evepupil/ccsm.git
cd ccsm
pnpm install
pnpm tauri dev
```

`pnpm tauri dev` starts the Vite frontend and the Tauri desktop application. CCSM currently supports Windows only.

## Build

Build the frontend:

```powershell
pnpm build
```

Build the Tauri installers:

```powershell
pnpm tauri build
```

Build the Windows executable without bundling installers:

```powershell
pnpm tauri build --no-bundle
```

The unbundled executable is written to:

```text
src-tauri\target\release\ccsm.exe
```

Bundled installers are written under `src-tauri\target\release\bundle\`.

## Project Checks

Run these checks before committing:

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

## Project Layout

```text
src/                              React UI, filtering, sorting, and presentation
src/components/                   Sidebar, title bar, and session list components
src/hooks/                        Session scanning, filtering, and terminal launch state
src/lib/                          Formatting, filtering, and launch-command helpers
src-tauri/src/catalog.rs          Claude Code and Codex project aggregation
src-tauri/src/providers/claude.rs Claude session and Desktop index parsing
src-tauri/src/providers/codex.rs  Read-only Codex SQLite indexing
src-tauri/src/launcher.rs         CLI detection and controlled terminal launching
docs/roadmap.md                   Milestone status
docs/模块设计/                    Design, verification, and change records
```

## Project Status

See [docs/roadmap.md](docs/roadmap.md) for current milestones and exit criteria. M0, M1, and M2 are still being refined; real session titles, timestamps, and terminal resume behavior should be verified in the Windows release application.
