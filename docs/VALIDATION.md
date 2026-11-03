# Step 01 validation record

Final follow-up: `pnpm install --frozen-lockfile` passed after synchronizing the pinned pnpm entry. Sandboxed attempts encountered a Windows pnpm store-lock permission error; the successful final installation used approved sandbox elevation. The Tauri JSON configuration also passed validation against the installed official CLI schema, including numeric format validators. This is not Rust compilation or runtime ACL validation. The temporary Vite server was stopped after verification.

Environment: Windows x64, 2026-11-03. Node 24.18.0, pnpm 12.14.0. Commands used `pnpm.cmd` because the host's PowerShell policy blocks the `.ps1` shim. No policy changes were made.

## Executed commands and actual outcomes

| Command                                                                        | Result                                                                                                                                         |
| ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `pnpm create tauri-app … --template react-ts --manager pnpm --tauri-version 2` | Official scaffold created in a temporary workspace directory and copied into the initially empty project; temporary scaffold removed afterward |
| `pnpm add …` / `pnpm add -D …`                                                 | Installed frontend, Tauri, styling, lint, formatting, and test dependencies; registry access required elevated sandbox network access          |
| `pnpm add -D typescript@~6.0.3`                                                | Resolved the initial TypeScript 7 / TypeScript ESLint incompatibility                                                                          |
| `pnpm install`                                                                 | Passed; dependencies up to date                                                                                                                |
| `pnpm build`                                                                   | Passed after fixing strict test tuple typing; TypeScript check and Vite production bundle produced                                             |
| `pnpm lint`                                                                    | Passed after fixing the initial compatibility issue and scheduling initial backend IPC outside the synchronous effect body                     |
| `pnpm test`                                                                    | Passed: 3 test files, 14 tests                                                                                                                 |
| `pnpm format`                                                                  | Applied formatting to frontend, JSON/config, and Markdown files                                                                                |
| `pnpm format:check`                                                            | Passed                                                                                                                                         |
| `pnpm peers check`                                                             | Passed, no peer dependency issues                                                                                                              |
| `pnpm tauri icon public/brand.svg`                                             | Passed; original branding generated into native icons                                                                                          |
| `pnpm dev`                                                                     | Started Vite successfully at `http://127.0.0.1:1420`; frontend preview only                                                                    |
| `cargo check --manifest-path src-tauri/Cargo.toml`                             | Could not start: `cargo` is not installed/on PATH                                                                                              |
| `pnpm tauri dev`                                                               | Failed before compilation/window launch: unable to run `cargo metadata`, program not found                                                     |
| `pnpm tauri info`                                                              | Ran; found WebView2 156.0.4314.58 and confirmed missing rustc, Cargo, rustup, MSVC Build Tools/SDK components                                  |

The first unaligned TypeScript 7 build passed, but lint failed because the installed lint tooling does not support TypeScript 7. An intermediate build also caught an unchecked tuple index in the navigation test; that was fixed with a readonly tuple. Final frontend checks pass. ESLint 9 emits an installation deprecation notice; it is retained for compatibility with the installed accessibility plugin. No Rust compile or native IPC success is claimed.

## Automated test coverage

- Navigation across all five screens, active state, route-change heading focus, document title, and unknown-route fallback.
- Navigation remains accessible after sidebar collapse.
- Zero backup/archive counts, unmeasured storage, no device connection, and archive empty state.
- Disabled archive search and import file selection; encrypted-backup explanation remains visible.
- Light/dark/system changes and persisted preference restoration.
- Live system appearance changes, explicit-theme precedence, invalid saved values, storage-write failure, and keyboard radio selection.
- Browser/native boundary, visible safe IPC error, retry recovery, and unexpected health status rejection. IPC success in tests is mocked; this does not verify Rust.

## Visual/native limitations

Browser automation reported **The browser access host is unavailable**. Windows fallback browser selection then reported **Computer Use app approval timed out**. No visual screenshot review, native resize check, or desktop IPC check was completed. This was an app-access timeout, not a successful native smoke test.

