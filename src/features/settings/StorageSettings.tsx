import { Database, FolderOpen, RotateCw } from 'lucide-react';
import { Badge } from '../../shared/components/Page';
import { Button } from '../../shared/components/ui/button';
import { useStorage } from '../storage/StorageProvider';
import { StorageFeedback } from '../storage/StorageFeedback';
import { formatBytes } from '../../services/storage';

export function StorageSettings() {
  const {
    overview,
    loading,
    error,
    success,
    initialize,
    changeLocation,
    refresh,
  } = useStorage();
  const browser = error?.code === 'runtime_unavailable';
  return (
    <section
      className="panel settings-section"
      aria-labelledby="storage-heading"
      aria-busy={loading}
    >
      <div className="settings-title">
        <h2 id="storage-heading">Storage & Vault</h2>
        <Badge>
          {loading
            ? 'Checking…'
            : overview?.vaultStatus === 'ready'
              ? 'Local storage ready'
              : 'Unavailable'}
        </Badge>
      </div>
      <div className="settings-description">
        <Database size={20} aria-hidden="true" />
        <p>
          Your database stays in application-local storage. ChatVault files stay
          in the vault you choose.
        </p>
      </div>
      <dl className="storage-details">
        <div>
          <dt>Current vault location</dt>
          <dd className="storage-path">
            {overview?.vaultPath ??
              (loading ? 'Checking local storage…' : 'Unavailable')}
          </dd>
        </div>
        <div>
          <dt>Vault usage</dt>
          <dd>{overview ? formatBytes(overview.vaultBytes) : '—'}</dd>
        </div>
        <div>
          <dt>Database</dt>
          <dd>
            {overview
              ? `Connected · schema ${overview.schemaVersion}`
              : loading
                ? 'Initializing…'
                : 'Unavailable'}
          </dd>
        </div>
        {overview && (
          <div>
            <dt>Database location</dt>
            <dd className="storage-path">{overview.databasePath}</dd>
          </div>
        )}
      </dl>
      <StorageFeedback
        error={error ?? overview?.vaultError ?? null}
        success={success}
      />
      <div className="storage-actions">
        <Button
          variant="outline"
          onClick={() => void changeLocation()}
          disabled={loading || !overview || overview.vaultStatus !== 'ready'}
          aria-describedby="vault-change-note"
        >
          <FolderOpen size={16} aria-hidden="true" />
          Change Location
        </Button>
        <Button
          variant="ghost"
          onClick={() =>
            void (overview?.vaultStatus === 'ready' ? refresh() : initialize())
          }
          disabled={loading || browser}
        >
          <RotateCw size={15} aria-hidden="true" />
          {overview?.vaultStatus === 'ready'
            ? 'Refresh storage'
            : 'Retry initialization'}
        </Button>
      </div>
      <p id="vault-change-note" className="helper-text">
        Choose a dedicated, empty local folder. Location changes are blocked
        when the current vault contains files or archive records. Migration is
        not supported yet.
      </p>
      <p className="helper-text">
        No files are moved or deleted when changing location. Vault files and
        the database are not encrypted by ChatVault in this step.
      </p>
    </section>
  );
}
