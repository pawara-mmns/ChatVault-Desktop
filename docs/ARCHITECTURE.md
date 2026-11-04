# Architecture — Step 03

## Chat export import architecture

The importer extends the existing Rust StorageService and managed SQLite connection. The main window has six additional application permissions, with no generic filesystem, SQL, or dialog permission. Native selection returns a session token, filename, size, format, and duplicate metadata. The frontend never supplies an arbitrary source path. Expensive reads, hashing, parsing, ZIP processing, persistence, and recovery run through the existing blocking-worker/mutex boundary.

| Module                               | Responsibility                                                                                                                   |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------- |
| `storage/imports/input.rs`           | Selected-file validation, exclusive staged copy, streaming SHA-256, source-change detection, bounded text reads                  |
| `parser.rs`                          | Strict encoding decode, Android/iOS headers, multiline records, local dates, sender/classification/media references, diagnostics |
| `zip_processor.rs`                   | Bounded EOCD preflight, untrusted-entry validation, Stored/Deflate streaming, generated filenames, CRC/size verification         |
| `media.rs`                           | NFC/case-normalized exact/unique-basename media matching and missing/ambiguous diagnostics                                       |
| `staging.rs`                         | Session-owned TempDir, identity records, leases, no-overwrite final copies, validated cleanup                                    |
| `archive_repository.rs`              | Parameterized archive/record/media persistence, duplicate lookup, paginated metadata filtering and summaries                     |
| `imports/service.rs`                 | Session lifecycle, preview/date choice, explicit confirmation, durable import journal, recovery                                  |
| `imports/commands.rs`                | Narrow async IPC and native file picker                                                                                          |
| `src/services/imports.ts`            | Typed ImportService/ArchiveService wrappers, browser rejection, safe error mapping                                               |
| `src/features/import/ImportChat.tsx` | Five-stage accessible workflow with real data and explicit save confirmation                                                     |
| `src/features/archives/`             | SQLite metadata library, filtering/pagination/summary, recent archive cards; no message viewer                                   |

### IPC contract

- `select_import_file()` → null on picker cancellation or SelectedImport. Only the Rust picker can supply the source file.
- `analyze_import(token, dateOrder)` → counts, participants (first 20), diagnostics, date-order requirement, encoding, and at most five real message excerpts (2,000 characters each). Reanalysis uses the staged export.
- `confirm_import(token, title)` → persisted ArchiveSummary. Requires a valid analyzed session and resolved ambiguous dates.
- `cancel_import(token)` → releases that session's staging. It does not change the source or remove saved archives.
- `list_archives(query, offset, limit)` → parameterized literal substring filtering of title/original filename, total count, and up to 100 metadata rows per page.
- `get_archive_summary(id)` → metadata and diagnostics only. It does not expose a full conversation viewer.

The existing four health/storage commands remain. The main-window capability grants only these ten commands. Archive counts and usage refresh through StorageProvider after confirmation; the archive hooks reload with the real count or route mount. Source/analysis errors are visible, cancellation has no success claim, and a frontend request guard prevents conflicting operations. Leaving Import Chat cancels any outstanding preview. A save already explicitly confirmed can finish; the UI refreshes authoritative storage state even if its response fails.

### Parser representation

The parser retains message bodies, normalized LF multiline text, original records, original timestamp representations, and nullable local ISO timestamps. It supports UTF-8 (optional BOM), BOM-marked UTF-16 LE/BE, day/month or month/day numeric dates with explicit ambiguity handling, dot/slash/dash separators, year-first dates, optional seconds, and AM/PM. Two-digit years use the documented 2000–2099 policy and emit a warning. Unsupported records/invalid times are retained with diagnostics; a file with no compatible message headers is rejected. Sender/system classification is intentionally conservative and language-dependent. Media reference matching recognizes common Android `(file attached)`, iOS `<attached: …>`, and selected omitted-media phrases.

Chat times have no timezone in these exports. `messages.timestamp` remains NULL. `original_timestamp`, `local_timestamp`, and `timezone_status='unknown'` preserve what is known. Import/creation timestamps remain real UTC epoch milliseconds. Preview samples are plain text, not HTML.

