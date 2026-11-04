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

## Step 05 — not started

No Step 05 functionality is included. Sending/editing/deleting messages, guessed account ownership, cloud services and automatic external links are not introduced. Viewer limits and platform-dependent playback remain documented rather than presented as universal codec support.

## Later boundaries

Device/USB adapters, accessible-file backup creation, overlap merging, explicit identical-file reimport, vault migration, encryption/key management, retention/deletion, recovery tooling, and release signing require separate work. Cloud services, authentication, analytics, WhatsApp Web automation, root access, private messaging APIs, and encryption bypass remain outside scope.
