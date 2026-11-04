import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Archives } from '../features/archives/Archives';
import type * as ImportModule from '../services/imports';
import { StorageProvider } from '../features/storage/StorageProvider';
import { storageFixture } from './storageFixture';
import { archiveService } from '../services/imports';
import { archiveFixture } from './importFixture';
vi.mock('../services/imports', async (original) => {
  const actual = await original<typeof ImportModule>();
  return { ...actual, archiveService: { list: vi.fn(), summary: vi.fn() } };
});
function mount() {
  return render(
    <StorageProvider service={storageFixture()}>
      <MemoryRouter>
        <Archives />
      </MemoryRouter>
    </StorageProvider>,
  );
}
beforeEach(() => {
  vi.mocked(archiveService.list).mockResolvedValue({
    items: [],
    total: 0,
    offset: 0,
    limit: 20,
  });
  vi.mocked(archiveService.summary).mockResolvedValue(archiveFixture);
});
describe('archive metadata library', () => {
  it('shows an actual empty result without fake archives', async () => {
    mount();
    expect(
      await screen.findByRole('heading', { name: 'No archives yet' }),
    ).toBeInTheDocument();
  });
  it('lists stored metadata and reads a summary without a conversation viewer', async () => {
    vi.mocked(archiveService.list).mockResolvedValue({
      items: [archiveFixture],
      total: 1,
      offset: 0,
      limit: 20,
    });
    const user = userEvent.setup();
    mount();
    await screen.findByText('Synthetic conversation');
    expect(screen.getByText('Synthetic export.txt')).toBeInTheDocument();
    await user.click(
      screen.getByRole('button', {
        name: 'Summary for Synthetic conversation',
      }),
    );
    expect(
      await screen.findByRole('heading', {
        name: 'Archive summary: Synthetic conversation',
      }),
    ).toBeInTheDocument();
    expect(archiveService.summary).toHaveBeenCalledWith('synthetic-archive');
  });
  it('passes a basic filter to the backend', async () => {
    const user = userEvent.setup();
    mount();
    await screen.findByRole('heading', { name: 'No archives yet' });
    await user.type(
      screen.getByRole('searchbox', { name: 'Search archives' }),
      'fixture',
    );
    await waitFor(() =>
      expect(archiveService.list).toHaveBeenLastCalledWith('fixture', 0, 20),
    );
  });
  it('shows and retries a library failure', async () => {
    vi.mocked(archiveService.list).mockRejectedValueOnce({
      code: 'import_database',
      message: 'The local library could not be read.',
    });
    const user = userEvent.setup();
    mount();
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'could not be read',
    );
    await user.click(screen.getByRole('button', { name: 'Retry library' }));
    expect(
      await screen.findByRole('heading', { name: 'No archives yet' }),
    ).toBeInTheDocument();
  });
});
