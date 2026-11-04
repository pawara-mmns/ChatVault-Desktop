# Step 06 validation and acceptance

Date: 2026-11-04, Asia/Colombo. Windows x64; Rust/Cargo 1.99.1 MSVC, Node 24.18.0, pnpm 12.14.0, Tauri 2.12.5, windows crate 0.62.2. No Git operations or Step 07 work. Historical reports: [Step 05](VALIDATION_STEP05.md), [Step 04](VALIDATION_STEP04.md).

**Status: implemented, but not fully validated on hardware or through the native wizard.** The authorized physical transfer probe observed zero WPD devices and copied no phone bytes. The user then requested that hardware validation remain pending. Prior Step 05 folder discovery is not evidence of content transfer.

## Implementation summary and changed files

The app now provides real one-source preflight, category/individual file selection, explicit confirmation, bounded WPD transfers, SHA-256 verification, durable snapshots/jobs/files, manifests, real progress, cancellation, conservative restart recovery, history/details/local verification, and validated snapshot-directory opening. Existing imported chat archives remain distinct from opaque native backup files.

| Files                                                                                            | Change                                                                                                                                  |
| ------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| `src-tauri/src/storage/backups/{models,planner,transfer,files,repository,mod,commands,tests}.rs` | New backup domain, safe storage, native stream orchestration, persistence, IPC and deterministic tests                                  |
| `src-tauri/src/devices/{adapter,mod,windows}.rs`                                                 | Injectable read stream/metadata boundary, larger bounded backup inventory, native GetStream/STGM_READ, shared device worker reservation |
| `src-tauri/migrations/005_local_backups.sql`, `storage/{migrations,mod,service,repository}.rs`   | Schema 5, recovery bootstrap, successful-only counts, all-record vault relocation guard                                                 |
| `src-tauri/{build.rs,src/lib.rs,capabilities/default.json}`, generated command ACL/schema files  | Backup state, ten narrow commands, local event permissions, test executable native resource linkage                                     |
| `src/services/{backups,contracts,availability}.ts`                                               | Typed real backup service and availability                                                                                              |
| `src/features/backups/{BackupPage,BackupHistory,BackupProvider}.tsx`                             | Six-stage wizard, inventory/file selection, progress, history/actions, application-wide job observation                                 |
| `src/app/App.tsx`, dashboard, device provider/page, settings, `src/styles.css`                   | Route/navigation, visible-route device scans, actual recent/current backups and privacy copy                                            |
| `src/test/backups.test.tsx`, device/storage regression fixtures/tests, Rust device/storage tests | Deterministic workflow and regression coverage; migration expectation updates                                                           |
| `scripts/check-schema.mjs`, README and architecture/roadmap/security/validation docs             | Schema 5 checks and documented behavior/limits; Step 05 report preserved                                                                |

## Executed checks

| Command                                                                                                                 | Result                                                                                                                                                                                       |
| ----------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pnpm.cmd build`                                                                                                        | Passed: TypeScript and production Vite bundle                                                                                                                                                |
| `pnpm.cmd lint`                                                                                                         | Passed: zero warnings                                                                                                                                                                        |
| `pnpm.cmd test`                                                                                                         | Passed: 12 files / 92 tests                                                                                                                                                                  |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked`                                                             | Passed                                                                                                                                                                                       |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked`                                                              | Passed: 104 tests, 2 explicitly ignored hardware tests; library/binary/doc targets passed                                                                                                    |
| `node scripts/check-schema.mjs`                                                                                         | Passed: in-memory migrations 1-5 and constraints/foreign keys/rollback                                                                                                                       |
| `pnpm.cmd format:check`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`                                    | Passed                                                                                                                                                                                       |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib live_one_small_image_readonly_transfer -- --ignored --nocapture` | Attempted; failed the zero-device precondition. No file selected or stream opened                                                                                                            |
| `pnpm.cmd tauri dev --config .cache/backups-e2e/tauri.dev.json`                                                         | Native process started with isolated identifier `app.chatvault.desktop.backup.smoke06`; isolated SQLite schema 5, quick_check=ok, snapshots=0, jobs=0. Full UI interaction was not automated |

Rust uses `CARGO_BUILD_JOBS=1`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0`, `.cache/viewer-e2e/target` to preserve the established paging-file workaround. No OS/pagefile setting changed. Native startup first failed with access denied while creating the isolated Windows profile under the sandbox; the same isolated launch succeeded with that access allowed. Regular application data was not used.

