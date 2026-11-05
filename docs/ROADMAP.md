# Roadmap

## Steps 01–02 — preserved foundation

The desktop shell, five routes, responsive sidebar, keyboard focus behavior, light/dark/system appearance, original branding, native health check, Rust-owned SQLite, transactional migration framework, managed vault settings, path safety, and storage statistics are preserved. Step 02's isolated tests remain in the suite.

## Phase 01, Step 03 — WhatsApp export importer

- Native .txt/.zip selection and content validation, strict UTF-8/BOM UTF-16 decoding, streaming original SHA-256, and controlled session tokens.
- Common Android/iOS numeric date/time headers, Unicode/Sinhala/multiline records, diagnostics, original timestamp retention, explicit ambiguous date choice, and timezone uncertainty.
- Bounded ZIP preflight/decompression, CRC checks, filename/link/bomb/collision protection, media mapping, missing references, and opaque storage of supported attachments.
- Real preview, editable title, explicit save, accurate indeterminate stage labels, cancellation, failures, duplicate detection, and completion after persistence.
- Schema migration 2 for archive provenance/counts/status, participants, message sequence/timestamps/classification/raw records, media metadata, missing attachments, diagnostics, and an import recovery journal.
- Atomic SQLite persistence and exclusively created, collision-safe archive storage. Journal/ownership/OS locks support crash recovery without adopting or deleting existing archives.
- Real paginated/filterable archive metadata library and summaries; dashboard counts/usage/recent archives refresh after imports.
- Synthetic fixtures and parser/ZIP/database/frontend regression coverage. No production seeded chats or simulated backend successes.

Current execution results and native smoke-test evidence are recorded in VALIDATION.md. Supported formats and limits are in README.md. App-provided encryption is not implemented.

## Step 04 — Local Chat Viewer, Media Viewer & Search

The read-only desktop reader opens actual imported conversations from Archives or import completion. It includes genuine last-message previews, sender labels, dates, multiline Unicode, a bounded 120-record timeline with cursor pages and search context, local literal message/sender search, safe highlights, collapsible participant/source/media information, an image lightbox, WebView audio/video controls, missing/unsupported feedback and explicit registered document opening. Migration 3 preserves old rows/order and backfills an NFC/lowercase FTS5 trigram index; new imports update it in their existing atomic transaction. Media uses expiring ID-authorized tokens and a narrow range-aware custom protocol. Exact tests, measurements and native evidence are in VALIDATION.md.

## Step 05 — Android USB detection and read-only discovery

Implemented real Windows WPD enumeration, verified local USB ancestry, friendly name/manufacturer metadata, conservative OS/category identity, explicit read-only access checks, bounded modern/legacy WhatsApp and Business folder discovery, typed status/diagnostics, cancellation/backpressure, live lifecycle-aware polling, opaque session handles, consented historical device notes, migration 4, Devices UI, and genuine dashboard status. Physical-phone and deterministic mock evidence are separated in VALIDATION.md. ADB is optional and intentionally omitted. No phone bytes are read or copied, and no backup snapshot is created.

## Step 06 - Local WhatsApp Backup Engine

Implemented explicit six-stage selection/preflight, separate WhatsApp/Business sources, actual category/individual file authorization, WPD GetStream/STGM_READ bounded transfers, SHA-256 incoming/local checks and optional independent source reread, durable jobs/files/snapshots/manifests, cancellation, conservative interrupted recovery, owned staging cleanup, history/details/reverification/validated Explorer opening, dashboard integration and migration 5. Existing import/viewer/search/media/device/storage tests remain in the suite.

Step 06.2 implements bounded scoped preflight, operation timing/progressive counters, incomplete results, slow/pending states, Cancel Scan, safe retry and stale-event guards. Real scoped Business discovery and one 29,816-byte image transfer with independent source/local SHA-256 and persisted history passed. Larger hardware and native wizard interaction checks remain pending; exact evidence is in VALIDATION.md.

Step 06 remains the read-only copy engine. Native decryption is a separate Step 07 workflow; restore, auto-resume, incremental scheduling, cloud and retention/deletion remain outside scope.

## Step 07 - Native backup reader and integration

Implemented owned snapshot inspection, bounded crypt14/crypt15 standalone-envelope authentication using only matching user-provided keys, restricted plaintext staging, read-only SQLite/schema validation, legacy/modern synthetic schema parsing, explicit plaintext persistence consent, versioned provenance/deduplication, transaction/recovery integration, verified snapshot media references and existing reader/search integration. Migrations 6-7 preserve existing data. Independent Node/OpenSSL fixture tests and Rust/frontend regression evidence are in VALIDATION.md.

The real Business snapshot contains eight header/hash-verified crypt14 candidates, not readable chats yet. Real decryption/parser validation remains PENDING because no matching local key file was provided. No root extraction, key guessing or cloud password assumption is made. Step 08 has not started.

## Later boundaries

Overlap merging, explicit identical-file reimport, vault migration, encryption/key management, retention/deletion, recovery tooling, and release signing require separate work. Cloud services, authentication, analytics, WhatsApp Web automation, root access, private messaging APIs, and encryption bypass remain outside scope.
