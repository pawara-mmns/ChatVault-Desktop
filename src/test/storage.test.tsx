import type * as DeviceModule from '../services/devices';
import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { App } from '../app/App';
import { ThemeProvider } from '../features/settings/ThemeProvider';
import { freshStorage } from './storageFixture';
import { formatBytes, nativeStorageService } from '../services/storage';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
vi.mock('../services/devices', async (original) => {
  const actual = await original<typeof DeviceModule>();
  return {
    ...actual,
    deviceService: { ...actual.deviceService, available: () => false },
  };
});
function mount(path = '/settings') {
  return render(
    <ThemeProvider>
      <MemoryRouter initialEntries={[path]}>
        <App />
      </MemoryRouter>
    </ThemeProvider>,
  );
}
function respond(overrides: Partial<typeof freshStorage> = {}) {
  vi.mocked(isTauri).mockReturnValue(true);
  vi.mocked(invoke).mockImplementation(async (command) =>
    command === 'health_check'
      ? { status: 'ok', version: '0.1.0', platform: 'windows' }
      : { ...freshStorage, ...overrides },
  );
}
describe('local storage integration', () => {
  it('renders persistent database counts and actual vault bytes on the dashboard', async () => {
    respond({ backupSnapshots: 2, chatArchives: 3, vaultBytes: 1536 });
    mount('/dashboard');
    const summary = screen.getByRole('region', { name: 'Backup summary' });
    expect(await within(summary).findByText('2')).toBeInTheDocument();
    expect(within(summary).getByText('3')).toBeInTheDocument();
    expect(
      within(summary).getByLabelText('Vault storage used'),
    ).toHaveTextContent('1.5 KB');
    expect(screen.getByText(freshStorage.vaultPath)).toBeInTheDocument();
    expect(
      screen.queryByText('Your story starts here'),
    ).not.toBeInTheDocument();
  });
  it('shows genuine empty database results as 0, 0, and 0 B', async () => {
    respond();
    mount('/dashboard');
    expect(await screen.findByText('0 B')).toBeInTheDocument();
    expect(screen.getByText('No backups created')).toBeInTheDocument();
    expect(screen.getByText('No chats imported')).toBeInTheDocument();
    expect(screen.getByText('Your story starts here')).toBeInTheDocument();
  });
  it('saves a picker choice, synchronizes routes, and restores the backend preference after remount', async () => {
    const user = userEvent.setup();
    vi.mocked(isTauri).mockReturnValue(true);
    let persisted = { ...freshStorage };
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'health_check')
        return { status: 'ok', version: '0.1.0', platform: 'windows' };
      if (command === 'choose_vault_directory')
        persisted = { ...persisted, vaultPath: 'D:\\TestOnly\\NewVault' };
      return { ...persisted };
    });
    const view = mount();
    await screen.findByText(freshStorage.vaultPath);
    await user.click(screen.getByRole('button', { name: 'Change Location' }));
    expect(
      await screen.findByText(
        'Vault location saved. Existing files were not moved or deleted.',
      ),
    ).toBeInTheDocument();
    await user.click(screen.getByRole('link', { name: 'Overview' }));
    expect(screen.getByText('D:\\TestOnly\\NewVault')).toBeInTheDocument();
    view.unmount();
    mount();
    expect(
      await screen.findByText('D:\\TestOnly\\NewVault'),
    ).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('choose_vault_directory');
    expect(localStorage.getItem('vault_path')).toBeNull();
  });
  it('keeps the original path and explains a blocked location change', async () => {
    respond();
    const base = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'choose_vault_directory')
        throw {
          code: 'vault_not_empty',
          message:
            'The current vault contains files. Migration is not supported yet.',
        };
      return base(command);
    });
    const user = userEvent.setup();
    mount();
    await screen.findByText(freshStorage.vaultPath);
    await user.click(screen.getByRole('button', { name: 'Change Location' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Migration is not supported yet',
    );
    expect(screen.getByText(freshStorage.vaultPath)).toBeInTheDocument();
    expect(screen.queryByText(/Vault location saved/)).not.toBeInTheDocument();
  });
  it('does not claim success when the native picker is cancelled', async () => {
    respond();
    const base = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation((command) =>
      command === 'choose_vault_directory'
        ? Promise.resolve(null)
        : base(command),
    );
    const user = userEvent.setup();
    mount();
    await screen.findByText(freshStorage.vaultPath);
    await user.click(screen.getByRole('button', { name: 'Change Location' }));
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Change Location' }),
      ).toBeEnabled(),
    );
    expect(screen.queryByText(/Vault location saved/)).not.toBeInTheDocument();
    expect(screen.getByText(freshStorage.vaultPath)).toBeInTheDocument();
  });
  it('serializes location requests and disables controls while the picker is open', async () => {
    respond();
    const base = vi.mocked(invoke).getMockImplementation()!;
    let resolvePicker: (value: typeof freshStorage) => void = () => undefined;
    vi.mocked(invoke).mockImplementation((command) =>
      command === 'choose_vault_directory'
        ? new Promise((resolve) => {
            resolvePicker = resolve;
          })
        : base(command),
    );
    const user = userEvent.setup();
    mount();
    await screen.findByText(freshStorage.vaultPath);
    await user.dblClick(
      screen.getByRole('button', { name: 'Change Location' }),
    );
    expect(
      vi
        .mocked(invoke)
        .mock.calls.filter(([command]) => command === 'choose_vault_directory'),
    ).toHaveLength(1);
    expect(
      screen.getByRole('button', { name: 'Change Location' }),
    ).toBeDisabled();
    await act(async () => resolvePicker(freshStorage));
  });
  it('shows missing storage as unavailable without substituting a false zero', async () => {
    respond({
      vaultStatus: 'unavailable',
      vaultBytes: null,
      vaultError: {
        code: 'vault_unavailable',
        message: 'Reconnect the saved drive. The location has been preserved.',
      },
    });
    mount();
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Reconnect the saved drive',
    );
    expect(screen.getByText(freshStorage.vaultPath)).toBeInTheDocument();
    expect(screen.queryByText('0 B')).not.toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Change Location' }),
    ).toBeDisabled();
  });
  it('shows a database error and retries initialization without resetting storage', async () => {
    respond();
    const base = vi.mocked(invoke).getMockImplementation()!;
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === 'get_storage_overview')
        throw {
          code: 'database_unavailable',
          message:
            'The local database cannot be opened. No database was reset.',
        };
      return base(command);
    });
    const user = userEvent.setup();
    mount();
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'No database was reset',
    );
    await user.click(
      screen.getByRole('button', { name: 'Retry initialization' }),
    );
    expect(
      await screen.findByText('Local database and vault are ready.'),
    ).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('initialize_vault');
  });
  it('does not invoke native storage or show fabricated counts in a browser', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    mount('/dashboard');
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Desktop storage is unavailable in browser preview',
    );
    expect(screen.queryByText('0 B')).not.toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });
  it('blocks native storage calls in browser settings and hides arbitrary errors', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    await expect(
      nativeStorageService.chooseVaultDirectory(),
    ).rejects.toMatchObject({ code: 'runtime_unavailable' });
    expect(invoke).not.toHaveBeenCalled();
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockRejectedValueOnce(
      new Error('sensitive OS diagnostic'),
    );
    await expect(nativeStorageService.getOverview()).rejects.toMatchObject({
      code: 'storage_unavailable',
    });
  });
  it('formats zero and unavailable bytes distinctly', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(null)).toBe('Unavailable');
    expect(formatBytes(1024)).toBe('1 KB');
  });
});
