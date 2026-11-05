# Architecture - Step 07

## Step 07 native reader

Rust storage/native separates inspector, crypto, schema parser, privacy ACLs, service state and six typed IPC commands: inspect_native_backup, verify_native_key, verify_native_key_file, import_native_backup, get_native_reader_progress and cancel_native_reader. No command accepts a frontend filesystem path. The key-file dialog consumes its path in Rust. NativeState serializes work, owns cancellation/progress and a short-lived plaintext preview; the installation coordinator prevents conflicting import/backup/vault mutations. Heavy work runs on blocking workers; progress polling uses a separate mutex.

Inspection binds the snapshot UUID to the current vault/installation marker, validates manifest schema and its complete database inventory, stats snapshot files without reading media contents, hashes candidates, and identifies supported headers/footer envelopes. No phone access or key attempt occurs. Ciphertext is hashed again before decryption. GCM authentication precedes any plaintext output; a restricted owned stage holds the bounded decompressed SQLite. Source opens read-only/query-only with integrity/schema checks and cancellation hooks. Keys are consumed and zeroized where practical before a preview is retained. Compatibility and supply-chain evidence are in NATIVE_COMPATIBILITY.md.

The parser fingerprints ordinary legacy messages or modern message/chat/jid tables and streams ordered IDs/rows. Native identifiers/direction/UTC milliseconds/types and unique reply references augment the existing messages. Raw JIDs/LIDs stay distinct and contact names are never guessed. Unknown content is retained as stored text with diagnostics. Verified application-relative path, size and hash evidence associates snapshot media; missing/ambiguous references remain unresolved.

Migration 6 adds native provenance/parser/diagnostic columns to archives, native message semantics and native_media_links. Migration 7 adds nullable self-reply references/indexing. Original tables/FKs and TXT/ZIP rows are retained. The old source_type constraint stays as a legacy discriminator; native_format/native_source_app/native_snapshot_id are authoritative provenance, and archive summaries prefer native_format. A partial unique index on snapshot/parser rejects duplicates; a newer parser permits a separate explicit revision.

After the second consent, the existing durable import_jobs intent precedes creation of an owned archive directory. A transaction saves conversations/messages/associations/search and deletes that intent, then commits. Errors roll back records and clean only that owned output. Existing recovery reconciles abandoned intents and stages. The archive directory needs only its ownership marker; media remains in the snapshot through native_media_links. The existing viewer resolver checks both archive and snapshot ownership/inventory/hash and retains all token/range/signature/size/origin safeguards. No generic backup directory is exposed as a media URL.

The React inspector lives in history details. Separate consent gates temporary processing and persistent plaintext; key entry is cleared on submission. The reader receives native direction/UTC timestamps/unique reply IDs, using the same sidebar, cursor pages, search navigation and media components. Storage counts refresh after committed import. Successful synthetic imports prove this path; actual encrypted Business SQLite schemas/messages remain unvalidated without the matching key.

## Step 06.2 discovery fixes

Preflight uses a dedicated owned worker and a bounded monitor. ScanTrace tracks operation start/return, object/folder/file observations, inventoried entries/known bytes, errors, last successful code-defined operation and elapsed/pending-call duration. The monitor publishes real progress every 500 ms, even when the worker is waiting in a vendor call; snapshot polling refreshes elapsed/pending state after the response deadline. Normal transfer events keep their prior throttle.

ScanOptions are validated in Rust: one-source category subset, 1-100,000 objects, 1-50,000 files, 0-16 category depth, 1-180 seconds (default 60). Scoped results explicitly identify scannedCategories. Fast folder lookup stops enumeration after the matching directory instead of fetching every sibling. Enumeration requests size/name/type together and streams usable metadata into the planner, avoiding a second GetValues per file. Returned files survive a later enumeration error. Duplicate object IDs/cycles, limits and errors never yield a falsely complete inventory. Single-category scans do not walk unselected media libraries.

The response monitor permits one second of grace after the time budget for cooperative completion. A call still pending after that returns driver_pending with phase scan_waiting, requests cancellation and keeps the worker/COM/device/coordinator alive. Conflicting jobs remain blocked. When Windows returns, abandoned results are discarded and scan_failed is published; retry is safe only after resource release. Cancellation returning normally publishes scan_cancelled and discards the plan. Operation UUIDs plus monotonically increasing revisions prevent delayed scan/event responses from replacing terminal or later jobs. No SQLite snapshot/job row is created by discovery.

