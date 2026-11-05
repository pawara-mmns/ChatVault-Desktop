# Security and privacy - Step 07

## Native reader and plaintext boundary

Step 07 decrypts only a selected, owned, hash-verified standalone supported envelope after an explicitly supplied compatible key and processing consent. AES-GCM authentication is mandatory; no force mode, key extraction, brute force, lock bypass, root or network service exists. Unknown formats/password/passkey modes remain encrypted. Research/licensing/test scope is documented in NATIVE_COMPATIBILITY.md. No upstream decryptor binary or script is executed.

Keys and derived material use zeroizing Rust buffers and zeroizing cipher support where practical, with no persistence or logging. Key files are selected/consumed only in Rust. JavaScript clears the password field on submission and uses no local/session storage for keys, but IPC/JavaScript copies and every memory allocation cannot be guaranteed erased.

Authenticated plaintext is bounded and decompressed into exclusively created owned staging. Windows sets protected current-user/SYSTEM inheritable DACLs before plaintext files; Unix uses 0700 directories. SQLite opens read-only/query-only with trusted_schema disabled, ordinary-table/schema/integrity checks, field/row/time limits and cancellation hooks. Original ciphertext is never edited. Owned stage leases/recovery support cancellation/abandoned-stage cleanup without deleting other files; unavailable drives or unsafe ownership are preserved for recovery. This is not secure SSD erasure or protection against privileged/same-user software.

A second explicit consent is required before readable rows, FTS and journals are committed to unencrypted ChatVault storage. No failed or empty parse creates a readable archive. Media associations require scoped path/size/hash evidence and revalidation; filename-only guesses and ambiguous links remain missing. Existing renderer/origin/token/signature/range restrictions still apply. No generic filesystem/key-path IPC or broad permission was introduced.

## Backup trust and integrity boundary

Step 06 adds read-only WPD resource streams only after explicit Backup Now confirmation. Native database/media files are copied as opaque bytes; encrypted .crypt backups stay encrypted. The Step 06 copy engine performs no decryption, phone mutation, restore, scheduling, cloud transfer or automatic attachment execution; the separate Step 07 reader requires matching-key/consent gates. The vault, manifests, SQLite and journals are unencrypted and may expose filenames and backup metadata.

The backend owns device/plan/file UUID authorization, fresh USB transport checks, configured vault paths, disk-capacity checks, generated snapshot destinations and ownership markers. Frontend cannot submit arbitrary WPD IDs or filesystem paths. Incoming SHA-256 must match an independently synced/local-read hash and reliable expected size before promotion. Optional source re-reading is distinct from local write verification. Hashes/markers are neither signatures nor authentication against a hostile same-user process; live files are not atomic snapshots.

All file creation is exclusive. Safe relative components reject traversal, Windows reserved names/streams, control characters and links. Original untrusted filenames render as escaped text. Before publish/cleanup the owned tree is checked for ordinary nonlinked files. Cleanup targets only a matching incomplete job's staging directory; previous snapshots are never auto-deleted. Interrupted commits are never inferred successful from filesystem presence. Vault relocation is blocked by any snapshot record, including failed/empty records. Local verification checks the manifest against authoritative stored hashes/paths before reading; corruption removes a snapshot from successful counts.

Ten narrow backup commands plus local event listen/unlisten augment the prior capabilities. Folder opening uses a fixed Windows open verb on an authorized snapshot directory, without command-line arguments or shell commands. No generic filesystem/shell/SQL permission was added. Current filename/progress events remain local to main; application logs contain no private source names, contents or hardware identifiers. Step 06.2 scan diagnostics contain only phase/operation constants, counters, durations and numeric HRESULT categories. The authorized small-file transfer logged only its coarse size and verification outcomes; contents, source names and hardware IDs were not displayed. Cancellation/timeout never releases live COM handles or bypasses the single-worker reservation. No schema or existing snapshot data is changed by preflight.

## Read-only device boundary

Windows WPD enumeration is real and local. Before opening any content interface, CfgMgr32 must establish local USB ancestry. Network/Bluetooth/unverified transports are never opened. Every open explicitly requests GENERIC_READ; no phone write, delete, move, format, decryption, root, debugging, lock-bypass, cloud, telemetry, or shell-command operation exists. ADB is not included or installed. No administrator elevation is requested by application device access.

