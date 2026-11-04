import { Link } from 'react-router-dom';
import {
  Archive,
  ArrowRight,
  ArrowUpFromLine,
  Database,
  HardDrive,
  LockKeyhole,
  ShieldCheck,
  Smartphone,
} from 'lucide-react';
import { Badge, EmptyState, PageHeading } from '../../shared/components/Page';
import { buttonVariants } from '../../shared/components/ui/button';
import { Button } from '../../shared/components/ui/button';
import { useStorage } from '../storage/StorageProvider';
import { StorageFeedback } from '../storage/StorageFeedback';
import { formatBytes } from '../../services/storage';
import { useDevices } from '../devices/DeviceProvider';
import { accessLabel } from '../../services/devices';
import { RecentArchives } from '../archives/RecentArchives';
export function Dashboard() {
  const { overview, loading, error, initialize, refresh } = useStorage();
  const devices = useDevices();
  const firstDevice = devices.list.devices[0];
  return (
    <>
      <PageHeading
        eyebrow="YOUR PERSONAL ARCHIVE"
        title="A home for your conversations."
        description="Keep your chat history close. Keep control of your data."
        action={
          <Link to="/import" className={buttonVariants()}>
            <ArrowUpFromLine size={16} aria-hidden="true" />
            Import Chat
          </Link>
        }
      />
      <section className="welcome-panel" aria-labelledby="welcome-heading">
        <div className="welcome-copy">
          <Badge accent>PRIVATE BY DESIGN</Badge>
          <h2 id="welcome-heading">Your memories, on your terms.</h2>
          <p>
            A quiet, private space for the conversations that matter.
            <br className="desktop-break" /> Start by learning how to bring your
            chat exports here.
          </p>
          <Link to="/import" className="text-link">
            Explore the import workflow{' '}
            <ArrowRight size={15} aria-hidden="true" />
          </Link>
        </div>
        <div className="vault-art" aria-hidden="true">
          <div className="vault-outline">
            <ShieldCheck size={58} strokeWidth={1} />
          </div>
          <span>
            <LockKeyhole size={11} /> STORED WITH YOU
          </span>
        </div>
      </section>
      <section className="summary-grid" aria-label="Backup summary">
        <article className="summary-card">
          <div className="summary-title">
            Backup snapshots
            <HardDrive size={18} aria-hidden="true" />
          </div>
          <strong className="summary-value">
            {overview?.backupSnapshots ?? '—'}
          </strong>
          <p>
            {overview
              ? `${overview.backupSnapshots === 0 ? 'No backups created' : 'Saved in the local database'}`
              : loading
                ? 'Loading local database…'
                : 'Database unavailable'}
          </p>
        </article>
        <article className="summary-card">
          <div className="summary-title">
            Chat archives
            <Archive size={18} aria-hidden="true" />
          </div>
          <strong className="summary-value">
            {overview?.chatArchives ?? '—'}
          </strong>
          <p>
            {overview
              ? overview.chatArchives === 0
                ? 'No chats imported'
                : 'Saved in the local database'
              : loading
                ? 'Loading local database…'
                : 'Database unavailable'}
          </p>
        </article>
        <article className="summary-card">
          <div className="summary-title">
            Storage used
            <Database size={18} aria-hidden="true" />
          </div>
          <strong className="summary-value" aria-label="Vault storage used">
            {overview ? formatBytes(overview.vaultBytes) : '—'}
          </strong>
          <p>
            {loading
              ? 'Checking vault files…'
              : overview?.vaultStatus === 'ready'
                ? 'Actual files in your local vault'
                : 'Vault unavailable'}
          </p>
        </article>
      </section>
      <section
        className="panel dashboard-storage"
        aria-label="Local storage status"
        aria-busy={loading}
      >
        <div>
          <strong>
            {loading
              ? 'Checking local storage…'
              : overview
                ? `Database connected · schema ${overview.schemaVersion}`
                : 'Local storage unavailable'}
          </strong>
          <p className="storage-path">
            {overview?.vaultPath ??
              'Open the desktop app to access your local database and vault.'}
          </p>
          <StorageFeedback
            error={error ?? overview?.vaultError ?? null}
            success={null}
          />
        </div>
        <Button
          size="sm"
          variant="outline"
          disabled={loading || error?.code === 'runtime_unavailable'}
          onClick={() =>
            void (overview?.vaultStatus === 'ready' ? refresh() : initialize())
          }
        >
          <Database size={14} aria-hidden="true" />
          {overview?.vaultStatus === 'ready' ? 'Refresh' : 'Retry storage'}
        </Button>
      </section>
      <div className="dashboard-grid">
        <section
          className="panel recent-panel"
          aria-labelledby="recent-heading"
        >
          <div className="panel-heading">
            <h2 id="recent-heading">Recent backups</h2>
            <Badge>
              {overview
                ? `${overview.backupSnapshots} snapshots`
                : 'Not loaded'}
            </Badge>
          </div>
          <EmptyState
            icon={HardDrive}
            title={
              overview?.backupSnapshots === 0
                ? 'Your story starts here'
                : 'Backup history is not available yet'
            }
            description={
              overview?.backupSnapshots === 0
                ? 'Once backups are available, your snapshots will appear here. Nothing has been backed up yet.'
                : 'Backup creation and history browsing are planned for a later step. Local database counts appear above when available.'
            }
          >
            <Link
              to="/import"
              className={buttonVariants({ variant: 'outline', size: 'sm' })}
            >
              See how it works
              <ArrowRight size={14} aria-hidden="true" />
            </Link>
          </EmptyState>
        </section>
        <section
          className="panel device-panel"
          aria-labelledby="device-heading"
        >
          <div className="panel-heading">
            <h2 id="device-heading">Your device</h2>
            <span className="status-dot" />
          </div>
          <div className="device-illustration">
            <Smartphone size={46} strokeWidth={1.2} aria-hidden="true" />
            {!firstDevice && <span className="device-cross">×</span>}
          </div>
          <h3>
            {devices.busy
              ? 'Scanning devices…'
              : firstDevice
                ? (firstDevice.name ?? 'Portable device detected')
                : devices.available && !devices.list.stale
                  ? 'No device connected'
                  : 'Live device state unavailable'}
          </h3>
          <p>
            {firstDevice
              ? accessLabel[firstDevice.access]
              : 'Connect and unlock your Android phone, choose File Transfer, then refresh in Devices.'}
          </p>
          <Link to="/devices" className="text-link">
            Connection guide <ArrowRight size={14} aria-hidden="true" />
          </Link>
          <div className="device-footnote">
            {devices.available
              ? 'Live WPD detection · checks every 15 seconds while visible'
              : 'USB detection requires the Windows desktop app'}
          </div>
        </section>
      </div>
      <RecentArchives />
      <div className="privacy-line">
        <ShieldCheck size={17} aria-hidden="true" />
        <p>
          <strong>A little peace of mind.</strong> Your chats will never need to
          leave your computer. No uploads, no tracking, no account required.
        </p>
      </div>
    </>
  );
}
