# Step 05 validation and acceptance

Date: 2026-11-04 (Asia/Colombo). Windows x64, Rust/Cargo 1.99.1 MSVC, Node 24.18.0, pnpm 12.14.0, Tauri 2.12.5, windows crate 0.62.2, WebView2. No Git operations were performed. Step 06 is not started. The prior [Step 04 report](VALIDATION_STEP04.md) is retained as historical evidence.

## Implementation

Real local Windows Portable Devices enumeration replaces the placeholder Devices page. Temporary app UUID handles, USB ancestry verification, worker-local COM lifetimes, explicit GENERIC_READ access checks, bounded metadata-only WhatsApp/Business discovery, accurate diagnostic categories, cancellation/backpressure, lifecycle-aware polling, and consented historical notes are implemented. The dashboard consumes live state without changing backup counts. No phone bytes are copied or read, and no phone mutation, root, ADB, decryption, cloud, telemetry, or automatic backup is introduced.

The adapter/service/discovery/persistence/IPC/frontend boundaries and official API references are documented in [ARCHITECTURE](ARCHITECTURE.md); least privilege and privacy limits are in [SECURITY](SECURITY.md). [README](../README.md) contains connection instructions and precise scan limits.

## Executed checks

| Check                                                                                             | Actual result                                                                                                        |
| ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `pnpm.cmd build`                                                                                  | Passed: strict TypeScript and Vite production bundle                                                                 |
| `pnpm.cmd lint`                                                                                   | Passed, zero warnings                                                                                                |
| `pnpm.cmd test`                                                                                   | Passed: 11 files, 83 tests                                                                                           |
| `cargo check --manifest-path src-tauri/Cargo.toml --locked`                                       | Passed                                                                                                               |
| `cargo test --manifest-path src-tauri/Cargo.toml --locked`                                        | Passed: 87 tests; 1 explicitly ignored manual hardware probe; binary/doc targets passed                              |
| `cargo test --manifest-path src-tauri/Cargo.toml native_wpd_enumeration -- --ignored --nocapture` | Separately executed and passed: one device while connected; zero after physical disconnection; no identifier printed |
| `node scripts/check-schema.mjs`                                                                   | Passed: in-memory migrations 1–4, constraints/FKs/transactional rollback                                             |

The successful final Rust test execution took 6.83 seconds; frontend tests took 13.10 seconds. These are suite timings, not device speed or frame-rate claims. `pnpm.cmd format:check` and `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` passed after the final documentation pass.

