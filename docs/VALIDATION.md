# Step 06.2 - Discovery scanning and progress validation

Date: 2026-11-04, Asia/Colombo. Windows x64, Rust/Cargo 1.99.1 MSVC, Node 24.18.0, pnpm 12.14.0, Tauri 2.12.5, windows crate 0.62.2. No Git operations or Step 07 work. The original [Step 06 report](VALIDATION_STEP06.md), [Step 05](VALIDATION_STEP05.md) and [Step 04](VALIDATION_STEP04.md) reports remain historical evidence.

**Result: implemented; real scoped discovery and one real small-file transfer passed.** Full-library completion, native wizard interaction/visual acceptance and physical unplug/lock/storage failure tests remain unvalidated. These are separate from the passing native backend hardware checks and native startup.

## Established cause and limits of the diagnosis

The appearance of being stuck had two verified code causes:

1. `BackupState::prepare` published scanning once, then awaited the whole inventory. It exposed no directory/object counters, elapsed time, heartbeat or pending-operation duration. React reused actual backup progress for preflight, showing 0/0 verified, zero copied bytes and Cancel Backup before any transfer had begun.
2. Planner lookup enumerated all siblings to find directories, including repeated parent enumeration. WPD fetched name/type properties for every object, then the planner requested name/size again for each file. A large directory was collected before files were inventoried; a later enumeration failure discarded its returned entries. Scanning all media before content selection amplified this work.

On the connected device, the instrumented image scan showed steadily advancing object_properties calls: at 10.015 seconds, 407 object observations had completed and another metadata call had been pending for 11 ms. It reached the configured 500-object boundary at about 12.5 seconds. This reproduction showed slow ongoing metadata enumeration, **not an uninterruptible driver call**. A blocked call in the user's earlier session was not established retrospectively; no original-session trace existed. The new diagnostics distinguish these conditions.

## Fixes and resulting behavior

- Dedicated worker and monitor. COM initialization, WPD calls, Close/release and uninitialization stay on the worker. The monitor only reads sanitized counters.
- Actual progress every 500 ms, even during a pending Windows call: directory visits; object/folder/file observations; supported inventory count; known bytes; elapsed/pending time; last successful operation; successful calls; error/HRESULT category. Polling refreshes elapsed/pending state after the response deadline.
- Structured backup_scan diagnostics on start, ten-second intervals, waiting and terminal transitions. Only code-defined labels, source category, counts, durations and numeric HRESULT/error categories are logged. No names, full paths, object/PnP IDs, serials, contents or hash values are logged.
- Rust-validated ScanOptions: category subset, 1-100,000 objects, 1-50,000 files, depth 0-16, time 1-180 seconds (default 60). The UI exposes scope/object/depth/time controls. Database-only quick scans skip media. Scoped plans/new manifests record scannedCategories.
- Folder lookup stops after the matching directory. Name/type/size are requested together, eliminating the second per-file GetValues call. Usable native metadata is inventoried as objects arrive, rather than after a directory finishes.
- Next uses batches of 32, processes short successful batches, validates fetched counts, frees returned strings and checks budgets/cancellation between calls/objects. Only documented S_FALSE with zero objects is accepted as EOF. Repeated IDs, visited sets and depth limits stop repeated traversal/cycles.
- Returned entries survive later errors. Limits and restrictions mark inventory incomplete. Disconnect/cancel produce explicit outcomes. Partial results never silently become a complete inventory.
- Cancel Scan stops future calls at cooperative boundaries. After the time budget plus one-second response grace, an outstanding worker returns driver_pending/scan_waiting, requests cancellation and retains its COM/device/coordinator reservation. Retry stays disabled until safe release; abandoned results are discarded. No thread is killed and no live COM object is freed on another thread.
- Operation UUIDs and increasing publication revisions prevent stale updates replacing terminal/later jobs. Frontend also rejects stale/unmounted request replies.
- Dedicated discovery UI shows actual counts, elapsed seconds, animated indeterminate work, slow guidance after ten seconds, sanitized details and Cancel Scan. Copy/verification counters appear only for actual snapshot jobs. Unknown work has no percentage or full bar.

Schema remains 5; no migration or existing SQLite row rewrite was needed. Discovery creates no backup snapshot/job rows. Streaming, local verification and successful snapshot finalization remain mandatory before a selected backup completes. Source files and previous snapshots are preserved.

## WPD API/cancellation review

