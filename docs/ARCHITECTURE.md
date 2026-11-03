# Architecture

## Boundaries

The React application owns presentation, navigation, and appearance. `src/platform/backend.ts` is the sole native IPC entry point. The Rust application exposes only `health_check`, which returns status, application version, and OS identifier. It performs no IO and receives no user input.

`src/domain/models.ts` defines Device, BackupSnapshot, Archive, Conversation, Message, and Attachment. Domain types have no dependency on React, Tauri, SQL, or USB APIs. IDs and ISO timestamps are contracts; future parsers and repositories must validate them at runtime.

`src/services/contracts.ts` reserves DeviceService, BackupService, ArchiveService, and ImportService. No fake implementations are registered. All content capabilities in `availability.ts` are false. Zero counts represent the untouched foundation, not a filesystem scan or database query. Storage is deliberately unmeasured.

Future presentation code must use service interfaces for content access. Repository implementations will encapsulate SQLite behind ArchiveService/BackupService; parsers and USB adapters will remain separate from views. The import contract accepts a future scoped selection token rather than unrestricted filesystem paths.

## Folder responsibilities

| Location                  | Responsibility                                                    |
| ------------------------- | ----------------------------------------------------------------- |
| `src/app/`                | Desktop shell, sidebar, route definitions, focus/title management |
| `src/features/dashboard/` | Initial overview, empty summaries, workflow links                 |
| `src/features/archives/`  | Empty library and unavailable search shell                        |
| `src/features/import/`    | Export/native-backup explanation and planned workflow             |
| `src/features/devices/`   | Undetected device state and connection preparation                |
| `src/features/settings/`  | Appearance context, About, backend status/retry                   |
| `src/shared/components/`  | Shared headings, badges, empty states, native button primitive    |
| `src/shared/utils.ts`     | Class merging for reusable UI                                     |
| `src/domain/`             | Storage-agnostic domain contracts                                 |
| `src/services/`           | Future use-case interfaces and explicit capability availability   |
| `src/platform/`           | Tauri detection and typed IPC boundary                            |
| `src/test/`               | Vitest environment and behavior tests                             |
| `src/styles.css`          | Local typography, theme tokens, shared styling, responsive rules  |
| `public/brand.svg`        | Original vector branding; no externally hosted assets             |
| `src-tauri/src/`          | Minimal Rust command and application entry points                 |
| `src-tauri/capabilities/` | Main-window-only health-check permission                          |
| `src-tauri/build.rs`      | Build-time application command ACL generation                     |
| `docs/`                   | Scope, security, architecture, validation records                 |

## Runtime and routing

HashRouter resolves all five routes from bundled assets without server rewrites. Both in-app workflow links and sidebar links use React Router. Navigation sets the document title, resets the scroll position, and focuses the new page heading. Sidebar collapse is session-only. Under 900 CSS pixels, the sidebar becomes an icon rail; accessible names remain present. The native window is configured at 1280×860 with a minimum of 760×600. Narrow web previews also reflow at 680 CSS pixels.

## Appearance persistence

ThemeProvider reads only the namespaced `chatvault.appearance.v1` localStorage key. Accepted values are light, dark, and system. Missing, invalid, or unreadable values fall back to system. A media-query listener follows OS changes while the system preference is selected; explicit choices take precedence. A failed write changes the current session's theme and displays a persistence warning. Appearance is not sensitive data. No other product data or settings are saved.

## Backend status

Settings mounts BackendStatus and schedules an initial request. Browser preview is detected before invoking IPC. The health result is awaited with a five-second timeout, stale/unmounted results are ignored, and retry starts a new request. Arbitrary backend error strings are replaced with a safe message. UI tests mock IPC at the boundary; they cannot substitute for native runtime validation.

## Build and quality

Vite integrates React and Tailwind v4. Tailwind scans only `src/`, keeping caches and scaffolding out of production CSS. Strict TypeScript includes unchecked-index and exact-optional checks. ESLint includes Hooks and JSX accessibility rules. Prettier handles supported frontend/config/docs formats; Rust formatting requires rustfmt on a Rust-capable machine.

The shared button follows the shadcn/ui CVA/native-button pattern without adding a component runtime for simple controls. Theme choices use native radios for keyboard and screen-reader behavior. Icons come from lucide-react; system fonts keep the runtime offline.
