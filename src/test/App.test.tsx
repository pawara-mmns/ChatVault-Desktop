import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter } from 'react-router-dom';
import { describe, it, expect } from 'vitest';
import { App } from '../app/App';
import { ThemeProvider } from '../features/settings/ThemeProvider';

function renderApp(path = '/dashboard') {
  return render(
    <ThemeProvider>
      <MemoryRouter initialEntries={[path]}>
        <App />
      </MemoryRouter>
    </ThemeProvider>,
  );
}
describe('desktop navigation and truthful foundation states', () => {
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
      screen.getByRole('heading', { name: 'No archives yet' }),
    ).toBeInTheDocument();
  });
  it('shows zero snapshots, zero archives, unknown storage, and no connected device', () => {
    renderApp();
    const summary = screen.getByRole('region', { name: 'Backup summary' });
    expect(within(summary).getAllByText('0')).toHaveLength(2);
    expect(within(summary).getByLabelText('Not calculated')).toHaveTextContent(
      '—',
    );
    expect(screen.getByText('Not connected')).toBeInTheDocument();
    expect(
      screen.getByText('Device detection is not yet available'),
    ).toBeInTheDocument();
    expect(screen.getByText('0 snapshots')).toBeInTheDocument();
  });
  it('disables archive search and explains its unavailable state', () => {
    renderApp('/archives');
    expect(
      screen.getByRole('searchbox', { name: 'Search archives' }),
    ).toBeDisabled();
    expect(
      screen.getByRole('heading', { name: 'No archives yet' }),
    ).toBeInTheDocument();
  });
  it('disables file selection and distinguishes encrypted backups from chat exports', () => {
    renderApp('/import');
    expect(
      screen.getByRole('button', { name: 'Choose chat export' }),
    ).toBeDisabled();
    expect(
      screen.getByRole('heading', { name: 'Encrypted native backups' }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/No files can be selected, read, or imported/),
    ).toBeInTheDocument();
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
