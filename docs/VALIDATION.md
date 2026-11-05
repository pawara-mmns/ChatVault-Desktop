# Step 07 - Native backup reader validation

Validation started 2026-11-04 and completed 2026-11-05, Asia/Colombo, on Windows x64 with Node 24.18.0, pnpm 12.14.0 and Rust/Cargo 1.99.1 MSVC. The earlier [Step 06.2 report](VALIDATION_STEP062.md) preserves discovery/physical-transfer evidence. No Git operations or Step 08 work were performed.

**Result: the supported native-reader workflow is implemented and validated with synthetic encrypted databases. Real snapshot inspection passed. Real decryption, schema identification and readable import remain PENDING because no matching user-provided key was available.** No real messages were imported or claimed readable.

## 1. Real saved-backup inspection

Inspection used the existing owned WhatsApp Business snapshot, read-only. Its manifest records 1,096 transferred/verified files, 1,995,017,693 bytes, zero failed files and local integrity `verified`. Its phase is `completed_with_warnings`, inventory completeness is false and its warning is `timeout`. The transferred inventory is partial; verification of those files does not prove the entire phone library was discovered.

| Actual database candidate        |       Bytes |
| -------------------------------- | ----------: |
| msgstore.db.crypt14              | 233,005,701 |
| msgstore-2026-10-28.1.db.crypt14 | 229,403,418 |
| msgstore-2026-10-29.1.db.crypt14 | 233,564,454 |
| msgstore-2026-10-30.1.db.crypt14 | 233,906,409 |
| msgstore-2026-10-31.1.db.crypt14 | 230,987,072 |
| msgstore-2026-11-01.1.db.crypt14 | 231,125,213 |
| msgstore-2026-11-03.1.db.crypt14 | 232,950,381 |
| msgstore-2026-11-04.1.db.crypt14 | 232,834,900 |

All eight candidates have matching stored sizes/SHA-256 and standalone MD5 footers. Their protobuf prefixes are 194 bytes with a two-byte varint length, crypt14 cipher information and WA-provided-key mode. No candidate hashes are duplicates; no inventoried files are missing. These are complete-looking encrypted standalone candidates, including `msgstore.db.crypt14`; authentication, decrypted SQLite integrity, message schema and history completeness cannot be established without the matching key.

Individual candidates are `SUPPORTED_AND_KEY_REQUIRED`; the overall snapshot remains `INCOMPLETE_BACKUP`. Required input is a matching local crypt14 key file with the supported 131-byte layout or exact 158-byte Java byte-array wrapper. No user key was read, extracted, guessed or requested for automated validation.

Inventoried media metadata: audio 40 files / 35,677,580 bytes; voice notes 713 / 31,204,599; other 335 / 70,357,966. Images, videos and documents were not present in this partial saved inventory. That does not establish absence on the phone. Inspection stats media without reading its contents or displaying private material.

## 2. Compatibility and research findings

The [compatibility report](NATIVE_COMPATIBILITY.md) links reviewed primary protobuf, encryption/key and Android schema sources, licenses, pinned dependencies and implementation limits. No upstream GPL decryptor code, executable or runtime was bundled or executed. No cloud service is used. Dependency license metadata was checked; a release security/legal audit or cargo-audit result is not claimed.

Supported scope is the bounded standalone crypt14/crypt15 envelope with a 16-byte IV, authenticated AES-256-GCM tag and MD5 format footer. Crypt14 uses the matching validated local key structure. Crypt15 uses the matching 64-hex-character root with HKDF-SHA256 for supported root-key modes. Authentication must succeed before plaintext output, decompression or parsing. Password/passkey encryption, crypt12/crypt20, unknown headers, tag-only/multifile/incremental backups and unsupported decrypted schemas are rejected. Extension recognition alone never enables Read Chats.

## 3. Implementation

The inspector validates ownership markers, manifest/database inventory agreement, safe ordinary paths, size, SHA-256, header and footer. A worker authenticates bounded ciphertext, decompresses into restricted owned staging, validates SQLite read-only and produces actual row counts. A preview is not an archive. Only a second consent and successful transaction create readable rows, provenance, FTS and viewer links. Empty, failed, cancelled, unauthenticated or unsupported input creates no readable archive.

Hashing/decompression/row processing use bounded chunks. Limits: 256 MiB encrypted input, 1 GiB SQLite output, decompression ratio 200 plus 1 MiB allowance, 1,000,000 messages, 100,000 conversations, 1 MiB fields and cooperative ten-minute worker deadline. SQLite sorts integer IDs/timestamps before fetching rows. Cancellation is checked between operations and by SQLite's VM progress hook; no single native/cryptographic call is claimed forcibly interruptible.

