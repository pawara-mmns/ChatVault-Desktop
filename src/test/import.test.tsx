import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ImportChat } from '../features/import/ImportChat';
import type * as ImportModule from '../services/imports';
import { StorageProvider } from '../features/storage/StorageProvider';
import { storageFixture } from './storageFixture';
import { importService, desktopImportAvailable } from '../services/imports';
import {
  selectedFixture,
  previewFixture,
  archiveFixture,
} from './importFixture';
vi.mock('../services/imports', async (original) => {
  const actual = await original<typeof ImportModule>();
  return {
    ...actual,
    desktopImportAvailable: vi.fn(),
    importService: {
      select: vi.fn(),
      analyze: vi.fn(),
      confirm: vi.fn(),
      cancel: vi.fn(),
    },
  };
});
function mount() {
  return render(
    <StorageProvider service={storageFixture()}>
      <MemoryRouter>
        <ImportChat />
      </MemoryRouter>
    </StorageProvider>,
  );
}
async function choose(user: ReturnType<typeof userEvent.setup>) {
  await waitFor(() =>
    expect(
      screen.getByRole('button', { name: 'Choose WhatsApp export' }),
    ).toBeEnabled(),
  );
  await user.click(
    screen.getByRole('button', { name: 'Choose WhatsApp export' }),
  );
}
beforeEach(() => {
  vi.mocked(desktopImportAvailable).mockReturnValue(true);
  vi.mocked(importService.select).mockResolvedValue({ ...selectedFixture });
  vi.mocked(importService.analyze).mockResolvedValue({ ...previewFixture });
  vi.mocked(importService.confirm).mockResolvedValue({ ...archiveFixture });
  vi.mocked(importService.cancel).mockResolvedValue(undefined);
});
describe('native import workflow presentation', () => {
  it('renders real preview data and requires explicit confirmation before saving', async () => {
    const user = userEvent.setup();
    mount();
    await choose(user);
    expect(await screen.findByText('Synthetic export.txt')).toBeInTheDocument();
    expect(
      screen.getByDisplayValue('Synthetic conversation'),
    ).toBeInTheDocument();
    expect(screen.getByText(/Synthetic hello/)).toBeInTheDocument();
    expect(importService.confirm).not.toHaveBeenCalled();
    await user.click(
      screen.getByRole('button', { name: 'Import to local vault' }),
    );
    expect(
      await screen.findByRole('heading', { name: 'Archive saved locally' }),
    ).toBeInTheDocument();
    expect(importService.confirm).toHaveBeenCalledWith(
      'synthetic-session',
      'Synthetic conversation',
    );
    expect(
      screen.getByRole('link', { name: 'Open Archives' }),
    ).toBeInTheDocument();
  });
  it('does not fabricate success after a cancelled picker', async () => {
    vi.mocked(importService.select).mockResolvedValue(null);
    const user = userEvent.setup();
    mount();
    await choose(user);
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Choose WhatsApp export' }),
      ).toBeEnabled(),
    );
    expect(importService.analyze).not.toHaveBeenCalled();
    expect(screen.queryByText('Archive saved locally')).not.toBeInTheDocument();
  });
  it('reports invalid file failures and keeps confirmation unavailable', async () => {
    vi.mocked(importService.select).mockRejectedValue({
      code: 'unsupported_format',
      message: 'The selected contents are not a supported export.',
    });
    const user = userEvent.setup();
    mount();
    await choose(user);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'not a supported export',
    );
    expect(
      screen.queryByRole('button', { name: 'Import to local vault' }),
    ).not.toBeInTheDocument();
  });
  it('releases preview resources when the user cancels', async () => {
    const user = userEvent.setup();
    mount();
    await choose(user);
    await screen.findByText('Synthetic export.txt');
    await user.click(screen.getByRole('button', { name: 'Cancel import' }));
    expect(importService.cancel).toHaveBeenCalledWith('synthetic-session');
    expect(importService.confirm).not.toHaveBeenCalled();
    expect(
      await screen.findByRole('button', { name: 'Choose WhatsApp export' }),
    ).toBeInTheDocument();
  });
  it('requires a date choice and reanalyzes before enabling confirmation', async () => {
    vi.mocked(importService.analyze).mockResolvedValueOnce({
      ...previewFixture,
      dateOrderRequired: true,
    });
    const user = userEvent.setup();
    mount();
    await choose(user);
    await screen.findByText('Synthetic export.txt');
    expect(
      screen.getByRole('button', { name: 'Import to local vault' }),
    ).toBeDisabled();
    vi.mocked(importService.analyze).mockResolvedValueOnce({
      ...previewFixture,
      dateOrder: 'day-first',
    });
    await user.selectOptions(
      screen.getByRole('combobox', { name: 'Date format' }),
      'day-first',
    );
    await waitFor(() =>
      expect(
        screen.getByRole('button', { name: 'Import to local vault' }),
      ).toBeEnabled(),
    );
    expect(importService.analyze).toHaveBeenLastCalledWith(
      'synthetic-session',
      'day-first',
    );
  });
  it('shows duplicate detection and offers cancellation without reimport', async () => {
    vi.mocked(importService.select).mockResolvedValue({
      ...selectedFixture,
      duplicateArchive: 'existing',
    });
    const user = userEvent.setup();
    mount();
    await choose(user);
    expect(await screen.findByRole('alert')).toHaveTextContent(
      'already imported',
    );
    expect(importService.analyze).not.toHaveBeenCalled();
    expect(
      screen.queryByRole('button', { name: 'Import to local vault' }),
    ).not.toBeInTheDocument();
  });
  it('renders imported HTML-looking text as plain text', async () => {
    const unsafe = '<img src=x onerror=alert(1)>';
    vi.mocked(importService.analyze).mockResolvedValue({
      ...previewFixture,
      samples: [{ ...previewFixture.samples[0]!, text: unsafe }],
    });
    const user = userEvent.setup();
    const view = mount();
    await choose(user);
    expect(await screen.findByText(unsafe)).toBeInTheDocument();
    expect(view.container.querySelector('img')).toBeNull();
  });
  it('shows a save error instead of a success when persistence fails', async () => {
    vi.mocked(importService.confirm).mockRejectedValue({
      code: 'import_database',
      message: 'The import transaction was rolled back.',
    });
    const user = userEvent.setup();
    mount();
    await choose(user);
    await screen.findByText('Synthetic export.txt');
    await user.click(
      screen.getByRole('button', { name: 'Import to local vault' }),
    );
    expect(await screen.findByRole('alert')).toHaveTextContent('rolled back');
    expect(screen.queryByText('Archive saved locally')).not.toBeInTheDocument();
  });
  it('disables native operations in an ordinary browser', () => {
    vi.mocked(desktopImportAvailable).mockReturnValue(false);
    mount();
    expect(screen.getByText(/Desktop runtime unavailable/)).toBeInTheDocument();
    expect(
      screen.getByRole('button', { name: 'Choose WhatsApp export' }),
    ).toBeDisabled();
    expect(importService.select).not.toHaveBeenCalled();
  });
});