Rust checks used `CARGO_BUILD_JOBS=1`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_INCREMENTAL=0`, and `.cache/viewer-e2e/target`, preserving the established workaround for Windows paging-file error 1455. No OS/pagefile/security setting was changed. The windows crate was already cached and resolved offline. Cargo.lock supports `--locked`; no frontend runtime dependency was added.

An intermediate regression run exposed a completion race: the channel delivered a result just before releasing the native-worker busy slot, so an immediate next access could return busy. The worker now releases its slot before delivering completion; the final complete suite passed. An initial native scan also revealed that a WPD interface path is not necessarily a PnP instance ID. CfgMgr32 interface-to-instance resolution was added and the real USB workflow then passed.

## Real hardware and native evidence

The user explicitly confirmed that their own Android phone was connected, unlocked, and ready for read-only discovery. Native tests used **ChatVault USB Smoke Test**, configured in `.cache/devices-e2e/tauri.conf.json`, using the existing disposable Step 04 identifier `app.chatvault.desktop.viewer.smoke04`. The regular application profile/vault was not used. The original Step 03 smoke window was left alone. Synthetic exports were the only chat content used in repository/profile regression tests.

Executed against one physical phone:

1. Real `IPortableDeviceManager` enumeration returned one portable device. The native dashboard/Devices page displayed its genuine friendly name and manufacturer, not test data.
2. The local device-interface path resolved through CfgMgr32 to USB ancestry. The page showed USB / Windows Portable Devices; no drive-path or Explorer namespace access was used by the implementation.
3. Explicit **Check file access** succeeded using `GENERIC_READ`, storage functional objects, and folder enumeration. The UI showed File access available at last check. Missing category and OS identity remained Unavailable/Unknown; brand or MTP was not treated as Android proof.
4. Explicit **Scan WhatsApp Storage** completed, examining **146 object metadata entries**. It found the modern ordinary WhatsApp candidate folder, with checked category paths not exposed there, and the modern WhatsApp Business candidate with accessible Databases, Images, Video, Documents, Audio and Voice Notes folders. An encrypted backup filename was detected as a boolean. No filename list, private database contents, or media bytes were read, returned, copied, or stored.
5. Candidate not-found results were reported per checked location. The page did not claim that WhatsApp data was absent. Discovery time and examined-entry count were shown without a fake progress bar.
6. Repeated live enumeration and explicit access/discovery remained responsive and did not create notes, archives, or snapshots from phone discovery. File permission was reset to unchecked on the next live enumeration; discovery remained a dated observation.

Follow-up unplug check **passed** after the user reconfirmed the cable was physically disconnected. Initial fresh Windows observations still returned one device and successful read-only access, so no pass was claimed at that point. Subsequent independent real-WPD probes returned **zero portable devices**, including a repeat with the original enumeration code after temporary test diagnostics were removed. The already-running native app's polling cleared the phone and selected details, showed **0 detected / No device detected**, and explained that the selected device disconnected or its handle expired. Explicit Refresh also completed with zero devices. The dashboard then showed **No device connected**, four unchanged archives and zero backups. No device note or backup was saved (both SQLite counts remained zero).

No production behavior change was needed to obtain the zero-device result; the native app exercised the existing build. The exact cable-removal time and cause of the earlier Windows observation mismatch were not independently measured, so this confirms a disconnection observation rather than a physical-removal latency benchmark. During cleanup Windows displayed a phone-arrival notification and the live list returned to one device, unselected with access unchecked; the app therefore also observed a subsequent device reappearance without reusing the old selection/permission. The exact physical reconnect action was not independently timed. The isolated test window/server were closed after verification.

Explicit access/discovery after the observed device reappearance, charging-only transition, phone-lock transition, unplug during an active discovery, two simultaneous phones, other vendors/drivers, and direct comparison of findings with Explorer remain incomplete. These scenarios are covered by injected tests where applicable and remain on the physical manual checklist. They are not reported as physical passes.

## Native regression and data preservation

The isolated profile started at schema 3 with two existing archives, two conversations, nine messages, three media rows, four attachments, nine FTS rows and zero backups. Migration 4 opened this profile without resetting it; the dashboard showed schema 4 and the same original counts/127.2 KB vault usage.

Native actions executed after migration:

- Archives listed the existing TXT and ZIP imports. The real media conversation opened and displayed the original German text and references.
- The existing synthetic PNG decoded through a freshly authorized private protocol token and displayed actual checker pixels.
- Scoped local search for `waveform` returned one genuine highlighted result from the saved synthetic media conversation.
- A fresh native picker selected `SyntheticIOS.txt`; the actual preview showed two records/two participants/no media, including multiline text. Explicit import completed: two records saved.
- A fresh native picker selected `SyntheticMedia.zip`; the actual preview showed two records/two participants/one media/zero missing references. Explicit import completed: two records/one media saved.
- Settings remained available, displayed the unchanged vault path/schema 4, and changed the disposable profile to light appearance for restart validation. Dark/system appearance had already rendered the Devices flow.
- A full native close, rebuild and restart with the phone connected preserved light appearance, the selected vault path, four archives and zero snapshots. WPD performed a fresh live observation, and the phone returned with file access unchecked rather than inheriting an old permission claim.

Read-only SQLite verification after these two intentional synthetic imports:

| Check                               | Actual value                                                                                                                                                       |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Schema                              | 4                                                                                                                                                                  |
| Archives / conversations / messages | 4 / 4 / 13                                                                                                                                                         |
| Media / attachments / FTS rows      | 4 / 5 / 13                                                                                                                                                         |
| Backup snapshots / legacy devices   | 0 / 0                                                                                                                                                              |
| Consented device notes              | 0 (no consent was given during hardware testing)                                                                                                                   |
| Pending imports                     | 0                                                                                                                                                                  |
| SQLite quick_check                  | ok                                                                                                                                                                 |
| Original pre-migration rows         | SHA-256 digests of the original row ranges matched for all eight inspected tables: devices, backups, archives, conversations, messages, media, attachments, search |

Only existing synthetic profile rows were inspected for this preservation check. Phone metadata findings were not persisted. Before/after count/digest summaries are retained under `.cache/devices-e2e/`. Native audio/video controls and full reader regression remain exercised by the passing automated suites; prior native WAV completion and damaged-video feedback evidence are preserved in the historical Step 04 report rather than falsely claimed as newly rerun here.

## Mock/injected coverage

The 62 previous Windows Rust tests remain, with 25 additional device tests. They exercise no devices versus not-scanned, one/multiple/duplicate devices, missing/malformed/oversized metadata, unverified non-USB transport, unknown Android identity, explicit driver report, locked/access-denied, empty storage, metadata-only discovery, WhatsApp Business naming, missing candidates, denied folders, unplug during open, observed unplug/reconnect with new handles, refresh permission invalidation, restart identity limitations, invalid/stale issued handles, entry/cancel budgets, actual cooperative worker cancellation/backpressure, consented note persistence across database reopen, and migration 3→4 idempotency with legacy archive/device/message rows preserved.

The 66 prior frontend tests remain, with 17 new tests. Device tests use only injected services: completed no-device observation, browser fallback, explicit selection/access/discovery, permission diagnostic, disconnect/refresh replacement, multiple devices/missing metadata, escaped malicious names, per-device consent, cancellable busy state, polling cleanup and dashboard access status without snapshot changes. Narrow IPC and diagnostic filtering are tested separately. Existing storage tests isolate device availability so a storage-only invoke mock cannot masquerade as live device data.

## Files changed

Created:

- `src-tauri/src/devices/{mod,adapter,windows,discovery,models,repository,commands,tests}.rs`
- `src-tauri/migrations/004_device_notes.sql`
- `src/services/devices.ts`, `src/features/devices/DeviceProvider.tsx`
- `src/test/{deviceFixture.ts,deviceService.test.ts,devices.test.tsx}`
- Eight generated scoped command permission TOMLs under `src-tauri/permissions/autogenerated/`
- `docs/VALIDATION_STEP04.md` (historical report preserved)

Modified:

- `src-tauri/{Cargo.toml,Cargo.lock,build.rs,capabilities/default.json}` and generated capability schemas
- `src-tauri/src/lib.rs`, `src-tauri/src/storage/{mod,migrations,service}.rs`
- `src/features/devices/Devices.tsx`, `src/features/dashboard/Dashboard.tsx`, `src/features/settings/SettingsPage.tsx`
- `src/app/App.tsx`, `src/domain/models.ts`, `src/services/{contracts,availability}.ts`, `src/styles.css`
- `src/test/{App,storage}.test.tsx`, `scripts/check-schema.mjs`
- `README.md`, `docs/{ARCHITECTURE,ROADMAP,SECURITY,VALIDATION}.md`

No Step 06 engine, snapshot creation, copying, ADB installer, broad fs/shell capability, phone content fixture, private identifier log, branch, commit, merge, or push was added.

## Manual Windows repeat procedure

1. From the repository run `pnpm.cmd tauri dev` with Cargo on PATH. Open Devices without a phone, click Refresh, and verify a completed no-device observation rather than a fabricated phone.
2. Connect/unlock your phone with a data USB cable. Select File Transfer/MTP and approve the phone's prompt yourself. Refresh, select the actual device, inspect available metadata, and click Check file access. A charging-only connection can be absent from WPD entirely.
3. Click Scan WhatsApp Storage to explicitly authorize only metadata discovery. Compare candidate/category visibility with Explorer if available, without opening/copying private databases or media. Test both ordinary WhatsApp and Business only if actually installed/exposed.
4. Unplug while idle: the next visible polling interval (15 seconds plus API duration) should remove the device. Refresh explicitly if needed. Reconnect and select again; do not use an old handle.
5. Repeat during discovery, while locking the phone, and while changing USB mode to charging. Expect denied/disconnected/unavailable/incomplete, not a no-data claim. Cancel and retry after any pending Windows call returns. A vendor call may outlast the cooperative deadline; backpressure prevents additional workers.
6. If possible connect two phones, including one with access denied. Verify distinct issued handles, no duplicate enumeration, per-device selection and no cross-device folder findings.
7. Close/reopen with a phone connected, then without one. Connection state must come from a fresh live scan. Check archives, search, media and theme/vault preferences across restart. Discovery must not create backups or archives.
8. Optionally consent to Save device note. Verify only displayed fields/preference persist and the note never claims a connection or automatically matches an identically named phone after reconnect.

## Acceptance checklist

- [x] Real Windows WPD enumeration and USB transport resolution executed with one phone.
- [x] No-device, scanning, unknown identity, access, denial, disconnection and unsupported states implemented; relevant injected tests passed.
- [x] Android identification is conservative and never inferred from brand, phone type or MTP alone.
- [x] Explicit read-only file-access check succeeded on real hardware.
- [x] Refresh, lifecycle cleanup, backpressure, cancellation and stale-handle invalidation implemented/tested; real idle unplug, native selection clearing, explicit no-device Refresh and dashboard status verified. Remaining reconnect/mode/lock matrix is pending.
- [x] Actual WhatsApp/Business metadata discovery succeeded on the connected phone; restrictions and incomplete results are distinct from absent data.
- [x] No phone content read/copy/mutation/decryption, root, debugging, telemetry, cloud or automatic backup path introduced.
- [x] Existing archives/viewer/search/media/import/storage regression passed; migration preserves existing rows.
- [x] Optional consented notes persist without hardware identifiers or false live state.
- [x] Rust/frontend/build/lint/schema checks executed and passed; real versus mock evidence disclosed.
- [x] Native application exercised with real device and synthetic local content.
- [ ] Complete the remaining physical hardware scenario matrix and broader vendor/driver/installer validation before claiming universal compatibility.