### Schema migration 2

`002_chat_import.sql` runs within the existing identity/ledger migration transaction and preserves Step 02 rows. Archives gain original filename, source format, original SHA-256, import status, counts, total bytes, encoding, and date order. A partial unique hash index prevents identical-file imports, including cross-process confirmation races. Existing records retain `legacy` status rather than being mislabeled as successful new imports.

Conversations gain participants JSON. Messages gain original/local timestamps, timezone uncertainty, classification, original record, and stable sequence. Attachments are transactionally rebuilt while preserving old rows to support archive-local paths, nullable paths for missing references, original names, and links to `archive_media`. New `archive_media` records retain every supported extracted file, even unmatched files. `import_diagnostics` stores aggregated codes, descriptions, counts, and first line numbers. `import_jobs` records durable incomplete-save intent without creating a visible archive. No media binaries are stored in SQLite.

### File/database commit and recovery

1. Acquire an installation-wide OS file lock in the app-local database directory. It remains held by an active session; vault switching also acquires it. This prevents another process from cleaning an active import or changing its vault.
2. Create an exclusively owned, leased staging directory under vault/temp. Copy/hash the original, validate it, extract bounded files into payload/media with generated names, and stage candidate text under work/. No user paths become extraction paths.
3. Analyze and preview in memory, bounded by the text/record limits. No archive/conversation/message rows exist yet.
4. On explicit confirmation, commit an import_jobs intent, then reserve `archives/<new UUID>` using exclusive directory creation. Record installation/operation ownership. Copy the payload with create_new files and sync writes; existing files cannot be overwritten.
5. Begin an immediate SQLite transaction, recheck duplicate hash and configured vault, insert all archive/conversation/message/media/attachment/diagnostic rows, and remove the job. Commit once. The archive becomes visible only with the complete transaction.
6. On failure, roll back rows and clean only the new, identity-validated directory. If cleanup cannot be validated/performed, preserve the journal/files for safe recovery. The original source and existing archives remain intact. Normal staging is released when its session drops.
7. Startup/new selection checks abandoned stages and pending jobs under the same coordinator lock. Active leases are skipped. Recovery requires matching installation/version/UUID, safe ancestors/entries, and no existing archive row for that operation. Unknown/corrupt ownership is preserved and blocks further import with a recovery error. A crash can leave disposable owned staging; it cannot expose half a conversation or silently delete an existing archive.

Sessions expire after 30 minutes and are checked/released on subsequent operations. Limits live in ImportLimits. Standard defaults: 256 MiB original ZIP, 16 MiB text, 64 MiB entry, 512 MiB expansion, 5,000 entries, 200:1 ratio, 32 text candidates, 100,000 records and media references. EOCD metadata is bounded before ZipArchive allocation; ZIP64/split/encrypted/unsupported compression is rejected. File sizes stream; text and parsed records are held in bounded memory. Supported media types are opaque stored attachments with extension-based MIME hints.

## Preserved Step 02 storage baseline

The sections below describe the initial schema/storage foundation. Migration 2 and the import commands above extend that baseline; schema version 2 is current.

## Responsibilities

React owns presentation, routing, accessibility, and the existing localStorage appearance key. Rust owns SQLite, migration state, vault identity/configuration, path validation, file metadata, and all filesystem writes. Content access remains behind service boundaries. No SQL, generic file operation, or frontend-provided path is exposed over IPC.

