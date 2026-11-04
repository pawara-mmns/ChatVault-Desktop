import { useEffect, useState } from 'react';
import { Button } from '../../shared/components/ui/button';
import type { BackupProgress } from '../../services/backups';
const labels: Record<string, string> = {
  enum_objects: 'Opening a directory enumeration',
  next_objects: 'Reading the next object batch',
  object_properties: 'Reading object metadata',
  enumerate_children: 'Enumerating a directory',
  initialize_com: 'Initializing Windows access',
  create_manager: 'Opening Windows device discovery',
  enumerate_devices: 'Checking the connected device',
  refresh_devices: 'Refreshing Windows devices',
  open_device: 'Opening the read-only device session',
  open_content: 'Opening device content',
  open_properties: 'Opening device metadata',
  device_properties: 'Checking device properties',
  close_device: 'Closing the device session',
};
export function ScanPanel({
  progress,
  pending,
  startedAt,
  cancel,
  onError,
}: {
  progress: BackupProgress;
  pending: boolean;
  startedAt: number;
  cancel(): Promise<void>;
  onError(e: unknown): void;
}) {
  const [elapsed, setElapsed] = useState(0);
  const [requested, setRequested] = useState(false);
  const scan = progress.scan;
  const scanning =
    pending || ['scanning', 'scan_waiting'].includes(progress.phase);
  useEffect(() => {
    if (!scanning) return;
    const timer = window.setInterval(
      () => setElapsed(startedAt ? Date.now() - startedAt : 0),
      500,
    );
    return () => clearInterval(timer);
  }, [scanning, startedAt]);
  const seconds = Math.floor((scan?.elapsedMs ?? elapsed) / 1000);
  const cancelling = requested || progress.cancelRequested;
  return (
    <section
      className="scan-panel"
      aria-label="Backup discovery progress"
      aria-live="polite"
    >
      <h2>
        {scanning
          ? 'Discovering accessible files'
          : progress.phase === 'ready'
            ? 'Discovery finished'
            : 'Discovery stopped'}
      </h2>
      <p>
        Elapsed: {seconds} s.{' '}
        {scan
          ? `${scan.directories ?? 0} directory visits; ${scan.objects} objects examined; ${scan.folders} folder observations; ${scan.files} file observations; ${scan.inventoriedFiles} supported files inventoried.`
          : 'Waiting for Windows to report scan counters.'}
      </p>
      {scanning && (
        <div
          className="scan-indeterminate"
          role="progressbar"
          aria-label="Discovery in progress"
        >
          <span />
        </div>
      )}
      {scan?.operationPending && (
        <p>
          {labels[scan.operation ?? ''] ?? 'Waiting for Windows'} (
          {Math.floor(scan.operationElapsedMs / 1000)} s in this call).
        </p>
      )}
      {scanning && (scan?.slow || seconds >= 10) && (
        <p role="status">
          This scan is taking longer. Large media folders and Windows MTP
          metadata calls can be slow. Try a single category or a smaller object
          budget on the next scan. No file contents are being copied.
        </p>
      )}
      {scanning &&
        (progress.phase === 'scan_waiting' || scan?.deadlineReached) && (
          <p role="status">
            The time budget has elapsed. Waiting for the Windows worker to
            return safely; it may still be inside a driver call or closing its
            session. A conflicting scan cannot start yet.
          </p>
        )}
      {scanning && (
        <Button
          variant="outline"
          disabled={cancelling}
          onClick={() => {
            setRequested(true);
            void cancel().catch((e) => {
              setRequested(false);
              onError(e);
            });
          }}
        >
          Cancel Scan
        </Button>
      )}
      {scanning && cancelling && (
        <p>
          Cancel requested. The scan will stop between Windows operations. A
          pending driver call cannot always be interrupted; retry becomes
          available after the worker releases the device.
        </p>
      )}
      {scan && (
        <details>
          <summary>Sanitized scan diagnostics</summary>
          <p>
            Last successful operation:{' '}
            {scan.lastSuccessfulOperation ?? 'None yet'}. Successful calls:{' '}
            {scan.successfulCalls}. Errors: {scan.errors}.{' '}
            {scan.errorCategory ?? ''} {scan.diagnostic ?? ''}
          </p>
          <p>
            Object budget: {scan.maxObjects}; depth: {scan.maxDepth}; time:{' '}
            {scan.timeoutSeconds} s. Folder/object counts describe actual
            observations, including repeated parent visits.
          </p>
        </details>
      )}
    </section>
  );
}
