import { beforeEach, expect, it, vi } from 'vitest';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { deviceService, deviceFailure } from '../services/devices';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
beforeEach(() => {
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(invoke).mockResolvedValue({});
});
it('browser USB calls fail locally without native IPC', async () => {
  vi.mocked(isTauri).mockReturnValue(false);
  await expect(deviceService.refresh()).rejects.toMatchObject({
    code: 'unsupported',
  });
  expect(invoke).not.toHaveBeenCalled();
});
it('exposes narrow issued-handle commands and explicit local consent action', async () => {
  await deviceService.check('issued');
  expect(invoke).toHaveBeenLastCalledWith('check_device_file_access', {
    deviceId: 'issued',
  });
  await deviceService.discover('issued');
  expect(invoke).toHaveBeenLastCalledWith('discover_whatsapp_storage', {
    deviceId: 'issued',
  });
  await deviceService.remember('issued', true);
  expect(invoke).toHaveBeenLastCalledWith('remember_device', {
    deviceId: 'issued',
    preferred: true,
  });
});
it('filters unexpected diagnostics and preserves only numerical WPD errors', () => {
  expect(deviceFailure(new Error('private serial'))).toMatchObject({
    code: 'failed',
    diagnostic: null,
  });
  expect(
    deviceFailure({
      code: 'access_denied',
      message: 'Denied',
      diagnostic: 'private PnP value',
    }).diagnostic,
  ).toBeNull();
  expect(
    deviceFailure({
      code: 'access_denied',
      message: 'Denied',
      diagnostic: 'WPD HRESULT 0x80070005',
    }).diagnostic,
  ).toBe('WPD HRESULT 0x80070005');
});
