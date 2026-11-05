import { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { Button } from '../../shared/components/ui/button';
import {
  nativeService,
  nativeError,
  type NativeService,
  type NativeInspection,
  type NativePreview,
  type NativeProgress,
} from '../../services/native';
import { formatBytes } from '../../services/storage';
export function NativeInspector({
  snapshotId,
  service = nativeService,
  onImported,
}: {
  snapshotId: string;
  service?: NativeService;
  onImported?: () => void;
}) {
  const [inspection, setInspection] = useState<NativeInspection | null>(null);
  const [preview, setPreview] = useState<NativePreview | null>(null);
  const [selected, setSelected] = useState('');
  const [key, setKey] = useState('');
  const [processConsent, setProcessConsent] = useState(false);
  const [plaintextConsent, setPlaintextConsent] = useState(false);
  const [busy, setBusy] = useState(false);
  const [opened, setOpened] = useState(false);
  const [error, setError] = useState<{ code: string; message: string } | null>(
    null,
  );
  const [progress, setProgress] = useState<NativeProgress | null>(null);
  const generation = useRef(0);
  const candidate = inspection?.candidates.find((c) => c.fileId === selected);
  useEffect(
    () => () => {
      generation.current++;
      void service.cancel().catch(() => {});
    },
    [service, snapshotId],
  );
  useEffect(() => {
    if (!opened) return;
    let mounted = true;
    let pending = false;
    const timer = window.setInterval(() => {
      if (pending) return;
      pending = true;
      void service
        .progress()
        .then((p) => {
          if (mounted) setProgress(p);
        })
        .catch(() => {})
        .finally(() => {
          pending = false;
        });
    }, 500);
    return () => {
      mounted = false;
      clearInterval(timer);
    };
  }, [service, opened]);
  async function inspect() {
    const id = ++generation.current;
    setOpened(true);
    setBusy(true);
    setError(null);
    try {
      const result = await service.inspect(snapshotId);
      if (generation.current !== id) return;
      setInspection(result);
      setSelected(
        result.candidates.find(
          (c) => c.capability === 'SUPPORTED_AND_KEY_REQUIRED',
        )?.fileId ?? '',
      );
    } catch (e) {
      if (generation.current === id) setError(nativeError(e));
    } finally {
      if (generation.current === id) setBusy(false);
    }
  }
  async function verify() {
    if (!candidate) return;
    const secret = key;
    setPlaintextConsent(false);
    setKey('');
    setBusy(true);
    setError(null);
    const id = ++generation.current;
    try {
      const result =
        candidate.format === 'crypt14'
          ? await service.keyFile(snapshotId, selected, processConsent)
          : await service.verify(snapshotId, selected, secret, processConsent);
      if (generation.current === id) setPreview(result);
    } catch (e) {
      if (generation.current === id) setError(nativeError(e));
    } finally {
      if (generation.current === id) setBusy(false);
    }
  }
  async function save() {
    if (!preview) return;
    const id = ++generation.current;
    setBusy(true);
    setError(null);
    try {
      const result = await service.import(preview.token, plaintextConsent);
      if (generation.current === id) {
        setPreview(result);
        if (result.archiveId) onImported?.();
      }
    } catch (e) {
      if (generation.current === id) {
        setError(nativeError(e));
        setPreview(null);
      }
    } finally {
      if (generation.current === id) setBusy(false);
    }
  }
  async function cancel() {
    setKey('');
    try {
      await service.cancel();
      if (!busy) {
        setPreview(null);
        setProcessConsent(false);
        setPlaintextConsent(false);
      }
    } catch (e) {
      setError(nativeError(e));
    }
  }
  return (
    <section className="native-inspector" aria-label="Native backup inspector">
      <h3>Native backup reader</h3>
      <p>
        Inspect saved encrypted files without a phone connection. No key is
        extracted or guessed.
      </p>
      {!inspection && (
        <Button
          variant="outline"
          disabled={busy}
          onClick={() => void inspect()}
        >
          Inspect Backup
        </Button>
      )}
      {error && (
        <p role="alert">
          {error.message} <small>{error.code}</small>
        </p>
      )}
      {busy && (
        <div role="status">
          <p>
            {progress?.phase.replaceAll('_', ' ') ?? 'Processing locally'} -{' '}
            {formatBytes(progress?.bytes ?? 0)} checked;{' '}
            {progress?.messages ?? 0} messages processed.
          </p>
          <div
            className="scan-indeterminate"
            role="progressbar"
            aria-label="Native reader operation"
          >
            <span />
          </div>
        </div>
      )}
      {inspection && (
        <>
          <p>
            <strong>{inspection.capability.replaceAll('_', ' ')}</strong> -{' '}
            {inspection.sourceApp === 'business'
              ? 'WhatsApp Business'
              : 'WhatsApp'}{' '}
            - {new Date(inspection.createdAt).toLocaleString()}
          </p>
          <p>
            Local integrity: {inspection.localIntegrity ?? 'not supplied'}.{' '}
            {inspection.totalFiles} stored files; {inspection.missingFiles}{' '}
            unavailable files. Inventory{' '}
            {inspection.inventoryComplete
              ? 'complete within scanned scope'
              : 'incomplete'}
            . {inspection.warnings.join(', ')}
          </p>
          <p>
            Media inventory:{' '}
            {inspection.media
              .map(
                ([category, count, bytes]) =>
                  `${category}: ${count} (${formatBytes(bytes)})`,
              )
              .join('; ') || 'No media inventoried'}
            .
          </p>
          <fieldset disabled={busy || !!preview}>
            <legend>
              Database candidates (header and stored hash checked)
            </legend>
            {inspection.candidates.map((c) => (
              <label key={c.fileId}>
                <input
                  type="radio"
                  name="native-database"
                  checked={selected === c.fileId}
                  disabled={c.capability !== 'SUPPORTED_AND_KEY_REQUIRED'}
                  onChange={() => {
                    setSelected(c.fileId);
                    setKey('');
                  }}
                />
                {c.filename} - {formatBytes(c.bytes)} - {c.format} -{' '}
                {c.capability.replaceAll('_', ' ')}
                {c.duplicateOf ? ' - duplicate stored hash' : ''}
                {c.issue ? ` - ${c.issue}` : ''}
              </label>
            ))}
          </fieldset>
          {inspection.candidates.length === 0 && (
            <p>
              No message database candidate was found. Media alone does not
              contain readable conversations.
            </p>
          )}
          <p>
            This encrypted backup is safely stored, but its messages cannot
            currently be opened without a matching key. An extension alone does
            not establish readability. Passwords, passkeys and unsupported
            formats are not tried.
          </p>
          {inspection.existingArchive && (
            <Link
              to={`/chats?archive=${encodeURIComponent(inspection.existingArchive)}`}
            >
              Read Chats - existing native archive
            </Link>
          )}
          {candidate?.capability === 'SUPPORTED_AND_KEY_REQUIRED' &&
            !preview &&
            !inspection.existingArchive && (
              <>
                <h4>Unlock &amp; Read Chats</h4>
                <p>
                  Required: {candidate.keyRequirement}. Processing stays on this
                  computer; keys are not saved or logged.
                </p>
                {candidate.format === 'crypt15' && (
                  <label>
                    Matching crypt15 root key
                    <input
                      type="password"
                      autoComplete="off"
                      maxLength={64}
                      value={key}
                      onChange={(e) => setKey(e.target.value)}
                      disabled={busy}
                    />
                  </label>
                )}
                <label>
                  <input
                    type="checkbox"
                    checked={processConsent}
                    onChange={(e) => setProcessConsent(e.target.checked)}
                    disabled={busy}
                  />
                  I consent to local plaintext processing in restricted
                  temporary staging. The vault is unencrypted. Temporary
                  plaintext may remain after a crash until recovery; secure SSD
                  erasure is not promised.
                </label>
                <Button
                  disabled={
                    busy ||
                    !processConsent ||
                    (candidate.format === 'crypt15' &&
                      !/^[a-fA-F0-9]{64}$/.test(key))
                  }
                  onClick={() => void verify()}
                >
                  {candidate.format === 'crypt14'
                    ? 'Select matching key file & verify compatibility'
                    : 'Verify compatibility'}
                </Button>
              </>
            )}
        </>
      )}
      {preview && (
        <>
          <p>
            SUPPORTED AND KEY AVAILABLE - authenticated input and{' '}
            {preview.schema} schema validated. Parser {preview.parserVersion}.
          </p>
          <p>
            {preview.counts.conversations} conversations;{' '}
            {preview.counts.messages} messages; {preview.counts.mediaMatched}{' '}
            media matched; {preview.counts.mediaMissing} media missing or
            uncertain; {preview.counts.unsupported} unsupported records.
          </p>
          {preview.archiveId ? (
            <>
              <p>Readable messages saved successfully.</p>
              <Link
                to={`/chats?archive=${encodeURIComponent(preview.archiveId)}`}
              >
                Read Chats
              </Link>
            </>
          ) : (
            <>
              <p role="status">
                The ChatVault database, search index and vault are unencrypted.
                Saving will persist readable private messages on this computer.
                The encrypted snapshot remains unchanged.
              </p>
              <label>
                <input
                  type="checkbox"
                  checked={plaintextConsent}
                  onChange={(e) => setPlaintextConsent(e.target.checked)}
                  disabled={busy}
                />
                I explicitly consent to persisting these readable conversations
                and search records in unencrypted local storage.
              </label>
              <Button
                disabled={busy || !plaintextConsent}
                onClick={() => void save()}
              >
                Import readable chats
              </Button>
            </>
          )}
        </>
      )}
      {(busy || (preview && !preview.archiveId)) && (
        <Button
          variant="outline"
          onClick={() => void cancel()}
          disabled={!!progress?.cancelRequested}
        >
          Cancel native processing / discard preview
        </Button>
      )}
      {progress?.cancelRequested && busy && (
        <p>
          Cancellation requested. Waiting for the current cryptographic or
          SQLite operation to reach a safe boundary.
        </p>
      )}
      <p>
        <Link to="/import">
          Use the supported manual TXT/ZIP chat export importer instead
        </Link>
        . Closing this panel discards the pending preview; readable imports are
        read-only and never merged with earlier archives.
      </p>
    </section>
  );
}
