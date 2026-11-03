# ChatVault Desktop

A privacy-first Windows desktop foundation for local chat archives. Built with Tauri 2, Rust, React, TypeScript, Vite, pnpm, Tailwind CSS, shadcn/ui button conventions, React Router, and Lucide icons.

**Phase 1, Step 01 only.** Navigation, appearance preferences, empty states, and the backend health command are implemented. Chat import, device detection, backups, archive browsing, and SQLite are not implemented. There are no sample archives or simulated successes.

ChatVault Desktop is independent and unofficial. It is not affiliated with, endorsed by, or sponsored by WhatsApp or Meta. Original shield/vault branding is used.

## Windows prerequisites

- Windows 10/11 with Microsoft Edge WebView2 Runtime.
- Node.js 24 LTS and pnpm 12 (the lockfile was generated using pnpm 12.14.0).
- Stable Rust installed through rustup, using the `x86_64-pc-windows-msvc` toolchain. Restart your terminal after installation so `cargo` and `rustc` are on PATH.
- Visual Studio 2022 Build Tools with **Desktop development with C++**, MSVC, and a Windows 10/11 SDK. Installing development prerequisites may require your machine administrator; the application itself does not request elevation.

Follow the [official Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/). This repository follows the [Tauri Vite configuration](https://v2.tauri.app/start/frontend/vite/), [capabilities model](https://v2.tauri.app/security/capabilities/), and [Tailwind Vite integration](https://tailwindcss.com/docs/installation/using-vite/).

## Install and run

```powershell
pnpm install
pnpm tauri dev
```

If PowerShell blocks the `pnpm.ps1` shim, use `pnpm.cmd` for these commands. No execution-policy change is needed.

To review just the frontend:

```powershell
pnpm dev
```

Open `http://127.0.0.1:1420`. The Settings backend check deliberately reports that the Rust backend is unavailable in a browser. A browser preview does not validate native IPC.

The development server listens only on loopback. A packaged application uses bundled assets and needs no web server or internet connection. Dependencies and Windows build/installer tooling may need downloads during setup/build.

## Commands

| Command                                            | Purpose                                                   |
| -------------------------------------------------- | --------------------------------------------------------- |
| `pnpm install --frozen-lockfile`                   | Reproduce the frontend dependency installation            |
| `pnpm dev`                                         | Loopback-only Vite frontend preview                       |
| `pnpm tauri dev`                                   | Native window with Vite hot reload                        |
| `pnpm build`                                       | Strict TypeScript check and production frontend build     |
| `pnpm lint`                                        | ESLint, React Hooks, and JSX accessibility checks         |
| `pnpm test`                                        | Run Vitest component/unit tests once                      |
| `pnpm test:watch`                                  | Watch tests during development                            |
| `pnpm format` / `pnpm format:check`                | Apply/check Prettier formatting                           |
| `cargo check --manifest-path src-tauri/Cargo.toml` | Check Rust backend and generated command permissions      |
| `pnpm tauri info`                                  | Diagnose native prerequisites                             |
| `pnpm tauri build`                                 | Build the Windows application and per-user NSIS installer |
| `pnpm tauri icon public/brand.svg`                 | Regenerate original application icons                     |

TypeScript stays on compatible 6.0.x because the installed TypeScript ESLint tooling does not support TypeScript 7. ESLint 9 is retained for plugin compatibility. Exact frontend versions are recorded in `pnpm-lock.yaml`. Cargo dependencies follow stable Tauri 2 ranges; a Cargo lockfile must be generated and reviewed on the first Rust-capable build.

## What works in Step 01

- Five React Router routes: `/dashboard`, `/archives`, `/import`, `/devices`, `/settings`. Hash routing keeps navigation compatible with packaged WebView assets (`#/archives`, for example). Unknown paths return to the overview.
- A collapsible sidebar, active navigation state, skip link, route-change heading focus, keyboard-accessible links/buttons, and native appearance radio controls.
- Light, dark, and system appearance. System changes are observed live; only `chatvault.appearance.v1` is saved in this app origin's localStorage. A failed preference write is visible.
- Empty backup/archive counts, unmeasured storage, and an explicitly undetected device state. Import selection and archive search are disabled with explanations.
- A side-effect-free Rust `health_check` command, called from Settings → About. Failures are visible, retryable, bounded by a five-second UI timeout, and do not display arbitrary backend diagnostics.
- Domain and service contracts for later implementation, with no placeholder data-access adapters.

## WhatsApp access limitations

Chat exports are distinct from encrypted native database backups. Export formats, included media, and export limits can vary. Future parsers must validate explicitly chosen inputs. Accessible-file copying does not imply chat decryption or complete chat recovery. Android scoped storage and USB file-transfer visibility limit what can be accessed. No rooting, encryption bypass, private messaging APIs, scraping, or cloud restoration is planned.

## Validation in this environment (2026-11-03)

Frontend installation, build, lint, formatting, and component tests were executed; see [the validation record](docs/VALIDATION.md) for final results and the acceptance checklist.

Native validation is blocked: Rust/Cargo and MSVC/Windows SDK components are missing. WebView2 is present. `cargo check` could not start, and `pnpm tauri dev` failed at `cargo metadata`. Native window launch and actual Rust IPC remain unverified. Visual browser review was also blocked by unavailable browser automation and a Windows app-access approval timeout.

After installing the prerequisites, run the Rust check, launch the native app, open Settings, and verify **Desktop backend connected**. Follow the manual checks in the validation record before considering native acceptance complete.

## Project documentation

- [Architecture and folder responsibilities](docs/ARCHITECTURE.md)
- [Roadmap and future boundaries](docs/ROADMAP.md)
- [Security and privacy](docs/SECURITY.md)
- [Validation and acceptance checklist](docs/VALIDATION.md)

No Git branches, commits, pushes, or merges were performed during implementation.
