# Qbit Toolbox

Qbit Toolbox is a Windows desktop toolbox for Qbit workflows. The Foundation bootstrap establishes the monorepo, desktop shell, shared UI, and typed IPC contract; Hotkeys are not implemented yet.

See [the architecture](QBIT_TOOLBOX_ARCHITECTURE.md) for the system design.

## Stack

- Bun workspaces and TypeScript
- Next.js and React desktop frontend
- Tauri 2 and Rust desktop host
- Cargo workspace for shared Rust crates

## Prerequisites

- Windows, the current desktop shipping target
- Bun 1.3.14
- Rust 1.97.1, selected by `rust-toolchain.toml`
- The general [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/) for WebView2 and Microsoft C++ build tools

## Commands

Run these from the repository root:

```sh
bun run dev
bun run build
bun run check
bun run version:check
bun run architecture:check
bun run test
bun run typecheck
bun run lint
bun run ipc:generate
bun run ipc:check
bun run tauri:dev
```

## Layout

```text
apps/desktop/        Next.js frontend and Tauri desktop host
crates/              Shared Rust workspace crates
crates/diagnostics/  Shared diagnostics crate
packages/i18n/       Shared internationalization
packages/ipc/        TypeScript IPC client and generated types
packages/ui/         Shared UI components and theme
tools/               Repository tooling
```

`packages/ipc/generated/` is generated from the Rust IPC contracts. Never hand-edit its contents; run `bun run ipc:generate` instead.
