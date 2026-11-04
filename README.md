# ChatVault Desktop

A privacy-first, offline Windows application foundation for local chat archives. Tauri 2 + Rust, React + TypeScript + Vite, pnpm, Tailwind CSS, shadcn/ui button conventions, React Router, and Lucide icons.

**Phase 01, Step 05: Android USB Device Detection & Read-Only Discovery** adds real Windows Portable Devices (WPD) enumeration, local USB transport verification, explicit file-access checks, bounded WhatsApp folder discovery, and consented local device notes. Steps 01–04 navigation, themes, import, SQLite, vault settings, statistics, chat viewer, search, and media remain available. Backup creation is reserved for Step 06. There are no production demo devices or simulated native successes.

## Connect a phone and discover storage

1. Connect your own Android phone with a data-capable USB cable and unlock it.
2. Open the phone's USB notification, select **File Transfer / MTP**, and approve its file-access prompt if offered. USB debugging, ADB, root, and administrator privileges are unnecessary.
3. Open **Devices**, click **Scan Devices / Refresh**, and select the actual device. Available friendly name/manufacturer come from Windows; missing metadata says Unavailable. A phone brand, phone category, or MTP protocol alone cannot confirm Android. Most WPD drivers do not report the OS, so Android identity may remain unknown even on a working Android phone.
4. Click **Check file access**, or explicitly click **Scan WhatsApp Storage** to authorize folder/filename metadata discovery. The app opens only a confirmed local USB transport, with `GENERIC_READ`. It does not read phone file contents, copy files, decrypt backups, or modify the phone.
5. Review actual findings. Modern `Android/media/com.whatsapp/WhatsApp` and `Android/media/com.whatsapp.w4b/WhatsApp Business`, plus legacy root-level folders, are candidates rather than guarantees. Database, image, video, document, audio, and voice-note categories are checked. An encrypted filename finding is not a readable chat archive. Missing candidate paths do not establish that WhatsApp data is absent.

The app refreshes the live device list every 15 seconds while its document is visible, clears claims on a new scan, and expires backend observations after 30 seconds. File permission is rechecked explicitly and reset on each enumeration; dated discovery findings describe their own scan. Unplugging invalidates handles; reconnecting requires selecting again. A charging-only phone can be invisible to WPD, so absence is not proof that no USB cable is attached. Cameras and other portable devices may appear with unknown identity. Devices whose local USB ancestry cannot be verified are listed but never opened; network/Bluetooth transports and non-Windows discovery are unsupported.

Limits: 32 devices, 8 storage roots, 256 metadata entries per enumerated directory, 2,500 total entries, fixed candidate paths (no whole-phone recursion), and 25 seconds checked between Windows calls. The response returns after at most 28 seconds plus scheduling overhead; cancellation is cooperative. A synchronous vendor COM call cannot safely be killed, so one native worker remains reserved until it returns and blocks additional native scans. No repeated worker spawning or indefinite UI wait is used. Media directories are probed without listing their file metadata; database directories are inspected for encrypted backup filenames only.

**Save device note** requires checking explicit consent. Migration 4 adds `portable_device_notes` with random local record ID, display name, category, last detected time, and preference. It leaves the original Android-only `devices` table and backup references intact. No serial, PnP ID, recognition hash, object ID, or current-connection flag is persisted. Notes cannot identify a phone automatically after disconnection/restart; the same name is not proof of identity. ADB is not included, detected, downloaded, or installed. **Backup Now** remains unavailable.

Architecture, official API references, hardware evidence, and remaining validation limits are in [ARCHITECTURE](docs/ARCHITECTURE.md) and [VALIDATION](docs/VALIDATION.md).

ChatVault Desktop is independent and unofficial. It is not affiliated with, endorsed by, or sponsored by WhatsApp or Meta. Original branding is used.

## Import a chat export