Enumeration does not imply permission. Explicit access/discovery and backup preparation/copy actions open a selected live issued handle. Discovery authorizes only a bounded set of conventional folder/filename metadata checks. It never reads encrypted database or media bytes. Media category checks do not enumerate each file's metadata. Encrypted backup findings are boolean, not a filename list or chat recovery claim. Vendor restrictions and absent paths are distinct from proof that data is absent.

PnP/device/object identifiers are held only in operation/session memory, never returned to React, logged, or persisted. Serial, IMEI, account identity, OS version, battery and capacity are never queried. Windows error strings are not logged/exposed: diagnostics contain stable categories and numeric HRESULT only. Metadata strings are length/control/bidi-filtered; UI uses escaped React text. COM memory/interfaces/apartments have worker-local RAII cleanup. Bounded enumeration/caches/time budgets, single-worker backpressure, cooperative cancellation, and frontend request generations protect responsiveness and stale results. A synchronous hung vendor call cannot be safely killed; cancellation does not promise instant driver interruption.

Migration 4 adds only explicit-consent local device notes. Names can themselves be personal and remain unencrypted SQLite data; the consent text lists the saved fields. Device notes store no raw identifier, hashed fingerprint, object path, discovery filename or connected flag. Explicit backups separately persist logical file paths and original filenames, but never WPD object IDs. Historical notes cannot recognize a device after reconnect/restart. The original Android devices and all archive/backup records are preserved. Live observations expire and refresh independently from notes. No preference initiates any access or backup automatically.

Eight scoped app commands augment the existing 20 permissions. No shell/fs/SQL/WPD mutation capabilities or broad resource scopes are enabled. The custom media protocol remains limited to the local archive vault; a device handle cannot be used as a media/vault path. Offline code paths introduce no HTTP client or listener. The development server uses loopback only; packet capture and physical network isolation are separate validations rather than claimed guarantees against OS/vendor behavior.

## Read-only viewer and media boundary

Ten additional main-window reader commands return bounded conversation/message/search/media DTOs. No frontend filesystem scope, editable SQL, arbitrary path, message mutation, cloud service or automatic external link is added. Messages, filenames, snippets and sender names render as React text nodes; highlights preserve original escaped text. Account ownership is never guessed. Local chat dates keep Step 03 uncertainty. Only a conversation ID is retained in sessionStorage; message/media content is not cached there.

Search uses a local NFC/lowercase FTS5 trigram index, backfilled by preserving migration 3 and maintained in the import transaction. It duplicates sensitive text inside the unencrypted database/WAL. Search phrases are literal quoted parameters; malformed FTS syntax is not executed. Short queries have an explicit bounded-scan continuation rather than silently incomplete results. The read-only viewer supplies no edit/delete endpoints and trusted_schema remains OFF.

Media is requested by a persisted media ID. Rust verifies committed archive ownership, vault/archive markers, generated UUID path shape, canonical ancestors, ordinary nonlinked handles, stored size, fixed extension/signature MIME pairs and size/pixel limits. Links/junctions/reparse points/hard links, traversal, changed sizes and unsupported/active formats fail safely. File signatures are not a guarantee that a decoder supports a recording; WebView decode failures are shown. Media is never loaded from a message's external URL.

The private `chatvault-media` protocol has no network listener and accepts only the main webview, GET/HEAD, and expiring random tokens. Unexpected origins are rejected; Origin CORS is exact rather than wildcard. IDs and file ownership are revalidated for each byte-range request. At most 2 MiB per range, no multipart ranges, no oversized non-range body, no-store and nosniff headers, and bounded token grants constrain access. Tokens expire after 30 minutes and components revoke unused grants. CSP permits only this exact media scheme/host; broad HOME/all-files Tauri asset scopes remain disabled. Documents cannot be served as executable application content.

PDF/office/plain-text documents open only on an explicit Open document action. Rust revalidates the file then uses Windows' registered application with a literal open verb, no arbitrary command string and no arguments. HTML/SVG/scripts/executables are not authorized as application documents. In-app PDF preview and media transcoding are not provided. A registered external viewer has its own behavior and access policy; it is outside ChatVault's renderer sandbox.

