import { useEffect, useMemo, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { PageHeading } from '../../shared/components/Page';
import { Button } from '../../shared/components/ui/button';
import { useDevices } from '../devices/DeviceProvider';
import { useStorage } from '../storage/StorageProvider';
import { useBackups } from './BackupProvider';
import { BackupError, BackupHistory } from './BackupHistory';
import {
  activePhase,
  backupFailure,
  categoryLabels,
  phaseLabel,
  sourceLabel,
  type BackupFailure,
  type BackupFile,
  type Category,
  type Plan,
  type Source,
} from '../../services/backups';
import { ScanPanel } from './ScanPanel';
import { formatBytes } from '../../services/storage';
const steps = [
  'Device',
  'WhatsApp source',
  'Content selection',
  'Review',
  'Backup',
  'Results',
];
export function BackupPage() {
  const devices = useDevices();
  const storage = useStorage();
  const { service, progress, setProgress } = useBackups();
  const [step, setStep] = useState(0);
  const [deviceId, setDeviceId] = useState('');
  const [source, setSource] = useState<Source>('whatsapp');
  const [scope, setScope] = useState<Category | 'all'>('all');
  const [scanSeconds, setScanSeconds] = useState(60);
  const [scanObjects, setScanObjects] = useState(100000);
  const [scanDepth, setScanDepth] = useState(16);
  const [scanStartedAt, setScanStartedAt] = useState(0);
  const generation = useRef(0);
  useEffect(
    () => () => {
      generation.current++;
    },
    [],
  );
  const [plan, setPlan] = useState<Plan | null>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [onlyPicked, setOnlyPicked] = useState(false);
  const [picked, setPicked] = useState<Map<string, BackupFile>>(new Map());
  const [page, setPage] = useState<BackupFile[]>([]);
  const [fileOffset, setFileOffset] = useState(0);
  const [reread, setReread] = useState(false);
  const [ackIncomplete, setAckIncomplete] = useState(false);
  const [ackCapacity, setAckCapacity] = useState(false);
  const [error, setError] = useState<BackupFailure | null>(null);
  const [busy, setBusy] = useState(false);
  const selectedDevice = devices.list.devices.find((d) => d.id === deviceId);
  const active = activePhase(progress.phase);
  const canUse = service.available();
  const selection = useMemo(() => {
    const items = [...picked.values()].filter((f) =>
      categories.includes(f.category),
    );
    return onlyPicked
      ? {
          count: items.length,
          known: items.reduce((n, f) => n + (f.expectedSize ?? 0), 0),
          unknown: items.filter((f) => f.expectedSize === null).length,
          ids: items.map((f) => f.id),
        }
      : {
          count:
            plan?.categories
              .filter((c) => categories.includes(c.category))
              .reduce((n, c) => n + c.files, 0) ?? 0,
          known:
            plan?.categories
              .filter((c) => categories.includes(c.category))
              .reduce((n, c) => n + c.knownBytes, 0) ?? 0,
          unknown:
            plan?.categories
              .filter((c) => categories.includes(c.category))
              .reduce((n, c) => n + c.unknownSizes, 0) ?? 0,
          ids: null,
        };
  }, [categories, picked, onlyPicked, plan]);
  useEffect(() => {
    let mounted = true;
    if (plan)
      service
        .planFiles(plan.id, null, fileOffset)
        .then((v) => {
          if (mounted) setPage(v);
        })
        .catch((e) => {
          if (mounted) setError(backupFailure(e));
        });
    return () => {
      mounted = false;
    };
  }, [service, plan, fileOffset]);
  useEffect(() => {
    if (step === 4 && progress.snapshotId && !active) {
      const timer = setTimeout(() => {
        setStep(5);
        void storage.refresh();
      }, 0);
      return () => clearTimeout(timer);
    }
  }, [step, progress.snapshotId, active, storage]);
  async function scan() {
    const request = ++generation.current;
    setBusy(true);
    setPlan(null);
    setScanStartedAt(Date.now());
    setError(null);
    try {
      const result = await service.prepare(deviceId, source, {
        categories: scope === 'all' ? [] : [scope],
        maxObjects: scanObjects,
        maxFiles: 50000,
        maxDepth: scanDepth,
        timeoutSeconds: scanSeconds,
      });
      if (generation.current !== request) return;
      setPlan(result);
      setCategories(
        result.categories.filter((c) => c.files > 0).map((c) => c.category),
      );
      setPicked(new Map());
      setFileOffset(0);
      setAckCapacity(false);
      setAckIncomplete(false);
      setStep(2);
    } catch (e) {
      if (generation.current === request) setError(backupFailure(e));
    } finally {
      if (generation.current === request) setBusy(false);
    }
  }
  async function start() {
    if (!plan) return;
    setBusy(true);
    setError(null);
    try {
      setProgress(
        await service.start(plan.id, {
          categories,
          fileIds: selection.ids,
          sourceReread: reread,
          acknowledgeIncomplete: ackIncomplete,
          acknowledgeCapacity: ackCapacity,
        }),
      );
      setStep(4);
    } catch (e) {
      setError(backupFailure(e));
      setStep(1);
    } finally {
      setBusy(false);
    }
  }
  const blocked = busy || active;
  const noSpace =
    plan?.availableBytes !== null &&
    plan?.availableBytes !== undefined &&
    selection.known + 16 * 1024 * 1024 > plan.availableBytes;
  return (
    <>
      <PageHeading
        eyebrow="LOCAL PHONE COPIES"
        title="Back up WhatsApp files"
        description="Copy accessible encrypted backups and media to your local vault, then verify the copies."
      />
      <ol className="backup-steps" aria-label="Backup workflow">
        {steps.map((label, i) => (
          <li key={label} aria-current={step === i ? 'step' : undefined}>
            {i + 1}. {label}
          </li>
        ))}
      </ol>
      <BackupError error={error} />
      {!canUse && (
        <p role="status">
          USB backups require ChatVault Desktop on Windows. Browser preview does
          not copy phone files.
        </p>
      )}
      <section className="panel backup-wizard" aria-busy={busy}>
        {step === 0 && (
          <>
            <h2>1. Select your device</h2>
            <p>
              Connect and unlock your phone, choose File Transfer/MTP, then
              refresh. Only files your phone permits can be copied.
            </p>
            <div className="backup-actions">
              <Button
                variant="outline"
                disabled={!canUse || blocked}
                onClick={() => void devices.refresh()}
              >
                Refresh devices
              </Button>
              <Link to="/devices">Connection guide and diagnostics</Link>
            </div>
            {devices.list.devices.length === 0 ? (
              <p>No device connected or live device state is unavailable.</p>
            ) : (
              <fieldset disabled={blocked}>
                <legend>Connected portable devices</legend>
                {devices.list.devices.map((d) => (
                  <label key={d.id}>
                    <input
                      type="radio"
                      name="backup-device"
                      checked={deviceId === d.id}
                      disabled={d.transport !== 'usb'}
                      onChange={() => setDeviceId(d.id)}
                    />
                    {d.name ?? 'Name unavailable'} ·{' '}
                    {d.transport === 'usb' ? 'USB' : 'File access unavailable'}
                  </label>
                ))}
              </fieldset>
            )}
            <Button
              disabled={!canUse || blocked || !selectedDevice}
              onClick={() => setStep(1)}
            >
              Next: WhatsApp source
            </Button>
          </>
        )}
        {step === 1 && (
          <>
            <h2>2. Choose one WhatsApp source</h2>
            <fieldset disabled={blocked}>
              <legend>Source application</legend>
              <label>
                <input
                  type="radio"
                  name="source"
                  checked={source === 'whatsapp'}
                  onChange={() => setSource('whatsapp')}
                />
                WhatsApp
              </label>
              <label>
                <input
                  type="radio"
                  name="source"
                  checked={source === 'business'}
                  onChange={() => setSource('business')}
                />
                WhatsApp Business
              </label>
            </fieldset>
            <p>
              Discovery checks conventional accessible folders for this
              application only. Sources are never merged. No file contents are
              read during this inventory scan.
            </p>
            <label>
              Discovery scope
              <select
                value={scope}
                disabled={blocked}
                onChange={(e) => setScope(e.target.value as Category | 'all')}
              >
                <option value="all">All accessible categories</option>
                <option value="databases">
                  Encrypted databases only (quick scan)
                </option>
                {(
                  [
                    'images',
                    'videos',
                    'documents',
                    'audio',
                    'voice_notes',
                    'other',
                  ] as Category[]
                ).map((c) => (
                  <option key={c} value={c}>
                    {categoryLabels[c]} only
                  </option>
                ))}
              </select>
            </label>
            <details className="scan-limits">
              <summary>Scan limits</summary>
              <label>
                Time budget (seconds)
                <input
                  type="number"
                  min="1"
                  max="180"
                  value={scanSeconds}
                  disabled={blocked}
                  onChange={(e) =>
                    setScanSeconds(
                      Math.min(180, Math.max(1, Number(e.target.value) || 1)),
                    )
                  }
                />
              </label>
              <label>
                Object budget
                <input
                  type="number"
                  min="1"
                  max="100000"
                  value={scanObjects}
                  disabled={blocked}
                  onChange={(e) =>
                    setScanObjects(
                      Math.min(
                        100000,
                        Math.max(1, Number(e.target.value) || 1),
                      ),
                    )
                  }
                />
              </label>
              <label>
                Maximum category depth
                <input
                  type="number"
                  min="0"
                  max="16"
                  value={scanDepth}
                  disabled={blocked}
                  onChange={(e) =>
                    setScanDepth(
                      Math.min(16, Math.max(0, Number(e.target.value) || 0)),
                    )
                  }
                />
              </label>
              <p>
                Limits return an explicitly incomplete inventory. A Windows call
                may outlast the time budget; the device stays reserved until it
                returns.
              </p>
            </details>
            <div className="backup-actions">
              <Button
                variant="outline"
                disabled={blocked}
                onClick={() => setStep(0)}
              >
                Back to device
              </Button>
              <Button
                disabled={!canUse || blocked || !deviceId}
                onClick={() => void scan()}
              >
                Discover backup files
              </Button>
            </div>
          </>
        )}
        {step === 2 && plan && (
          <>
            <h2>3. Choose content</h2>
            <p>
              Scanned scope:{' '}
              {plan.scannedCategories?.length
                ? plan.scannedCategories
                    .map((c) => categoryLabels[c])
                    .join(', ')
                : 'All supported categories'}
              . Unscanned categories are not inventoried.
            </p>
            <p>
              {sourceLabel(plan.source)} ·{' '}
              {plan.deviceName ?? 'Device name unavailable'} · {plan.totalFiles}{' '}
              inventoried files · {formatBytes(plan.knownBytes)} known bytes ·{' '}
              {plan.unknownSizes} unknown-size files. {plan.skippedEntries}{' '}
              unsupported entries skipped; {plan.inaccessibleLocations}{' '}
              restricted locations or entries observed. Files inside
              inaccessible folders cannot be counted.
            </p>
            <p>
              Inventory:{' '}
              {plan.complete
                ? 'Complete within supported candidate locations'
                : 'Incomplete — restrictions or limits encountered'}
              . {plan.warnings.join(', ')}
            </p>
            {plan.totalFiles === 0 && (
              <p role="status">
                No supported files were inventoried. Missing or inaccessible
                folders do not prove that WhatsApp data is absent. Unlock the
                phone and rescan.
              </p>
            )}
            <div className="backup-actions">
              <Button
                variant="outline"
                onClick={() =>
                  setCategories(plan.categories.map((c) => c.category))
                }
              >
                Select All
              </Button>
              <Button variant="outline" onClick={() => setCategories([])}>
                Deselect All
              </Button>
            </div>
            <fieldset>
              <legend>Available content categories</legend>
              {(Object.keys(categoryLabels) as Category[]).map((category) => {
                const count = plan.categories.find(
                  (c) => c.category === category,
                );
                return (
                  <label key={category}>
                    <input
                      type="checkbox"
                      checked={categories.includes(category)}
                      disabled={!count?.files || blocked}
                      onChange={(e) =>
                        setCategories((old) =>
                          e.target.checked
                            ? [...old, category]
                            : old.filter((c) => c !== category),
                        )
                      }
                    />
                    {categoryLabels[category]} · {count?.files ?? 0} files ·{' '}
                    {count ? formatBytes(count.knownBytes) : 'Unavailable'}
                    {count?.unknownSizes
                      ? ` + ${count.unknownSizes} unknown-size`
                      : ''}
                  </label>
                );
              })}
            </fieldset>
            <label>
              <input
                type="checkbox"
                checked={onlyPicked}
                onChange={(e) => setOnlyPicked(e.target.checked)}
              />
              Choose individual files instead of all files in the selected
              categories
            </label>
            {onlyPicked && (
              <>
                <p>
                  Selected {selection.count} files across pages (maximum
                  10,000). Filenames are shown locally only.
                </p>
                <ul className="backup-file-list">
                  {page.map((f) => (
                    <li key={f.id}>
                      <label>
                        <input
                          type="checkbox"
                          disabled={!categories.includes(f.category) || blocked}
                          checked={picked.has(f.id)}
                          onChange={(e) =>
                            setPicked((old) => {
                              const next = new Map(old);
                              if (e.target.checked) next.set(f.id, f);
                              else next.delete(f.id);
                              return next;
                            })
                          }
                        />
                        {f.originalName} · {formatBytes(f.expectedSize)}
                      </label>
                      <small>{categoryLabels[f.category]}</small>
                    </li>
                  ))}
                </ul>
                <div className="backup-actions">
                  <Button
                    variant="outline"
                    disabled={fileOffset === 0}
                    onClick={() => setFileOffset((v) => Math.max(0, v - 50))}
                  >
                    Previous inventory files
                  </Button>
                  <Button
                    variant="outline"
                    disabled={page.length < 50}
                    onClick={() => setFileOffset((v) => v + 50)}
                  >
                    Next inventory files
                  </Button>
                </div>
              </>
            )}
            <p>
              Selection: {selection.count} files ·{' '}
              {formatBytes(selection.known)} known bytes
              {selection.unknown
                ? ` + ${selection.unknown} unknown-size files`
                : ''}
              .
            </p>
            <div className="backup-actions">
              <Button
                variant="outline"
                onClick={() => {
                  setPlan(null);
                  setStep(1);
                }}
              >
                Rescan source
              </Button>
              <Button
                disabled={blocked || selection.count === 0}
                onClick={() => setStep(3)}
              >
                Review backup
              </Button>
            </div>
          </>
        )}
        {step === 3 && plan && (
          <>
            <h2>4. Review before copying</h2>
            <p>
              <strong>{sourceLabel(plan.source)}</strong> ·{' '}
              {plan.deviceName ?? 'Unavailable'} · {selection.count} selected
              files
            </p>
            <p>
              Estimated minimum: {formatBytes(selection.known)}
              {selection.unknown ? ' plus unknown-size files' : ''}. Available
              on vault drive: {formatBytes(plan.availableBytes)}. Space is
              checked again immediately before copying.
            </p>
            <p className="storage-path">Destination vault: {plan.vaultPath}</p>
            {noSpace && (
              <p role="alert">
                Insufficient space for this selection and safety reserve. Choose
                fewer files or free space.
              </p>
            )}
            <label>
              <input
                type="checkbox"
                checked={reread}
                onChange={(e) => setReread(e.target.checked)}
              />
              Independently re-read each source stream and compare SHA-256 (uses
              additional USB reads)
            </label>
            <p>
              Every file always gets incoming-stream SHA-256 and an independent
              local read-back hash. Without the optional source re-read,
              verification covers transfer/write consistency, not an independent
              source checksum. Expected-size checks apply only when the driver
              supplies a size.
            </p>
            {!plan.complete && (
              <label>
                <input
                  type="checkbox"
                  checked={ackIncomplete}
                  onChange={(e) => setAckIncomplete(e.target.checked)}
                />
                I understand this inventory is incomplete and only the listed
                selected files will be copied.
              </label>
            )}
            {(plan.availableBytes === null || selection.unknown > 0) && (
              <label>
                <input
                  type="checkbox"
                  checked={ackCapacity}
                  onChange={(e) => setAckCapacity(e.target.checked)}
                />
                I understand available capacity or some file sizes are unknown;
                copying may fail if the vault fills.
              </label>
            )}
            <p>
              Native databases remain encrypted. The vault is currently
              unencrypted. Live WhatsApp files may change during copying; this
              is not an atomic database snapshot or a guarantee of native
              restorability.
            </p>
            <div className="backup-actions">
              <Button
                variant="outline"
                disabled={blocked}
                onClick={() => setStep(2)}
              >
                Back to selection
              </Button>
              <Button
                disabled={
                  blocked ||
                  !canUse ||
                  noSpace ||
                  (!plan.complete && !ackIncomplete) ||
                  ((plan.availableBytes === null || selection.unknown > 0) &&
                    !ackCapacity)
                }
                onClick={() => void start()}
              >
                Backup Now
              </Button>
            </div>
          </>
        )}
        {((progress.scan && !progress.snapshotId) || (busy && step === 1)) && (
          <ScanPanel
            key={progress.operationId ?? 'pending-scan'}
            progress={progress}
            pending={busy && step === 1}
            startedAt={scanStartedAt}
            cancel={() => service.cancel()}
            onError={(e) => setError(backupFailure(e))}
          />
        )}
        {(step >= 4 || (active && !!progress.snapshotId)) && (
          <section aria-label="Backup progress">
            <h2>{phaseLabel(progress.phase)}</h2>
            <p>
              {sourceLabel(progress.source)} · {progress.successfulFiles}/
              {progress.totalFiles} files verified · {progress.failedFiles}{' '}
              failed
            </p>
            {progress.currentFilename && (
              <p>Current file: {progress.currentFilename}</p>
            )}
            <p>
              {formatBytes(progress.copiedBytes)} copied /{' '}
              {progress.unknownSizes
                ? 'total size unknown'
                : formatBytes(progress.knownTotalBytes)}{' '}
              · {formatBytes(progress.verifiedBytes)} locally re-read
              {progress.rateBytesPerSecond !== null
                ? ` · ${formatBytes(progress.rateBytesPerSecond)}/s average transfer rate`
                : ''}
            </p>
            {active &&
            progress.unknownSizes === 0 &&
            progress.knownTotalBytes > 0 ? (
              <progress
                aria-label="Copied bytes"
                value={Math.min(progress.copiedBytes, progress.knownTotalBytes)}
                max={progress.knownTotalBytes}
              />
            ) : active ? (
              <progress aria-label="Backup operation in progress" />
            ) : null}
            <BackupError error={progress.error} />
            {active ? (
              <>
                <Button
                  variant="outline"
                  disabled={progress.cancelRequested}
                  onClick={() =>
                    void service
                      .cancel()
                      .catch((e) => setError(backupFailure(e)))
                  }
                >
                  Cancel Backup
                </Button>
                {progress.cancelRequested && (
                  <p>
                    Cancel requested. Waiting for the current Windows call or
                    safe stream boundary; do not assume cancellation has
                    completed yet.
                  </p>
                )}
              </>
            ) : (
              <>
                <p>
                  {['completed', 'completed_with_warnings'].includes(
                    progress.phase,
                  )
                    ? 'The selected files and manifest were published as a verified local snapshot. Review any warnings in history.'
                    : 'No incomplete work is counted as a successful backup. Review the job in history, then rescan to retry.'}
                </p>
                <Button
                  variant="outline"
                  onClick={() => {
                    setStep(0);
                    setPlan(null);
                  }}
                >
                  Start a new backup
                </Button>
              </>
            )}
          </section>
        )}
      </section>
      <BackupHistory />
    </>
  );
}
