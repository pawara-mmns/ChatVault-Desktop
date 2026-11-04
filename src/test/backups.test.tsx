import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { expect, it, vi } from 'vitest';
import { BackupPage } from '../features/backups/BackupPage';
import { BackupProvider } from '../features/backups/BackupProvider';
import { DeviceProvider } from '../features/devices/DeviceProvider';
import { StorageProvider } from '../features/storage/StorageProvider';
import {
  idleProgress,
  type BackupFile,
  type BackupService,
  type Plan,
  type Snapshot,
} from '../services/backups';
import { deviceFixture, phone } from './deviceFixture';
import { storageFixture } from './storageFixture';

const file: BackupFile = {
  id: 'file-1',
  category: 'images',
  sourceRelative: 'Media/Images/synthetic.jpg',
  localRelative: 'media/images/id-synthetic.jpg',
  originalName: 'synthetic.jpg',
  expectedSize: 8193,
  actualBytes: 8193,
  sha256: 'a'.repeat(64),
  localVerification: 'verified',
  sourceVerification: 'verified',
  sizeValidation: 'matched',
  state: 'verified',
  errorCode: null,
};
const plan: Plan = {
  skippedEntries: 0,
  inaccessibleLocations: 0,
  id: 'plan-1',
  deviceId: 'fixture-handle-1',
  deviceName: 'Synthetic phone',
  source: 'business',
  complete: true,
  warnings: [],
  categories: [
    { category: 'images', files: 1, knownBytes: 8193, unknownSizes: 0 },
  ],
  totalFiles: 1,
  knownBytes: 8193,
  unknownSizes: 0,
  availableBytes: 1024 * 1024 * 1024,
  vaultPath: 'C:\\Synthetic\\vault',
};
const snapshot: Snapshot = {
  id: 'snapshot-1',
  sourceApp: 'business',
  deviceName: 'Synthetic phone',
  phase: 'completed',
  createdAt: 1000,
  completedAt: 2000,
  fileCount: 1,
  successfulCount: 1,
  failedCount: 0,
  skippedCount: 0,
  totalBytes: 8193,
  verifiedBytes: 8193,
  manifestPath: 'backups/snapshot-1/manifest.json',
  localIntegrity: 'verified',
  localVerifiedAt: 2000,
  warnings: [],
  destination: 'C:\\Synthetic\\vault\\backups\\snapshot-1',
};
function service(): BackupService {
  return {
    available: () => true,
    prepare: vi.fn().mockResolvedValue(plan),
    planFiles: vi.fn().mockResolvedValue([file]),
    start: vi.fn().mockResolvedValue({
      ...idleProgress,
      phase: 'copying',
      snapshotId: 'snapshot-1',
      source: 'business',
      currentFilename: 'synthetic.jpg',
      totalFiles: 1,
      copiedBytes: 4000,
      knownTotalBytes: 8193,
      updatedAt: 1,
    }),
    progress: vi.fn().mockResolvedValue(idleProgress),
    cancel: vi.fn().mockResolvedValue(undefined),
    history: vi.fn().mockResolvedValue({
      items: [],
      total: 0,
      completedCount: 0,
      completedBytes: 0,
    }),
    files: vi.fn().mockResolvedValue([file]),
    verify: vi.fn().mockResolvedValue({
      ...idleProgress,
      snapshotId: 'snapshot-1',
      phase: 'verification_completed',
      totalFiles: 1,
      successfulFiles: 1,
      updatedAt: 3,
    }),
    open: vi.fn().mockResolvedValue(undefined),
    cleanup: vi.fn().mockResolvedValue(undefined),
  };
}
function page(s: BackupService, connected = true) {
  const devices = deviceFixture(connected ? [phone()] : []);
  return render(
    <MemoryRouter>
      <StorageProvider service={storageFixture()}>
        <DeviceProvider service={devices}>
          <BackupProvider service={s}>
            <BackupPage />
          </BackupProvider>
        </DeviceProvider>
      </StorageProvider>
    </MemoryRouter>,
  );
}
async function discover() {
  await userEvent.click(
    await screen.findByRole('radio', { name: /Synthetic phone/ }),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Next: WhatsApp source' }),
  );
  await userEvent.click(
    screen.getByRole('radio', { name: 'WhatsApp Business' }),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Discover backup files' }),
  );
  await screen.findByRole('heading', { name: '3. Choose content' });
}
it('requires a device and never starts copying during discovery', async () => {
  const s = service();
  page(s);
  expect(
    screen.getByRole('button', { name: 'Next: WhatsApp source' }),
  ).toBeDisabled();
  await discover();
  expect(s.prepare).toHaveBeenCalledWith('fixture-handle-1', 'business', {
    categories: [],
    maxObjects: 100000,
    maxFiles: 50000,
    maxDepth: 16,
    timeoutSeconds: 60,
  });
  expect(s.start).not.toHaveBeenCalled();
  expect(screen.getByRole('checkbox', { name: /^Videos/ })).toBeDisabled();
});
it('a quick database-only preflight passes the bounded scope before media traversal', async () => {
  const s = service();
  page(s);
  await userEvent.click(
    await screen.findByRole('radio', { name: /Synthetic phone/ }),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Next: WhatsApp source' }),
  );
  await userEvent.selectOptions(
    screen.getByLabelText('Discovery scope'),
    'databases',
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Discover backup files' }),
  );
  await waitFor(() =>
    expect(s.prepare).toHaveBeenCalledWith(
      'fixture-handle-1',
      'whatsapp',
      expect.objectContaining({
        categories: ['databases'],
        timeoutSeconds: 60,
      }),
    ),
  );
  expect(s.start).not.toHaveBeenCalled();
});
it('an active discovery shows Cancel Scan and no copy verification counters', async () => {
  const s = service();
  vi.mocked(s.progress).mockResolvedValue({
    ...idleProgress,
    phase: 'scanning',
    source: 'business',
    revision: 1,
    scan: {
      elapsedMs: 15000,
      objects: 128,
      folders: 14,
      files: 114,
      inventoriedFiles: 80,
      knownBytes: 1234,
      operation: 'next_objects',
      operationPending: true,
      operationElapsedMs: 400,
      lastSuccessfulOperation: 'object_properties',
      successfulCalls: 200,
      errors: 0,
      errorCategory: null,
      diagnostic: null,
      slow: true,
      deadlineReached: false,
      timeoutSeconds: 60,
      maxObjects: 100000,
      maxDepth: 16,
    },
  });
  page(s);
  expect(
    await screen.findByRole('button', { name: 'Cancel Scan' }),
  ).toBeEnabled();
  expect(
    screen.queryByRole('region', { name: 'Backup progress' }),
  ).not.toBeInTheDocument();
  expect(
    screen.queryByText(/files verified|locally re-read/),
  ).not.toBeInTheDocument();
  expect(
    screen.getByRole('button', { name: 'Next: WhatsApp source' }),
  ).toBeDisabled();
});
it('selection supports deselect and only real discovered categories', async () => {
  page(service());
  await discover();
  await userEvent.click(screen.getByRole('button', { name: 'Deselect All' }));
  expect(screen.getByRole('button', { name: 'Review backup' })).toBeDisabled();
  await userEvent.click(screen.getByRole('button', { name: 'Select All' }));
  expect(screen.getByRole('checkbox', { name: /^Images/ })).toBeChecked();
  expect(screen.getByRole('button', { name: 'Review backup' })).toBeEnabled();
});
it('individual file selection passes only issued file IDs after explicit confirmation', async () => {
  const s = service();
  page(s);
  await discover();
  await userEvent.click(
    screen.getByRole('checkbox', { name: /Choose individual files/ }),
  );
  await userEvent.click(
    await screen.findByRole('checkbox', { name: /synthetic.jpg/ }),
  );
  await userEvent.click(screen.getByRole('button', { name: 'Review backup' }));
  expect(screen.getByText(/Destination vault:/)).toHaveTextContent(
    plan.vaultPath,
  );
  expect(s.start).not.toHaveBeenCalled();
  await userEvent.click(
    screen.getByRole('checkbox', { name: /Independently re-read/ }),
  );
  await userEvent.click(screen.getByRole('button', { name: 'Backup Now' }));
  await waitFor(() =>
    expect(s.start).toHaveBeenCalledWith('plan-1', {
      categories: ['images'],
      fileIds: ['file-1'],
      sourceReread: true,
      acknowledgeIncomplete: false,
      acknowledgeCapacity: false,
    }),
  );
  expect(
    await screen.findByText('Current file: synthetic.jpg'),
  ).toBeInTheDocument();
  expect(
    screen.getByRole('progressbar', { name: 'Copied bytes' }),
  ).toHaveAttribute('value', '4000');
  await userEvent.click(screen.getByRole('button', { name: 'Cancel Backup' }));
  expect(s.cancel).toHaveBeenCalledTimes(1);
});
it('incomplete inventory and unknown capacity each require acknowledgement', async () => {
  const s = service();
  vi.mocked(s.prepare).mockResolvedValue({
    ...plan,
    complete: false,
    availableBytes: null,
    warnings: ['access_denied'],
  });
  page(s);
  await discover();
  await userEvent.click(screen.getByRole('button', { name: 'Review backup' }));
  const start = screen.getByRole('button', { name: 'Backup Now' });
  expect(start).toBeDisabled();
  await userEvent.click(
    screen.getByRole('checkbox', { name: /I understand this inventory/ }),
  );
  expect(start).toBeDisabled();
  await userEvent.click(
    screen.getByRole('checkbox', { name: /I understand available capacity/ }),
  );
  expect(start).toBeEnabled();
});
it('insufficient known space prevents confirmation', async () => {
  const s = service();
  vi.mocked(s.prepare).mockResolvedValue({ ...plan, availableBytes: 8193 });
  page(s);
  await discover();
  await userEvent.click(screen.getByRole('button', { name: 'Review backup' }));
  expect(screen.getByRole('alert')).toHaveTextContent('Insufficient space');
  expect(screen.getByRole('button', { name: 'Backup Now' })).toBeDisabled();
});
it('vault access failure is actionable and never implies success', async () => {
  const s = service();
  vi.mocked(s.prepare).mockRejectedValue({
    code: 'vault_unavailable',
    message: 'Reconnect the vault drive.',
    diagnostic: null,
  });
  page(s);
  await userEvent.click(
    await screen.findByRole('radio', { name: /Synthetic phone/ }),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Next: WhatsApp source' }),
  );
  await userEvent.click(
    screen.getByRole('button', { name: 'Discover backup files' }),
  );
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Reconnect the vault drive.',
  );
  expect(s.start).not.toHaveBeenCalled();
});
it('browser preview and no-device states do not invent connected phones', async () => {
  const s = service();
  s.available = () => false;
  page(s, false);
  expect(screen.getByText(/USB backups require/)).toBeInTheDocument();
  expect(
    screen.getByRole('button', { name: 'Refresh devices' }),
  ).toBeDisabled();
  expect(s.progress).not.toHaveBeenCalled();
});
it('history exposes actual hashes, verification, and validated open actions', async () => {
  const s = service();
  vi.mocked(s.history).mockResolvedValue({
    items: [snapshot],
    total: 1,
    completedCount: 1,
    completedBytes: 8193,
  });
  page(s);
  await userEvent.click(await screen.findByRole('button', { name: 'Details' }));
  expect(await screen.findByText('a'.repeat(64))).toBeInTheDocument();
  expect(screen.getByText(snapshot.destination)).toBeInTheDocument();
  await userEvent.click(
    screen.getByRole('button', { name: 'Re-verify local integrity' }),
  );
  await waitFor(() => expect(s.verify).toHaveBeenCalledWith('snapshot-1'));
  await userEvent.click(
    screen.getByRole('button', { name: 'Open backup folder' }),
  );
  expect(s.open).toHaveBeenCalledWith('snapshot-1');
  expect(s.open).toHaveBeenCalledTimes(1);
});
it('interrupted history offers owned staging cleanup without a successful count', async () => {
  const s = service();
  vi.mocked(s.history).mockResolvedValue({
    items: [
      {
        ...snapshot,
        phase: 'interrupted',
        manifestPath: null,
        localIntegrity: 'incomplete',
        successfulCount: 0,
      },
    ],
    total: 1,
    completedCount: 0,
    completedBytes: 0,
  });
  page(s);
  await userEvent.click(await screen.findByRole('button', { name: 'Details' }));
  expect(screen.getByText(/0 completed snapshots/)).toBeInTheDocument();
  expect(
    screen.getByRole('button', { name: 'Re-verify local integrity' }),
  ).toBeDisabled();
  await userEvent.click(
    screen.getByRole('button', { name: 'Clean owned staging only' }),
  );
  expect(s.cleanup).toHaveBeenCalledWith('snapshot-1');
});