[IPortableDevice::Cancel](https://learn.microsoft.com/en-us/windows/win32/api/portabledeviceapi/nf-portabledeviceapi-iportabledevice-cancel) and [enumerator Cancel](https://learn.microsoft.com/en-us/windows/win32/api/portabledeviceapi/nf-portabledeviceapi-ienumportabledeviceobjectids-cancel) were reviewed. The latter cancels all pending session I/O rather than one selected operation. This implementation intentionally retains cooperative checks between calls and does not share a live worker-owned COM interface with an uninitialized control thread or imply every vendor call is interruptible. COM initialization, use, Close/release and uninitialization remain worker-local.

The UI separates preflight observations from copy/verification counters, exposes scopes/limits, explains scans longer than ten seconds and pending cancellation, animates indeterminate work without a percentage, and ignores stale publication revisions. Scan diagnostics are sanitized; original source metadata is never included in the log payload.

## Step 06 backup engine

The React six-stage wizard and history use `services/backups.ts` and an app-level `BackupProvider`. Main-window IPC is limited to `prepare_backup`, `get_backup_plan_files`, `start_backup`, `get_backup_progress`, `cancel_backup`, `list_backup_snapshots`, `get_backup_files`, `reverify_backup`, `cleanup_backup_staging` and `open_backup_location`. Event listen/unlisten is scoped to the local application capability. No generic filesystem, SQL, shell or device-object endpoint exists. Device observations refresh only on visible dashboard/device/backup routes; job-progress polling reads memory, not the phone.

Rust `storage/backups` separates models, planner, transfer, safe files/manifests, repository, coordinator and commands. The planner traverses only actual category folders under one selected package's modern/legacy candidates. It records known bytes and unknown sizes, observed unsupported skips and restriction counts, never guesses counts inside inaccessible subtrees, and retains original names as local metadata. Only encrypted-looking .cryptNNN database files are selected from Databases; discovered media folders are opaque binary files. Issued plan/file UUIDs authorize a subset; WPD object IDs remain memory-only. Collision-safe UUID-prefixed paths preserve hierarchy and original names remain in metadata. Bounds and capacity behavior are listed in README.

### Native read pipeline

The same worker that opens GENERIC_READ WPD content calls `IPortableDeviceContent::Transfer`, then `IPortableDeviceResources::GetStream` with `WPD_RESOURCE_DEFAULT` and `STGM_READ`. `IStream::Read` is looped until EOF, honoring short successful reads; the suggested buffer is clamped to 16-512 KiB. The stream, content and COM apartment are released on that worker. No COM interfaces cross threads. No Explorer automation or phone path-as-object-ID conversion exists. Official references: [Microsoft content transfer](https://learn.microsoft.com/en-us/windows/win32/wpd_sdk/transferring-content-from-the-device-to-a-pc), [GetStream](https://learn.microsoft.com/en-us/windows/win32/api/portabledeviceapi/nf-portabledeviceapi-iportabledeviceresources-getstream).

Each incoming stream is hashed while written to an exclusively created .partial-UUID file. The file is flushed/synced, reopened through ordinary-file/path validation, and independently SHA-256 hashed. Reliable reported size must match. Source name/size metadata is checked before/after. Optional source rereading opens another stream and compares its SHA-256 and byte count. Only successful files are renamed to final relative names. Same-size live mutations may escape metadata checks; even matching second reads do not establish atomic consistency across files or WhatsApp-native restorability.

### Publication and recovery protocol

An installation-level OS lease shared with imports/vault changes prevents conflicting operations across processes. DeviceState reserves one device worker; backup cancellation has its own token so a discovery page unmount cannot cancel a backup. SQLite locks are short-lived and never span a WPD read or file hash.

1. Persist an in-progress snapshot, job and selected file rows in a transaction before copying.
2. Exclusively create temp/backup-UUID with a snapshot/installation marker. Copy and verify files there. Persist per-file records and job progress; chunk summaries are best-effort every two seconds and phase/file boundaries require persistence.
3. Persist committing, sync and atomically rename a newly created manifest, validate marker/tree, and rename the owned directory into backups/UUID without replacement (MoveFileExW WRITE_THROUGH on Windows).
4. Only after filesystem promotion commit completed/completed_with_warnings in SQLite. A failed selected file prevents completion. Final stored byte counts sum verified file records; partial incoming bytes are progress, not completed storage.

Filesystem publication and SQLite commit are separate; no cross-system atomicity is claimed. Restart obtains the coordinator lease and marks unfinished jobs interrupted, including when the vault drive is absent. It never auto-completes from a manifest and never deletes or resumes files automatically. A crash after directory promotion but before database commit may leave a published directory associated with an interrupted row; it is preserved for manual inspection, excluded from successful counts, and not removed by staging cleanup. Retry always makes a new snapshot. Cleanup requires an incomplete database status, an exact temp/backup-UUID directory, matching installation/snapshot marker, and a bounded tree free of symlinks/reparse/hard links. Previous snapshots and unrelated files are retained.

### Persistence and manifests

Migration `005_local_backups.sql` extends the existing backup_snapshots table without rebuilding it: source/display name, phase, file/success/failure/skipped counts, verified bytes, manifest path, warning JSON, local integrity and last-check timestamp. New STRICT backup_jobs stores durable phase/progress/errors/staging relative path; backup_files stores source-relative logical path, safe local path, original filename, expected/actual bytes, SHA-256 and separate local/source/size states. The legacy device FK stays nullable: an unknown portable device is not coerced into an Android identity, and no hardware identifier or automatic friendly-name association is persisted. Schema version/ledger advance transactionally to 5.

Manifest schema 1 records snapshotId, createdAt, source, optional deviceDisplayName, null deviceAssociation, backupMode, phase, inventoryComplete, skippedEntries, inaccessibleLocations, warnings, sourceConsistency, and files. Each file records ID/category, logical/local relative paths, original name, reported/actual bytes, SHA-256, localVerification, sourceVerification, sizeValidation, state and errorCode. Failed staging manifests include failed/pending selected records when writing is possible. Manifests contain no encryption key or parsed message content. Original filenames may themselves be sensitive. Manifests describe copy-time evidence; later local checks update SQLite states rather than rewriting immutable original evidence.

History paginates 20 snapshots and 50 files. Reverification validates ownership, manifest schema/snapshot ID and the complete database hash/path inventory before reading local bytes. It persists per-file hash outcomes and snapshot integrity; a crash while checking is marked verification_interrupted on restart. Corruption is excluded from completed counts/bytes. Dashboard counts are authoritative and history exposes current/last job state separately from imported chats. Explorer opens only a validated completed snapshot directory after an explicit action, never a copied attachment.

Progress events target only main, at most every 250 ms plus phase/file transitions. A one-second fallback polls current memory. Fields are actual phase/source/current name, verified-file counts, failed count, written bytes, known/unknown totals, average bytes/elapsed and local-read bytes. Unknown totals use indeterminate progress. Filenames appear in local UI/manifest only, not application logs. Cancellation is cooperative between chunks/Windows calls; driver calls cannot safely be killed. App exit requests cancellation; durable startup recovery handles abrupt termination.

## Windows USB discovery

`src-tauri/src/devices/` separates `DeviceAdapter` / `Content` (injectable read-only boundaries), `WindowsPortableDeviceAdapter`, discovery rules/budgets, live `DeviceState`, typed models/commands, and `repository` for consented notes. Frontend `services/devices.ts`, `DeviceProvider`, Devices page, and dashboard consume issued UUID handles; React never receives PnP identifiers or WPD object IDs. Production creates the real adapter only; deterministic synthetic adapters exist under Rust/React tests only.

`IPortableDeviceManager::{RefreshDeviceList,GetDevices,GetDeviceFriendlyName,GetDeviceManufacturer}` enumerate locally installed portable interfaces and available metadata. `CM_Locate_DevNodeW`, `CM_Get_Device_Interface_PropertyW(DEVPKEY_Device_InstanceId)`, `CM_Get_Device_IDW`, and `CM_Get_Parent` establish local USB ancestry without parsing an interface path as an instance ID. Unverified transports remain visible but cannot be opened. No remote WPD device is opened. A device-interface symbolic link must first resolve to its PnP instance; this was exercised with real hardware.

Each operation creates a worker with `CoInitializeEx(COINIT_MULTITHREADED)`. All COM creation/use/release and `CoUninitialize` stay on that thread; interfaces never cross threads or enter global cache. `CoTaskMemFree` releases every returned WPD string, including partial failure batches. `IPortableDeviceFTM` is opened with `WPD_CLIENT_DESIRED_ACCESS=GENERIC_READ` from the first attempt. `IPortableDeviceContent`, `IPortableDeviceProperties::GetValues`, explicit key collections, `EnumObjects`, and `IEnumPortableDeviceObjectIDs::Next` implement metadata-only access. Client property setters configure the in-memory client-information object only; no device property setter is used. These metadata discovery commands do not read content streams. Explicit Step 06 backup commands add read-only resource streams as described below; no device mutation API is called.

Only device type/protocol/transport, object name/original filename/content type, and functional storage category are queried. No serial, IMEI, account, battery, capacity, Android version, or private database properties are requested. A `Phone` WPD type is not Android evidence. Only an exact explicit Android protocol report establishes `driver_reported_android`; otherwise identity stays unknown. A successful storage-root enumeration probe establishes access at that moment, not access to every file or a permanent authorization.

Discovery walks only exact case-insensitive modern/legacy WhatsApp and Business candidate segments, then fixed media/database category names. It does not recursively walk arbitrary child objects or enumerate private app storage. An operation-local bounded directory cache reuses shared prefixes. Media-category access uses one `Next` probe without file-property reads; database directories return only an encrypted-filename presence boolean, never filenames or contents. Results distinguish found/accessible/not-found/access-denied/disconnected/unsupported/incomplete/cancelled. Duplicate matching folder names are treated as incomplete rather than arbitrarily choosing one.

Limits: 32 devices, 8 storage roots, 256 children/directory, 2,500 examined objects, depth limited by the fixed paths (at most 8 including the leaf probe), and 25 seconds between calls. Metadata display strings are control/bidi-filtered and bounded to 160 characters; internal IDs copied from COM are bounded to 4,096 UTF-16 units. Scan/cancel state is independent from StorageState so slow WPD never holds the SQLite/import lock. Exactly one native worker is allowed. The async command waits on a bounded channel off the event thread and reports timeout after 28 seconds plus scheduling overhead. Cooperative cancellation and a pre-commit budget check discard late work. Synchronous driver calls cannot safely be force-terminated: a hung call retains the single worker slot until return; further requests fail busy. No background retry storm is created.

The shared provider polls enumeration every 15 seconds while visible, suppresses overlaps during access/discovery, cancels on hide/unmount, discards stale frontend generations, and clears stale connection claims. Backend live observations expire after 30 seconds. Every explicit access re-enumerates before using its issued handle. A completed refresh resets permission to unchecked; dated findings are historical observations. Disconnected devices are removed, and reconnect/restart generates new handles. No persisted note creates a live connection.

### Device commands and SQLite

The existing 20 commands remain; eight narrow main-window permissions add `list_connected_devices`, `refresh_connected_devices`, `get_device_details`, `check_device_file_access`, `discover_whatsapp_storage`, `cancel_device_operation`, `get_known_devices`, and `remember_device`. Expensive commands are asynchronous; no arbitrary path, executable, SQL, or WPD write endpoint exists. The remember command requires the current issued handle and a boolean preference; the UI requires explicit consent.

Migration 4 adds a separate STRICT `portable_device_notes` table (random local ID, display name, optional category, last detected UTC milliseconds, preferred flag). The initial `devices` table enforces Android-only platform and already participates in backup foreign keys, so it is preserved rather than rebuilt to adopt unknown portable devices. Notes are bounded to 100 and are separate from live observations. No raw/hashed hardware recognition identifier is saved, and no automatic matching by friendly name is attempted. Preferences are historical notes only after reconnect/restart. Step 06 reuses these adapter boundaries with fresh explicit selection and read-only resource streams. Device discovery/copying does not decrypt; the separate Step 07 reader requires an explicitly provided matching key. ADB is intentionally omitted.

Official API semantics: [enumerating WPD devices](https://learn.microsoft.com/en-us/windows/win32/wpd_sdk/enumerating-devices), [read-only device Open](https://learn.microsoft.com/en-us/windows/win32/api/portabledeviceapi/nf-portabledeviceapi-iportabledevice-open), [object enumeration](https://learn.microsoft.com/en-us/windows/win32/wpd_sdk/enumerating-content), [device properties](https://learn.microsoft.com/en-us/windows/win32/wpd_sdk/device-properties), [CfgMgr32 interface-to-instance example](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/porting-from-setupapi-to-cfgmgr32), and [WPD cancellation semantics](https://learn.microsoft.com/en-us/windows/win32/api/portabledeviceapi/nf-portabledeviceapi-iportabledevice-cancel). The app uses cooperative cancellation between calls rather than sharing a COM interface across threads to force a cancel.

## Read-only chat reader

The reader at `/archives/read` extends Archives and import completion. React owns selection URLs, the current session's selected ID, presentation and bounded windows. Rust owns content reads, search and media access through the existing StorageState worker/mutex. No production seed/fallback messages exist. Dates use original or normalized local values without timezone invention; neutral bubbles never guess the account owner.

Frontend modules under `src/features/viewer/`: ChatViewerPage coordinates routing/details/gallery; ConversationSidebar filters/pages real conversations; MessageTimeline and MessageBubble render ordered pages, sender/system/deleted records and date separators; ChatSearch navigates scoped/global result pages; Highlight maps normalized grapheme matches onto original escaped text; ConversationInfoPanel loads participants/source/media metadata; MediaMessage/AudioPlayer load requested files; MediaLightbox uses a native modal dialog with zoom/fit/page-local navigation. `src/services/viewer.ts` is the typed IPC boundary.

### Pagination and large records

Migration 3 preserves imported sequence values and assigns legacy unsequenced records stable rowid order after existing sequences. An indexed cursor is an existing message ID verified against its conversation. Latest/older/newer reads return at most 60 records. Context returns at most 25 preceding records, the exact anchor and 25 following records. Messages are never ordered using guessed dates.

React requests 50 records, retains at most 120 and trims the opposite end when paging. It preserves the first visible retained record's position, jumps to newest, or focuses an exact search target. No entire chat/media array is loaded. Long bodies use 4,096-unit text sections with surrogate-boundary handling; all saved text stays readable through section controls. Highlights use bounded React text/mark nodes. See VALIDATION.md for measured 12,000-record results; no unmeasured FPS claim is made.

### Search index and migration 3

`003_chat_viewer.sql` backfills a full-content FTS5 trigram index of NFC/lowercase message text and sender. An innocuous deterministic Rust scalar is registered before migrations. The index is sensitive and unencrypted. Immutable new imports explicitly update it inside the same transaction as all message/attachment rows. An index failure rolls back the import; no unsafe virtual-table triggers or trusted_schema relaxation are used.

Queries of at least three Unicode characters become quoted literal phrase parameters, with embedded quotes escaped. FTS operators and SQL are never accepted from the UI. Trigrams support substrings and retain combining marks. Shorter queries use the same normalization with at most 2,000 candidate records and 50 matches per request. A continuation is exposed even for a zero-match scanned section, so incomplete scans are not mislabeled exhaustive. Queries are 1–200 characters. Results identify conversation/message IDs, contain original snippets and navigate through exact context lookup. See [SQLite's trigram documentation](https://www.sqlite.org/fts5.html#the_trigram_tokenizer) for the short-query boundary.

### Typed reader commands

Ten new main-window commands are added to the existing ten: `list_conversations`, `get_conversation`, `read_messages`, `get_message_context`, `search_messages`, `list_conversation_media`, `get_media_metadata`, `authorize_media`, `revoke_media`, `open_document`. They accept bounded filters/cursors/record IDs and return typed DTOs. Conversation details include participants; the existing archive summary command is reused. There is no generic SQL/path reader.

### Private media service

`storage/viewer/{models,repository,search,media,protocol,commands}.rs` resolve media ownership in SQLite. A committed archive/conversation, matching vault/archive owner markers, and exact `archives/<archive UUID>/media/<generated UUID>.<extension>` path are required. Directory ancestors are canonicalized/revalidated; file opens reject hard links/reparse points and changed sizes. Fixed extension/signature pairs select MIME. Images are capped at 16 MiB and known dimensions at 25 megapixels; JPEG dimensions require a bounded header scan. The existing 64 MiB entry limit bounds playable files.

Authorization returns a random, revocable in-memory token (30-minute lifetime, 128-grant bound), never a path. `chatvault-media` is an asynchronous Tauri custom protocol with no TCP listener. It accepts main-webview GET/HEAD only, rejects unexpected origins/query/path syntax, revalidates ownership for every request and reads the validated handle. Single byte ranges return exact 206 responses in at most 2 MiB chunks; invalid/multiple ranges return 416. Non-range reads are capped at 16 MiB. Responses include fixed MIME, nosniff, no-store and exact allowed-Origin CORS. CSP adds only the private scheme/host to image/media sources; no broad Tauri asset scope exists. The [asynchronous Tauri protocol API](https://docs.rs/tauri/latest/tauri/struct.Builder.html#method.register_asynchronous_uri_scheme_protocol) keeps file reads off the event thread.

PNG/JPEG/WebP/GIF use the WebView decoder. WAV/MP3/Ogg/Opus/AAC/M4A and MP4/WebM use native controls with codec/damage feedback and no transcoder. Documents do not become application HTML: supported PDF/office/text files open only through an explicit, revalidated Windows ShellExecuteW action with a literal verb and no command arguments. In-app PDF preview is deferred. Original imports remain unchanged.

## Preserved Step 03 importer

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

The existing four health/storage commands remain. These ten baseline commands remain; Step 04 adds the ten reader commands above. Archive counts and usage refresh through StorageProvider after confirmation; the archive hooks reload with the real count or route mount. Source/analysis errors are visible, cancellation has no success claim, and a frontend request guard prevents conflicting operations. Leaving Import Chat cancels any outstanding preview. A save already explicitly confirmed can finish; the UI refreshes authoritative storage state even if its response fails.

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

The sections below describe the initial schema/storage foundation. Migration 2 and the import commands above extend that baseline; schema version 7 is current.

## Responsibilities

React owns presentation, routing, accessibility, and the existing localStorage appearance key. Rust owns SQLite, migration state, vault identity/configuration, path validation, file metadata, and all filesystem writes. Content access remains behind service boundaries. No SQL, generic file operation, or frontend-provided path is exposed over IPC.

| Location                                    | Responsibility                                                                        |
| ------------------------------------------- | ------------------------------------------------------------------------------------- |
| `src/app/`                                  | Existing shell/navigation; mounts the shared StorageProvider                          |
| `src/features/dashboard/`                   | Real database counts, vault bytes/path/status, explicit loading/error states          |
| `src/features/settings/StorageSettings.tsx` | Storage & Vault controls and feedback inside the existing Settings design             |
| `src/features/storage/`                     | Shared state, serialized requests, safe feedback; synchronizes routes                 |
| `src/services/storage.ts`                   | Typed storage contract and sole storage invoke boundary; browser rejection            |
| `src/services/contracts.ts`                 | Import/archive interface exports and typed device/backup interfaces                   |
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

`PRAGMA application_id = 0x43565431` identifies ChatVault. `PRAGMA user_version` and `schema_migrations(version, applied_at)` both track the current version 7. All pending embedded migrations, ledger entries, and version/identity changes share one immediate transaction. Failure drops/rolls back that transaction. Reapplying a completed version preserves rows. Unknown, inconsistent, and newer schemas fail safely. A nonempty unrecognized SQLite database is never adopted. Orphan WAL/SHM/journal files without the main database cause an error rather than replacement creation.

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

Only the confirmed import service writes archives, conversations, messages, media, and attachments. Original Android device creation remains unused; Step 05 saves separate consented portable-device notes and Step 06 creates snapshots with optional device display metadata. The table above describes migration 1; migration 2's additions are detailed earlier.

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

Errors serialize as `{ code, message }` with safe static messages. Counts/path/settings/statistics are returned together rather than duplicating multiple IPC reads across components. Generic SQL/filesystem/dialog permissions remain ungranted. These four baseline commands, six import/library commands and ten reader commands are permitted for the main local window.