Reviewed [IPortableDevice::Cancel](https://learn.microsoft.com/en-us/windows/win32/api/portabledeviceapi/nf-portabledeviceapi-iportabledevice-cancel), [enumerator Cancel](https://learn.microsoft.com/en-us/windows/win32/api/portabledeviceapi/nf-portabledeviceapi-ienumportabledeviceobjectids-cancel) and [Next](https://learn.microsoft.com/en-us/windows/win32/api/portabledeviceapi/nf-portabledeviceapi-ienumportabledeviceobjectids-next). Enumerator/resource cancellation affects all pending I/O on the session, not a targeted enumeration. Microsoft recommends separate sessions for thread isolation. This implementation uses cooperative boundaries and safe pending-worker ownership rather than sharing a worker-owned COM pointer with a control thread. No guarantee that cancellation/time budgets interrupt every vendor call is made.

## Physical evidence

These checks use the production WPD adapter and backup services with isolated storage, not mocks or a filesystem source copy. One connected portable device was observed. No hardware identifier or private name/content was printed.

| Check                             | Observed result                                                                                                                                                                                       |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fresh WPD enumeration             | One portable device; passed                                                                                                                                                                           |
| Business database-only preflight  | Complete: 8 encrypted database files; 1,857,777,548 known bytes; no restriction warnings; about 0.365 s in the first scoped run                                                                       |
| Image-only bounded preflight      | Reached 500-object boundary at about 12.5 s: 499 completed object/property observations, 448 file observations, 51 folder observations, 439 supported files; explicitly incomplete with limit warning |
| Actual GetStream/IStream transfer | Exactly one selected accessible image, 29,816 bytes; source unchanged; no content displayed                                                                                                           |
| Integrity                         | Incoming SHA-256 matched independent local read-back and independently opened source reread; expected and copied sizes matched                                                                        |
| Finalization/history              | Selected-file snapshot completed with warnings because inventory was bounded/incomplete; local reverify passed; reopened service reported one successful snapshot                                     |
| Isolation                         | Retained isolated temporary profile outside repository/regular vault; private copied file was not added to fixtures/docs                                                                              |

A later repeat of the scoped hardware probe returned zero WPD devices and failed its one-device precondition before any scan or content read. No physical disconnect cause is inferred. The earlier successful discovery/transfer evidence remains valid; reconnect and broader driver-availability checks remain pending.

The image scan/copy/verify/reopen test took about 13.19 s. These are limited observations, not universal performance claims. The database byte total is metadata: those databases were not copied. The small-file snapshot is not a whole-WhatsApp backup, atomic snapshot or native-restorability guarantee.

## Executed checks

| Command                                                                                                                 | Result                                                                                                      |
| ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `pnpm.cmd build`                                                                                                        | Passed: TypeScript/Vite                                                                                     |
| `pnpm.cmd lint`                                                                                                         | Passed, zero warnings                                                                                       |
| `pnpm.cmd test`                                                                                                         | Passed: 13 files / 100 tests                                                                                |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked`                                                             | Passed                                                                                                      |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked`                                                              | Passed: 113 tests; 3 ignored hardware tests; library/binary/doc targets passed                              |
| `cargo test --manifest-path src-tauri/Cargo.toml native_wpd_enumeration -- --ignored --nocapture`                       | Separately executed; passed with one device                                                                 |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib live_business_scoped_discovery -- --ignored --nocapture`         | Initially passed with complete scoped inventory; final repeat found zero devices and failed before scanning |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib live_one_small_image_readonly_transfer -- --ignored --nocapture` | Separately executed; real transfer, source/local hashes, local reverify and reopen passed                   |
| `node scripts/check-schema.mjs`                                                                                         | Passed: migrations 1-5, constraints/FKs/rollback                                                            |
| `pnpm.cmd format:check`; `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`                                    | Passed                                                                                                      |
| `pnpm.cmd tauri dev --config .cache/backups-e2e/tauri.scan062.dev.json`                                                 | Native process started/responding on loopback 1426; isolated schema 5, quick_check=ok, zero snapshots/jobs  |

The initial native launch encountered port 1420 in use by the user's app. Testing switched to 1426, a separate identifier/profile and isolated target, without stopping that app. The test app was stopped afterward. Profile creation used permitted access to its own Windows/WebView2 directory. UI automation tools were unavailable, so native clicking/visual/keyboard acceptance is not claimed.

Rust retains CARGO_BUILD_JOBS=1, CARGO_PROFILE_DEV_DEBUG=0, CARGO_INCREMENTAL=0 and .cache/viewer-e2e/target for the established paging-file workaround. No OS/pagefile settings changed.

New mocks cover 10,000-entry directories, a 100,000-file unselected scope, depth/cycles, empty categories, partial access failures, disconnects, cooperative deadlines, a blocked Next-like call, cancellation/retry, progressive publications and stale updates. The blocked test asserts driver_pending retains reservations, then releases the call and verifies retry. Existing transfer/recovery/import/viewer/search/media/settings/device regressions remain in the full suite.

Frontend coverage includes counters/elapsed time, indeterminate semantics, slow/pending states, Cancel Scan/error recovery, revision ordering, polling cleanup, quick scope selection, conflict disabling and absence of copy/verification counters while scanning. Prior wizard/history tests remain passing.

## Changed files

- `src-tauri/src/devices/{adapter,windows,scan,mod,tests}.rs`: diagnostics/budgets, efficient bounded enumeration and fixture compatibility.
- `src-tauri/src/storage/backups/{planner,models,mod,commands,tests,scan_tests}.rs`: progressive scoped planning, partial retention, lifecycle/revision guards, options, real/mock checks and additive manifest scope.
- `src/services/backups.ts`, `src/features/backups/{BackupPage,BackupProvider,ScanPanel}.tsx`, `src/styles.css`: typed progress/options and dedicated UI.
- `src/test/{backups.test.tsx,backupScan.test.tsx}`: scoped workflow, cancellation/progress and stale-result tests.
- README, ARCHITECTURE, ROADMAP, SECURITY and validation reports. Existing Tauri-generated command/schema metadata refreshes through builds. No dependency or broad capability was added.

## Remaining manual validation

Restart the native app to load the new backend. In Backups select the authorized device and Business, choose **Encrypted databases only (quick scan)** and Discover. Check real counts without copy counters. Try Images only with a smaller object budget; partial inventory must stay visibly incomplete. Select a file and explicitly Backup Now only when desired.

For larger scans, inspect advancing counts/elapsed time, slow guidance and operation timing. Cancel, await safe release and retry. Check both themes/keyboard navigation, physical unplug/lock/mode changes, larger selections and actual wizard restart/history. Full-library completion, physical blocked-call behavior and broader driver/storage/installer compatibility remain pending.

Native backups remain encrypted; the vault/SQLite are unencrypted. No restore, decryption, auto-resume, root, ADB, cloud or Step 07 feature was added.