Rust/Cargo, the stable MSVC Rust toolchain, and the C++ build/Windows SDK prerequisites must be installed manually. See the [official prerequisites](https://v2.tauri.app/start/prerequisites/). No system installation or administrator privilege request was made by the application. The first successful Rust build must generate and review Cargo.lock; Rust dependency resolution is not yet verified.

## Explicit acceptance checklist

| #   | Criterion                                       | Status and evidence                                                                                                                                                                      |
| --- | ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Native Windows window opens                     | **Blocked/unverified.** Tauri configured; launch fails because Cargo is missing.                                                                                                         |
| 2   | All five routes work                            | **Passed in component tests.** Native smoke check pending.                                                                                                                               |
| 3   | Sidebar navigation works                        | **Passed in component tests**, including collapsed sidebar.                                                                                                                              |
| 4   | Light/dark/system theme works and persists      | **Passed in component tests**, including system changes, reload-equivalent remount, keyboard control, and failed persistence. Native visual check pending.                               |
| 5   | Rust backend health check works                 | **Implemented, native verification blocked.** Safe frontend error/retry tested using mocked IPC.                                                                                         |
| 6   | No fake connection or successful backup shown   | **Passed in source and tests.** Empty counts and unavailable capability states only.                                                                                                     |
| 7   | No cloud dependencies/network services required | **Passed in source/dependency review for runtime.** No remote runtime service or API client; packaged offline smoke check pending. Development downloads and loopback Vite are separate. |
| 8   | Responsive at common desktop sizes              | **Implemented, visual verification pending.** Responsive grid/sidebar rules and native minimum dimensions configured.                                                                    |
| 9   | Automated checks pass or failures reported      | **Passed with disclosed native blockers.** Build, lint, 14 tests, format, and frontend peer checks pass; Rust unavailable.                                                               |
| 10  | No Git operations                               | **Passed.** No Git commands, branches, commits, pushes, or merges.                                                                                                                       |

## Manual checks after prerequisite installation

1. Run `pnpm install --frozen-lockfile`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `pnpm tauri dev`.
2. Confirm a native **ChatVault Desktop** window opens. Open Settings → About and verify **Desktop backend connected**, the Rust version response, and Retry check.
3. Navigate all screens with mouse and keyboard. Confirm selected navigation, visible focus, skip link, and destination heading focus. Collapse and expand the sidebar.
4. Select light and dark appearance, restart the app, and verify persistence. Select system, change Windows appearance manually, and verify the app follows it.
5. Review every screen at native content sizes near 1280×860, 1024×768, 900×650, and minimum 760×600, plus 125%/150% Windows display scaling. Verify scrolling, text wrapping, actions, and no horizontal clipping. Below 900 CSS pixels, expect the icon rail and named navigation links.
6. Confirm all import/search controls remain unavailable, no file chooser opens, and no phone, chats, snapshots, or measured usage are fabricated.
7. Build with `pnpm tauri build` and smoke-test the packaged app offline. Verify per-user installation, original icons, bundled styling, and route navigation after restart. No installer or release has been produced in this environment.

## Files delivered

- Frontend entry/config: `index.html`, `package.json`, `pnpm-lock.yaml`, `vite.config.ts`, `tsconfig.json`, `eslint.config.js`, `.prettierrc.json`, `.prettierignore`, `.npmrc`, `components.json`, `.gitignore`.
- Shell/styles/shared UI: `src/main.tsx`, `src/app/App.tsx`, `src/styles.css`, `src/shared/`.
- Feature screens and theme/backend status: `src/features/dashboard/`, `archives/`, `import/`, `devices/`, `settings/`.
- Domain/service/platform boundaries: `src/domain/models.ts`, `src/services/contracts.ts`, `src/services/availability.ts`, `src/platform/backend.ts`.
- Tests: `src/test/setup.ts`, `App.test.tsx`, `theme.test.tsx`, `backend.test.tsx`.
- Native source/config: `src-tauri/Cargo.toml`, `build.rs`, `src/main.rs`, `src/lib.rs`, `tauri.conf.json`, `capabilities/default.json`, and generated original icons.
- Branding/docs: `public/brand.svg`, `README.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/SECURITY.md`, and this record.
- Removed starter demo assets, greeting/opener behavior, and the temporary scaffold. Project-wide caches/build artifacts are ignored. No existing user project files were present before scaffolding.

Step 02 has not been started.