| Location                                    | Responsibility                                                                        |
| ------------------------------------------- | ------------------------------------------------------------------------------------- |
| `src/app/`                                  | Existing shell/navigation; mounts the shared StorageProvider                          |
| `src/features/dashboard/`                   | Real database counts, vault bytes/path/status, explicit loading/error states          |
| `src/features/settings/StorageSettings.tsx` | Storage & Vault controls and feedback inside the existing Settings design             |
| `src/features/storage/`                     | Shared state, serialized requests, safe feedback; synchronizes routes                 |
| `src/services/storage.ts`                   | Typed storage contract and sole storage invoke boundary; browser rejection            |
| `src/services/contracts.ts`                 | Import/archive interface exports and reserved device/backup interfaces                |
| `src/domain/models.ts`                      | UI-facing domain types, including nullable sender and timestamp uncertainty           |
| `src/platform/backend.ts`                   | Existing side-effect-free health command wrapper                                      |
| `src-tauri/src/storage/database.rs`         | Safe database opening, foreign keys, timeout, WAL, durability, quick check            |
| `src-tauri/src/storage/migrations.rs`       | Identity/version validation, transactional migration ledger and DDL                   |
| `src-tauri/migrations/001_initial.sql`      | Initial strict relational schema and indexes                                          |
| `src-tauri/src/storage/repository.rs`       | Parameterized settings queries/upserts and real aggregate counts                      |
| `src-tauri/src/storage/paths.rs`            | Local path/ancestor validation, protected roots, file/link checks, write probes       |
| `src-tauri/src/storage/vault.rs`            | Installation marker validation and controlled directory initialization                |
| `src-tauri/src/storage/usage.rs`            | Bounded iterative metadata scan; no chat content parsing                              |
| `src-tauri/src/storage/service.rs`          | Vault configuration, initialization, relocation gates, consistent overview            |
| `src-tauri/src/storage/mod.rs`              | Managed runtime state, initialization failure retention, serialized connection access |
| `src-tauri/src/storage/commands.rs`         | Narrow async IPC, worker tasks, Rust-only native folder picker                        |
| `src-tauri/src/storage/tests.rs`            | Temporary databases/directories only; never real app-local storage                    |
| `src/test/`                                 | Existing regressions plus storage integration tests with explicit mocked IPC fixtures |
| `scripts/check-schema.mjs`                  | Auxiliary in-memory execution of real SQL using Node SQLite                           |

## Startup and concurrency

Tauri resolves application-local data via its path API and manages a StorageState before the UI requests storage. Startup schedules initialization on a blocking worker. Each async storage command also runs its blocking work on Tauri's worker pool and accesses the same mutex-protected service. rusqlite's connection is never shared without that mutex. The event/UI thread does not perform database scans, filesystem validation, or wait on the folder picker.

Initialization creates only the app-local database directory and an initial default vault. Existing databases are reopened, not reset. Startup failures are retained as safe typed errors while the app UI and health command remain usable. Explicit initialize/retry can reattempt failed database initialization. A healthy database with an unavailable vault can still report counts and the saved path.

SQLite immediate transactions serialize migrations, initial configuration, and location changes across processes. Settings are re-read from SQLite for each overview/change. Selection is revalidated after the picker; a pre-picker empty check is only an early user-facing gate. SQLite commit is the authoritative location switch. The connection uses a five-second busy timeout, foreign keys, WAL, FULL synchronous durability, trusted_schema=OFF, and quick_check.

Frontend StorageProvider prevents conflicting requests, disables controls while busy, retains cancellation without a success message, and shares successful results across screens. After a failed mutation response it re-reads backend state, since a commit may have succeeded before a response failure. Failed reads show unknown/unavailable values rather than fake zeros. No vault preference is saved to browser localStorage.

## Database location and migration protocol

`<app_local_data_dir>/database/chatvault.sqlite3` is independent of the current vault. Windows normally resolves the base under `%LOCALAPPDATA%\app.chatvault.desktop`. The application does not store databases in the repository or `public/`.

`PRAGMA application_id = 0x43565431` identifies ChatVault. `PRAGMA user_version` and `schema_migrations(version, applied_at)` both track the current version 2. All pending embedded migrations, ledger entries, and version/identity changes share one immediate transaction. Failure drops/rolls back that transaction. Reapplying a completed version preserves rows. Unknown, inconsistent, and newer schemas fail safely. A nonempty unrecognized SQLite database is never adopted. Orphan WAL/SHM/journal files without the main database cause an error rather than replacement creation.

