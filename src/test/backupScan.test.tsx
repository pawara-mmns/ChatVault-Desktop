import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi, afterEach } from 'vitest';
import { ScanPanel } from '../features/backups/ScanPanel';
import { BackupProvider, useBackups } from '../features/backups/BackupProvider';
import {
  idleProgress,
  type BackupProgress,
  type BackupService,
  type ScanReport,
} from '../services/backups';
const scan: ScanReport = {
  elapsedMs: 14000,
  objects: 330,
  folders: 12,
  files: 318,
  inventoriedFiles: 180,
  knownBytes: 12345,
  operation: 'next_objects',
  operationPending: true,
  operationElapsedMs: 4000,
  lastSuccessfulOperation: 'object_properties',
  successfulCalls: 348,
  errors: 0,
  errorCategory: null,
  diagnostic: null,
  slow: true,
  deadlineReached: false,
  timeoutSeconds: 60,
  maxObjects: 100000,
  maxDepth: 16,
};
const progress: BackupProgress = {
  ...idleProgress,
  phase: 'scanning',
  operationId: 'scan-1',
  revision: 2,
  scan,
};
afterEach(() => vi.useRealTimers());
it('discovery shows real counters and elapsed time without copy/verification feedback or percentages', () => {
  render(
    <ScanPanel
      progress={progress}
      pending
      startedAt={0}
      cancel={vi.fn()}
      onError={vi.fn()}
    />,
  );
  expect(screen.getByText(/330 objects examined/)).toHaveTextContent('14 s');
  expect(screen.getByText(/scan is taking longer/)).toBeInTheDocument();
  const indicator = screen.getByRole('progressbar', {
    name: 'Discovery in progress',
  });
  expect(indicator).not.toHaveAttribute('aria-valuenow');
  expect(
    screen.queryByText(/files verified|locally re-read|0\/0/),
  ).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Cancel Scan' })).toBeEnabled();
});
it('cancel scan requests cancellation and truthfully waits for Windows', async () => {
  const cancel = vi.fn().mockResolvedValue(undefined);
  render(
    <ScanPanel
      progress={progress}
      pending
      startedAt={0}
      cancel={cancel}
      onError={vi.fn()}
    />,
  );
  await userEvent.click(screen.getByRole('button', { name: 'Cancel Scan' }));
  expect(cancel).toHaveBeenCalledOnce();
  expect(screen.getByRole('button', { name: 'Cancel Scan' })).toBeDisabled();
  expect(
    screen.getByText(/pending driver call cannot always be interrupted/),
  ).toBeInTheDocument();
});
it('a returned timeout retains a waiting state until the worker releases the device', () => {
  render(
    <ScanPanel
      progress={{
        ...progress,
        phase: 'scan_waiting',
        cancelRequested: true,
        scan: { ...scan, elapsedMs: 62000, deadlineReached: true },
      }}
      pending={false}
      startedAt={0}
      cancel={vi.fn()}
      onError={vi.fn()}
    />,
  );
  expect(
    screen.getByText(/conflicting scan cannot start yet/),
  ).toBeInTheDocument();
  expect(screen.getByRole('progressbar')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Cancel Scan' })).toBeDisabled();
});
it('cancel errors remain actionable and permit another cancel attempt', async () => {
  const error = { code: 'busy', message: 'Windows is returning.' };
  const onError = vi.fn();
  render(
    <ScanPanel
      progress={progress}
      pending
      startedAt={0}
      cancel={vi.fn().mockRejectedValue(error)}
      onError={onError}
    />,
  );
  await userEvent.click(screen.getByRole('button', { name: 'Cancel Scan' }));
  await waitFor(() => expect(onError).toHaveBeenCalledWith(error));
  expect(screen.getByRole('button', { name: 'Cancel Scan' })).toBeEnabled();
});
function Probe() {
  const { progress, setProgress } = useBackups();
  return (
    <>
      <p data-testid="phase">{progress.phase}</p>
      <button
        onClick={() =>
          setProgress({
            ...idleProgress,
            phase: 'ready',
            revision: 10,
            updatedAt: 100,
          })
        }
      >
        Finish
      </button>
      <button
        onClick={() =>
          setProgress({
            ...idleProgress,
            phase: 'scanning',
            revision: 9,
            updatedAt: 200,
          })
        }
      >
        Old event
      </button>
    </>
  );
}
it('revision ordering rejects late scan events even if their timestamp is newer', async () => {
  const s = { available: () => false } as BackupService;
  render(
    <BackupProvider service={s}>
      <Probe />
    </BackupProvider>,
  );
  await userEvent.click(screen.getByRole('button', { name: 'Finish' }));
  await userEvent.click(screen.getByRole('button', { name: 'Old event' }));
  expect(screen.getByTestId('phase')).toHaveTextContent('ready');
});
it('progress polling keeps counts advancing and is cleared on unmount', async () => {
  vi.useFakeTimers();
  let revision = 1;
  const s = {
    available: () => true,
    progress: vi.fn().mockImplementation(async () => ({
      ...progress,
      revision: revision++,
      updatedAt: revision,
      scan: { ...scan, objects: revision * 10 },
    })),
  } as unknown as BackupService;
  function Counts() {
    const { progress } = useBackups();
    return <p data-testid="objects">{progress.scan?.objects}</p>;
  }
  const view = render(
    <BackupProvider service={s}>
      <Counts />
    </BackupProvider>,
  );
  await act(async () => {
    await vi.advanceTimersByTimeAsync(2100);
  });
  expect(s.progress).toHaveBeenCalledTimes(3);
  expect(Number(screen.getByTestId('objects').textContent)).toBeGreaterThan(20);
  view.unmount();
  const calls = vi.mocked(s.progress).mock.calls.length;
  await act(async () => {
    await vi.advanceTimersByTimeAsync(3000);
  });
  expect(s.progress).toHaveBeenCalledTimes(calls);
});
