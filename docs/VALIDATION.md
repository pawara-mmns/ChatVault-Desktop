# Step 04 validation and acceptance

Date: 2026-11-04 (Asia/Colombo). Windows x64, Node 24.18.0, pnpm 12.14.0, Rust/Cargo 1.99.1 MSVC, Tauri 2.12.5 and WebView2. Only deterministic synthetic inputs were used. No private conversations entered the repository or regular vault. No Git operations were performed. Step 05 is not started.

## Implementation and exact check results

The reader opens existing imports from Archives and import completion, uses real metadata/message repositories, retains source order and timezone uncertainty, provides neutral sender/system/deleted bubbles, cursor/context navigation, scoped/global literal search, private on-demand image/audio/video access, and explicit registered document opening. Existing shell/themes/import/storage behavior remains in place.

| Check                                                                     | Actual final result                                                       |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| `pnpm.cmd build`                                                          | Passed: strict TypeScript and Vite production bundle                      |
| `pnpm.cmd lint`                                                           | Passed: zero warnings                                                     |
| `pnpm.cmd test --disableConsoleIntercept`                                 | Passed: 9 files, 66 tests                                                 |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked`               | Passed                                                                    |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked -- --nocapture` | Passed: 62 tests; binary/doc targets also passed                          |
| `node scripts/check-schema.mjs`                                           | Passed: migrations 1–3 and in-memory schema/constraint/FK/rollback checks |
| `pnpm.cmd format:check`                                                   | Passed                                                                    |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`               | Passed                                                                    |

The 49 existing Windows Rust tests and 41 existing frontend tests remain in the passing suite. New coverage adds conversation counts/previews, exact cursor order, foreign/missing cursors, anchor context, sender/text/Unicode/Sinhala/German searches, literal FTS syntax, short-query continuation, preserving/index-backfilling migration, transactional index failure rollback, reopening, owned media/token/origin/range validation, missing/changed/invalid files, unauthorized references, hard-link escape, PNG/WAV metadata, and the large-chat benchmark.

Frontend coverage includes selection by keyboard, date/sender/multiline/Unicode/plain-HTML rendering, scoped/global result navigation, repeated selection of the same target, safe metacharacter/canonical-equivalence highlights, bounded pages/text sections, real metadata, loading/error/retry/empty/browser states, lazy media authorization/revocation, missing/invalid images, codec feedback, audio speed/native controls, explicit document opening, lightbox zoom/fit/close and light/dark bubble rendering.

An initial default debug build hit Windows paging-file error 1455 while mapping dependency metadata. The successful Rust checks used `CARGO_BUILD_JOBS=1`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0`, and the isolated `.cache/native-e2e/target` directory. No OS security/pagefile setting was changed. Vitest is limited to two workers. Unix-only filesystem symlink tests were not executed on Windows; Windows hard-link/path checks and the portable hostile-ZIP symlink test passed.

## Native end-to-end evidence

A real development Tauri binary was built and launched with identifier `app.chatvault.desktop.viewer.smoke04`, title **ChatVault Viewer Smoke Test**, separate app-local SQLite/vault, and the loopback Vite frontend. The config/build artifacts live under `.cache/viewer-e2e/`. The main test window was closed after verification; the disposable test data is retained.

Using the computer-use skill and the native Windows picker:

1. Fresh profile: schema 3, zero archives/snapshots, 0 B.
2. Imported `SyntheticAndroid.txt`; confirmed day/month/year after the ambiguity guard. Five records, two participants and one missing reference were saved.
3. Clicked **Read imported chat**. The reader displayed genuine SQLite records, source-order date separator, sender labels, system/deleted records, Sinhala/emoji/multiline text, literal HTML-looking text and a missing attachment notice.
4. Searched **ආයුබෝවන්** within the conversation. One real result appeared with a safe highlight. Selecting it opened the exact message ID/context.
5. Fixed an observed outer-workspace scroll issue: search jumps now scroll only the timeline; search sits beside the timeline on wide windows. Native global-result navigation subsequently retained the workspace position.
6. Imported `SyntheticViewerMedia.zip`: four records, two participants, three extracted media references, zero missing references.
7. Loaded the 160×100 checker PNG through the private protocol. Actual pixels rendered inline and in the lightbox; zoom changed to 125%.
8. Loaded the synthetic PCM WAV. WebView showed a four-second duration; **play** changed to **pause**, and the time scrubber subsequently reached **4** with the completed 0:04 / 0:04 display.
9. Loaded the deliberately truncated MP4. The UI displayed an unsupported/damaged-codec error with Retry, without executing or converting the file.
10. Global sender search for **Fixture Bob** returned four results across both archives. Selecting the TXT result switched from the media conversation to its correct stored message/context, retaining the search results and workspace position.
11. Closed and relaunched the native executable. Dashboard still showed two archives, zero snapshots, schema 3 and 127.2 KB usage. Reopened the media archive from Archives: stored German text and attachment references remained readable, and a newly authorized PNG decoded again.

