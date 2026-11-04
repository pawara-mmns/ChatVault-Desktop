import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Button } from '../../shared/components/ui/button';
import { useBackups } from './BackupProvider';
import {
  activePhase,
  backupFailure,
  categoryLabels,
  phaseLabel,
  sourceLabel,
  type BackupFailure,
  type BackupFile,
  type History,
  type Snapshot,
} from '../../services/backups';
import { formatBytes } from '../../services/storage';
export function BackupError({ error }: { error: BackupFailure | null }) {
  return error ? (
    <div role="alert" className="backup-error">
      <strong>{error.message}</strong>
      <small>
        {error.code}
        {error.diagnostic ? ` · ${error.diagnostic}` : ''}
      </small>
    </div>
  ) : null;
}
export function BackupHistory({ compact = false }: { compact?: boolean }) {
  const { service, progress, setProgress } = useBackups();
  const [history, setHistory] = useState<History | null>(null);
  const [error, setError] = useState<BackupFailure | null>(null);
  const [offset, setOffset] = useState(0);
  const [selected, setSelected] = useState<Snapshot | null>(null);
  const [files, setFiles] = useState<BackupFile[]>([]);
  const [fileOffset, setFileOffset] = useState(0);
  const [busy, setBusy] = useState(false);
  const load = useCallback(async () => {
    if (!service.available()) return;
    try {
      const data = await service.history(offset);
      if (!data || !Array.isArray(data.items)) throw Error();
      setHistory(data);
      setSelected((old) =>
        old ? (data.items.find((item) => item.id === old.id) ?? old) : null,
      );
      setError(null);
    } catch (e) {
      setError(backupFailure(e));
    }
  }, [service, offset]);
  useEffect(() => {
    const timer = setTimeout(() => void load(), 0);
    return () => clearTimeout(timer);
  }, [load, progress.phase]);
  useEffect(() => {
    let mounted = true;
    if (selected)
      service
        .files(selected.id, fileOffset)
        .then((v) => {
          if (mounted) setFiles(v);
        })
        .catch((e) => {
          if (mounted) setError(backupFailure(e));
        });
    return () => {
      mounted = false;
    };
  }, [service, selected, fileOffset]);
  async function action(
    kind: 'verify' | 'cleanup' | 'open',
    snapshot: Snapshot,
  ) {
    setBusy(true);
    setError(null);
    try {
      if (kind === 'verify') setProgress(await service.verify(snapshot.id));
      else if (kind === 'cleanup') await service.cleanup(snapshot.id);
      else await service.open(snapshot.id);
      await load();
    } catch (e) {
      setError(backupFailure(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section
      className="panel backup-history"
      aria-labelledby={
        compact ? 'recent-snapshot-heading' : 'backup-history-heading'
      }
    >
      <div className="panel-heading">
        <h2 id={compact ? 'recent-snapshot-heading' : 'backup-history-heading'}>
          {compact ? 'Recent backups' : 'Backup history'}
        </h2>
        <Link to="/backups">
          {compact ? 'Create / view backups' : 'New backup'}
        </Link>
      </div>
      <BackupError error={error} />
      {!service.available() ? (
        <p>
          Backup history requires the desktop runtime. No snapshots are
          simulated.
        </p>
      ) : !history ? (
        <p>Loading snapshot history…</p>
      ) : (
        <>
          <p>
            {history.completedCount} completed snapshots ·{' '}
            {formatBytes(history.completedBytes)} verified file bytes in
            completed snapshots. Failed and interrupted jobs are excluded.
          </p>
          {history.items.length === 0 ? (
            <p>
              No backup snapshots yet. Connect your phone and choose files to
              create one.
            </p>
          ) : (
            <ul className="backup-snapshot-list">
              {history.items.slice(0, compact ? 3 : 20).map((s) => (
                <li key={s.id}>
                  <div>
                    <strong>
                      {sourceLabel(s.sourceApp)} · {phaseLabel(s.phase)}
                    </strong>
                    <span>
                      {s.deviceName ?? 'Device name unavailable'} ·{' '}
                      {new Date(s.createdAt).toLocaleString()}
                    </span>
                    <small>
                      {s.successfulCount}/{s.fileCount} files ·{' '}
                      {formatBytes(s.totalBytes)} transferred · Local integrity:{' '}
                      {s.localIntegrity}
                    </small>
                  </div>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      setSelected(s);
                      setFileOffset(0);
                    }}
                  >
                    Details
                  </Button>
                </li>
              ))}
            </ul>
          )}
          {!compact && (
            <div className="backup-actions">
              <Button
                variant="outline"
                disabled={offset === 0}
                onClick={() => setOffset((v) => Math.max(0, v - 20))}
              >
                Previous snapshots
              </Button>
              <Button
                variant="outline"
                disabled={offset + 20 >= history.total}
                onClick={() => setOffset((v) => v + 20)}
              >
                Next snapshots
              </Button>
              <Button variant="ghost" onClick={() => void load()}>
                Refresh history
              </Button>
            </div>
          )}
        </>
      )}
      {selected && (
        <section
          className="backup-snapshot-details"
          aria-label="Snapshot details"
        >
          <div className="panel-heading">
            <h3>Snapshot details</h3>
            <Button size="sm" variant="ghost" onClick={() => setSelected(null)}>
              Close details
            </Button>
          </div>
          <p>
            {sourceLabel(selected.sourceApp)} · {phaseLabel(selected.phase)} ·{' '}
            {selected.deviceName ?? 'Unavailable'}
          </p>
          <p className="storage-path">{selected.destination}</p>
          <p>
            Local read-back: {selected.localIntegrity}. Verified bytes:{' '}
            {formatBytes(selected.verifiedBytes)}. Last local check:{' '}
            {selected.localVerifiedAt
              ? new Date(selected.localVerifiedAt).toLocaleString()
              : 'Unavailable'}
            .
          </p>
          <p>
            Warnings: {selected.warnings.join(', ') || 'None'}. Encrypted
            databases remain encrypted. No restore-to-phone or atomic WhatsApp
            snapshot is promised.
          </p>
          <div className="backup-actions">
            <Button
              variant="outline"
              disabled={
                busy || activePhase(progress.phase) || !selected.manifestPath
              }
              onClick={() => void action('verify', selected)}
            >
              Re-verify local integrity
            </Button>
            <Button
              variant="outline"
              disabled={busy || !selected.manifestPath}
              onClick={() => void action('open', selected)}
            >
              Open backup folder
            </Button>
            {['failed', 'cancelled', 'interrupted'].includes(
              selected.phase,
            ) && (
              <Button
                variant="outline"
                disabled={busy || activePhase(progress.phase)}
                onClick={() => void action('cleanup', selected)}
              >
                Clean owned staging only
              </Button>
            )}
          </div>
          <p>
            Cleanup removes only this incomplete job’s owned temporary
            directory. Published snapshots and phone files are preserved. Retry
            by creating a fresh backup after rescanning.
          </p>
          <ul className="backup-file-list">
            {files.map((f) => (
              <li key={f.id}>
                <strong>{f.originalName}</strong>
                <span>
                  {categoryLabels[f.category]} · {f.state} ·{' '}
                  {formatBytes(f.actualBytes)}
                </span>
                <small>
                  Local: {f.localVerification} · Source re-read:{' '}
                  {f.sourceVerification} · Expected size: {f.sizeValidation}
                </small>
                <code>{f.sha256 ?? f.errorCode ?? 'Not transferred'}</code>
                <small>{f.localRelative}</small>
              </li>
            ))}
          </ul>
          <div className="backup-actions">
            <Button
              variant="outline"
              disabled={fileOffset === 0}
              onClick={() => setFileOffset((v) => Math.max(0, v - 50))}
            >
              Previous files
            </Button>
            <Button
              variant="outline"
              disabled={files.length < 50}
              onClick={() => setFileOffset((v) => v + 50)}
            >
              Next files
            </Button>
          </div>
        </section>
      )}
    </section>
  );
}
