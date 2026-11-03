# Roadmap

## Phase 1, Step 01 — foundation

Implemented source:

- Tauri 2/Rust application configuration, original branding, and least-privilege health command.
- React/TypeScript/Vite desktop shell and five routes.
- Dark/light/system appearance and local preference persistence.
- Truthful empty states and clear future-feature labels.
- Typed domain models and future service interfaces.
- Lint, formatting, behavior tests, and documentation.

Native launch and actual IPC validation are pending installation of the Windows development prerequisites. Step 01 acceptance is therefore partial in this environment, even though the requested source foundation is implemented.

## Step 02 — local persistence (not started)

Introduce SQLite schema, migrations, explicit repository boundaries, lifecycle/error handling, and real persisted empty summaries. Decide storage location and deletion/retention behavior before content is written. No SQLite driver, database file, repository, or migration is included in Step 01.

## Later boundaries (sequence to be agreed)

1. Chat-export parsing: explicit file selection, format detection, locale/date handling, parse reports, attachment matching, and invalid/malicious input limits.
2. Archive browsing: real repository-backed conversations, messages, media, and local search.
3. Device adapters: authorized Android accessible-file access and accurate connection/permission states. Evaluate MTP and any separate adapters before adding permissions.
4. Backup snapshots: real accessible-file copying, capacity checks, cancellation, integrity checks, error handling, and retention.
5. Packaging/release: Rust and frontend lockfile verification, native smoke checks, installer QA, accessibility review, and signing policy.

Each step must replace disabled controls only when an actual implementation and error path exist. Never replace unavailable behavior with simulated success.

## Out of scope

Cloud synchronization, login, analytics, WhatsApp Web scraping, private messaging APIs, root access, encryption bypass, recovering protected app storage, and claims of complete history recovery. Exported chats and encrypted native backups remain separate concepts.