Input coordination was required while another app was active; stale controls were refreshed before input. No unrelated app was operated. Native testing used synthetic export formats, not a supplied private WhatsApp export.

Read-only verification of only this isolated profile after restart:

| Evidence                                        | Actual value                                            |
| ----------------------------------------------- | ------------------------------------------------------- |
| Archives / conversations / messages             | 2 / 2 / 9                                               |
| Search-index rows                               | 9                                                       |
| Stored media                                    | 3                                                       |
| Chat UTC timestamps                             | All 9 NULL; local/original metadata retained            |
| Pending import jobs / temporary stages          | 0 / 0                                                   |
| Original copies                                 | Both byte-identical to source fixtures; SHA-256 matches |
| Extracted PNG / WAV / intentionally invalid MP4 | 398 / 64,044 / 24 bytes; stored lengths match SQLite    |

No network upload code, remote image loading or telemetry service was introduced. The native test used loopback HMR and local IPC/custom media delivery. Physical network disconnection and packet capture were not performed.

## Performance measurements

The Rust test creates **12,000 synthetic messages**, indexes them and reads every record through **200 cursor pages of 60**. It asserts exact traversal/counts, stable sequence and a maximum returned page of 60. This is a debug-symbol-disabled test build, not a release benchmark.

| Measured operation                                  | Elapsed                                       |
| --------------------------------------------------- | --------------------------------------------- |
| Insert 12,000 records + FTS indexing                | 1.285 seconds                                 |
| Read all 200 cursor pages                           | 637.2 ms total (about 3.19 ms/page)           |
| Indexed literal substring yielding one known record | 2.91 ms                                       |
| Four older-page transitions in jsdom                | 434.4 ms including user-event/test scheduling |
| React message records after repeated paging         | 120 maximum                                   |

Earlier runs under heavier concurrent build load measured 2.83 seconds insertion/indexing, 2.17 seconds for all pages and 9.92 ms for the same search. Results vary with load. No native FPS, peak-memory, huge-vault throughput or universal hardware claim is made. DOM tests bound the window and long-message text sections; media is not authorized until requested.

## Migrations, commands and changed files

Migration `src-tauri/migrations/003_chat_viewer.sql` preserves old data and explicit sequence values, assigns stable order to legacy NULL sequences, adds the cursor index, and backfills normalized FTS5 trigrams. New immutable imports index their conversation inside the existing atomic transaction; index failure rolls back records/files safely.

Added IPC: `list_conversations`, `get_conversation`, `read_messages`, `get_message_context`, `search_messages`, `list_conversation_media`, `get_media_metadata`, `authorize_media`, `revoke_media`, `open_document`. Participants are included in details; the archive summary command remains available.

Created:

- `src-tauri/src/storage/viewer/{mod,models,repository,search,media,protocol,commands,tests}.rs`.
- `src/services/viewer.ts` and `src/features/viewer/{ChatViewerPage,ConversationSidebar,ConversationInfoPanel,MessageTimeline,ChatSearch,Highlight,MediaMessage,MediaLightbox}.tsx`.
- `src/test/{viewerFixture.ts,viewer.test.tsx,viewerService.test.ts}`.
- `scripts/generate-viewer-fixtures.mjs` and `src/test/fixtures/SyntheticViewerMedia.zip`.