Images, recordings and originals remain unencrypted local files. The application cannot defend against an administrator or a malicious same-user process racing filesystem changes; ancestor/handle checks are re-run and detectable changes are rejected. No chat/media bytes, tokens or contact details are logged. No request uploads data to a service.

## Import trust boundary

Only a native Rust file picker authorizes a source read. IPC accepts session tokens, date order, titles, and bounded library filters/IDs; it has no arbitrary path reader, SQL endpoint, or frontend filesystem grant. The selected file is copied into controlled vault staging, hashed while streaming, and inspected for supported contents. Source files are never modified or removed. Parser bodies and personal identifiers are never logged. Only explicitly requested preview excerpts and library metadata cross IPC, and React renders all imported strings as plain text.

Every ZIP entry is untrusted, including files that will not be extracted. The importer rejects traversal/absolute/drive/UNC paths, backslashes, control characters, Windows reserved names/streams, special files/symlinks, Unicode/case-normalized name collisions, encrypted/split/ZIP64 archives, and unsupported compression. Untrusted names are metadata, never output paths. Files use generated UUID names and create_new writes. EOCD entry/metadata counts are checked before allocating a ZipArchive. Declared and actual expanded sizes, per-file/text/entry/count/ratio bounds, and CRC reads limit bombs and corruption. Unsupported active files are not extracted; the original ZIP is stored opaquely and never executed.

Staging and all extracted files remain inside the configured local vault. Stages have random identities and OS leases. A separate installation-wide OS coordinator prevents concurrent import/recovery/vault-change races. Explicit confirmation is required before structured persistence. Duplicate SHA-256 hashes have both preview warnings and a database uniqueness guard; overlap merging/reimport is not implemented. Diagnostics preserve unknown records and missing media rather than silently losing messages.

A durable import journal precedes final directory creation. SQLite commits every related row atomically. Failure/recovery cleanup is restricted to the new operation's validated ownership/UUID and refuses to delete anything referenced by an existing archive. Invalid ownership, links, permissions, or recovery state preserve files for inspection. Cleanup may leave owned artifacts when safety cannot be established; it never guesses ownership or resets the database. There is no archive deletion UI/IPC.

Chat timestamps remain timezone-unknown; ambiguous date order requires user selection. UTC conversion is not invented. The original bytes, original timestamp strings, local normalized values, chosen date order, hashes, media names, missing references, and diagnostics are retained. The read-only viewer preserves this metadata. Stored media types are extension hints and no media decoder/browser is invoked by importing.

**Imports, SQLite, device notes, originals, extracted media, and temporary staging are not encrypted by ChatVault.** Use a trusted local vault and consider OS disk encryption/access control. Export files contain sensitive plaintext; no content or hash is uploaded to a service. Phone discovery is limited to explicit read-only metadata access. Native-backup decryption is confined to the matching-key workflow described above; account access and scraping remain outside scope.

## Data and offline behavior

SQLite and the default vault live under Tauri's application-local data directory, normally `%LOCALAPPDATA%\app.chatvault.desktop` on Windows. The database is always `<app-local-data>/database/chatvault.sqlite3`; the vault root is a separate persisted setting. Browser localStorage holds only the existing appearance key. Application-local storage contains migration/configuration metadata and an initially empty relational content schema.

There is no cloud database, authentication server, analytics, telemetry, external API request, updater, or WhatsApp file modification. File processing and the read-only local USB adapter remain in Rust. Bundled assets and IPC work without an internet service; development installation/build tools can require downloads. Only explicitly chosen exports are imported. Phone discovery is metadata-only; backup creation remains disabled.

## Least privilege

The main local window receives the ten reader permissions documented above, the four health/storage permissions plus `allow-select-import-file`, `allow-analyze-import`, `allow-confirm-import`, `allow-cancel-import`, `allow-list-archives`, and `allow-get-archive-summary`. Application commands are registered with build-time ACL generation. The dialog plugin is used from Rust; frontend generic dialog permissions are not granted. No generic filesystem or SQL command is registered, and no arbitrary SQL/path endpoint exists. A picker choice is consumed directly by Rust, not returned as an editable arbitrary path to frontend code.

