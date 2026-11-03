import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { BackendStatus } from '../features/settings/BackendStatus';
import { checkBackend } from '../platform/backend';
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: vi.fn() }));
describe('backend boundary', () => {
  it('does not pretend the browser is a desktop runtime', async () => {
    vi.mocked(isTauri).mockReturnValue(false);
    render(<BackendStatus />);
    expect(
      await screen.findByText('Desktop backend unavailable'),
    ).toBeInTheDocument();
    expect(screen.getByText(/Browser preview:/)).toBeInTheDocument();
    expect(invoke).not.toHaveBeenCalled();
  });
  it('shows an IPC failure safely, and recovers on retry using a real health command contract', async () => {
    const user = userEvent.setup();
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockRejectedValueOnce(new Error('sensitive diagnostic'));
    render(<BackendStatus />);
    expect(
      await screen.findByText('Desktop backend unavailable'),
    ).toBeInTheDocument();
    expect(screen.queryByText('sensitive diagnostic')).not.toBeInTheDocument();
    vi.mocked(invoke).mockResolvedValueOnce({
      status: 'ok',
      version: '0.1.0',
      platform: 'windows',
    });
    await user.click(screen.getByRole('button', { name: 'Retry check' }));
    expect(
      await screen.findByText('Desktop backend connected'),
    ).toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('health_check');
  });
  it('rejects an unexpected health result', async () => {
    vi.mocked(isTauri).mockReturnValue(true);
    vi.mocked(invoke).mockResolvedValueOnce({ status: 'bad' });
    await expect(checkBackend()).rejects.toThrow('unexpected response');
  });
});