Modified: Cargo.toml features (rusqlite functions and Windows registered-app API); storage service/mod/migrations/vault and importer repository/staging/service/test integration; lib.rs/build.rs/capabilities/tauri.conf.json command/protocol/CSP registration; App routing/layout, Archives read actions, import completion action, Settings About copy, styles, two-worker test config and schema helper. Tauri-generated ACL/schema metadata updates during builds. README, ARCHITECTURE, SECURITY, ROADMAP and this report are current. No new frontend runtime package or transcoder was added.

## Repeatable manual checks

Use a disposable profile/vault for fixtures. Normal development: `pnpm.cmd tauri dev`. For an isolated profile, use a unique Tauri identifier; the smoke config under `.cache/viewer-e2e/tauri.conf.json` disables beforeDevCommand and requires an already running `pnpm.cmd dev` loopback server.

1. Import SyntheticAndroid.txt, choose day-first and confirm. Read the saved chat, inspect Sinhala/multiline/system/deleted/missing records and source timestamps.
2. Search within the conversation, then globally. Try sender names, Sinhala, German, punctuation, a one-character term and no-match text. Use Continue search for bounded short-query sections, previous/next results, and repeated selection of the same target.
3. Import SyntheticViewerMedia.zip. Load/enlarge/zoom/fit the image, play/seek/change speed on the tone, and verify invalid video feedback. Open/close details and metadata gallery; files should not prefetch.
4. Navigate away/back, close/restart, reopen from Archives, and reload media. Counts, records, originals and hashes must remain unchanged.
5. Exercise older/newer/newest/context pages with a large synthetic chat; the Rust benchmark is `cargo test --manifest-path src-tauri/Cargo.toml performance_12000 -- --nocapture`.
6. In test storage only, simulate a missing file or drive. Expect a clear error and preserved records. Retest themes, keyboard controls, Settings/vault safety and browser-only fallback.

## Limits and unexecuted checks

- WebView codec support varies. Native PNG and PCM WAV playback plus invalid-video feedback were verified. A valid MP4/WebM playback matrix, every image format, hardware acceleration, full-screen video and every seek/volume control were not exhaustively tested.
- PDF/office/plain-text files use explicit registered Windows application opening; no in-app PDF renderer. The actual registered external application launch was not invoked in this smoke test.
- No captions/transcripts, guessed local sender ownership, message sending/editing/deletion, overlap merging, phone access, native-backup decryption or Step 05 features.
- Media tokens expire after 30 minutes; reload an attachment after expiration. Range chunks are at most 2 MiB; non-range bodies at most 16 MiB; files at most 64 MiB; images at most 16 MiB and known dimensions at most 25 megapixels. MIME/signature checks do not guarantee decodability.
- Short queries need explicit continuation across 2,000-record sections. Search is literal substring matching, not Boolean, fuzzy, stemming or linguistic word segmentation. Results and gallery navigation are paged. Long message bodies use readable text sections.
- Legacy/unsupported media without archive ownership information is not authorized. Source-format/parser limitations from Step 03 remain.
- ChatVault does not encrypt vault files, SQLite, WAL or the search index. Same-user/admin filesystem races cannot be fully isolated by these checks.
- No private export, packaged installer, broad locale/device matrix or packet-capture audit was supplied/performed.

## Step 04 acceptance checklist

- [x] Real imported conversations and entire saved text are accessible through bounded pages/sections.
- [x] Correct imported order, senders, original/local dates, Unicode and multiline content are preserved.
- [x] In-conversation/global text and sender search work; exact results navigate to saved message IDs.
- [x] Private images render locally; zoom/fit/gallery controls and missing/damage feedback are implemented.
- [x] Supported native media controls are implemented; WAV playback and unsupported-video behavior ran natively.
- [x] Large chats avoid whole-history React state; 12,000-record paging/search and a 120-record DOM bound are measured/tested.
- [x] Full restart preserves archives/messages/media; originals and hashes are unchanged.
- [x] IDs/tokens, ownership/path/handle checks, fixed MIME, ranges and narrow CSP scope protect media access.
- [x] Existing Step 01–03 regressions and all required automated checks pass.
- [x] Native smoke test ran with exact evidence and remaining platform/media limitations reported.
- [x] Documentation updated; Step 05 unstarted; no Git operations.