Application creation/import timestamps are nonnegative UTC Unix epoch **milliseconds** in INTEGER columns. SQLite supplies initial timestamps with `unixepoch('subsec')`. Imported chat timestamps retain their original string and nullable local ISO value with timezone uncertainty; their UTC INTEGER timestamp remains NULL.

| Table               | Core fields and constraints                                                                                                                |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `app_settings`      | Nonempty text key PK, text value, UTC updated_at; real installation_id, vault_path, vault_initialized settings                             |
| `devices`           | Text ID PK, display_name, Android platform check, nullable last_connected_at, created_at                                                   |
| `backup_snapshots`  | Text ID PK, nullable device FK with SET NULL, constrained status, relative backups/ path, nonnegative total_bytes, created/completed times |
| `archives`          | Text ID PK, nonempty title, chat-export source type, relative archives/ path, created_at                                                   |
| `conversations`     | Text ID PK, archive FK with CASCADE, title, created_at                                                                                     |
| `messages`          | Text ID PK, conversation FK with CASCADE, nullable sender, text, nullable UTC timestamp                                                    |
| `attachments`       | Text ID PK, message FK with CASCADE, nullable media_type/file_size, relative media/ path                                                   |
| `schema_migrations` | Version PK and UTC application time; managed internally                                                                                    |

Strict tables enforce storage types. Foreign key and chronological indexes cover device snapshots, archive creation order, conversations per archive, messages per conversation/time/ID, and attachments per message. SQL path checks reject traversal markers, backslashes, colons, and paths outside the relevant managed prefix. Future content writes must additionally validate canonical paths in Rust; schema checks alone are not file-access authorization.

Only the confirmed import service writes archives, conversations, messages, media, and attachments. Device and backup creation endpoints remain future work. The table above describes migration 1; migration 2's additions are detailed earlier.

## Vault protocol

Default root: `<app_local_data_dir>/ChatVault/vault`. It contains backups/, archives/, media/, temp/, and a small `.chatvault-vault.json` with format version 1 and the database installation UUID. The marker is not an encryption key or a cryptographic integrity guarantee.

New roots must be empty. Marker creation is exclusive and synced; existing markers are never overwritten. A known empty vault can be reused only after matching its identity/version and validating contents. Foreign/corrupt markers and occupied unknown folders fail without adoption or cleanup. Current roots containing any content or database snapshot/archive references block relocation. Unexpected or nested empty directories also conservatively block relocation. Overlapping roots are rejected. No content migration, copying, or removal occurs.

Missing saved roots are never recreated on startup or retry. An existing validated root can have missing managed subdirectories repaired by explicit retry. Unavailable drives retain their saved configuration. A failed new-root preparation/commit can leave harmless owned folders/marker; no cleanup tries to guess which folders belong to the user.

## Storage usage and limits

The worker scans only the current validated vault, summing regular file lengths without reading chat/media contents. The marker is excluded; the database and sidecars live outside the vault. Temporary vault files count. Links, hard links, junctions, reparse points, unreadable entries, and special files cause an unavailable result rather than traversal or a partial total. Scans stop at 100,000 entries or depth greater than 64. Sizes are logical lengths, not allocated blocks or free disk capacity. Concurrent external changes can cause a retryable error or a point-in-time approximation.

## IPC

| Command                  | Arguments | Result                                                                                                    |
| ------------------------ | --------- | --------------------------------------------------------------------------------------------------------- |
| `health_check`           | None      | Existing backend status/version/platform                                                                  |
| `get_storage_overview`   | None      | Database status/path/schema, saved vault path/status/error, snapshot/archive counts, nullable vault bytes |
| `initialize_vault`       | None      | Retry initialization/validate or repair the configured vault; same overview                               |
| `choose_vault_directory` | None      | Rust native folder picker followed by validated empty-vault switch; overview or null on cancel            |

Errors serialize as `{ code, message }` with safe static messages. Counts/path/settings/statistics are returned together rather than duplicating multiple IPC reads across components. Generic SQL/filesystem/dialog permissions remain ungranted. These four baseline commands and the six import/library commands above are permitted for the main local window.