The app requests no elevation, runs no shell/subprocess for storage, and uses a current-user installer. CSP remains restricted to bundled assets and IPC (plus loopback HMR in development). The frontend cannot use a storage operation successfully in a normal browser and clearly reports the missing native runtime.

## Database protection

- Bundled SQLite, foreign keys, five-second busy timeout, WAL, FULL synchronous mode, and trusted_schema=OFF.
- Versioned embedded SQL, a migration ledger, application identity, and schema version checks.
- Immediate transactions cover pending migrations and metadata updates. Failed migrations roll back; there is no destructive reset path.
- Unknown/nonempty foreign databases, newer schemas, inconsistent migration history, invalid configuration, and orphaned sidecars fail safely.
- Existing database and WAL/SHM/journal paths must be ordinary nonlinked files; symbolic links, junctions, and hard links are rejected.
- Repository operations use parameters. Dynamic SQL is limited to reviewed embedded schema/constant queries; no user SQL endpoint exists.

## Vault path and write safety

Rust requires absolute local paths and checks ancestors before and after creation. Windows device/UNC/network paths, disconnected/nonlocal volumes, reserved names/streams, control characters, traversal components, symbolic links, junctions, mount reparse points, and cloud placeholders are rejected. Existing ancestors are canonicalized before writes to account for aliases. System/program directories, known OneDrive roots, the database, executable/source directories, and parent/child overlap with the current vault are protected. These checks intentionally favor a narrower supported folder set over silently adopting unusual storage.

Directories are created for storage initialization or an explicitly selected/confirmed import. Write probes use random exclusive tempfile creation, write/sync, and removal of only the owned probe. Marker creation uses exclusive creation and sync. Existing user files/markers are never overwritten. Internal import cancellation/recovery can recursively remove only validated owned staging/uncommitted operation directories. There is no generic content deletion, database reset, or vault migration cleanup command.

The marker records only installation identity and format. Matching a marker is necessary but insufficient for destination adoption: the selected destination must also be empty of content and validated. Foreign/corrupt markers and unknown nonempty folders are rejected. An existing vault with files or archive/snapshot records blocks automatic relocation. Old empty directories are retained. Failed preparation/commit can leave empty initialized folders, which are not automatically removed.

Saved missing roots are preserved as unavailable, never silently recreated or replaced. The user must reconnect the original drive before switching. Explicit retry can repair missing managed child directories only inside an existing validated root. No files are moved, copied, deleted, or imported during a vault change.

## Statistics

Only regular files within the managed root contribute logical lengths. The identity marker is excluded and the database is outside the root. Hard links and symbolic/reparse links are rejected so they cannot alias outside content. No chat or media content is parsed to calculate usage. Unreadable paths, scan limits, and missing drives produce unavailable/error states rather than partial totals or false zero usage. Scans run on workers and are bounded to 100,000 entries and depth 64.

## Current limits

**Neither the vault nor SQLite is encrypted by ChatVault.** WAL/SHM/journal files may also contain database information. The marker is not authentication, a signature, or a backup secret. Native-reader keys are ephemeral inputs, not managed or recoverable secrets. Secure deletion, vault migration and retention remain outside scope.

The app does not isolate storage from other software running as the same user or an administrator. OS permissions and disk encryption matter. Path checks cannot provide a complete security boundary against a hostile local process racing filesystem mutations; an error is reported when detectable changes occur. Keep storage in a trusted, non-synchronized local directory. Known OneDrive paths are rejected, but the app cannot discover/control every third-party sync program or manual copy. Usage totals can change while outside software writes files.

Frontend success is shown only after a native response. On a mutation-response failure the frontend rereads authoritative SQLite-backed settings, avoiding a stale-location claim. Safe error codes/messages exclude arbitrary OS/SQL diagnostics. Application code never logs chat bodies, sensitive files, keys, or backup secrets.

## Future content boundaries

Future parser/viewer extensions must preserve these input/path/size/date/SQL boundaries and validate media before decoding it. Never treat stored relative paths as unrestricted authorization. Android exposes only accessible files; copying encrypted native backups does not decrypt their chats. Rooting, encryption bypass, private messaging APIs, scraping WhatsApp Web, and complete-history recovery claims remain outside scope.

ChatVault Desktop is independent and unofficial and is not affiliated with WhatsApp or Meta.