The first new native event-linked Rust test executable failed to load with STATUS_ENTRYPOINT_NOT_FOUND. Import inspection identified Common Controls v6/TaskDialogIndirect. Unit-test linkage now includes Tauri's generated native resource, while normal binary resource handling remains Tauri-owned. Full library, binary and doc test execution then passed. This was a test executable loader issue, not a claimed transfer result.

## WPD architecture, commands and schema

The existing GENERIC_READ WPD adapter now calls `IPortableDeviceContent::Transfer`, `IPortableDeviceResources::GetStream(WPD_RESOURCE_DEFAULT, STGM_READ)`, and repeated `IStream::Read`. Driver buffer hints are clamped to 16-512 KiB. Short reads and EOF are handled; no full-file buffering or Explorer copying. COM interfaces stay on their worker and drop before CoUninitialize. Microsoft references: [transfer sample](https://learn.microsoft.com/en-us/windows/win32/wpd_sdk/transferring-content-from-the-device-to-a-pc), [GetStream](https://learn.microsoft.com/en-us/windows/win32/api/portabledeviceapi/nf-portabledeviceapi-iportabledeviceresources-getstream).

The ten commands are `prepare_backup`, `get_backup_plan_files`, `start_backup`, `get_backup_progress`, `cancel_backup`, `list_backup_snapshots`, `get_backup_files`, `reverify_backup`, `cleanup_backup_staging`, `open_backup_location`. Inputs are issued device/plan/file/snapshot IDs and a typed selection, never arbitrary WPD IDs or filesystem paths. No broad filesystem, SQL or shell endpoint was added.

Migration 5 extends the existing backup_snapshots table and adds STRICT backup_jobs and backup_files. It records phase, source/display name, actual file/byte counts, errors, warnings, safe paths, hashes and separate verification states. Device identity remains optional; no raw/hashed hardware identifier is saved. Migration/reapplication tests preserve existing archive and backup rows. Failed/incomplete/corrupt snapshots do not count as successful backups. Any snapshot record blocks vault relocation even after its empty staging is cleaned.

## Manifest and verification evidence

Published files live in `backups/<UUID>/`, separate from imports. The exclusive ownership marker binds the snapshot to its installation. Manifest schema 1 includes snapshot/source/display metadata, mode/phase/completeness/warnings, observed skipped entries/restrictions and files. Each file includes category, source-relative logical path, generated safe local relative path, original filename, expected/actual bytes, SHA-256, local/source/size verification and error/state. No keys or parsed messages are written. Original filenames themselves can be private and remain local.

Incoming SHA-256 is computed while copying into an exclusive temporary file. Flush/sync precedes an independent local read/hash and size comparison. Only valid files are renamed. Optional separately opened source reads compare another SHA-256 and byte count; otherwise source verification explicitly says not requested. Metadata changes or truncation fail required files. Live same-size mutations may escape metadata-only checks; even independent rereads do not promise atomic consistency or WhatsApp restorability.

Local history verification validates ownership and the manifest against the complete persisted hash/path inventory before reading. A corrupted local file fails verification and is excluded from successful totals. Per-file and snapshot outcomes persist across service reopen; manifest copy-time evidence is retained unchanged.

## Progress, cancellation and recovery

Actual main-window progress events are throttled to 250 ms plus phase/file transitions. Fields include source, current name, verified files, failures, written/known bytes, unknown-size count, average transfer rate and local-read bytes. A one-second fallback reads job memory. Unknown totals are indeterminate. No filename or phone identifier enters application logs.

Jobs persist before copying. File/phase boundaries are durable, with best-effort two-second chunk progress. All files remain in an exclusively owned staging directory until selection verification and manifest publication. Filesystem promotion precedes the SQLite completed commit; these are explicitly separate operations.

Cancel stops scheduling/reading at a safe boundary, releases streams/worker state and records cancelled. A synchronous WPD call may delay response; no forced thread termination or immediate-cancellation promise. Navigation does not cancel a job. App exit requests cancellation, and abrupt termination is handled at startup.

Recovery under the installation coordinator marks unfinished jobs interrupted even with a missing vault. It never guesses success from a manifest, automatically resumes, or removes files. Explicit cleanup validates the exact incomplete job marker/tree before deleting staging only. A crash after promotion but before database completion preserves the published directory as interrupted/manual-inspection data; it never counts complete and staging cleanup does not delete it. Fresh attempts use new UUIDs and retain older snapshots.

## Automated scenario evidence

New Rust fixtures generate binary bytes on demand rather than allocate entire source files. Tests cover successful single/multiple/empty files, a streamed 20 MiB file, bounded short reads, collision-safe names, invalid/traversal/reserved paths, truncated transfers, same-size source mutation detected by reread, disconnect/access-denied/unsupported streams, unknown-size acknowledgements, cancellation and worker release, independent local verification/corruption, repeat attempts, reopen/history, interrupted recovery including missing vault, hardlink/foreign-marker cleanup rejection, writer failures and migration safety. Disk-full/access-denied/drive-removal write failures are injected at the actual writer boundary; no real drive was filled or removed. Existing storage read-only-destination tests remain in the full suite.

Frontend tests cover required device choice, separate source selection, actual category enablement, select/deselect, individual issued file IDs, preflight/review, explicit confirmation, copied-byte progress, cancellation calls, incomplete/capacity acknowledgements, insufficient space, vault errors, browser/no-device behavior, completed history/details/hashes, local verification/open actions and interrupted staging cleanup. All prior import, archive viewer, literal Unicode/Sinhala search, media, settings and USB service tests remain passing in the suite. Prior native reader/media evidence is historical, not claimed newly repeated.

## Physical device evidence and pending validation

The user authorized a read-only one-small-image test with independent source reread and local hash verification. The fresh real WPD enumeration returned **zero devices**. The test failed before inventory selection, GetStream or copy. No private phone bytes, filenames, hashes or image content were logged. When informed of this state, the user chose **Continue with hardware validation pending**. No further phone content transfer was attempted.

Pending: actual GetStream transfer on TECNO POVA 5, source/local hashes and size match, unplug during transfer, post-unplug local verification, larger database/media selection, reconnect/restart native wizard behavior, device lock/mode/driver matrix, external-drive removal/storage-full physical checks, both-theme keyboard/visual acceptance in the native webview. Computer UI automation was unavailable in this run. Successful native process startup and schema creation are not native wizard acceptance.

The isolated failed live-probe temporary profile contains no phone data. The ignored test retains its isolated vault on successful future execution and prints only coarse size/hash-match outcomes; it must be invoked only with current explicit authorization and exactly one intended phone.

## Manual acceptance steps

1. Launch `pnpm.cmd tauri dev` on Windows with Rust/MSVC/WebView2 available. Prefer an isolated test identifier/vault. Check light/dark layouts, keyboard focus and no-device state.
2. Connect and unlock the authorized phone, choose File Transfer/MTP, refresh, select the device. Select Business and discover; verify actual counts/restrictions. Rescan regular WhatsApp separately and verify no package merge.
3. Choose exactly one small accessible image via individual selection, review real space, enable independent source reread, then explicitly Backup Now. Verify progress and final local/source/size states without opening private content.
4. Open details, compare actual local file length to manifest metadata, run Re-verify, then unplug and rerun local verification. Restart and verify persisted history. Open folder only by explicit action.
5. With permission, test a larger selection including encrypted databases/media. Confirm originals remain encrypted and phone files untouched. Create another attempt and verify older snapshots remain intact.
6. Cancel a slow job and separately unplug during copying. Check failed/cancelled status, successful counts unchanged, isolated staging, safe explicit cleanup and fresh retry. Test crash/restart, full/removed/read-only destination without using valuable files.
7. Recheck TXT/ZIP import, Archives reader, Sinhala/global search, image/audio/video, theme persistence and vault settings. Confirm there is no restore/decrypt path.

## Acceptance checklist

- [x] Real WPD read-only stream implementation, bounded memory/short-read handling and worker-local COM lifetime.
- [x] Separate source inventories, actual categories/files, count/size uncertainty, capacity and explicit confirmation.
- [x] Incoming/local SHA-256, optional independent source read, expected-size and consistency states.
- [x] Durable schema/manifests/history, safe publication, cancellation/interruption/cleanup and previous-snapshot preservation.
- [x] Real progress, dashboard successful counts, history/details/reverification/validated folder opening.
- [x] Automated backend/frontend regression and native startup executed; observed results distinguished from simulations.
- [ ] Physical phone content-transfer success and full native wizard/visual/keyboard acceptance.
- [ ] Broader physical lock/unplug/driver/storage failure matrix and release/installer validation.

Native databases remain encrypted. ChatVault vault/SQLite are unencrypted. No native restore, atomic snapshot, complete-history recovery, decryption, auto-resume or automatic scheduling guarantee is made. Step 07 is untouched.