Implementation entry points: `src-tauri/src/storage/native/*`, `src-tauri/migrations/006_native_reader.sql`, `007_native_reply_links.sql`, storage migration/registration and viewer/media repositories, `src/services/native.ts`, `src/features/backups/NativeInspector.tsx`, `BackupHistory.tsx`, viewer timeline/page/service models, settings/privacy copy, `src/test/nativeReader.test.tsx`, `src/styles.css`, Tauri permissions/build registration, `Cargo.toml`/`Cargo.lock`, schema/fixture scripts and the updated README/architecture/roadmap/security/compatibility/validation documents. Existing backup helper visibility is reused; the scan-test setup change is test-only.

## 4. Rust modules and IPC

`src-tauri/src/storage/native/` contains `inspector.rs`, `crypto.rs`, `parser.rs`, `privacy.rs`, `models.rs`, `mod.rs`, `commands.rs`, tests and public synthetic fixtures. Six commands have narrow Tauri permissions:

- `inspect_native_backup`
- `verify_native_key`
- `verify_native_key_file`
- `import_native_backup`
- `get_native_reader_progress`
- `cancel_native_reader`

The frontend cannot supply an arbitrary filesystem/key-file path. A Rust-native picker reads a bounded key file. Blocking work runs off the UI thread with serialized state, an installation coordinator, separate progress locking and stale-response guards. Conflicting import/backup/vault changes are excluded during an active native preview.

## 5. Preserving migrations

Schema version is 7. Migration 006 adds nullable native provenance/schema/parser/diagnostic columns, native message fields, a unique snapshot/parser-version index and `native_media_links`. Migration 007 adds nullable resolved reply links and an index. Existing snapshots, export archives, messages, search data and settings are preserved. A final read-only check of the regular profile found schema 7, quick_check=ok, the original one snapshot and 1,096 verified inventory entries / 1,995,017,693 bytes, with zero readable archives/messages. Original `source_type='chat-export'` is retained for compatibility with its existing constraint; explicit native provenance and format identify native archives. Later parser versions can create separate revisions without overwriting archives.

Import-job intent and the existing owned staging/lease/recovery system protect restart cleanup. SQLite/FTS changes commit atomically. Failure rolls back changes and only verified owned temporary destinations are eligible for cleanup. Unavailable drives or unverified ownership are left for safe recovery. Tests cover upgrade from schema 5, repeated initialization, forced transaction failure and orphan plaintext cleanup.

## 6. Supported SQLite fingerprints

- `messages-v1`: legacy `messages` columns `_id`, `key_remote_jid`, `key_from_me`, `key_id`, `data`, `timestamp`, `remote_resource`, `media_wa_type`.
- `message-chat-jid-v1`: modern ordinary `message`, `chat`, `jid` tables with required identifiers, joins, direction, timestamp, type and text fields. Verified optional `message_media` and `message_quoted` columns enable conservative media/reply evidence.

Read-only/query-only SQLite uses `trusted_schema=OFF`, bounded fields, integrity checks, ordinary-table fingerprints and cancellation hooks. Views/virtual tables cannot satisfy required tables. Raw JIDs and LIDs remain separate. Stored group subjects are used; contact names are not guessed. UTC milliseconds and original identifiers are preserved. Deleted/system/unknown types remain explicit with unsupported counts. Reactions, receipts, detailed system actions, external contacts and other schema families are not claimed. The real snapshot's decrypted schema remains pending.

## 7. Keys and plaintext privacy

Temporary plaintext processing and persistent readable import require separate explicit consent. Keys/derived buffers use zeroization where practical and are never persisted or logged. The text field clears on submission. JavaScript/IPC copies, SQLite caches and every allocation cannot be guaranteed erased.

Before plaintext creation, Windows staging gets a protected inheritable current-user/SYSTEM DACL; Unix uses 0700. Previews expire after 15 minutes and are cleaned on subsequent command/progress processing, cancellation or owned-stage recovery. Cleanup is not secure SSD erasure. Persisted messages, FTS, SQLite WAL/journals and the vault remain unencrypted; the UI states that before import. Original encrypted snapshots remain the source of truth.

## 8. Media association

Matching requires a normalized application-relative path, expected size, SHA-256 reference and a unique snapshot entry, followed by rehashing. Filename-only, ambiguous, missing and unsupported references stay unresolved. Blob-32 and hex-64 hashes are supported. Native media links reference existing verified snapshot bytes without another media copy. Existing viewer origin/token/signature/size/range rules still apply. Synthetic integration resolves the linked image inside the snapshot and checks the archive contains only its owner marker.

## 9. Frontend and existing reader integration

Backup History offers explicit Inspect Backup, per-candidate integrity/capability/key requirements, completeness warnings and media counts. A supported candidate can be unlocked after processing consent. The preview shows actual conversations/messages/media/unsupported counts and requires separate persistent-import consent. Read Chats appears for a committed or previously imported archive. Cancellation clears owned plaintext; stale polling cannot replace newer panel state.