On your own phone, open the conversation in WhatsApp. On Android, use the conversation menu → More → Export chat; on iPhone, open contact/group information → Export Chat. Choose with or without media and save/transfer the export locally to your PC. Labels and export limits can vary by app version; see [WhatsApp's export instructions](https://faq.whatsapp.com/1180414079177245/).

In ChatVault, open **Import Chat**, choose the .txt or .zip, review the detected records, participants, media, diagnostics, and sample messages, then explicitly confirm **Import to local vault**. Review/edit the conversation title. Ambiguous dates require **Day / month / year** or **Month / day / year**; no order is silently guessed. After persistence, open Archives to filter by title/filename and inspect metadata/diagnostics. The dashboard updates from SQLite and actual vault files.

Supported text includes common Android `31/12/2026, 09:30 - Alice: …`, iOS `[31/12/26, 9:30:15 AM] Bob: …`, dot/dash-separated dates, year-first dates, 12/24-hour times with optional seconds, multiline Unicode/Sinhala text, system-like/unknown-sender records, deleted placeholders, UTF-8 and BOM-marked UTF-16 LE/BE, and CRLF/LF endings. Original files are retained byte-for-byte. Parsed line endings normalize to LF; original timestamp strings and normalized local times are stored separately. No timezone-free chat timestamp is converted to UTC; the legacy UTC timestamp column remains null. Two-digit years use 2000–2099 and produce a warning.

ZIPs must contain exactly one compatible conversation text. Supported media/text attachments are stored as opaque files with generated collision-safe names; original filenames are metadata. Missing/omitted/ambiguous references do not fail the conversation. Media MIME values are extension hints, not decoded content guarantees. Unsupported/active attachment types are not extracted and produce a diagnostic; their bytes remain in the retained original ZIP. Encrypted ZIPs, ZIP64/split archives, multiple conversations in one ZIP, unsupported encodings/header layouts, and encrypted .crypt14/.crypt15 native backups are unsupported. Some localized media/system phrases may remain unknown rather than classified. Date-looking continuation lines can be ambiguous; inspect the preview/diagnostics. Unknown records are retained rather than silently discarded.

Identical original SHA-256 hashes are rejected as duplicates; cancel and open the existing archive. Reimporting an identical file and merging overlapping histories are not enabled. Different files are independent archives.

Default safety limits are centrally configurable in Rust `ImportLimits`: 256 MiB source ZIP, 16 MiB text, 64 MiB per expanded entry, 512 MiB total expansion, 5,000 ZIP entries, 200:1 compression ratio, 32 text candidates, and 100,000 records/media references. Source text is read into bounded memory; hashing, media extraction, and file persistence stream in chunks. Analysis/save use indeterminate, accurate stage labels.

Synthetic fixtures are provided in `src/test/fixtures/SyntheticAndroid.txt`, `SyntheticIOS.txt`, and `SyntheticMedia.zip`. They contain no private conversations. The Android fixture deliberately needs a date-order choice and includes Sinhala, multiline text, a missing media placeholder, and HTML-looking plain text. The ZIP fixture contains a synthetic conversation and a tiny image. Use a disposable test profile/vault when manually testing.

## Read and search saved conversations

In **Archives**, choose **Read chat**, or select **Read imported chat** after import. The reader shows genuine last-message previews, counts and dates, a neutral sender timeline, date separators, missing-media notices, and collapsible participant/source/media details. Conversation titles can be filtered. The selected conversation ID is retained for the current desktop session. No local account owner is guessed and messages cannot be sent, edited or deleted.

Use **Load older/newer messages** or **Newest available**. Rust uses indexed message-ID cursors in original import order; React retains at most 120 records rather than loading the whole chat. Very long records are readable through 4,096-unit text sections. Chat dates remain local/unknown-timezone or show their original string; the reader does not convert them to UTC.

**Search messages** searches message text and sender within the selected conversation or all archives. Literal substrings of three or more Unicode characters use an NFC/lowercase FTS5 trigram index, including Sinhala and German combining-character cases. Shorter queries scan at most 2,000 records per request: **Continue search** examines the next section, even when the current section has no results. Each result page contains at most 50 matches. Previous/next results and direct result selection open the saved message context. Highlights preserve escaping and original text. Search has no Boolean/FTS query-language operators.

Images load only when requested. PNG, JPEG, WebP and GIF are supported when the file signature and WebView decoder agree; images are limited to 16 MiB and known dimensions to 25 megapixels. The lightbox has zoom, fit and previous/next images in its current gallery page. Audio/video use native WebView controls: WAV/MP3/Ogg/Opus/AAC/M4A and MP4/WebM depend on installed WebView codec support. No conversion framework is bundled. Unsupported/damaged files produce feedback. Captions/transcripts are not fabricated.

PDF and supported office/text attachments have an explicit **Open document** action using the registered Windows application after Rust revalidation. In-app PDF rendering is not enabled. Imported HTML/SVG/scripts/executables are not treated as application documents. Media URLs contain temporary tokens, never local paths; Rust checks database ownership, vault/archive markers, safe ancestors, file type/size and range bounds on every request. No broad Tauri asset scope is enabled.

`SyntheticViewerMedia.zip` adds a generated 160×100 image, a four-second synthetic PCM tone, and a deliberately undecodable MP4 container for error testing. Recreate it with `node scripts/generate-viewer-fixtures.mjs`. No real voice recordings or private images are included. See the validation report for measured 12,000-record performance and exact native coverage. Vault files, the database and the search index are **not encrypted by ChatVault**.

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

Readable chat exports are different from encrypted native backup databases. Copying encrypted native backups does not decrypt them or guarantee complete chat recovery. Export formats, media inclusion, and limits vary. Android scoped storage and USB permissions constrain accessible files. The importer accepts explicitly selected readable exports; device discovery only examines authorized folder/filename metadata. The app does not create exports, copy phone files, modify WhatsApp files, bypass encryption, recover keys, root devices, scrape WhatsApp Web, or call private messaging APIs. Encrypted `.crypt14` and `.crypt15` backups cannot be imported as conversations.

## Validation and current limitations

Build, lint, frontend tests, schema checks and Windows Rust tests are executed for each completed step. Step 04 adds preserving migration 3, cursor/context/search tests, media authorization/range/path checks, bounded frontend rendering, and measured 12,000-record tests. Exact current counts, native evidence, environment constraints and remaining limitations are in the validation report. Rust/Cargo 1.99.1 and Cargo.lock are available.

See [validation and manual acceptance](docs/VALIDATION.md), [architecture](docs/ARCHITECTURE.md), [security](docs/SECURITY.md), and [roadmap](docs/ROADMAP.md). Steps 01–05 are implemented; Step 06 is not started. No Git operations were performed.
