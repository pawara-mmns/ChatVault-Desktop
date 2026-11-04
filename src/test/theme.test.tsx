import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import {
  ThemeProvider,
  THEME_STORAGE_KEY,
} from '../features/settings/ThemeProvider';
import { SettingsPage } from '../features/settings/SettingsPage';
import { setSystemDark } from './setup';
import { StorageProvider } from '../features/storage/StorageProvider';
import { storageFixture } from './storageFixture';
function mountSettings() {
  return render(
    <ThemeProvider>
      <StorageProvider service={storageFixture()}>
        <SettingsPage />
      </StorageProvider>
    </ThemeProvider>,
  );
}
describe('appearance preference', () => {
  it('switches and persists light, dark and system, and restores the saved choice', async () => {
    const user = userEvent.setup();
    const view = mountSettings();
    expect(screen.getByRole('radio', { name: 'System' })).toBeChecked();
    await user.click(screen.getByRole('radio', { name: 'Dark' }));
    expect(document.documentElement).toHaveAttribute('data-theme', 'dark');
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('dark');
    view.unmount();
    mountSettings();
    expect(screen.getByRole('radio', { name: 'Dark' })).toBeChecked();
    await user.click(screen.getByRole('radio', { name: 'Light' }));
    expect(document.documentElement).toHaveAttribute('data-theme', 'light');
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('light');
    await user.click(screen.getByRole('radio', { name: 'System' }));
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('system');
  });
  it('responds to system changes only when system appearance is selected', async () => {
    const user = userEvent.setup();
    mountSettings();
    act(() => setSystemDark(true));
    expect(document.documentElement).toHaveAttribute('data-theme', 'dark');
    await user.click(screen.getByRole('radio', { name: 'Light' }));
    act(() => setSystemDark(false));
    act(() => setSystemDark(true));
    expect(document.documentElement).toHaveAttribute('data-theme', 'light');
  });
  it('falls back to system for an invalid preference', () => {
    localStorage.setItem(THEME_STORAGE_KEY, 'invalid');
    setSystemDark(true);
    mountSettings();
    expect(screen.getByRole('radio', { name: 'System' })).toBeChecked();
    expect(document.documentElement).toHaveAttribute('data-theme', 'dark');
  });
  it('reports a failed preference write while still changing the theme', async () => {
    const user = userEvent.setup();
    mountSettings();
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('Storage denied');
    });
    await user.click(screen.getByRole('radio', { name: 'Dark' }));
    expect(document.documentElement).toHaveAttribute('data-theme', 'dark');
    expect(screen.getByRole('alert')).toHaveTextContent('could not be saved');
  });
  it('supports keyboard selection with native radio behavior', async () => {
    const user = userEvent.setup();
    mountSettings();
    screen.getByRole('radio', { name: 'System' }).focus();
    await user.keyboard('{ArrowLeft}');
    expect(screen.getByRole('radio', { name: 'Dark' })).toBeChecked();
  });
});