Native archives use existing conversation pagination, message rendering, offline search/FTS and media resolution. Direction, UTC time, deleted placeholders and uniquely resolved reply links extend the viewer without changing exported TXT timestamp behavior. The storage summary refreshes after import. Settings disclose the plaintext boundary.

## 10. Automated validation

Final frontend validation passed: `pnpm build`, `pnpm lint`, all 109 frontend tests across 14 files, migrations 1-7 via `node scripts/check-schema.mjs`, and repository-wide `pnpm format:check`. Rust `cargo check --locked`, `cargo fmt -- --check` and the final full suite passed: 128 tests, zero failures, four optional hardware/read-only tests ignored by default (20.37 seconds for the library suite). The final suite includes cancellation during preview handoff and persisted diagnostic counts.

Native tests use independently generated Node/OpenSSL crypt14/crypt15 fixtures with a deliberately public synthetic key. Coverage includes correct/wrong/malformed keys, exact Java wrapper parsing, authentication/truncation/header failures, unsupported modes/multifile envelopes, SQLite/schema failures, decompression bounds/trailing bytes, Unicode/Sinhala/emoji, individual/group/LID separation, ordering/direction/deleted/system/unknown types, unique replies, media matching/missing/ambiguity, consent, preview cleanup, duplicate prevention, rollback/recovery, preserving migrations, 30,000-row bounded parsing, search/viewer/reopen and source byte equality.

An existing one-second cooperative scan test initially exceeded its deadline because it included first-run database migrations under parallel test load. Its setup now initializes the database before measuring enumeration; the unchanged production scan behavior passes the full suite. Existing regressions exercise export imports, viewer/search/media, device/discovery/backup behavior, settings, recovery and frontend progress/lifecycle handling. Automated results do not substitute for new physical unplug/lock tests or manual visual acceptance.

## 11. Native validation evidence

An isolated native Tauri development app launched under `app.chatvault.desktop.native.smoke07` on port 1427 with a separate local profile and target directory. Its process responded and its SQLite schema was 7 with `quick_check=ok`, zero snapshots and zero archives. It was closed without stopping the user's regular application. No computer/browser automation tools were available; native click-through, key-dialog interaction and visual acceptance are not claimed.

The final ignored Rust read-only inspector passed against the real saved Business snapshot in 201.17 seconds in the unoptimized development/test build: eight crypt14 candidates, matching SHA-256 and standalone MD5 footers, zero missing inventoried files, incomplete overall state and matching-key requirement. All names/sizes and verification outcomes matched the initial read-only inspection. This measured time includes two complete checksum passes over the large database candidates; it is not evidence of a blocked phone driver or a real-decryption benchmark. A separate initial read-only Node inspection independently verified standalone MD5 footers. No new phone scan or physical transfer was needed for the saved-snapshot reader. Step 06.2 one-file transfer evidence is historical, not a Step 07 decryption test.

## 12. Blockers and limits

No matching real crypt14 key was supplied. Real GCM authentication/decompression, actual WhatsApp schema compatibility, extraction, media association and native readable-archive acceptance remain PENDING. No private key access or bypass was attempted. Synthetic success does not prove every WhatsApp version/field/schema is supported. Unsupported/ambiguous inputs fail with useful statuses and preserve encrypted sources. The saved snapshot is partial and cannot support complete-history recovery claims.

## 13. Acceptance checklist

| Criterion                        | Evidence / state                                              |
| -------------------------------- | ------------------------------------------------------------- |
| Stored backups can be inspected  | PASS: real read-only snapshot and synthetic failures          |
| Encryption accurately identified | PASS for verified envelope families; unknown layouts rejected |
| Matching-key decryption          | PASS synthetic crypt14/crypt15; **PENDING real snapshot**     |
| Invalid keys fail safely         | PASS synthetic; no plaintext archive                          |
| SQLite read-only validation      | PASS synthetic; **PENDING real snapshot**                     |
| Actual conversations extracted   | PASS real rows in synthetic SQLite; **PENDING real snapshot** |
| Text/sender/time persist         | PASS synthetic import/reopen; **PENDING real snapshot**       |
| Supported media association      | PASS synthetic verified reference; **PENDING real snapshot**  |
| Existing viewer and search       | PASS automated integration; manual native acceptance pending  |
| Source files stay byte-identical | PASS synthetic equality and real read-only hash comparison    |
| No key persisted/logged          | PASS implementation/test-fixture review; no real key used     |
| Existing features regressions    | Automated suites pass; no claim of new physical device tests  |

Later owner-key validation: Inspect the saved snapshot, select a supported candidate, acknowledge temporary plaintext processing, choose its matching local key file, review the authenticated schema/count preview, then explicitly consent to unencrypted persistent import. Verify viewer/search/media and restart persistence. Incompatible keys/schemas must preserve original snapshots and earlier archives. No Step 08 behavior is implemented.
