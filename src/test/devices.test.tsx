import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';
import { Devices } from '../features/devices/Devices';
import { DeviceProvider } from '../features/devices/DeviceProvider';
import { Dashboard } from '../features/dashboard/Dashboard';
import { StorageProvider } from '../features/storage/StorageProvider';
import { archiveService } from '../services/imports';
import type { DeviceService, ConnectedDevice } from '../services/devices';
import { deviceFixture, phone } from './deviceFixture';
import { storageFixture } from './storageFixture';
afterEach(() => {
  vi.useRealTimers();
});
function page(service: DeviceService) {
  return render(
    <MemoryRouter>
      <DeviceProvider service={service}>
        <Devices />
      </DeviceProvider>
    </MemoryRouter>,
  );
}
async function select() {
  await userEvent.click(
    await screen.findByRole('button', { name: /Synthetic phone/ }),
  );
}
it('shows a genuine completed zero-device observation and a backup-wizard link', async () => {
  page(deviceFixture());
  expect(
    await screen.findByRole('heading', { name: 'No device detected' }),
  ).toBeInTheDocument();
  expect(screen.getByRole('link', { name: /Backup Now/ })).toHaveAttribute(
    'href',
    '/backups',
  );
  expect(screen.getByText(/charging-only/)).toBeInTheDocument();
});
it('browser fallback never fabricates a phone or invokes refresh', () => {
  const s = deviceFixture();
  s.available = () => false;
  page(s);
  expect(
    screen.getByText(/unavailable in browser preview/),
  ).toBeInTheDocument();
  expect(screen.getByRole('button', { name: /Scan Devices/ })).toBeDisabled();
  expect(s.refresh).not.toHaveBeenCalled();
});
it('selection shows only supplied metadata and does not implicitly inspect storage', async () => {
  const s = deviceFixture([phone()]);
  page(s);
  await select();
  expect(screen.getByText(/WPD does not establish/)).toBeInTheDocument();
  expect(screen.getAllByText('Unavailable').length).toBe(1);
  expect(s.check).not.toHaveBeenCalled();
  expect(s.discover).not.toHaveBeenCalled();
});
it('checks access explicitly without claiming Android from a phone category', async () => {
  const s = deviceFixture([phone()]);
  page(s);
  await select();
  await userEvent.click(
    screen.getByRole('button', { name: 'Check file access' }),
  );
  expect(await screen.findByText('Phone')).toBeInTheDocument();
  expect(screen.getByText(/WPD does not establish/)).toBeInTheDocument();
  expect(s.check).toHaveBeenCalledWith('fixture-handle-1');
  expect(s.discover).not.toHaveBeenCalled();
});
it('discovers only after explicit action and presents actual folder results', async () => {
  const s = deviceFixture([phone()]);
  page(s);
  await select();
  await userEvent.click(
    screen.getByRole('button', { name: 'Scan WhatsApp Storage' }),
  );
  expect(await screen.findByText('Folder accessible')).toBeInTheDocument();
  expect(screen.getByText('WhatsApp · images')).toBeInTheDocument();
  expect(s.discover).toHaveBeenCalledWith('fixture-handle-1');
  expect(s.remember).not.toHaveBeenCalled();
});
it('permission denial is diagnostic rather than an empty-data claim', async () => {
  const s = deviceFixture([phone()]);
  vi.mocked(s.check).mockResolvedValue(
    phone({
      access: 'permission_required',
      diagnostic: {
        code: 'access_denied',
        message: 'Unlock the phone and approve File Transfer.',
        diagnostic: 'WPD HRESULT 0x80070005',
      },
    }),
  );
  page(s);
  await select();
  await userEvent.click(
    screen.getByRole('button', { name: 'Check file access' }),
  );
  expect(await screen.findByRole('alert')).toHaveTextContent('access_denied');
  expect(screen.getByRole('alert')).toHaveTextContent('0x80070005');
});
it('unplug during discovery clears the stale selection', async () => {
  const s = deviceFixture([phone()]);
  vi.mocked(s.discover).mockRejectedValue({
    code: 'disconnected',
    message: 'Device disconnected',
    diagnostic: null,
  });
  page(s);
  await select();
  await userEvent.click(
    screen.getByRole('button', { name: 'Scan WhatsApp Storage' }),
  );
  expect(await screen.findByRole('alert')).toHaveTextContent(
    'Device disconnected',
  );
  expect(
    screen.queryByRole('button', { name: 'Check file access' }),
  ).not.toBeInTheDocument();
});
it('refresh replaces live metadata and invalidates a disconnected selection', async () => {
  const s = deviceFixture([phone()]);
  page(s);
  await select();
  vi.mocked(s.refresh).mockResolvedValue({
    devices: [],
    scannedAt: 2000,
    stale: false,
    busy: false,
  });
  await userEvent.click(screen.getByRole('button', { name: /Scan Devices/ }));
  expect(
    await screen.findByRole('heading', { name: 'No device detected' }),
  ).toBeInTheDocument();
  expect(screen.getByText(/selected device disconnected/)).toBeInTheDocument();
});
it('handles multiple devices and missing metadata without invented identifiers', async () => {
  page(
    deviceFixture([
      phone(),
      phone({
        id: 'second',
        name: null,
        manufacturer: null,
        transport: 'unavailable',
        access: 'unsupported',
      }),
    ]),
  );
  await userEvent.click(
    await screen.findByRole('button', { name: /Name unavailable/ }),
  );
  expect(
    screen.getByRole('button', { name: 'Check file access' }),
  ).toBeDisabled();
  expect(screen.getByText(/USB transport unconfirmed/)).toBeInTheDocument();
});
it('device names are escaped React text', async () => {
  page(deviceFixture([phone({ name: '<img src=x onerror=alert(1)>' })]));
  expect(
    await screen.findByText('<img src=x onerror=alert(1)>'),
  ).toBeInTheDocument();
  expect(document.querySelector('.connected-device-list img')).toBeNull();
});
it('requires per-device consent before saving a historical note', async () => {
  const s = deviceFixture([phone()]);
  page(s);
  await select();
  const button = screen.getByRole('button', { name: 'Save device note' });
  expect(button).toBeDisabled();
  await userEvent.click(screen.getByLabelText(/I agree to save/));
  await userEvent.click(button);
  expect(
    await screen.findByRole('heading', { name: 'Saved device notes' }),
  ).toBeInTheDocument();
  expect(s.remember).toHaveBeenCalledWith('fixture-handle-1', false);
  expect(
    screen.getByText(/do not imply a current connection/),
  ).toBeInTheDocument();
});
it('shows cancellable in-progress discovery and prevents overlapping refresh', async () => {
  const s = deviceFixture([phone()]);
  let complete!: (d: ConnectedDevice) => void;
  vi.mocked(s.discover).mockReturnValue(
    new Promise((r) => {
      complete = r;
    }),
  );
  page(s);
  await select();
  await userEvent.click(
    screen.getByRole('button', { name: 'Scan WhatsApp Storage' }),
  );
  expect(screen.getByText('Discovering WhatsApp storage')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: /Scan Devices/ })).toBeDisabled();
  await userEvent.click(screen.getByRole('button', { name: 'Cancel scan' }));
  expect(s.cancel).toHaveBeenCalled();
  await act(async () => complete(phone()));
});
it('polls only while visible and cleans up its timer on unmount', async () => {
  vi.useFakeTimers();
  const s = deviceFixture();
  const view = page(s);
  await act(async () => {});
  expect(s.refresh).toHaveBeenCalledTimes(1);
  await act(async () => {
    await vi.advanceTimersByTimeAsync(15000);
  });
  expect(s.refresh).toHaveBeenCalledTimes(2);
  view.unmount();
  await act(async () => {
    await vi.advanceTimersByTimeAsync(30000);
  });
  expect(s.refresh).toHaveBeenCalledTimes(2);
  expect(s.cancel).toHaveBeenCalled();
});
it('dashboard uses live device access state without changing snapshot counts', async () => {
  vi.spyOn(archiveService, 'list').mockResolvedValue({
    items: [],
    total: 0,
    offset: 0,
    limit: 20,
  });
  const s = deviceFixture([phone({ access: 'unavailable' })]);
  render(
    <DeviceProvider service={s}>
      <StorageProvider service={storageFixture()}>
        <MemoryRouter>
          <Dashboard />
        </MemoryRouter>
      </StorageProvider>
    </DeviceProvider>,
  );
  expect(await screen.findByText('Synthetic phone')).toBeInTheDocument();
  expect(
    screen.getByText('Connection detected — file access unavailable'),
  ).toBeInTheDocument();
  await waitFor(() =>
    expect(screen.getByText('0 snapshots')).toBeInTheDocument(),
  );
});
