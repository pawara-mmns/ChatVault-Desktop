import { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import {
  ArrowRight,
  FileUp,
  LockKeyhole,
  LoaderCircle,
  CircleCheck,
} from 'lucide-react';
import { Badge, PageHeading } from '../../shared/components/Page';
import { Button, buttonVariants } from '../../shared/components/ui/button';
import {
  importService,
  desktopImportAvailable,
  importFailure,
  type ImportPreview,
  type SelectedImport,
  type ArchiveSummary,
  type DateOrder,
} from '../../services/imports';
import { formatBytes, type StorageFailure } from '../../services/storage';
import { useStorage } from '../storage/StorageProvider';

export function ImportChat() {
  const { overview, refresh } = useStorage();
  const [phase, setPhase] = useState<
    'idle' | 'selecting' | 'analyzing' | 'ready' | 'saving' | 'complete'
  >('idle');
  const [selection, setSelection] = useState<SelectedImport | null>(null);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [completed, setCompleted] = useState<ArchiveSummary | null>(null);
  const [title, setTitle] = useState('');
  const [error, setError] = useState<StorageFailure | null>(null);
  const pending = useRef(false);
  const token = useRef<string | null>(null);
  const mounted = useRef(true);
  const desktop = desktopImportAvailable();
  const busy =
    phase === 'selecting' || phase === 'analyzing' || phase === 'saving';
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      if (token.current)
        void importService.cancel(token.current).catch(() => undefined);
    };
  }, []);

  async function choose() {
    if (pending.current) return;
    pending.current = true;
    setError(null);
    setCompleted(null);
    setPhase('selecting');
    try {
      const selected = await importService.select();
      if (!selected) {
        if (mounted.current) setPhase('idle');
        return;
      }
      token.current = selected.token;
      if (!mounted.current) {
        await importService.cancel(selected.token);
        token.current = null;
        return;
      }
      setSelection(selected);
      if (selected.duplicateArchive) {
        setPhase('ready');
        return;
      }
      setPhase('analyzing');
      const result = await importService.analyze(selected.token, 'auto');
      if (mounted.current) {
        setPreview(result);
        setTitle(result.title);
        setPhase('ready');
      }
    } catch (failure) {
      token.current = null;
      if (mounted.current) {
        setError(importFailure(failure));
        setSelection(null);
        setPreview(null);
        setPhase('idle');
      }
    } finally {
      pending.current = false;
    }
  }
  async function changeDates(order: DateOrder) {
    if (pending.current || !token.current) return;
    pending.current = true;
    setError(null);
    setPhase('analyzing');
    try {
      const result = await importService.analyze(token.current, order);
      if (mounted.current) {
        setPreview(result);
        setPhase('ready');
      }
    } catch (failure) {
      token.current = null;
      if (mounted.current) {
        setError(importFailure(failure));
        setPreview(null);
        setSelection(null);
        setPhase('idle');
      }
    } finally {
      pending.current = false;
    }
  }
  async function cancel() {
    if (pending.current || !token.current) return;
    pending.current = true;
    setError(null);
    try {
      await importService.cancel(token.current);
      token.current = null;
      setSelection(null);
      setPreview(null);
      setPhase('idle');
    } catch (failure) {
      setError(importFailure(failure));
    } finally {
      pending.current = false;
    }
  }
  async function confirm() {
    if (
      pending.current ||
      !token.current ||
      !preview ||
      preview.dateOrderRequired
    )
      return;
    pending.current = true;
    setError(null);
    setPhase('saving');
    try {
      const result = await importService.confirm(token.current, title);
      token.current = null;
      await refresh();
      if (mounted.current) {
        setCompleted(result);
        setPreview(null);
        setSelection(null);
        setPhase('complete');
      }
    } catch (failure) {
      token.current = null;
      await refresh();
      if (mounted.current) {
        setError(importFailure(failure));
        setPreview(null);
        setSelection(null);
        setPhase('idle');
      }
    } finally {
      pending.current = false;
    }
  }
  return (
    <>
      <PageHeading
        eyebrow="BRING YOUR HISTORY"
        title="Import Chat"
        description="Preview a WhatsApp export, then save a structured archive on this computer."
      />
      <ol className="import-progress" aria-label="Import workflow">
        {[
          'Select file',
          'Analyze',
          'Preview',
          'Confirm import',
          'Complete',
        ].map((step, index) => (
          <li
            key={step}
            aria-current={
              (phase === 'idle' || phase === 'selecting'
                ? 0
                : phase === 'analyzing'
                  ? 1
                  : phase === 'ready'
                    ? 2
                    : phase === 'saving'
                      ? 3
                      : 4) === index
                ? 'step'
                : undefined
            }
          >
            <span>{index + 1}</span>
            {step}
          </li>
        ))}
      </ol>
      {!desktop && (
        <div className="notice" role="status">
          <p>
            Desktop runtime unavailable. Open ChatVault Desktop to select
            exports and save local archives.
          </p>
        </div>
      )}
      {desktop && overview?.vaultStatus !== 'ready' && (
        <div className="notice">
          <p>
            Your local vault must be ready before importing. Check Storage &
            Vault in Settings.
          </p>
        </div>
      )}
      {error && (
        <div className="notice import-error" role="alert">
          <p>{error.message}</p>
        </div>
      )}
      {busy && (
        <div className="import-status" role="status" aria-live="polite">
          <LoaderCircle
            className="import-spinner"
            size={18}
            aria-hidden="true"
          />
          {phase === 'selecting'
            ? 'Selecting and validating the source file…'
            : phase === 'analyzing'
              ? 'Analyzing messages and validating archive contents…'
              : 'Saving files and committing the local database…'}
        </div>
      )}
      {!selection && !completed && (
        <section
          className="panel information-card"
          aria-labelledby="select-heading"
        >
          <div className="card-icon">
            <FileUp size={23} aria-hidden="true" />
          </div>
          <h2 id="select-heading">Choose a chat export</h2>
          <p>
            Select a readable .txt export or a .zip containing one conversation
            and optional media. UTF-8 and BOM-marked UTF-16 are supported. Files
            are validated and processed by the local Rust backend.
          </p>
          <Button
            onClick={() => void choose()}
            disabled={busy || !desktop || overview?.vaultStatus !== 'ready'}
          >
            <FileUp size={16} aria-hidden="true" />
            Choose WhatsApp export
          </Button>
          <p className="helper-text">
            Your original source file is never changed. Nothing is imported
            until you confirm the preview.
          </p>
        </section>
      )}
      {selection && (
        <section
          className="panel information-card"
          aria-label="Selected export"
        >
          <div className="settings-title">
            <h2 className="storage-path">{selection.filename}</h2>
            <Badge>
              {selection.sourceFormat.toUpperCase()} ·{' '}
              {formatBytes(selection.sizeBytes)}
            </Badge>
          </div>
          {selection.duplicateArchive && (
            <div className="notice" role="alert">
              <p>
                This identical file is already imported. No duplicate will be
                created. Cancel this selection and open Archives to review the
                existing record.
              </p>
            </div>
          )}
          {preview && (
            <>
              <dl className="import-counts">
                <div>
                  <dt>Records</dt>
                  <dd>{preview.messageCount}</dd>
                </div>
                <div>
                  <dt>Participants</dt>
                  <dd>{preview.participantCount}</dd>
                </div>
                <div>
                  <dt>Media files</dt>
                  <dd>{preview.mediaCount}</dd>
                </div>
                <div>
                  <dt>Missing references</dt>
                  <dd>{preview.missingAttachmentCount}</dd>
                </div>
              </dl>
              <label className="form-field">
                Conversation title
                <input
                  value={title}
                  maxLength={200}
                  onChange={(event) => setTitle(event.target.value)}
                  disabled={busy}
                />
              </label>
              <label className="form-field">
                Date format
                <select
                  aria-label="Date format"
                  value={preview.dateOrder}
                  onChange={(event) =>
                    void changeDates(event.target.value as DateOrder)
                  }
                  disabled={busy}
                >
                  <option value="auto">Detect unambiguous dates only</option>
                  <option value="day-first">Day / month / year</option>
                  <option value="month-first">Month / day / year</option>
                </select>
              </label>
              {preview.dateOrderRequired && (
                <p className="error-text" role="alert">
                  Ambiguous dates detected. Select day-first or month-first
                  before importing.
                </p>
              )}
              <p className="helper-text">
                {preview.encoding} · {preview.attachmentCount} media references.
                Export times have no known timezone; ChatVault will not invent
                UTC timestamps.
              </p>
              <p className="helper-text">
                Participants:{' '}
                {preview.participants.join(', ') || 'No identifiable sender'}
                {preview.participantCount > preview.participants.length
                  ? ' (first 20 shown)'
                  : ''}
              </p>
              {preview.diagnostics.length > 0 && (
                <div className="parser-warnings">
                  <h3>Parser diagnostics</h3>
                  <ul>
                    {preview.diagnostics.map((item) => (
                      <li key={item.code}>
                        {item.description}{' '}
                        <span>
                          ({item.occurrenceCount}
                          {item.lineNumber
                            ? `, first at line ${item.lineNumber}`
                            : ''}
                          )
                        </span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}
              <div className="preview-records">
                <h3>Message preview</h3>
                <p className="helper-text">
                  First five records. Long samples are shortened; full parsed
                  text and the original export are retained.
                </p>
                {preview.samples.map((message, index) => (
                  <article key={index}>
                    <div>
                      <strong>
                        {message.sender ??
                          (message.classification === 'system'
                            ? 'System'
                            : 'Unknown sender')}
                      </strong>
                      <span>
                        {message.originalTimestamp ?? 'Unparsed record'}
                      </span>
                    </div>
                    <p>{message.text}</p>
                    {message.localTimestamp && (
                      <small>
                        Local time: {message.localTimestamp} · timezone unknown
                      </small>
                    )}
                  </article>
                ))}
              </div>
            </>
          )}
          <div className="storage-actions">
            {preview && (
              <Button
                onClick={() => void confirm()}
                disabled={busy || preview.dateOrderRequired || !title.trim()}
              >
                Import to local vault
                <ArrowRight size={15} aria-hidden="true" />
              </Button>
            )}
            <Button
              variant="outline"
              onClick={() => void cancel()}
              disabled={busy}
            >
              Cancel import
            </Button>
            {selection.duplicateArchive && (
              <Link
                className={buttonVariants({ variant: 'outline' })}
                to="/archives"
              >
                Open Archives
              </Link>
            )}
          </div>
        </section>
      )}
      {completed && (
        <section
          className="panel information-card"
          aria-labelledby="complete-heading"
        >
          <CircleCheck size={30} className="text-primary" aria-hidden="true" />
          <h2 id="complete-heading">Archive saved locally</h2>
          <p className="storage-path">
            {completed.title} · {completed.messageCount} records ·{' '}
            {completed.mediaCount} media files ·{' '}
            {formatBytes(completed.totalBytes)}
          </p>
          <p>
            Files and structured conversation data have been persisted in your
            vault and local SQLite database.
          </p>
          <div className="storage-actions">
            <Link to="/archives" className={buttonVariants()}>
              Open Archives
              <ArrowRight size={15} aria-hidden="true" />
            </Link>
            <Button
              variant="outline"
              onClick={() => {
                setCompleted(null);
                setPhase('idle');
              }}
            >
              Import another export
            </Button>
          </div>
        </section>
      )}
      <section className="panel information-card import-boundaries">
        <div className="settings-title">
          <h2>
            <LockKeyhole size={17} aria-hidden="true" /> Encrypted native
            backups
          </h2>
          <Badge>NOT SUPPORTED</Badge>
        </div>
        <p>
          .crypt14/.crypt15 databases are not readable chat exports. ChatVault
          does not bypass encryption or recover keys. Vault encryption is not
          yet implemented; imported originals, text, and media are stored
          unencrypted on this PC.
        </p>
      </section>
    </>
  );
}
