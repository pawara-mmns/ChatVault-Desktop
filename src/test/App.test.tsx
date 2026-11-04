import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { archiveService } from '../services/imports';
import { AppShell } from '../app/App';
import { StorageProvider } from '../features/storage/StorageProvider';
import { storageFixture } from './storageFixture';
import { ThemeProvider } from '../features/settings/ThemeProvider';

function renderApp(path = '/dashboard') {
  return render(
    <ThemeProvider>
      <StorageProvider service={storageFixture()}>
        <MemoryRouter initialEntries={[path]}>
          <AppShell />
        </MemoryRouter>
      </StorageProvider>
    </ThemeProvider>,
  );
}
describe('desktop navigation and truthful foundation states', () => {
  beforeEach(() => {
    vi.spyOn(archiveService, 'list').mockResolvedValue({
      items: [],
      total: 0,
      offset: 0,
      limit: 20,
    });
  });
  it('navigates all five routes, marks the active link, and focuses the page heading', async () => {
    const user = userEvent.setup();
    renderApp();
    const nav = screen.getByRole('navigation', { name: 'Main navigation' });
    for (const [label, title] of [
      ['Archives', 'Chat archives'],
      ['Import Chat', 'Import Chat'],
      ['Devices', 'Devices'],
      ['Overview', 'A home for your conversations.'],
    ] as const) {
      const link = within(nav).getByRole('link', { name: label });
      await user.click(link);
      expect(link).toHaveAttribute('aria-current', 'page');
      expect(
        screen.getByRole('heading', { level: 1, name: title }),
      ).toHaveFocus();
    }
    await user.click(screen.getByRole('link', { name: 'Settings' }));
    expect(
      screen.getByRole('heading', { level: 1, name: 'Settings' }),
    ).toHaveFocus();
    expect(document.title).toBe('Settings · ChatVault Desktop');
  });
  it('keeps navigation accessible when the sidebar is collapsed', async () => {
    const user = userEvent.setup();
    renderApp();
    await user.click(screen.getByRole('button', { name: 'Collapse sidebar' }));
    expect(
      screen.getByRole('button', { name: 'Expand sidebar' }),
    ).toHaveAttribute('aria-expanded', 'false');
    await user.click(screen.getByRole('link', { name: 'Archives' }));
    expect(
      await screen.findByRole('heading', { name: 'No archives yet' }),
    ).toBeInTheDocument();
  });
  it('shows database zeros, zero vault bytes, and no connected device', async () => {
    renderApp();
    const summary = screen.getByRole('region', { name: 'Backup summary' });
    expect(await within(summary).findAllByText('0')).toHaveLength(2);
    expect(
      within(summary).getByLabelText('Vault storage used'),
    ).toHaveTextContent('0 B');
    expect(screen.getByText('Not connected')).toBeInTheDocument();
    expect(
      screen.getByText('Device detection is not yet available'),
    ).toBeInTheDocument();
    expect(screen.getByText('0 snapshots')).toBeInTheDocument();
  });
  it('filters archive metadata while keeping the full viewer unavailable', async () => {
    renderApp('/archives');
    expect(
      screen.getByRole('searchbox', { name: 'Search archives' }),
    ).toBeEnabled();
    expect(
      await screen.findByRole('heading', { name: 'No archives yet' }),
    ).toBeInTheDocument();
  });
  it('disables native file selection in a browser and explains encrypted backups', () => {
    renderApp('/import');
    expect(
      screen.getByRole('button', { name: 'Choose WhatsApp export' }),
    ).toBeDisabled();
    expect(
      screen.getByRole('heading', { name: 'Encrypted native backups' }),
    ).toBeInTheDocument();
    expect(screen.getByText(/Desktop runtime unavailable/)).toBeInTheDocument();
  });
  it('redirects an unknown route to the overview', () => {
    renderApp('/unknown');
    expect(
      screen.getByRole('heading', {
        level: 1,
        name: 'A home for your conversations.',
      }),
    ).toBeInTheDocument();
  });
});
