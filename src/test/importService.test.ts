import { describe, it, expect, vi } from 'vitest';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { importService, archiveService } from '../services/imports';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
describe('narrow import IPC boundary', () => {
  it('rejects file selection and library requests without a desktop runtime', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(importService.select()).rejects.toMatchObject({
      code: 'runtime_unavailable',
    });
    await expect(archiveService.list()).rejects.toMatchObject({
      code: 'runtime_unavailable',
    });
    expect(invoke).not.toHaveBeenCalled();
  });
  it('sends only a session token, date order, and title for import operations', async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockResolvedValue(undefined);
    await importService.analyze('session', 'day-first');
    expect(invoke).toHaveBeenCalledWith('analyze_import', {
      token: 'session',
      dateOrder: 'day-first',
    });
    await importService.confirm('session', 'Synthetic title');
    expect(invoke).toHaveBeenCalledWith('confirm_import', {
      token: 'session',
      title: 'Synthetic title',
    });
  });
  it('hides arbitrary native diagnostics', async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockRejectedValueOnce(
      new Error('sensitive diagnostic fixture'),
    );
    await expect(importService.select()).rejects.toMatchObject({
      code: 'import_failed',
    });
  });
});
