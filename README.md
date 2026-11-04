# ChatVault Desktop

A privacy-first, offline Windows application foundation for local chat archives. Tauri 2 + Rust, React + TypeScript + Vite, pnpm, Tailwind CSS, shadcn/ui button conventions, React Router, and Lucide icons.

**Phase 01, Step 03: WhatsApp Chat Export Importer** adds a working local text/ZIP import workflow: native file selection, bounded analysis, a real preview, explicit date-format confirmation, atomic persistence, duplicate detection, and an archive metadata library. Step 01/02 navigation, themes, SQLite, vault settings, and real storage statistics are preserved. The full conversation viewer, backup creation, and device access remain for later steps. There are no production demo records or simulated native operations.

ChatVault Desktop is independent and unofficial. It is not affiliated with, endorsed by, or sponsored by WhatsApp or Meta. Original branding is used.

## Import a chat export

On your own phone, open the conversation in WhatsApp. On Android, use the conversation menu → More → Export chat; on iPhone, open contact/group information → Export Chat. Choose with or without media and save/transfer the export locally to your PC. Labels and export limits can vary by app version; see [WhatsApp's export instructions](https://faq.whatsapp.com/1180414079177245/).

In ChatVault, open **Import Chat**, choose the .txt or .zip, review the detected records, participants, media, diagnostics, and sample messages, then explicitly confirm **Import to local vault**. Review/edit the conversation title. Ambiguous dates require **Day / month / year** or **Month / day / year**; no order is silently guessed. After persistence, open Archives to filter by title/filename and inspect metadata/diagnostics. The dashboard updates from SQLite and actual vault files.

Supported text includes common Android `31/12/2026, 09:30 - Alice: …`, iOS `[31/12/26, 9:30:15 AM] Bob: …`, dot/dash-separated dates, year-first dates, 12/24-hour times with optional seconds, multiline Unicode/Sinhala text, system-like/unknown-sender records, deleted placeholders, UTF-8 and BOM-marked UTF-16 LE/BE, and CRLF/LF endings. Original files are retained byte-for-byte. Parsed line endings normalize to LF; original timestamp strings and normalized local times are stored separately. No timezone-free chat timestamp is converted to UTC; the legacy UTC timestamp column remains null. Two-digit years use 2000–2099 and produce a warning.

ZIPs must contain exactly one compatible conversation text. Supported media/text attachments are stored as opaque files with generated collision-safe names; original filenames are metadata. Missing/omitted/ambiguous references do not fail the conversation. Media MIME values are extension hints, not decoded content guarantees. Unsupported/active attachment types are not extracted and produce a diagnostic; their bytes remain in the retained original ZIP. Encrypted ZIPs, ZIP64/split archives, multiple conversations in one ZIP, unsupported encodings/header layouts, and encrypted .crypt14/.crypt15 native backups are unsupported. Some localized media/system phrases may remain unknown rather than classified. Date-looking continuation lines can be ambiguous; inspect the preview/diagnostics. Unknown records are retained rather than silently discarded.

Identical original SHA-256 hashes are rejected as duplicates; cancel and open the existing archive. Reimporting an identical file and merging overlapping histories are not enabled. Different files are independent archives.

Default safety limits are centrally configurable in Rust `ImportLimits`: 256 MiB source ZIP, 16 MiB text, 64 MiB per expanded entry, 512 MiB total expansion, 5,000 ZIP entries, 200:1 compression ratio, 32 text candidates, and 100,000 records/media references. Source text is read into bounded memory; hashing, media extraction, and file persistence stream in chunks. Analysis/save use indeterminate, accurate stage labels.

Synthetic fixtures are provided in `src/test/fixtures/SyntheticAndroid.txt`, `SyntheticIOS.txt`, and `SyntheticMedia.zip`. They contain no private conversations. The Android fixture deliberately needs a date-order choice and includes Sinhala, multiline text, a missing media placeholder, and HTML-looking plain text. The ZIP fixture contains a synthetic conversation and a tiny image. Use a disposable test profile/vault when manually testing.

## Windows setup

- Node.js 24 LTS and pnpm 12.14.0 (pinned in package.json).
- Stable Rust through rustup, with the `x86_64-pc-windows-msvc` toolchain.
- Visual Studio 2022 Build Tools: Desktop development with C++, MSVC, and a Windows 10/11 SDK. Bundled SQLite also requires the native C compiler.
- Microsoft Edge WebView2 Runtime on Windows 10/11.

Follow the [official Tauri prerequisites](https://v2.tauri.app/start/prerequisites/). Restart the terminal after installing Rust so Cargo is on PATH. The app requests no administrator privileges; the NSIS installer is configured per user.

```powershell
pnpm install --frozen-lockfile
pnpm tauri dev
```

If PowerShell blocks `pnpm.ps1`, use `pnpm.cmd` instead. No execution-policy change is required. TypeScript remains on 6.0.x and ESLint on 9 for compatibility with the installed lint plugins.

## Local storage

Tauri resolves `app.path().app_local_data_dir()`. On a typical Windows installation this is `%LOCALAPPDATA%\app.chatvault.desktop`; the actual resolved paths are shown in Settings → Storage & Vault.

```text
<app-local-data>/
  database/
    chatvault.sqlite3
    chatvault.sqlite3-wal / -shm   (when SQLite creates them)
  ChatVault/
    vault/                        (default; may be changed)
      .chatvault-vault.json        (installation identity; not encryption)
      backups/
      archives/
      media/
      temp/
```

Imported archives live in `vault/archives/<UUID>/original/source.txt` or `source.zip`, with extracted files in `media/<generated-name>`. A `.chatvault-import.json` marker supports safe recovery. The archive's original filename/hash, records, participants, media mapping, missing references, and diagnostics are in SQLite. Preview staging is private to a session under vault/temp and is released on cancellation/errors; crash recovery verifies ownership and leases before cleanup. There is no archive deletion endpoint.

The database stays at the same app-local location when the vault changes. SQLite uses bundled SQLite, foreign keys, a five-second busy timeout, WAL, FULL synchronous durability, and versioned transactional migrations. `PRAGMA application_id`, `PRAGMA user_version`, and a migration ledger identify the database and schema. No initialization path resets or deletes an existing database.

The only initial application records are real configuration and migration metadata. The devices, snapshots, archives, conversations, messages, and attachments tables start empty. A ready fresh installation reports **0 snapshots, 0 archives, 0 B**. Vault usage is the logical length of real files under the vault, including temporary files, excluding the identity marker and the database stored outside it. Unavailable storage is never presented as zero bytes.

## Changing the vault

In Settings → Storage & Vault, choose **Change Location** and select a dedicated empty local folder in the native Windows picker. You can create a folder through the picker. Rust validates the selection, checks write access with exclusively created temporary probes, creates the managed subfolders, and commits the setting to SQLite. No frontend command accepts an arbitrary path or SQL string.

Switching is blocked if the current vault contains files, unexpected/nested entries, or snapshot/archive records. A missing current drive must be reconnected before switching. An unrelated nonempty directory or another installation's vault is rejected. A previously used empty vault from this installation can be selected again after its identity and contents are validated. Parent/child overlap with the current vault, database, source, executable, or protected system locations is rejected. Windows network/device paths, links, junctions, hard links, and known OneDrive roots are unsupported.

No files are moved or deleted during a location change. The old empty vault remains. If preparation succeeds but SQLite commit fails, an empty initialized destination may remain; the saved location is unchanged. It can be selected again after safe validation. ChatVault performs no partial content migration or cleanup.

A saved vault that disappears is shown as unavailable and is not automatically recreated, replaced, or relocated. Reconnect the drive and use Refresh/Retry initialization. Explicit retry can repair missing managed subdirectories only after validating the existing vault identity; it does not recreate a missing saved root.

**Vault files and SQLite are not encrypted by ChatVault.** Keep the vault on a trusted, unsynchronized local folder. OS permissions, disk encryption, other software, and manual file changes are outside the application's control.

## Commands

| Command                                            | Purpose                                                                      |
| -------------------------------------------------- | ---------------------------------------------------------------------------- |
| `pnpm install --frozen-lockfile`                   | Install pinned frontend dependencies                                         |
| `pnpm dev`                                         | Frontend-only loopback preview at http://127.0.0.1:1420                      |
| `pnpm tauri dev`                                   | Native development window                                                    |
| `pnpm build`                                       | Strict TypeScript check and production frontend bundle                       |
| `pnpm lint`                                        | ESLint, Hooks, accessibility checks                                          |
| `pnpm test` / `pnpm test:watch`                    | Frontend behavior tests                                                      |
| `node scripts/check-schema.mjs`                    | Auxiliary in-memory SQL schema/constraint check; not Rust service validation |
| `cargo check --manifest-path src-tauri/Cargo.toml` | Rust compilation check                                                       |
| `cargo test --manifest-path src-tauri/Cargo.toml`  | Isolated Rust database/vault tests                                           |
| `cargo fmt --manifest-path src-tauri/Cargo.toml`   | Format Rust on a Rust-capable machine                                        |
| `pnpm format` / `pnpm format:check`                | Frontend, config, and Markdown formatting                                    |
| `pnpm tauri info`                                  | Diagnose Windows prerequisites                                               |
| `pnpm tauri build`                                 | Build application and per-user NSIS installer                                |

Browser previews show a clear desktop-runtime-unavailable state for storage, with no fabricated counts, file operations, or persistence success. Packaged assets and local IPC need no runtime network service. Development dependencies and build tools may need downloads during setup.

## WhatsApp access limitations

Readable chat exports are different from encrypted native backup databases. Copying encrypted native backups does not decrypt them or guarantee complete chat recovery. Export formats, media inclusion, and limits vary. Android scoped storage and USB permissions constrain accessible files. This step imports explicitly selected readable exports; it does not access a phone, create exports, modify WhatsApp files, bypass encryption, recover keys, root devices, scrape WhatsApp Web, or call private messaging APIs. Encrypted `.crypt14` and `.crypt15` backups are unsupported.

## Validation and current limitations

Build, lint, all 41 frontend tests, the auxiliary in-memory SQL check, `cargo check --locked`, and all 49 Windows Rust tests passed on 2026-11-04. The Rust suite includes parser, ZIP security, migration preservation, atomic rollback, duplicate detection, persisted messages/media, custom vaults, recovery, and service restart checks. Rust/Cargo 1.99.1 and Cargo.lock are available. Native smoke-test evidence and remaining acceptance checks are recorded in the validation report; synthetic fixtures contain no private conversations.

See [validation and manual acceptance](docs/VALIDATION.md), [architecture](docs/ARCHITECTURE.md), [security](docs/SECURITY.md), and [roadmap](docs/ROADMAP.md). Step 03 is implemented; Step 04's full conversation viewer remains unstarted. No Git operations were performed.
