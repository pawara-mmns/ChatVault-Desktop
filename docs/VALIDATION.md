# Step 03 validation and acceptance

Validated 2026-11-04 on Windows x64: Node 24.18.0, pnpm 12.14.0, Rust/Cargo 1.99.1 MSVC, WebView2. All inputs were synthetic. No private conversations entered the repository or regular application vault. No Git operations were performed.

## Automated results

| Check                                                       | Actual result                                                  |
| ----------------------------------------------------------- | -------------------------------------------------------------- |
| `pnpm.cmd build`                                            | Passed: strict TypeScript and Vite build                       |
| `pnpm.cmd lint`                                             | Passed: zero warnings                                          |
| `pnpm.cmd test`                                             | Passed: 7 files, 41 tests                                      |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked` | Passed                                                         |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked`  | Passed: 49 tests, zero failures; binary/doc targets passed     |
| `node scripts/check-schema.mjs`                             | Passed: migrations 1/2 in memory, constraints/FKs/DDL rollback |

The Rust suite has 21 preserved storage tests and 28 importer tests. Coverage includes Android/iOS, 12/24-hour clocks, seconds, numeric separators/year-first dates, date ambiguity, invalid/unparsed records, groups/system/deleted/unknown senders, multiline Sinhala/Unicode, CRLF, UTF-8/UTF-16, encoding/record limits, media/missing references, unsupported content, CRC corruption, traversal/absolute/reserved/colliding names, ZIP symlinks, expansion/entry/ratio limits, and multiple conversation rejection. Real temporary files and databases verify persistence, source preservation, SHA-256 duplicates, independent overlapping exports, custom vaults, filtering, transaction rollback, migration preservation, cross-instance locks, owned recovery, cancellation, and service restart.

Frontend tests cover confirmation, preview, ambiguous dates, cancelled selection/preview, duplicates, invalid/save errors, HTML-looking plain text, library empty/error/filter/summary behavior, browser rejection, and narrow token IPC. Mocked IPC exists only in tests; production invokes Rust. Existing navigation/theme/storage tests remain. The Unix-only baseline symlink test was not run on Windows; Windows path/link tests and portable ZIP symlink tests passed.

## Native end-to-end evidence

Built and launched the real Tauri application using isolated identifier `app.chatvault.desktop.importsmoke.run16b2ede14b`, title **ChatVault Import Smoke Test**, and a separate app-local database/vault. Development assets used the existing loopback Vite server. A distinctly named copy of the same debug executable, `ChatVaultImportSmoke.exe`, avoided ambiguity with the regular application. The disposable profile and build artifacts are retained; no regular-vault test records were created.

Using the computer-use skill and native Windows picker:

1. Fresh dashboard reported schema 2, zero archives, zero snapshots, and 0 B.
2. Selected `SyntheticAndroid.txt`: five records, two participants, zero media, one missing reference. Save was disabled until day/month/year was chosen. Sinhala, emoji, multiline content, and HTML-looking text rendered as plain text with local dates/timezone uncertainty.
3. Explicit save completed: **SyntheticAndroid, 5 records, 0 media, 469 B**.
4. Selected `SyntheticMedia.zip`: **2 records, 2 participants, 1 media, 0 missing references**. Explicit save completed: **557 B**.
5. Reselected the identical ZIP: duplicate warning, cancellation/library actions, no save action. Cancelled; count remained two.
6. Library showed both archives. Filtering `SyntheticMedia` returned one row; its summary showed provenance, counts, date policy, diagnostics, and SHA-256.
7. Dashboard showed **2 archives, 0 backup snapshots, 1 KB**, both recent archives, and no invented connected device.
8. Closed/relaunched the isolated executable. Both archives and recent entries remained.

A separate read-only query of only the isolated database/files confirmed:

| Evidence                                  | Value                                                          |
| ----------------------------------------- | -------------------------------------------------------------- |
| Schema                                    | 2                                                              |
| Archives / messages / media               | 2 / 7 / 1                                                      |
| Missing attachment references             | 1                                                              |
| Backup snapshots / pending import jobs    | 0 / 0                                                          |
| NULL UTC / known local message timestamps | 7 / 7                                                          |
| Stored Sinhala/emoji/multiline text       | Exact synthetic content retained                               |
| Original copies                           | Both byte-identical to fixture sources; SHA-256 matches        |
| Stored image                              | Existing 68-byte file at archive UUID/media/generated UUID.png |
| Remaining temporary stages                | 0                                                              |

