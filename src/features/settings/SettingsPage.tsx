import { Monitor, Moon, Sun, ShieldCheck, Database } from 'lucide-react';
import { Badge, PageHeading } from '../../shared/components/Page';
import { useTheme, type ThemePreference } from './ThemeProvider';
import { BackendStatus } from './BackendStatus';
const options = [
  { value: 'light', label: 'Light', icon: Sun },
  { value: 'dark', label: 'Dark', icon: Moon },
  { value: 'system', label: 'System', icon: Monitor },
] as const;
export function SettingsPage() {
  const { preference, resolvedTheme, setPreference, persistenceError } =
    useTheme();
  return (
    <>
      <PageHeading
        eyebrow="MAKE IT YOURS"
        title="Settings"
        description="A few thoughtful preferences. Everything stays on your computer."
      />
      <section
        className="panel settings-section"
        aria-labelledby="appearance-heading"
      >
        <h2 id="appearance-heading">Appearance</h2>
        <p>Choose a theme or follow your Windows appearance.</p>
        <fieldset className="theme-options">
          <legend className="sr-only">Appearance preference</legend>
          {options.map(({ value, label, icon: Icon }) => (
            <label
              className={`theme-option ${preference === value ? 'selected' : ''}`}
              key={value}
            >
              <input
                type="radio"
                name="theme"
                value={value}
                checked={preference === value}
                onChange={(event) =>
                  setPreference(event.target.value as ThemePreference)
                }
              />
              <Icon size={23} strokeWidth={1.5} aria-hidden="true" />
              <span>{label}</span>
              <span className="radio-indicator" />
            </label>
          ))}
        </fieldset>
        <p className="helper-text">
          {preference === 'system'
            ? `Following your system’s ${resolvedTheme} theme.`
            : `${preference === 'light' ? 'Light' : 'Dark'} theme selected.`}{' '}
          Appearance is the only saved preference in this version.
        </p>
        {persistenceError && (
          <p role="alert" className="error-text">
            Your theme changed, but could not be saved. Local preference storage
            is unavailable.
          </p>
        )}
      </section>
      <section
        className="panel settings-section"
        aria-labelledby="storage-heading"
      >
        <div className="settings-title">
          <h2 id="storage-heading">Storage & backups</h2>
          <Badge>Not yet available</Badge>
        </div>
        <div className="settings-description">
          <Database size={20} aria-hidden="true" />
          <p>
            Database setup, backup locations, retention, and import preferences
            will be introduced in future steps. SQLite is not implemented.
          </p>
        </div>
      </section>
      <section
        className="panel settings-section"
        aria-labelledby="about-heading"
      >
        <div className="settings-title">
          <h2 id="about-heading">About ChatVault Desktop</h2>
          <Badge>v0.1.0 · Step 01</Badge>
        </div>
        <div className="about-brand">
          <img src="/brand.svg" alt="" width="48" height="48" />
          <div>
            <h3>A private place for your chat history.</h3>
            <p>Built for local archives, with you in control.</p>
          </div>
        </div>
        <div className="privacy-note">
          <ShieldCheck size={21} aria-hidden="true" />
          <p>
            No accounts, cloud services, external APIs, or analytics. This
            foundation does not read chat files. Future file access will require
            explicit selection and scoped permissions.
          </p>
        </div>
        <p className="disclaimer">
          ChatVault Desktop is an independent, unofficial product. It is not
          affiliated with, endorsed by, or sponsored by WhatsApp or Meta.
          WhatsApp is a trademark of its respective owner.
        </p>
        <BackendStatus />
      </section>
    </>
  );
}