Early automation encountered minimized windows and stale native-dialog accessibility indexes; fresh screenshots and keyboard navigation completed the final workflow. An initial fixture creation shell had replaced Unicode with question marks. The fixture was corrected to actual UTF-8 before the successful saved import above. The completed distinctly named smoke-test process was closed; an earlier minimized isolated test picker may remain open.

No genuine private export was supplied. Compatibility evidence is representative synthetic formats and the native workflow above, plus the Rust suite. Packaged installer, physical network disconnection, packet capture, and a broad hardware/locale matrix were not tested. Runtime source adds no upload, API, analytics or telemetry service; development downloads and loopback HMR are separate from import processing.

## Repeatable manual checks

Use a disposable profile/vault for synthetic data. Normal development starts with `pnpm.cmd tauri dev`. A separate test profile requires a temporary Tauri config with a unique identifier and title. The smoke config at `.cache/native-e2e/tauri.conf.json` disables beforeDevCommand and requires a running loopback frontend server.

1. Choose `src/test/fixtures/SyntheticAndroid.txt` in Import Chat. Verify five records, one missing reference, plain Sinhala/emoji/multiline text, blocked save until explicit day/month/year, editable title, and successful confirmation.
2. Verify library filename/date/counts/size/status, literal title/filename filtering and summary; verify dashboard archive count, actual usage, recent archives, and independent backup count.
3. Import `SyntheticMedia.zip`: two records, one media, no missing references. Verify archive-local image and unchanged original source bytes.
4. Reselect the identical ZIP; cancel its duplicate warning. Preview `SyntheticIOS.txt` and cancel; also cancel the native picker. None should create another archive.
5. Select unsupported text or malformed ZIP. Verify error and unchanged saved data. Close/restart and verify persistence.
6. Repeat with a dedicated empty custom vault in a fresh profile; populated vault relocation remains blocked. Check appearance/settings/navigation and unavailable device/backup operations. Browser-only preview must explicitly reject native operations.

## Acceptance checklist

- [x] Existing shell, themes, routes, SQLite/vault safety and independent backup statistics preserved through regression tests; native shell/dashboard exercised.
- [x] Native TXT/ZIP picker, Rust content validation, cancellation and narrow capabilities.
- [x] Common Android/iOS parsing, Unicode/multiline/encodings, diagnostics and retained unknown records.
- [x] Explicit date ambiguity handling; original/local times without invented UTC.
- [x] Bounded hostile ZIP handling, generated output names, media/missing mapping, no overwrite/execution.
- [x] Preserving migration 2, atomic persistence, owned recovery journal, SHA-256 duplicate prevention.
- [x] Real five-stage workflow, library/filter/summary and dashboard refresh.
- [x] Native synthetic TXT/ZIP saves, media, duplicate cancellation and full application restart verified.
- [x] Plain-text rendering, local-only runtime design, no sensitive logging or encryption claim.
- [x] Required documentation, fixtures and automated checks completed.
- [ ] Genuine private export / broader locale compatibility (no private input supplied).

## Change inventory

Backend: new `src-tauri/src/storage/imports/{models,parser,input,zip_processor,media,staging,archive_repository,service,commands,mod,tests}.rs`; migration `002_chat_import.sql`; storage service/mod/migrations/tests integration; lib.rs, build.rs, capabilities/default.json, Cargo.toml and Cargo.lock. Checked i64-to-u64 count decoding is preserved. Existing Rust files were formatted.

Frontend: new services/imports.ts, updated contracts/domain/availability, importer page, archive library/hook/recent list, dashboard integration, Settings copy and CSS. Tests: import/archive/service tests and DTOs, App regression updates, three synthetic exports. Schema helper executes migration 2. README, ARCHITECTURE, SECURITY, ROADMAP and this report are updated.

## Known limitations

Supported numeric export headers only; language-dependent system/media phrase recognition; BOM required for UTF-16; short years mean 2000-2099 with diagnostics; exactly one compatible conversation per ZIP; ZIP64/split/encrypted/unsupported compression rejected; bounded inputs; MIME values are extension hints and media is stored opaquely. Identical-file reimport, overlap merging, vault encryption, phone backup access and Step 04's full viewer are not implemented. Unknown recovery ownership preserves files for inspection.
